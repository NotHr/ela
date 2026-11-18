//! English correction proposals: native Rust/Candle T5, then official ERRANT
//! with the official spaCy transformer pipeline in a separate local worker.
use crate::{analysis::Report, nlp::Unit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Edit {
    pub category: String,
    pub original: String,
    pub suggested: String,
    pub original_start: usize,
    pub original_end: usize,
    pub suggested_start: usize,
    pub suggested_end: usize,
    pub included_by_ela: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Correction {
    pub original: String,
    pub suggested: String,
    pub status: String,
    #[serde(default)]
    pub edits: Vec<Edit>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GrammarReport {
    pub status: String,
    pub backend: String,
    pub model_revision: Option<String>,
    pub task_context: String,
    pub total_sentences: usize,
    pub analyzed_sentences: usize,
    pub corrections: Vec<Correction>,
    pub category_counts: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
}
pub fn spacy_configured() -> bool {
    std::env::var("ELA_SPACY_MODEL").is_ok_and(|path| Path::new(&path).join("config.cfg").is_file())
        && std::env::var("ELA_NLP_PYTHON").is_ok_and(|path| Path::new(&path).is_file())
}
fn t5_directory() -> Option<PathBuf> {
    std::env::var_os("ELA_T5_MODEL_DIR").map(PathBuf::from)
}
pub fn configured() -> bool {
    cfg!(feature = "native-grammar")
        && spacy_configured()
        && t5_directory().is_some_and(|path| {
            ["model.safetensors", "config.json", "tokenizer.json"]
                .iter()
                .all(|name| path.join(name).is_file())
        })
}
pub async fn python(stage: &str, payload: serde_json::Value) -> Result<Vec<u8>, String> {
    if !spacy_configured() {
        return Err("spaCy runtime/model not configured".into());
    }
    if !enough_memory(3072) {
        return Err("Insufficient available memory for spaCy transformer worker".into());
    }
    let mut file = tempfile::NamedTempFile::new().map_err(|_| "Cannot create linguistic input")?;
    file.write_all(payload.to_string().as_bytes())
        .map_err(|_| "Cannot write linguistic input")?;
    let mut command = tokio::process::Command::new(
        std::env::var("ELA_NLP_PYTHON").map_err(|_| "Python worker not configured")?,
    );
    command
        .arg("scripts/ela-linguistics.py")
        .arg(stage)
        .arg(file.path());
    let output = crate::local_model(&mut command, Duration::from_secs(180))
        .await
        .map_err(|_| "Linguistic worker unavailable or timed out")?;
    if !output.status.success() || output.stdout.len() > 4_000_000 {
        return Err("Local linguistic worker failed".into());
    }
    Ok(output.stdout)
}
pub async fn spacy(text: &str) -> Result<Unit, String> {
    let bytes = python("analyze", serde_json::json!({"text":text})).await?;
    let unit: Unit = serde_json::from_slice(&bytes).map_err(|_| "Invalid spaCy output")?;
    if unit.language != "en" || unit.tokens == 0 || unit.sentence_texts.len() > 1000 {
        return Err("Invalid spaCy observations".into());
    }
    Ok(unit)
}
fn fingerprint(directory: &Path) -> Result<String, String> {
    let mut hash = Sha256::new();
    hash.update(b"candle-t5-0.9.2-f32-greedy-v1;errant-3.0.0;spacy-en_core_web_trf-3.8.0");
    for name in ["model.safetensors", "config.json", "tokenizer.json"] {
        hash.update(name.as_bytes());
        hash.update(
            ela_local::model_files::hash_file(&directory.join(name))
                .map_err(|_| "Cannot fingerprint T5")?
                .as_bytes(),
        );
    }
    if let Ok(directory) = std::env::var("ELA_SPACY_MODEL") {
        for name in [
            "config.cfg",
            "transformer/model",
            "parser/model",
            "tagger/model",
        ] {
            hash.update(name.as_bytes());
            hash.update(
                ela_local::model_files::hash_file(&Path::new(&directory).join(name))
                    .map_err(|_| "Cannot fingerprint spaCy")?
                    .as_bytes(),
            );
        }
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}
fn enough_memory(required_mib: u64) -> bool {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("MemAvailable:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .is_some_and(|kib| kib >= required_mib * 1024)
}
pub async fn analyze(report: &Report, task: &str, language: &str) -> GrammarReport {
    let mut result = GrammarReport {
        status: "not_applicable".into(),
        backend: "candle-T5/greedy+spaCy-en_core_web_trf+ERRANT-3.0.0".into(),
        model_revision: None,
        task_context: task.into(),
        total_sentences: 0,
        analyzed_sentences: 0,
        corrections: vec![],
        category_counts: BTreeMap::new(),
        warnings: vec![
            "grammar_proposals_on_asr_not_confirmed_child_errors".into(),
            "rust_t5_greedy_not_ela_beam_search".into(),
        ],
    };
    if task == "read_aloud" {
        result
            .warnings
            .push("read_aloud_grammar_belongs_to_passage_not_child_composition".into());
    }
    let sentences = report
        .nlp
        .as_ref()
        .map(|nlp| {
            nlp.units
                .iter()
                .filter(|unit| unit.language == "en")
                .flat_map(|unit| unit.sentence_texts.iter().cloned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    result.total_sentences = sentences.len();
    if sentences.is_empty() {
        let has_english = report.segments.iter().any(|s| {
            s.language == "en" && s.exclusion_reason.is_none() && !s.text.trim().is_empty()
        }) || (report.segments.is_empty()
            && language == "en"
            && !report.transcript.trim().is_empty());
        if has_english {
            result.status = if report
                .nlp
                .as_ref()
                .is_some_and(|nlp| nlp.warnings.iter().any(|w| w == "nlp_memory_budget_en"))
            {
                "memory_blocked"
            } else {
                "sentence_analysis_unavailable"
            }
            .into();
        }
        return result;
    }
    if report.outcome != "measured" {
        result.status = "insufficient_evidence".into();
        return result;
    }
    if !configured() {
        result.status = "not_configured".into();
        return result;
    }
    if !enough_memory(2048) {
        result.status = "memory_blocked".into();
        return result;
    }
    result.status = "failed".into();
    let directory = t5_directory().expect("configuration checked");
    let hash_directory = directory.clone();
    let Ok(Ok(hash)) = tokio::task::spawn_blocking(move || fingerprint(&hash_directory)).await
    else {
        result.warnings.push("grammar_fingerprint_failed".into());
        return result;
    };
    result.model_revision = Some(hash);
    let selected = sentences.into_iter().take(8).collect::<Vec<_>>();
    let mut file = match tempfile::NamedTempFile::new() {
        Ok(file) => file,
        Err(_) => return result,
    };
    let payload = serde_json::json!({"model_dir":directory,"sentences":selected});
    if file.write_all(payload.to_string().as_bytes()).is_err() {
        return result;
    }
    let executable = std::env::var_os("ELA_GRAMMAR_EXECUTABLE")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("ela-grammar")))
        });
    let Some(executable) = executable else {
        return result;
    };
    let mut command = tokio::process::Command::new(executable);
    command.arg(file.path()).env("RAYON_NUM_THREADS", "2");
    let Ok(output) = crate::local_model(&mut command, Duration::from_secs(300)).await else {
        return result;
    };
    if !output.status.success() || output.stdout.len() > 1_000_000 {
        return result;
    }
    let Ok(pairs) = serde_json::from_slice::<Vec<Correction>>(&output.stdout) else {
        return result;
    };
    if pairs.len() != selected.len()
        || pairs
            .iter()
            .zip(&selected)
            .any(|(pair, source)| &pair.original != source || pair.suggested.len() > 8192)
    {
        return result;
    }
    // The T5 process and its model mapping have exited before spaCy/ERRANT starts.
    let corrections = if pairs
        .iter()
        .any(|pair| pair.status == "complete" && pair.original != pair.suggested)
    {
        let Ok(bytes) = python("annotate", serde_json::json!({"pairs":pairs})).await else {
            return result;
        };
        let Ok(annotated) = serde_json::from_slice::<Vec<Correction>>(&bytes) else {
            return result;
        };
        if annotated.len() != pairs.len()
            || annotated.iter().zip(&pairs).any(|(a, b)| {
                a.original != b.original || a.suggested != b.suggested || a.status != b.status
            })
        {
            return result;
        }
        annotated
    } else {
        pairs
    };
    for pair in &corrections {
        if pair.status == "complete" {
            result.analyzed_sentences += 1;
        }
        for edit in &pair.edits {
            if edit.included_by_ela {
                *result
                    .category_counts
                    .entry(edit.category.clone())
                    .or_default() += 1;
            }
        }
    }
    result.corrections = corrections;
    let full_parse = report
        .nlp
        .as_ref()
        .is_some_and(|nlp| nlp.status == "complete");
    result.status = if result.analyzed_sentences == result.total_sentences && full_parse {
        "complete"
    } else {
        "partial"
    }
    .into();
    if result.analyzed_sentences < result.total_sentences {
        result
            .warnings
            .push("grammar_sentence_or_token_budget".into());
    }
    if !full_parse {
        result
            .warnings
            .push("grammar_incomplete_sentence_analysis".into());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn missing_english_parse_is_not_reported_as_correct_grammar() {
        let (_, audio) = crate::audio::decode(&crate::audio::fixture(16000, 1, 2000)).unwrap();
        let report = crate::analysis::analyze(
            crate::inference::Transcript {
                text: "He go to school.".into(),
                backend: "test".into(),
                ..Default::default()
            },
            audio,
            "",
        );
        let grammar = analyze(&report, "speaking", "en").await;
        assert_eq!(grammar.status, "sentence_analysis_unavailable");
        assert!(grammar.corrections.is_empty());
        assert!(grammar.category_counts.is_empty());
    }
    #[tokio::test]
    async fn english_corrector_is_not_applied_to_telugu() {
        let (_, audio) = crate::audio::decode(&crate::audio::fixture(16000, 1, 2000)).unwrap();
        let report = crate::analysis::analyze(
            crate::inference::Transcript {
                text: "నేను పాఠశాలకు వెళ్తాను".into(),
                backend: "test".into(),
                ..Default::default()
            },
            audio,
            "",
        );
        assert_eq!(
            analyze(&report, "speaking", "te").await.status,
            "not_applicable"
        );
    }
}
