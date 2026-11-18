//! Speaker labels are anonymous. A reference recording is required to assign
//! the student role; neither turn order nor talk time is used to guess it.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Segment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub speaker: Option<i32>,
    pub language: String,
    pub text: String,
    #[serde(default)]
    pub exclusion_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Evidence {
    pub diarization_applied: bool,
    pub language_detection_applied: bool,
    pub student_identified: bool,
    pub speaker_count: usize,
    pub selected_speaker: Option<i32>,
    pub speaker_similarity: Option<f32>,
    pub recording_seconds: f64,
    pub student_seconds: f64,
    pub overlap_excluded_seconds: f64,
    pub untranscribed_student_seconds: f64,
    pub detected_languages: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(any(feature = "native-asr", test))]
#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub start: f64,
    pub end: f64,
    pub speaker: i32,
}

/// Remove every interval occupied by a different speaker, then merge same-role
/// intervals. Unknown/overlapping material never gets assigned to the student.
#[cfg(any(feature = "native-asr", test))]
pub fn exclusive_intervals(segments: &[Interval], speaker: i32, duration: f64) -> Vec<Interval> {
    let valid = segments
        .iter()
        .filter(|s| s.start.is_finite() && s.end.is_finite())
        .filter_map(|s| {
            let start = s.start.max(0.0);
            let end = s.end.min(duration);
            (end > start).then_some(Interval {
                start,
                end,
                speaker: s.speaker,
            })
        })
        .collect::<Vec<_>>();
    let mut exclusive = Vec::new();
    for segment in valid.iter().filter(|s| s.speaker == speaker) {
        let mut parts = vec![*segment];
        for other in valid.iter().filter(|s| s.speaker != speaker) {
            parts = parts
                .into_iter()
                .flat_map(|part| {
                    if other.end <= part.start || other.start >= part.end {
                        return vec![part];
                    }
                    let mut remainder = Vec::new();
                    if other.start > part.start {
                        remainder.push(Interval {
                            end: other.start,
                            ..part
                        });
                    }
                    if other.end < part.end {
                        remainder.push(Interval {
                            start: other.end,
                            ..part
                        });
                    }
                    remainder
                })
                .collect();
        }
        exclusive.extend(parts);
    }
    exclusive.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut merged: Vec<Interval> = Vec::new();
    for interval in exclusive {
        if let Some(last) = merged.last_mut()
            && interval.start <= last.end
        {
            last.end = last.end.max(interval.end);
        } else {
            merged.push(interval);
        }
    }
    merged
}

#[cfg(any(feature = "native-asr", test))]
pub fn cosine(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() || a.iter().chain(b).any(|v| !v.is_finite()) {
        return None;
    }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum();
    let denominator = (norm_a * norm_b).sqrt();
    (denominator > 0.0).then(|| (dot / denominator).clamp(-1.0, 1.0) as f32)
}

#[cfg(any(feature = "native-asr", test))]
pub fn select_speaker(scores: &[(i32, f32)], threshold: f32, margin: f32) -> Option<(i32, f32)> {
    let mut ranked = scores
        .iter()
        .copied()
        .filter(|(_, s)| s.is_finite())
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best = *ranked.first()?;
    if best.1 < threshold
        || ranked
            .get(1)
            .is_some_and(|runner| best.1 - runner.1 < margin)
    {
        return None;
    }
    Some(best)
}

/// Approximate code-switch windows. Prefer energy gaps, but cap windows at four
/// seconds. This is not word-level language alignment or a trained speech VAD.
#[cfg(any(feature = "native-asr", test))]
pub fn language_windows(samples: &[f32]) -> Vec<(usize, usize)> {
    const RATE: usize = crate::audio::SAMPLE_RATE;
    let mut windows = Vec::new();
    let mut start = 0;
    while start < samples.len() {
        let limit = (start + 4 * RATE).min(samples.len());
        let mut end = limit;
        if limit < samples.len() {
            for (offset, frame) in samples[start + 2 * RATE..limit].chunks(1600).enumerate() {
                let power =
                    frame.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / frame.len() as f64;
                if power.sqrt() < 0.01 {
                    end = start + 2 * RATE + (offset + 1) * 1600;
                }
            }
        }
        if samples.len() - end < RATE {
            end = samples.len();
        }
        windows.push((start, end));
        start = end;
    }
    windows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removes_teacher_and_overlap_and_merges_student_ranges() {
        let segments = [
            Interval {
                start: 0.0,
                end: 5.0,
                speaker: 0,
            },
            Interval {
                start: 4.0,
                end: 8.0,
                speaker: 1,
            },
            Interval {
                start: 8.0,
                end: 10.0,
                speaker: 0,
            },
            Interval {
                start: 9.0,
                end: 11.0,
                speaker: 0,
            },
        ];
        let selected = exclusive_intervals(&segments, 0, 11.0);
        assert_eq!(
            selected
                .iter()
                .map(|s| (s.start, s.end))
                .collect::<Vec<_>>(),
            [(0.0, 4.0), (8.0, 11.0)]
        );
    }
    #[test]
    fn speaker_identity_requires_threshold_and_margin() {
        assert_eq!(
            select_speaker(&[(1, 0.9), (0, 0.5)], 0.65, 0.1).unwrap().0,
            1
        );
        assert!(select_speaker(&[(0, 0.8), (1, 0.79)], 0.65, 0.1).is_none());
        assert!(select_speaker(&[(0, 0.4)], 0.65, 0.1).is_none());
        assert!(cosine(&[0.0], &[1.0]).is_none());
    }
    #[test]
    fn language_windows_cover_audio_once_and_have_a_bounded_size() {
        let samples = vec![0.1; crate::audio::SAMPLE_RATE * 11];
        let windows = language_windows(&samples);
        assert_eq!(windows.first().unwrap().0, 0);
        assert_eq!(windows.last().unwrap().1, samples.len());
        assert!(windows.windows(2).all(|pair| pair[0].1 == pair[1].0));
        assert!(
            windows
                .iter()
                .all(|(a, b)| b - a <= 5 * crate::audio::SAMPLE_RATE)
        );
    }
}
