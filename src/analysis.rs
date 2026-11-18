//! Deterministic, automatic measurements. These are not inferred comprehension,
//! grammar judgments, pronunciation scores, or a replacement ELA proficiency scale.
use crate::{audio::AudioStats, inference::Transcript};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReferenceComparison {
    pub reference_words: usize,
    pub word_edits: usize,
    pub word_error_rate_percent: f64,
    pub grapheme_edits: usize,
    pub grapheme_error_rate_percent: f64,
    #[serde(default)]
    pub differences: Vec<WordDifference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WordDifference {
    pub kind: String,
    pub expected: Option<String>,
    pub recognized: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingScreen {
    pub score: f64,
    pub level: String,
    pub provisional: bool,
    pub policy: String,
    pub matched_words_per_minute: f64,
    pub practice_words: Vec<String>,
    pub practice_activities: Vec<String>,
    pub teacher_interventions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub analysis_version: String,
    pub outcome: String,
    pub transcript: String,
    pub backend: String,
    pub model_revision: Option<String>,
    pub audio: AudioStats,
    pub word_count: usize,
    pub unique_words: usize,
    pub type_token_ratio: Option<f64>,
    pub words_per_recording_minute: Option<f64>,
    /// ELA's speech-quantity thresholds, adapted to recognized Telugu words too.
    /// None when insufficient or suspicious evidence prevents a measurement.
    pub speech_quantity_band: Option<u8>,
    pub reference_comparison: Option<ReferenceComparison>,
    pub proficiency_score: Option<u8>,
    #[serde(default)]
    pub reading_screen: Option<ReadingScreen>,
    pub unavailable: Vec<String>,
    pub warnings: Vec<String>,
    pub next_steps: Vec<String>,
    #[serde(default)]
    pub speech: Option<crate::speech::Evidence>,
    #[serde(default)]
    pub segments: Vec<crate::speech::Segment>,
    #[serde(default)]
    pub ocr: Option<crate::ocr::OcrResult>,
    #[serde(default)]
    pub nlp: Option<crate::nlp::NlpReport>,
    #[serde(default)]
    pub grammar: Option<crate::grammar::GrammarReport>,
    #[serde(default)]
    pub slm: Option<crate::slm::SlmReport>,
    #[serde(default)]
    pub resource_policy: String,
}

fn words(text: &str) -> Vec<String> {
    let normalized: String = text.nfc().collect::<String>().to_lowercase();
    normalized.unicode_words().map(str::to_owned).collect()
}

fn edit_distance<T: Eq>(reference: &[T], actual: &[T]) -> usize {
    let mut previous: Vec<usize> = (0..=actual.len()).collect();
    let mut next = vec![0; actual.len() + 1];
    for (i, expected) in reference.iter().enumerate() {
        next[0] = i + 1;
        for (j, observed) in actual.iter().enumerate() {
            next[j + 1] = (previous[j + 1] + 1)
                .min(next[j] + 1)
                .min(previous[j] + usize::from(expected != observed));
        }
        std::mem::swap(&mut previous, &mut next);
    }
    previous[actual.len()]
}

pub fn speech_quantity(word_count: usize) -> u8 {
    match word_count {
        0 => 0,
        1..=10 => 1,
        11..=25 => 2,
        _ => 3,
    }
}

fn align_words(expected: &[String], actual: &[String]) -> Vec<WordDifference> {
    let width = actual.len() + 1;
    let mut costs = vec![0_u16; (expected.len() + 1) * width];
    for i in 0..=expected.len() {
        costs[i * width] = i as u16;
    }
    for (j, cell) in costs.iter_mut().take(width).enumerate() {
        *cell = j as u16;
    }
    for (i, word) in expected.iter().enumerate() {
        for (j, observed) in actual.iter().enumerate() {
            costs[(i + 1) * width + j + 1] = (costs[i * width + j + 1] + 1)
                .min(costs[(i + 1) * width + j] + 1)
                .min(costs[i * width + j] + u16::from(word != observed));
        }
    }
    let (mut i, mut j) = (expected.len(), actual.len());
    let mut differences = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && expected[i - 1] == actual[j - 1] {
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && costs[i * width + j] == costs[(i - 1) * width + j - 1] + 1 {
            differences.push(WordDifference {
                kind: "substitution".into(),
                expected: Some(expected[i - 1].clone()),
                recognized: Some(actual[j - 1].clone()),
            });
            i -= 1;
            j -= 1;
        } else if i > 0 && costs[i * width + j] == costs[(i - 1) * width + j] + 1 {
            differences.push(WordDifference {
                kind: "omission".into(),
                expected: Some(expected[i - 1].clone()),
                recognized: None,
            });
            i -= 1;
        } else {
            differences.push(WordDifference {
                kind: "insertion".into(),
                expected: None,
                recognized: Some(actual[j - 1].clone()),
            });
            j -= 1;
        }
    }
    differences.reverse();
    differences
}

fn reading_screen(comparison: &ReferenceComparison, audio: &AudioStats) -> ReadingScreen {
    let score = (100.0 - comparison.word_error_rate_percent).clamp(0.0, 100.0);
    // Explicit starter screening policy, NOT a normed/validated proficiency scale.
    let level = if score < 60.0 {
        "needs_support"
    } else if score < 85.0 {
        "developing"
    } else {
        "on_track"
    };
    let missing = comparison
        .differences
        .iter()
        .filter(|difference| difference.expected.is_some())
        .count();
    let matched = comparison.reference_words.saturating_sub(missing);
    let mut seen = HashSet::new();
    let practice_words = comparison
        .differences
        .iter()
        .filter_map(|difference| difference.expected.as_ref())
        .filter(|word| seen.insert((*word).clone()))
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    let practice_activities = if practice_words.is_empty() {
        vec!["reread_for_expression".into(), "try_next_passage".into()]
    } else {
        vec![
            "listen_and_repeat_focus_words".into(),
            "blend_focus_words".into(),
            "reread_same_passage".into(),
        ]
    };
    let teacher_interventions = match level {
        "needs_support" => vec![
            "verify_with_short_one_to_one_reading".into(),
            "model_then_echo_read".into(),
        ],
        "developing" => vec![
            "guided_small_group_reading".into(),
            "check_focus_words_in_context".into(),
        ],
        _ => vec!["extend_with_another_passage".into()],
    };
    ReadingScreen {
        score,
        level: level.into(),
        provisional: true,
        policy: "reference-match-v1".into(),
        matched_words_per_minute: 60.0 * matched as f64 / audio.duration_seconds,
        practice_words,
        practice_activities,
        teacher_interventions,
    }
}

fn compare(reference: &str, actual: &[String]) -> Option<ReferenceComparison> {
    let expected = words(reference);
    if expected.is_empty() {
        return None;
    }
    let expected_text = expected.join(" ");
    let actual_text = actual.join(" ");
    let expected_graphemes: Vec<_> = expected_text.graphemes(true).collect();
    let actual_graphemes: Vec<_> = actual_text.graphemes(true).collect();
    // Bound quadratic edit-distance work on untrusted uploads/model output.
    if expected_graphemes.len() > 4000 || actual_graphemes.len() > 4000 {
        return None;
    }
    let word_edits = edit_distance(&expected, actual);
    let grapheme_edits = edit_distance(&expected_graphemes, &actual_graphemes);
    Some(ReferenceComparison {
        reference_words: expected.len(),
        word_edits,
        word_error_rate_percent: 100.0 * word_edits as f64 / expected.len() as f64,
        grapheme_edits,
        grapheme_error_rate_percent: 100.0 * grapheme_edits as f64
            / expected_graphemes.len() as f64,
        differences: align_words(&expected, actual),
    })
}

pub fn analyze(transcription: Transcript, audio: AudioStats, reference: &str) -> Report {
    let audio = transcription.student_audio.clone().unwrap_or(audio);
    let incomplete = transcription.speech.as_ref().is_some_and(|e| {
        !e.student_identified
            || e.overlap_excluded_seconds > 0.05
            || e.untranscribed_student_seconds > 0.05
    });
    let tokens = words(&transcription.text);
    let count = tokens.len();
    let unique = tokens.iter().collect::<HashSet<_>>().len();
    let low_signal = audio.energy_active_seconds < 0.2;
    let repeated = tokens.windows(2).filter(|w| w[0] == w[1]).count();
    let suspicious_repetition = count >= 5 && repeated * 2 >= count;
    let outcome = if low_signal || count == 0 {
        "insufficient_evidence"
    } else if suspicious_repetition {
        "uncertain_transcription"
    } else if incomplete {
        "partial_student_audio"
    } else {
        "measured"
    };
    let mut warnings = vec!["asr_is_not_ground_truth".into()];
    let separated = transcription
        .speech
        .as_ref()
        .is_some_and(|e| e.diarization_applied);
    if !separated {
        warnings.push("learner_only_audio_required".into());
    }
    if let Some(evidence) = &transcription.speech {
        warnings.extend(evidence.warnings.iter().cloned());
    }
    if incomplete {
        warnings.push("incomplete_student_audio_score_withheld".into());
    }
    let mut next_steps = Vec::new();
    let unmatched_student = transcription
        .speech
        .as_ref()
        .is_some_and(|e| e.diarization_applied && !e.student_identified);
    if unmatched_student {
        next_steps.push("collect_clear_student_reference".into());
    } else if low_signal {
        warnings.push("low_audio_energy".into());
        next_steps.push("record_closer_to_microphone".into());
    }
    if count == 0 {
        warnings.push("no_recognized_words".into());
        next_steps.push("collect_another_speech_sample".into());
    }
    if suspicious_repetition {
        warnings.push("repetition_may_be_asr_error_or_real_speech".into());
        next_steps.push("collect_another_speech_sample".into());
    }
    if audio.clipped_sample_percent > 1.0 {
        warnings.push("audio_clipping".into());
        next_steps.push("reduce_recording_gain".into());
    }
    if audio.duration_seconds > 25.0 {
        warnings.push("asr_window_boundaries_may_cut_words".into());
    }
    let reference_comparison = if !reference.trim().is_empty() && !low_signal {
        let comparison = compare(reference, &tokens);
        if comparison.is_none() {
            warnings.push("reference_comparison_limit_or_empty_reference".into());
        } else {
            warnings.push("reference_errors_include_asr_errors_not_pronunciation".into());
            next_steps.push("practice_reference_then_record_again".into());
        }
        comparison
    } else {
        None
    };
    let reading_screen = if outcome == "measured" && audio.clipped_sample_percent <= 1.0 {
        reference_comparison
            .as_ref()
            .map(|comparison| reading_screen(comparison, &audio))
    } else {
        None
    };
    if reading_screen.is_some() {
        warnings.push("provisional_reading_screen_not_validated_proficiency".into());
    }
    if outcome == "measured" && count <= 10 {
        next_steps.push("invite_a_longer_response".into());
    }
    if next_steps.is_empty() {
        next_steps.push("compare_with_future_samples_same_activity".into());
    }
    Report {
        analysis_version: "automatic-reading-v4".into(),
        outcome: outcome.into(),
        transcript: transcription.text,
        backend: transcription.backend,
        model_revision: transcription.model_revision,
        word_count: count,
        unique_words: unique,
        type_token_ratio: (count > 0).then(|| unique as f64 / count as f64),
        words_per_recording_minute: (!low_signal && count > 0)
            .then(|| 60.0 * count as f64 / audio.duration_seconds),
        speech_quantity_band: (outcome == "measured").then(|| speech_quantity(count)),
        audio,
        reference_comparison,
        proficiency_score: None,
        reading_screen,
        unavailable: {
            let mut missing = vec![
                "ela_language_id_matrix_not_integrated".into(),
                "grammar_and_pos_models_not_integrated".into(),
                "comprehension_not_measured".into(),
            ];
            if !separated {
                missing.push("speaker_diarization_not_integrated".into());
            }
            missing
        },
        warnings,
        next_steps,
        speech: transcription.speech,
        segments: transcription.segments,
        ocr: None,
        nlp: None,
        grammar: None,
        slm: None,
        resource_policy: "serial-model-processes/disk-queue".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(text: &str, reference: &str, amplitude: i16) -> Report {
        let (_, audio) = crate::audio::decode(&crate::audio::fixture(16000, 1, amplitude)).unwrap();
        analyze(
            Transcript {
                text: text.into(),
                backend: "test-fixture".into(),
                ..Default::default()
            },
            audio,
            reference,
        )
    }

    #[test]
    fn speech_thresholds_match_upstream_intent() {
        for (count, band) in [(0, 0), (1, 1), (10, 1), (11, 2), (25, 2), (26, 3)] {
            assert_eq!(speech_quantity(count), band);
        }
    }

    #[test]
    fn telugu_marks_are_not_split_into_words() {
        let r = report("నా పేరు లక్ష్మి", "నా పేరు లక్ష్మి", 2000);
        assert_eq!(r.word_count, 3);
        assert_eq!(r.reference_comparison.unwrap().word_edits, 0);
        assert_eq!(r.speech_quantity_band, Some(1));
        assert_eq!(r.proficiency_score, None);
    }

    #[test]
    fn reference_comparison_handles_insertions_and_deletions() {
        let r = report("the red bird flies", "the bird", 2000);
        let comparison = r.reference_comparison.unwrap();
        assert_eq!(comparison.word_edits, 2);
        assert_eq!(comparison.word_error_rate_percent, 100.0);
    }

    #[test]
    fn reading_screen_targets_reference_differences() {
        let r = report("the cat sat", "the dog sat here", 2000);
        let screen = r.reading_screen.unwrap();
        assert_eq!(screen.score, 50.0);
        assert_eq!(screen.level, "needs_support");
        assert!(screen.provisional);
        assert_eq!(screen.practice_words, ["dog", "here"]);
        assert!(
            screen
                .teacher_interventions
                .contains(&"model_then_echo_read".into())
        );
        assert_eq!(r.reference_comparison.unwrap().differences.len(), 2);
    }

    #[test]
    fn telugu_reading_screen_and_empty_evidence() {
        let r = report("నా పేరు లక్ష్మి", "నా పేరు లక్ష్మి", 2000);
        assert_eq!(r.reading_screen.unwrap().score, 100.0);
        assert!(report("", "నా పేరు లక్ష్మి", 2000).reading_screen.is_none());
        assert!(
            report("నా పేరు లక్ష్మి", "నా పేరు లక్ష్మి", 0)
                .reading_screen
                .is_none()
        );
    }

    #[test]
    fn incomplete_student_speech_withholds_reading_scores_and_uses_student_time() {
        let (_, original) = crate::audio::decode(&crate::audio::fixture(16000, 10, 2000)).unwrap();
        let (_, student) = crate::audio::decode(&crate::audio::fixture(16000, 2, 2000)).unwrap();
        let r = analyze(
            Transcript {
                text: "నా పేరు లక్ష్మి".into(),
                backend: "test-fixture".into(),
                student_audio: Some(student),
                speech: Some(crate::speech::Evidence {
                    diarization_applied: true,
                    student_identified: true,
                    untranscribed_student_seconds: 0.2,
                    ..Default::default()
                }),
                ..Default::default()
            },
            original,
            "నా పేరు లక్ష్మి",
        );
        assert_eq!(r.outcome, "partial_student_audio");
        assert_eq!(r.audio.duration_seconds, 2.0);
        assert!(r.reading_screen.is_none());
        assert!(r.speech_quantity_band.is_none());
        assert!(
            !r.unavailable
                .contains(&"speaker_diarization_not_integrated".into())
        );
    }

    #[test]
    fn silence_and_repetition_do_not_receive_a_band() {
        assert_eq!(report("imagined words", "", 0).speech_quantity_band, None);
        assert_eq!(report("", "", 2000).outcome, "insufficient_evidence");
        assert_eq!(
            report("hi hi hi hi hi hi", "", 2000).speech_quantity_band,
            None
        );
    }
}
