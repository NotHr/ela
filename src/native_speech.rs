//! sherpa-onnx integration: diarization → reference speaker matching → exclusive
//! student turns → optional per-window LID → language-specific ASR.
use crate::{
    audio::{self, SAMPLE_RATE},
    inference::{self, Options, Transcript},
    speech::{self, Evidence, Interval, Segment},
};
use sherpa_onnx::{
    OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig, OfflineWhisperModelConfig,
    SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig,
};
use std::{collections::BTreeSet, path::Path};

fn threads() -> i32 {
    std::env::var("ELA_ASR_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
        .clamp(1, 16)
}
fn threshold(key: &str, default: f32) -> Result<f32, String> {
    let value = match std::env::var(key) {
        Ok(value) => value.parse::<f32>().map_err(|_| format!("Invalid {key}"))?,
        Err(_) => default,
    };
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{key} must be between 0 and 1"));
    }
    Ok(value)
}
fn recognizer(language: &str) -> Result<OfflineRecognizer, String> {
    inference::readiness(language)?;
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.provider = Some("cpu".into());
    config.model_config.num_threads = threads();
    config.decoding_method = Some("greedy_search".into());
    if language == "te" {
        config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
            model: Some(inference::required_file("ELA_TE_MODEL")?),
        };
        config.model_config.tokens = Some(inference::required_file("ELA_TE_TOKENS")?);
    } else {
        config.model_config.whisper = OfflineWhisperModelConfig {
            encoder: Some(inference::required_file("ELA_EN_ENCODER")?),
            decoder: Some(inference::required_file("ELA_EN_DECODER")?),
            language: Some("en".into()),
            task: Some("transcribe".into()),
            ..Default::default()
        };
        config.model_config.tokens = Some(inference::required_file("ELA_EN_TOKENS")?);
    }
    OfflineRecognizer::create(&config)
        .ok_or("Cannot load ASR model; check export and token table".into())
}
fn embedding(extractor: &SpeakerEmbeddingExtractor, samples: &[f32]) -> Option<Vec<f32>> {
    if samples.len() < SAMPLE_RATE {
        return None;
    }
    let stream = extractor.create_stream()?;
    stream.accept_waveform(SAMPLE_RATE as i32, samples);
    stream.input_finished();
    if !extractor.is_ready(&stream) {
        return None;
    }
    extractor.compute(&stream)
}
fn slice(samples: &[f32], interval: Interval) -> &[f32] {
    let start = (interval.start * SAMPLE_RATE as f64).floor() as usize;
    let end = (interval.end * SAMPLE_RATE as f64).ceil() as usize;
    &samples[start.min(samples.len())..end.min(samples.len())]
}
fn select_student(
    samples: &[f32],
    reference_path: &Path,
    evidence: &mut Evidence,
) -> Result<Vec<Interval>, String> {
    inference::diarization_readiness()?;
    let (reference, stats) = audio::decode(
        &std::fs::read(reference_path).map_err(|_| "Cannot read student reference")?,
    )?;
    if !(3.0..=30.0).contains(&stats.duration_seconds) || stats.energy_active_seconds < 2.0 {
        return Err("Student reference must contain 3–30 seconds of student-only audio with at least 2 seconds of active signal".into());
    }
    let embedding_config = SpeakerEmbeddingExtractorConfig {
        model: Some(inference::required_file("ELA_SPEAKER_MODEL")?),
        num_threads: threads(),
        ..Default::default()
    };
    let extractor = SpeakerEmbeddingExtractor::create(&embedding_config)
        .ok_or("Cannot load speaker embedding model")?;
    let reference_embedding = embedding(&extractor, &reference)
        .ok_or("Cannot extract student reference voice embedding")?;
    let mut config = OfflineSpeakerDiarizationConfig {
        embedding: embedding_config,
        ..Default::default()
    };
    config.segmentation.pyannote.model = Some(inference::required_file("ELA_DIARIZATION_MODEL")?);
    config.segmentation.num_threads = threads();
    // Unknown number of speakers: never assume teacher=0, student=1.
    config.clustering.num_clusters = -1;
    let diarizer =
        OfflineSpeakerDiarization::create(&config).ok_or("Cannot load diarization model")?;
    if diarizer.sample_rate() != SAMPLE_RATE as i32 {
        return Err("Diarization model must accept 16 kHz audio".into());
    }
    let result = diarizer.process(samples).ok_or("Diarization failed")?;
    evidence.diarization_applied = true;
    let segments = result
        .sort_by_start_time()
        .into_iter()
        .map(|segment| Interval {
            start: segment.start as f64,
            end: segment.end as f64,
            speaker: segment.speaker,
        })
        .collect::<Vec<_>>();
    let speakers = segments.iter().map(|s| s.speaker).collect::<BTreeSet<_>>();
    evidence.speaker_count = speakers.len();
    let mut scores = Vec::new();
    for speaker in speakers {
        let exclusive = speech::exclusive_intervals(&segments, speaker, evidence.recording_seconds);
        let candidate = exclusive
            .iter()
            .flat_map(|interval| slice(samples, *interval).iter().copied())
            .take(20 * SAMPLE_RATE)
            .collect::<Vec<_>>();
        if let Some(vector) = embedding(&extractor, &candidate)
            && let Some(similarity) = speech::cosine(&reference_embedding, &vector)
        {
            scores.push((speaker, similarity));
        }
    }
    let minimum = threshold("ELA_SPEAKER_MATCH_THRESHOLD", 0.65)?;
    let margin = threshold("ELA_SPEAKER_MATCH_MARGIN", 0.10)?;
    let Some((speaker, similarity)) = speech::select_speaker(&scores, minimum, margin) else {
        evidence.warnings.push("student_speaker_ambiguous".into());
        return Ok(Vec::new());
    };
    evidence.student_identified = true;
    evidence.selected_speaker = Some(speaker);
    evidence.speaker_similarity = Some(similarity);
    let exclusive = speech::exclusive_intervals(&segments, speaker, evidence.recording_seconds);
    let own = segments
        .iter()
        .filter(|s| s.speaker == speaker)
        .copied()
        .collect::<Vec<_>>();
    let total = speech::exclusive_intervals(&own, speaker, evidence.recording_seconds)
        .iter()
        .map(|s| s.end - s.start)
        .sum::<f64>();
    evidence.overlap_excluded_seconds =
        (total - exclusive.iter().map(|s| s.end - s.start).sum::<f64>()).max(0.0);
    let mut verified = Vec::new();
    // A matching cluster can still contain contamination. Verify each exclusive
    // turn against the reference; exclude short/unverifiable turns conservatively.
    for interval in exclusive {
        let similarity = embedding(&extractor, slice(samples, interval))
            .and_then(|v| speech::cosine(&reference_embedding, &v));
        if similarity.is_some_and(|score| score >= minimum) {
            verified.push(interval);
        } else {
            evidence.untranscribed_student_seconds += interval.end - interval.start;
        }
    }
    if evidence.overlap_excluded_seconds > 0.0 {
        evidence.warnings.push("overlapping_speech_excluded".into());
    }
    if evidence.untranscribed_student_seconds > 0.0 {
        evidence
            .warnings
            .push("unverified_student_turns_excluded".into());
    }
    if verified.is_empty() {
        evidence.warnings.push("student_speech_not_verified".into());
    }
    Ok(verified)
}
fn run_helper(
    mut command: std::process::Command,
    windows: &[ela_local::language_policy::Window],
) -> Result<Vec<String>, String> {
    use std::{io::Write, process::Stdio};
    let mut ranges = tempfile::NamedTempFile::new().map_err(|_| "Cannot create model request")?;
    ranges
        .write_all(&serde_json::to_vec(windows).map_err(|_| "Cannot encode model request")?)
        .map_err(|_| "Cannot write model request")?;
    let mut child = command
        .arg(ranges.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Cannot start isolated model helper")?;
    let keepalive = child.stdin.take();
    let result = child
        .wait_with_output()
        .map_err(|_| "Cannot read model helper output")?;
    drop(keepalive);
    if !result.status.success() || result.stdout.len() > 2_000_000 {
        return Err("Isolated model helper failed; verify local models and runtime".into());
    }
    let values: Vec<String> =
        serde_json::from_slice(&result.stdout).map_err(|_| "Invalid model helper result")?;
    if values.len() != windows.len() {
        return Err("Model helper returned wrong number of windows".into());
    }
    Ok(values)
}
fn detect_languages(
    path: &Path,
    windows: &[ela_local::language_policy::Window],
) -> Result<Vec<String>, String> {
    inference::language_readiness()?;
    let mut command = std::process::Command::new(inference::language_helper()?);
    command
        .arg(inference::required_file("ELA_LID_MODEL")?)
        .arg(path);
    let languages = run_helper(command, windows)?;
    if languages
        .iter()
        .any(|lang| !matches!(lang.as_str(), "te" | "en" | "und"))
    {
        return Err("Invalid language labels".into());
    }
    Ok(languages)
}

/// Internal isolated ASR worker: one recognizer, bounded windows, then exit.
pub fn decode_windows(
    path: &Path,
    language: &str,
    windows: &[ela_local::language_policy::Window],
) -> Result<Vec<String>, String> {
    if windows.len() > 300 {
        return Err("Too many ASR windows".into());
    }
    let (samples, _) = audio::decode(&std::fs::read(path).map_err(|_| "Cannot read ASR audio")?)?;
    let recognizer = recognizer(language)?;
    let mut text = Vec::new();
    for window in windows {
        if window.start >= window.end
            || window.end > samples.len()
            || window.end - window.start > 25 * SAMPLE_RATE + 1
        {
            return Err("Invalid ASR window".into());
        }
        let stream = recognizer.create_stream();
        stream.accept_waveform(SAMPLE_RATE as i32, &samples[window.start..window.end]);
        recognizer.decode(&stream);
        text.push(
            stream
                .get_result()
                .ok_or("No ASR result")?
                .text
                .trim()
                .to_string(),
        );
    }
    Ok(text)
}

pub fn run(path: &Path, language: &str, options: Options) -> Result<Transcript, String> {
    inference::readiness(language)?;
    let model_revision = inference::fingerprint(language, &options)?;
    let (samples, stats) = audio::decode(&std::fs::read(path).map_err(|_| "Cannot read WAV")?)?;
    let mut evidence = Evidence {
        recording_seconds: stats.duration_seconds,
        student_identified: options.student_reference.is_none(),
        ..Default::default()
    };
    let intervals = if let Some(reference) = &options.student_reference {
        select_student(&samples, reference, &mut evidence)?
    } else {
        vec![Interval {
            start: 0.0,
            end: stats.duration_seconds,
            speaker: 0,
        }]
    };
    let student_samples = intervals
        .iter()
        .flat_map(|interval| slice(&samples, *interval).iter().copied())
        .collect::<Vec<_>>();
    evidence.student_seconds = student_samples.len() as f64 / SAMPLE_RATE as f64;
    let student_audio = Some(audio::sample_stats(&student_samples));
    let mut output = Transcript {
        backend: "sherpa-onnx/student-speech".into(),
        model_revision: Some(model_revision),
        student_audio,
        ..Default::default()
    };
    if intervals.is_empty() {
        output.speech = Some(evidence);
        return Ok(output);
    }
    let detected_languages = if options.detect_language {
        let windows = intervals
            .iter()
            .flat_map(|interval| {
                let base = (interval.start * SAMPLE_RATE as f64).floor() as usize;
                speech::language_windows(slice(&samples, *interval))
                    .into_iter()
                    .map(move |(start, end)| ela_local::language_policy::Window {
                        start: base + start,
                        end: base + end,
                    })
            })
            .collect::<Vec<_>>();
        detect_languages(path, &windows)?
    } else {
        Vec::new()
    };
    let mut language_labels = detected_languages.into_iter();
    evidence.language_detection_applied = options.detect_language;
    let mut pending: Vec<(usize, ela_local::language_policy::Window)> = Vec::new();
    for interval in intervals {
        let turn = slice(&samples, interval);
        let windows = if options.detect_language {
            speech::language_windows(turn)
        } else {
            (0..turn.len())
                .step_by(25 * SAMPLE_RATE)
                .map(|start| (start, (start + 25 * SAMPLE_RATE).min(turn.len())))
                .collect()
        };
        for (start, end) in windows {
            let window = &turn[start..end];
            let base_segment = Segment {
                start_seconds: interval.start + start as f64 / SAMPLE_RATE as f64,
                end_seconds: (interval.start + end as f64 / SAMPLE_RATE as f64).min(interval.end),
                speaker: evidence.selected_speaker,
                ..Default::default()
            };
            let lang = if options.detect_language {
                let detected = language_labels
                    .next()
                    .ok_or("Missing language window label")?;
                if window.len() < SAMPLE_RATE * 3 / 2 {
                    evidence.untranscribed_student_seconds +=
                        window.len() as f64 / SAMPLE_RATE as f64;
                    evidence
                        .warnings
                        .push("short_language_window_excluded".into());
                    output.segments.push(Segment {
                        language: "und".into(),
                        exclusion_reason: Some("short_language_window_excluded".into()),
                        ..base_segment
                    });
                    continue;
                }
                detected
            } else {
                language.to_string()
            };
            if options.detect_language && !evidence.detected_languages.contains(&lang) {
                evidence.detected_languages.push(lang.clone());
            }
            if !matches!(lang.as_str(), "te" | "en") {
                evidence.untranscribed_student_seconds += window.len() as f64 / SAMPLE_RATE as f64;
                evidence
                    .warnings
                    .push("unsupported_language_excluded".into());
                output.segments.push(Segment {
                    language: lang,
                    exclusion_reason: Some("unsupported_language_excluded".into()),
                    ..base_segment
                });
                continue;
            }
            let absolute_start = (interval.start * SAMPLE_RATE as f64).floor() as usize + start;
            let absolute_end = absolute_start + window.len();
            // LID needs short windows; ASR does not. Coalesce neighboring
            // same-language student windows to reduce encoder runs and cut words.
            if let Some((index, range)) = pending.last_mut()
                && range.end == absolute_start
                && absolute_end - range.start <= 25 * SAMPLE_RATE
                && output.segments[*index].language == lang
                && output.segments[*index].speaker == base_segment.speaker
            {
                range.end = absolute_end;
                output.segments[*index].end_seconds = base_segment.end_seconds;
            } else {
                pending.push((
                    output.segments.len(),
                    ela_local::language_policy::Window {
                        start: absolute_start,
                        end: absolute_end,
                    },
                ));
                output.segments.push(Segment {
                    language: lang,
                    ..base_segment
                });
            }
        }
    }
    // One short-lived ASR process per language. It exits before the next model
    // starts, reclaiming native allocator/arena memory, not merely dropping a cache.
    for language in ["te", "en"] {
        let group = pending
            .iter()
            .filter(|(index, _)| output.segments[*index].language == language)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }
        let windows = group
            .iter()
            .map(|(_, range)| ela_local::language_policy::Window {
                start: range.start,
                end: range.end,
            })
            .collect::<Vec<_>>();
        let mut command = std::process::Command::new(
            std::env::current_exe().map_err(|_| "Cannot locate ASR worker")?,
        );
        command.arg("--asr-windows").arg(path).arg(language);
        let texts = run_helper(command, &windows)?;
        for ((index, range), text) in group.into_iter().zip(texts) {
            if text.is_empty() {
                evidence.untranscribed_student_seconds +=
                    (range.end - range.start) as f64 / SAMPLE_RATE as f64;
                evidence.warnings.push("empty_asr_window_excluded".into());
                output.segments[*index].exclusion_reason = Some("empty_asr_window_excluded".into());
            }
            output.segments[*index].text = text;
        }
    }
    evidence
        .warnings
        .push("speaker_and_language_models_can_be_wrong".into());
    if options.detect_language {
        evidence
            .warnings
            .push("windowed_language_detection_not_word_level".into());
        evidence
            .warnings
            .push("language_candidates_restricted_te_en".into());
    }
    evidence.warnings.sort();
    evidence.warnings.dedup();
    output.text = output
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !options.detect_language && options.student_reference.is_none() {
        output.backend = if language == "te" {
            "sherpa-onnx/indicconformer-ctc-te"
        } else {
            "sherpa-onnx/whisper-en"
        }
        .into();
    }
    output.speech = Some(evidence);
    Ok(output)
}
