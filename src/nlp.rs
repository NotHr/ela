//! Model-backed local UD tagging/parsing. Morphosyntactic observations are not
//! grammar-error diagnoses, especially when the input is an ASR transcript.
use crate::analysis::Report;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    time::Duration,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Unit {
    pub language: String,
    pub sentences: usize,
    #[serde(default)]
    pub sentence_texts: Vec<String>,
    pub tokens: usize,
    pub content_words: usize,
    pub lexical_density_percent: Option<f64>,
    pub unique_lemmas: usize,
    pub content_lemmas: Vec<String>,
    pub pos_counts: BTreeMap<String, usize>,
    pub dependency_counts: BTreeMap<String, usize>,
    pub morphological_features: BTreeMap<String, BTreeMap<String, usize>>,
    pub mean_dependency_distance: Option<f64>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct NlpReport {
    pub status: String,
    pub backend: String,
    pub model_revision: Option<String>,
    pub task_context: String,
    pub units: Vec<Unit>,
    pub reference: Option<Unit>,
    pub reference_content_lemma_coverage_percent: Option<f64>,
    pub warnings: Vec<String>,
}
fn model(language: &str) -> Option<String> {
    let key = match language {
        "te" => "ELA_NLP_TE_MODEL",
        "en" => "ELA_NLP_EN_MODEL",
        _ => return None,
    };
    std::env::var(key)
        .ok()
        .filter(|path| std::path::Path::new(path).is_file())
}
pub fn configured(language: &str) -> bool {
    if language == "en" && std::env::var_os("ELA_SPACY_MODEL").is_some() {
        crate::grammar::spacy_configured()
    } else {
        model(language).is_some()
    }
}
pub fn parse_conllu(data: &str, language: &str) -> Result<Unit, String> {
    let mut result = Unit {
        language: language.into(),
        sentences: 0,
        sentence_texts: Vec::new(),
        tokens: 0,
        content_words: 0,
        lexical_density_percent: None,
        unique_lemmas: 0,
        content_lemmas: Vec::new(),
        pos_counts: BTreeMap::new(),
        dependency_counts: BTreeMap::new(),
        morphological_features: BTreeMap::new(),
        mean_dependency_distance: None,
    };
    let mut lemmas = BTreeSet::new();
    let mut content = BTreeSet::new();
    let mut distance = 0;
    let mut edges = 0;
    let mut in_sentence = false;
    for line in data.lines() {
        if line.trim().is_empty() {
            in_sentence = false;
            continue;
        }
        if let Some(text) = line.strip_prefix("# text = ") {
            result.sentence_texts.push(text.into());
        }
        if line.starts_with('#') {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() != 10 {
            return Err("Invalid CoNLL-U row".into());
        }
        if columns[0].contains(['-', '.']) {
            continue;
        }
        let id = columns[0]
            .parse::<usize>()
            .map_err(|_| "Invalid UD token ID")?;
        if id == 0 {
            return Err("Invalid UD token ID".into());
        }
        if !in_sentence {
            result.sentences += 1;
            in_sentence = true;
        }
        let pos = columns[3];
        let head = columns[6]
            .parse::<usize>()
            .map_err(|_| "Missing dependency parse")?;
        *result.pos_counts.entry(pos.into()).or_default() += 1;
        *result
            .dependency_counts
            .entry(columns[7].into())
            .or_default() += 1;
        if head > 0 {
            distance += id.abs_diff(head);
            edges += 1;
        }
        for feature in columns[5].split('|').filter(|feature| *feature != "_") {
            if let Some((key, value)) = feature.split_once('=') {
                *result
                    .morphological_features
                    .entry(key.into())
                    .or_default()
                    .entry(value.into())
                    .or_default() += 1;
            }
        }
        if matches!(pos, "PUNCT" | "SYM") {
            continue;
        }
        result.tokens += 1;
        let lemma = if columns[2] == "_" {
            columns[1]
        } else {
            columns[2]
        }
        .to_lowercase();
        lemmas.insert(lemma.clone());
        if matches!(pos, "NOUN" | "PROPN" | "VERB" | "ADJ" | "ADV") {
            result.content_words += 1;
            content.insert(lemma);
        }
    }
    result.unique_lemmas = lemmas.len();
    result.content_lemmas = content.into_iter().take(500).collect();
    result.lexical_density_percent =
        (result.tokens > 0).then(|| 100.0 * result.content_words as f64 / result.tokens as f64);
    result.mean_dependency_distance = (edges > 0).then(|| distance as f64 / edges as f64);
    if result.tokens == 0 {
        return Err("Parser produced no word tokens".into());
    }
    Ok(result)
}
pub(crate) async fn run(text: &str, language: &str) -> Result<Unit, String> {
    if language == "en" && std::env::var_os("ELA_SPACY_MODEL").is_some() {
        return crate::grammar::spacy(text).await;
    }
    let model = model(language).ok_or("Local NLP model not configured")?;
    let mut file = tempfile::NamedTempFile::new().map_err(|_| "Cannot create private NLP input")?;
    file.write_all(text.as_bytes())
        .map_err(|_| "Cannot write NLP input")?;
    let binary = std::env::var("ELA_UDPIPE_BIN").unwrap_or_else(|_| "udpipe".into());
    let output = crate::local_model(
        tokio::process::Command::new(binary)
            .args(["--tokenize", "--tag", "--parse"])
            .arg(model)
            .arg(file.path()),
        Duration::from_secs(60),
    )
    .await
    .map_err(|_| "UDPipe unavailable or timed out")?;
    if !output.status.success() || output.stdout.len() > 4_000_000 {
        return Err("Local NLP failed or exceeded output limit".into());
    }
    parse_conllu(
        &String::from_utf8(output.stdout).map_err(|_| "NLP output is not UTF-8")?,
        language,
    )
}
pub async fn analyze(report: &Report, reference: &str, language: &str, task: &str) -> NlpReport {
    let mut output = NlpReport {
        status: "not_configured".into(),
        backend: if std::env::var_os("ELA_SPACY_MODEL").is_some() {
            "spaCy-en_core_web_trf/English+UDPipe/Telugu"
        } else {
            "udpipe-local/ud-pos+lemmas+morphology+dependencies"
        }
        .into(),
        model_revision: std::env::var("ELA_NLP_MODEL_REVISION").ok(),
        task_context: task.into(),
        units: Vec::new(),
        reference: None,
        reference_content_lemma_coverage_percent: None,
        warnings: vec![
            "nlp_on_asr_not_grammar_diagnosis".into(),
            "model_quality_requires_child_language_validation".into(),
        ],
    };
    if task == "read_aloud" {
        output
            .warnings
            .push("read_aloud_grammar_belongs_to_passage_not_child_composition".into());
    }
    let mut texts: Vec<(String, String)> = Vec::new();
    let mut previous_end: Option<f64> = None;
    for segment in &report.segments {
        if segment.exclusion_reason.is_some() || segment.text.trim().is_empty() {
            previous_end = None;
            continue;
        }
        if let Some((last, text)) = texts.last_mut()
            && last == &segment.language
            && previous_end.is_some_and(|end| (end - segment.start_seconds).abs() < 0.02)
        {
            text.push(' ');
            text.push_str(&segment.text);
        } else {
            texts.push((segment.language.clone(), segment.text.clone()));
        }
        previous_end = Some(segment.end_seconds);
    }
    if texts.is_empty() && !report.transcript.trim().is_empty() {
        texts.push((language.into(), report.transcript.clone()));
    }
    let requested = texts.len();
    let mut attempted = 0;
    if requested > 32 {
        output.warnings.push("nlp_unit_limit".into());
    }
    for (language, text) in texts.into_iter().take(32) {
        if !configured(&language) {
            output
                .warnings
                .push(format!("nlp_model_unavailable_{language}"));
            continue;
        }
        attempted += 1;
        match run(&text, &language).await {
            Ok(unit) => output.units.push(unit),
            Err(message) => output
                .warnings
                .push(if message.to_lowercase().contains("memory") {
                    format!("nlp_memory_budget_{language}")
                } else {
                    format!("nlp_failed_{language}")
                }),
        }
    }
    if !output.units.is_empty() {
        output.status = if output.units.len() == requested {
            "complete"
        } else {
            "partial"
        }
        .into();
    } else if attempted > 0 {
        output.status = "failed".into();
    } else if requested == 0 {
        output.status = "no_transcript".into();
    }
    if !reference.trim().is_empty() && configured(language) {
        let has_telugu = reference
            .chars()
            .any(|c| ('\u{0c00}'..='\u{0c7f}').contains(&c));
        let has_latin = reference.chars().any(|c| c.is_ascii_alphabetic());
        if has_telugu && has_latin {
            output
                .warnings
                .push("mixed_reference_not_dependency_parsed".into());
        } else if let Ok(unit) = run(reference, language).await {
            let expected = unit.content_lemmas.iter().collect::<BTreeSet<_>>();
            let actual = output
                .units
                .iter()
                .filter(|u| u.language == language)
                .flat_map(|u| u.content_lemmas.iter())
                .collect::<BTreeSet<_>>();
            if !expected.is_empty() && output.units.iter().any(|unit| unit.language == language) {
                output.reference_content_lemma_coverage_percent = Some(
                    100.0 * expected.intersection(&actual).count() as f64 / expected.len() as f64,
                );
            }
            output.reference = Some(unit);
        } else {
            output.warnings.push("reference_nlp_failed".into());
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measures_actual_ud_tags_lemmas_morphology_and_dependencies() {
        let unit=parse_conllu("1\tShe\tshe\tPRON\t_\tCase=Nom|Number=Sing\t2\tnsubj\t_\t_\n2\treads\tread\tVERB\t_\tTense=Pres\t0\troot\t_\t_\n3\tbooks\tbook\tNOUN\t_\tNumber=Plur\t2\tobj\t_\t_\n4\t.\t.\tPUNCT\t_\t_\t2\tpunct\t_\t_\n","en").unwrap();
        assert_eq!(unit.tokens, 3);
        assert_eq!(unit.content_words, 2);
        assert!(unit.content_lemmas.contains(&"read".into()));
        assert_eq!(unit.morphological_features["Tense"]["Pres"], 1);
        assert_eq!(unit.dependency_counts["nsubj"], 1);
    }
}
