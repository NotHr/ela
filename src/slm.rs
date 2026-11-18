//! Required, evidence-constrained report writer. The SLM selects and orders
//! reviewed bilingual explanations and approved actions; it cannot invent a
//! diagnosis, change a score or turn ASR/POS observations into proficiency claims.
use crate::analysis::Report;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
    time::Duration,
};

#[derive(Clone)]
pub enum Backend {
    Local,
    #[cfg(test)]
    Fixture,
    #[cfg(test)]
    Unavailable,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(default)]
    pub summary: Vec<String>,
    pub practice: Vec<String>,
    pub interventions: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub en: String,
    pub te: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SlmReport {
    pub status: String,
    pub reason: Option<String>,
    pub model_fingerprint: Option<String>,
    pub plan: Option<Plan>,
    #[serde(default)]
    pub summary: Option<Summary>,
}
impl SlmReport {
    pub fn unavailable(reason: &str) -> Self {
        Self {
            status: "failed".into(),
            reason: Some(reason.into()),
            model_fingerprint: None,
            plan: None,
            summary: None,
        }
    }
    pub fn ready(&self) -> bool {
        matches!(self.status.as_str(), "complete" | "cached") && self.summary.is_some()
    }
}
pub fn configured() -> bool {
    std::env::var("ELA_SLM_MODEL").is_ok_and(|path| std::path::Path::new(&path).is_file())
}
fn available_mib() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    info.lines()
        .find(|line| line.starts_with("MemAvailable:"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()
        .map(|kib| kib / 1024)
}
fn memory_admitted(available: Option<u64>, required: u64) -> bool {
    available.is_some_and(|value| value >= required)
}
struct Fact {
    id: String,
    en: String,
    te: String,
}
struct Evidence {
    facts: Vec<Fact>,
    practice: Vec<String>,
    interventions: Vec<String>,
}
impl Evidence {
    fn from_report(report: &Report, task: &str) -> Self {
        let mut facts = Vec::new();
        let mut add = |id: &str, en: String, te: String| {
            facts.push(Fact {
                id: id.into(),
                en,
                te,
            })
        };
        add(
            "recording",
            format!(
                "The recording lasted {:.1} seconds. The speech model recognized {} words—not necessarily {} correctly read words.",
                report.audio.duration_seconds, report.word_count, report.word_count
            ),
            format!(
                "{:.1} సెకన్ల రికార్డింగ్ నుంచి మాటల గుర్తింపు వ్యవస్థ {} పదాలను గుర్తించింది. ఇది గుర్తించిన పదాల సంఖ్య మాత్రమే; సరిగ్గా చదివిన పదాల సంఖ్య కాదు.",
                report.audio.duration_seconds, report.word_count
            ),
        );
        if let Some(screen) = &report.reading_screen {
            add(
                "assessment_scope",
                format!(
                    "The estimated match to the reference passage is {:.1} out of 100. This is a provisional text-match measure, not a grade-level proficiency result.",
                    screen.score
                ),
                format!(
                    "ఇచ్చిన పేరాతో అంచనా పదాల సరిపోలిక 100లో {:.1}. ఇది ప్రాథమిక పదాల సరిపోలిక కొలత మాత్రమే; తరగతి స్థాయి భాషా నైపుణ్య నిర్ధారణ కాదు.",
                    screen.score
                ),
            );
        } else {
            add("assessment_scope","No reliable reading-match score is available for this recording. Word count and speaking speed alone cannot establish reading accuracy.".into(),"ఈ రికార్డింగ్‌కు నమ్మదగిన పఠన సరిపోలిక స్కోరు అందుబాటులో లేదు. పదాల సంఖ్య, మాట్లాడే వేగం మాత్రమే పఠన ఖచ్చితత్వాన్ని నిర్ధారించవు.".into());
        }
        add("limits","The transcript may contain recognition errors. These measurements do not establish grammar correctness, comprehension or overall language proficiency.".into(),"గుర్తించిన పాఠ్యంలో మాటల గుర్తింపు లోపాలు ఉండవచ్చు. ఈ కొలతలు వ్యాకరణం సరైనదని, విషయ అవగాహన ఉందని లేదా మొత్తం భాషా నైపుణ్యాన్ని నిర్ధారించవు.".into());
        let next = if report.outcome != "measured" {
            (
                "Make a fresh recording with the child's voice clearly audible in a quiet place. Do not treat insufficient or uncertain evidence as a failure.",
                "నిశ్శబ్ద ప్రదేశంలో పిల్లల మాటలు స్పష్టంగా వినిపించేలా కొత్త రికార్డింగ్ చేయండి. సరిపోని లేదా అనిశ్చిత ఆధారాలను వైఫల్యంగా పరిగణించవద్దు.",
            )
        } else if task == "read_aloud" && report.reading_screen.is_none() {
            (
                "For a reading check, provide a clear, known paragraph and record the child reading it. Check that the extracted paragraph matches the original.",
                "పఠన పరిశీలన కోసం స్పష్టమైన, తెలిసిన పేరాను ఇచ్చి పిల్లలు చదివేటప్పుడు రికార్డు చేయండి. ఫోటో నుంచి గుర్తించిన పాఠ్యం అసలు పేరాతో సరిపోతుందో చూడండి.",
            )
        } else if report.reading_screen.is_some() {
            (
                "Practise the listed focus words and activities, then repeat the same passage under similar recording conditions. Compare like-for-like attempts, not speed alone.",
                "కింద ఇచ్చిన పదాలు, అభ్యాసాలను సాధన చేసి, ఇలాంటి రికార్డింగ్ పరిస్థితుల్లో అదే పేరాను మళ్లీ చదవండి. వేగాన్ని మాత్రమే కాకుండా సమాన పరిస్థితుల ప్రయత్నాలను పోల్చండి.",
            )
        } else {
            (
                "For speaking practice, describe a familiar picture or retell a short story in connected sentences. Use a separate read-aloud activity with an expected passage to measure reading accuracy.",
                "మాట్లాడే సాధన కోసం తెలిసిన చిత్రం గురించి చెప్పండి లేదా చిన్న కథను అనుసంధానమైన వాక్యాల్లో తిరిగి చెప్పండి. పఠన ఖచ్చితత్వం కోసం తెలిసిన పేరాతో ప్రత్యేక పఠన కార్యక్రమం నిర్వహించండి.",
            )
        };
        add("next_step", next.0.into(), next.1.into());
        if let Some(nlp) = &report.nlp {
            for (index, unit) in nlp.units.iter().take(2).enumerate() {
                let language = if unit.language == "te" {
                    "Telugu"
                } else {
                    "English"
                };
                add(
                    &format!("language_{}_{index}", unit.language),
                    format!(
                        "In an analyzed {language} portion, the parser identified {} sentences and {} distinct dictionary word forms. These describe the recognized text, not the child's vocabulary size or grammatical ability.",
                        unit.sentences, unit.unique_lemmas
                    ),
                    format!(
                        "{} పాఠ్యంలో విశ్లేషణ వ్యవస్థ {} వాక్యాలు, {} వేర్వేరు మూల పదరూపాలను గుర్తించింది. ఇవి గుర్తించిన పాఠ్య వివరాలు మాత్రమే; పిల్లల మొత్తం పదజాలం లేదా వ్యాకరణ సామర్థ్యానికి కొలతలు కావు.",
                        if unit.language == "te" {
                            "తెలుగు"
                        } else {
                            "ఆంగ్ల"
                        },
                        unit.sentences,
                        unit.unique_lemmas
                    ),
                );
            }
        }
        if let Some(grammar) = &report.grammar
            && grammar.status != "not_applicable"
        {
            let edits: usize = grammar.category_counts.values().sum();
            let (en, te) = if matches!(grammar.status.as_str(), "complete" | "partial") {
                (
                    format!(
                        "The English correction check reviewed {} of {} sentences and proposed {} categorized grammar edits. These concern the recognized transcript, not confirmed mistakes by the child; zero proposed edits does not prove correct grammar.",
                        grammar.analyzed_sentences, grammar.total_sentences, edits
                    ),
                    format!(
                        "ఆంగ్ల దిద్దుబాటు పరిశీలన {} వాక్యాల్లో {} పరిశీలించి, {} వర్గీకరించిన వ్యాకరణ మార్పులను సూచించింది. ఇవి గుర్తించిన పాఠ్యానికి సూచనలు మాత్రమే; పిల్లల నిర్ధారిత తప్పులు కావు. సూచనలు లేకపోవడం వ్యాకరణం పూర్తిగా సరైనదని నిరూపించదు.",
                        grammar.total_sentences, grammar.analyzed_sentences, edits
                    ),
                )
            } else {
                ("The English correction check is unavailable for this attempt. No grammar-correctness conclusion can be drawn from it.".into(),"ఈ ప్రయత్నానికి ఆంగ్ల దిద్దుబాటు పరిశీలన అందుబాటులో లేదు. దీనితో వ్యాకరణ ఖచ్చితత్వంపై నిర్ణయం తీసుకోలేము.".into())
            };
            add("grammar", en, te);
        }
        let mut practice = report
            .reading_screen
            .as_ref()
            .map(|s| s.practice_activities.clone())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                if task == "speaking" && report.outcome == "measured" {
                    vec![
                        "describe_familiar_picture".into(),
                        "retell_short_story".into(),
                        "speak_connected_sentences".into(),
                    ]
                } else if report.next_steps.is_empty() {
                    vec!["collect_another_speech_sample".into()]
                } else {
                    report.next_steps.clone()
                }
            });
        if task == "speaking"
            && let Some(grammar) = &report.grammar
            && matches!(grammar.status.as_str(), "complete" | "partial")
        {
            let mut targeted = Vec::new();
            for (pattern, action) in [
                ("VERB:SVA", "practice_subject_verb_agreement"),
                ("VERB:TENSE", "practice_verb_tense"),
                ("NOUN:NUM", "practice_noun_number"),
                ("DET", "practice_articles"),
                ("PREP", "practice_prepositions"),
            ] {
                if grammar
                    .category_counts
                    .keys()
                    .any(|category| category.contains(pattern))
                {
                    targeted.push(action.into());
                }
            }
            practice.splice(0..0, targeted);
        }
        let interventions = report
            .reading_screen
            .as_ref()
            .map(|s| s.teacher_interventions.clone())
            .unwrap_or_default();
        Self {
            facts,
            practice,
            interventions,
        }
    }
    fn valid(&self, plan: &Plan) -> bool {
        let allowed = |values: &[String], candidates: &[String], min: usize, max: usize| {
            (min..=max).contains(&values.len())
                && values.iter().collect::<BTreeSet<_>>().len() == values.len()
                && values.iter().all(|value| candidates.contains(value))
        };
        let ids = self
            .facts
            .iter()
            .map(|fact| fact.id.clone())
            .collect::<Vec<_>>();
        allowed(&plan.summary, &ids, 4, 6)
            && (!self.facts.iter().any(|f| f.id.starts_with("language_"))
                || plan.summary.iter().any(|id| id.starts_with("language_")))
            && (!self.facts.iter().any(|f| f.id == "grammar")
                || plan.summary.iter().any(|id| id == "grammar"))
            && self.facts[..4]
                .iter()
                .all(|fact| plan.summary.contains(&fact.id))
            && allowed(&plan.practice, &self.practice, 1, 2)
            && allowed(
                &plan.interventions,
                &self.interventions,
                usize::from(!self.interventions.is_empty()),
                2,
            )
    }
    fn orders(&self) -> Vec<Vec<&str>> {
        let languages = self
            .facts
            .iter()
            .filter(|f| f.id.starts_with("language_"))
            .map(|f| f.id.as_str())
            .collect::<Vec<_>>();
        let has_grammar = self.facts.iter().any(|f| f.id == "grammar");
        let mut selections = languages.iter().map(|id| vec![*id]).collect::<Vec<_>>();
        if selections.is_empty() {
            selections.push(vec![]);
        } else if languages.len() > 1 && !has_grammar {
            selections.push(languages);
        }
        let mut orders = Vec::new();
        for mut selection in selections {
            if has_grammar {
                selection.push("grammar");
            }
            for before_limits in [true, false] {
                let mut ids = vec!["recording", "assessment_scope"];
                if !before_limits {
                    ids.push("limits");
                }
                ids.extend(selection.iter().copied());
                if before_limits {
                    ids.push("limits");
                }
                ids.push("next_step");
                orders.push(ids);
            }
        }
        orders
    }
    fn render(&self, plan: Plan, fingerprint: String, status: &str) -> SlmReport {
        let mut en = Vec::new();
        let mut te = Vec::new();
        for id in &plan.summary {
            if let Some(fact) = self.facts.iter().find(|fact| &fact.id == id) {
                en.push(fact.en.as_str());
                te.push(fact.te.as_str());
            }
        }
        SlmReport {
            status: status.into(),
            reason: None,
            model_fingerprint: Some(fingerprint),
            summary: Some(Summary {
                en: en.join(" "),
                te: te.join(" "),
            }),
            plan: Some(plan),
        }
    }
}
fn array_literal(values: &[&str]) -> String {
    serde_json::to_string(&serde_json::to_string(values).unwrap()).unwrap()
}
fn action_arrays(codes: &[String]) -> String {
    if codes.is_empty() {
        return array_literal(&[]);
    }
    let mut arrays = Vec::new();
    for first in codes {
        arrays.push(array_literal(&[first]));
        for second in codes {
            if first != second {
                arrays.push(array_literal(&[first, second]));
            }
        }
    }
    arrays.join(" | ")
}
fn grammar(evidence: &Evidence) -> String {
    let arrays = evidence
        .orders()
        .iter()
        .map(|ids| array_literal(ids))
        .collect::<Vec<_>>();
    format!(
        r#"root ::= "{{" ws "\"summary\"" ws ":" ws summary-array ws "," ws "\"practice\"" ws ":" ws practice-array ws "," ws "\"interventions\"" ws ":" ws intervention-array ws "}}" ws
ws ::= [ \t\n\r]*
summary-array ::= {}
practice-array ::= {}
intervention-array ::= {}
"#,
        arrays.join(" | "),
        action_arrays(&evidence.practice),
        action_arrays(&evidence.interventions)
    )
}
fn fingerprint(path: &std::path::Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|_| "slm_model_unavailable")?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| "slm_invalid_model")?;
    if &magic != b"GGUF" {
        return Err("slm_invalid_model".into());
    }
    let digest = ela_local::model_files::hash_file(path).map_err(|_| "slm_model_unavailable")?;
    Ok(format!("sha256:{digest}"))
}
pub async fn recommend(
    report: &Report,
    db: &sqlx::SqlitePool,
    task: &str,
    backend: &Backend,
) -> SlmReport {
    let evidence = Evidence::from_report(report, task);
    #[cfg(test)]
    match backend {
        Backend::Fixture => {
            return evidence.render(
                Plan {
                    summary: evidence.orders()[0].iter().map(|id| (*id).into()).collect(),
                    practice: evidence.practice.iter().take(2).cloned().collect(),
                    interventions: evidence.interventions.iter().take(2).cloned().collect(),
                },
                "test-summary-provider".into(),
                "complete",
            );
        }
        Backend::Unavailable => return SlmReport::unavailable("slm_model_unavailable"),
        Backend::Local => {}
    }
    let _ = backend;
    let Ok(path) = std::env::var("ELA_SLM_MODEL") else {
        return SlmReport::unavailable("slm_model_unavailable");
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        return SlmReport::unavailable("slm_model_unavailable");
    };
    if metadata.len() > 600 * 1024 * 1024 {
        return SlmReport::unavailable("slm_model_too_large");
    }
    let path = std::path::PathBuf::from(path);
    let fingerprint = match tokio::task::spawn_blocking(move || fingerprint(&path)).await {
        Ok(Ok(hash)) => hash,
        _ => return SlmReport::unavailable("slm_invalid_model"),
    };
    // No identities, transcripts, photos or reference text enter the prompt.
    let payload=serde_json::json!({"facts":evidence.facts.iter().map(|f|serde_json::json!({"id":f.id,"explanation":f.en})).collect::<Vec<_>>(),"required":["recording","assessment_scope","limits","next_step"],"practice":evidence.practice,"interventions":evidence.interventions}).to_string();
    if payload.chars().count() > 4000 {
        return SlmReport::unavailable("slm_context_budget");
    }
    let key = format!(
        "{:x}",
        Sha256::digest(format!("slm-bilingual-summary-v4|{fingerprint}|{payload}"))
    );
    if let Ok(Some(cached)) =
        sqlx::query_scalar::<_, String>("SELECT plan_json FROM suggestion_cache WHERE cache_key=?")
            .bind(&key)
            .fetch_optional(db)
            .await
        && let Ok(plan) = serde_json::from_str::<Plan>(&cached)
        && evidence.valid(&plan)
    {
        return evidence.render(plan, fingerprint, "cached");
    }
    let required = std::env::var("ELA_SLM_MIN_FREE_MIB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1536)
        .max(1024);
    if !memory_admitted(available_mib(), required) {
        return SlmReport::unavailable("slm_memory_budget");
    }
    match generate(&payload, &evidence).await {
        Ok(plan) if evidence.valid(&plan) => {
            if let Ok(json) = serde_json::to_string(&plan) {
                let _ = sqlx::query(
                    "INSERT OR REPLACE INTO suggestion_cache(cache_key,plan_json) VALUES (?,?)",
                )
                .bind(key)
                .bind(json)
                .execute(db)
                .await;
            }
            evidence.render(plan, fingerprint, "complete")
        }
        _ => SlmReport::unavailable("slm_failed_or_unvalidated_output"),
    }
}
async fn generate(payload: &str, evidence: &Evidence) -> Result<Plan, String> {
    let prompt = format!(
        "<|im_start|>system\n/no_think\nCompose a clear school assessment summary by choosing and ordering the provided explanation IDs. Include all four required IDs exactly once, and one or two language observations when provided. Start with recording, explain assessment_scope, place next_step last. Select one or two practice IDs, and one or two intervention IDs if any exist, otherwise an empty interventions array. Return ONLY JSON with summary (an array of IDs), practice and interventions. Never invent IDs, scores, ability judgments or diagnoses.\n<|im_end|>\n<|im_start|>user\n{payload}\n<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n"
    );
    let mut input = tempfile::NamedTempFile::new().map_err(|_| "slm_temp_file")?;
    input
        .write_all(prompt.as_bytes())
        .map_err(|_| "slm_temp_file")?;
    let mut grammar_file = tempfile::NamedTempFile::new().map_err(|_| "slm_temp_file")?;
    grammar_file
        .write_all(grammar(evidence).as_bytes())
        .map_err(|_| "slm_temp_file")?;
    let model = std::env::var("ELA_SLM_MODEL").map_err(|_| "slm_model_unavailable")?;
    let binary = std::env::var("ELA_LLAMA_BIN").unwrap_or_else(|_| "llama-completion".into());
    let mut command = tokio::process::Command::new(binary);
    command
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(input.path())
        .arg("--grammar-file")
        .arg(grammar_file.path())
        .args([
            "--no-conversation",
            "--no-warmup",
            "-b",
            "128",
            "-ub",
            "64",
            "--simple-io",
            "--no-display-prompt",
            "--temp",
            "0",
            "-c",
            "2048",
            "-n",
            "320",
            "-t",
            "2",
            "-ngl",
            "0",
        ]);
    let output = crate::local_model(&mut command, Duration::from_secs(90))
        .await
        .map_err(|_| "slm_runtime_unavailable")?;
    if !output.status.success() || output.stdout.len() > 8192 {
        return Err("slm_invalid_output".into());
    }
    parse_completion(&output.stdout)
}
fn parse_completion(bytes: &[u8]) -> Result<Plan, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "slm_invalid_output")?
        .trim();
    let text = text.strip_suffix("[end of text]").unwrap_or(text).trim();
    serde_json::from_str(text).map_err(|_| "slm_invalid_output".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Report {
        let (_, audio) = crate::audio::decode(&crate::audio::fixture(16000, 1, 2000)).unwrap();
        crate::analysis::analyze(
            crate::inference::Transcript {
                text: "This is a short recording".into(),
                backend: "test".into(),
                ..Default::default()
            },
            audio,
            "",
        )
    }
    #[test]
    fn summary_is_grounded_even_without_a_reference() {
        let report = sample();
        let evidence = Evidence::from_report(&report, "speaking");
        let mut plan = Plan {
            summary: evidence.facts.iter().map(|f| f.id.clone()).collect(),
            practice: evidence.practice.iter().take(1).cloned().collect(),
            interventions: vec![],
        };
        assert!(evidence.valid(&plan));
        let rendered = evidence.render(
            Plan {
                summary: plan.summary.clone(),
                practice: plan.practice.clone(),
                interventions: vec![],
            },
            "test".into(),
            "complete",
        );
        assert!(
            rendered
                .summary
                .unwrap()
                .en
                .contains("No reliable reading-match score")
        );
        plan.summary.retain(|id| id != "limits");
        assert!(!evidence.valid(&plan));
        plan.summary.push("fluent_reader".into());
        assert!(!evidence.valid(&plan));
        assert!(
            serde_json::from_str::<Plan>(
                r#"{"summary":[],"practice":[],"interventions":[],"score":100}"#
            )
            .is_err()
        );
    }
    #[test]
    fn grammar_observation_cannot_be_omitted_or_turn_into_a_diagnosis() {
        let mut report = sample();
        report.grammar = Some(crate::grammar::GrammarReport {
            status: "complete".into(),
            backend: "fixture".into(),
            model_revision: None,
            task_context: "speaking".into(),
            total_sentences: 2,
            analyzed_sentences: 2,
            corrections: vec![],
            category_counts: std::collections::BTreeMap::from([("R:VERB:SVA".into(), 1)]),
            warnings: vec![],
        });
        let evidence = Evidence::from_report(&report, "speaking");
        let mut plan = Plan {
            summary: evidence.orders()[0].iter().map(|id| (*id).into()).collect(),
            practice: evidence.practice.iter().take(1).cloned().collect(),
            interventions: vec![],
        };
        assert!(evidence.valid(&plan));
        let rendered = evidence.render(
            Plan {
                summary: plan.summary.clone(),
                practice: plan.practice.clone(),
                interventions: vec![],
            },
            "fixture".into(),
            "complete",
        );
        assert!(
            rendered
                .summary
                .unwrap()
                .en
                .contains("not confirmed mistakes")
        );
        plan.summary.retain(|id| id != "grammar");
        assert!(!evidence.valid(&plan));
    }
    #[test]
    fn accepts_only_json_and_the_known_runtime_eos_marker() {
        assert!(
            parse_completion(br#"{"summary":[],"practice":[],"interventions":[]} [end of text]"#)
                .is_ok()
        );
        assert!(parse_completion(b"made up assessment: {}").is_err());
    }
    #[test]
    fn model_is_not_admitted_without_sufficient_known_memory() {
        assert!(!memory_admitted(None, 1536));
        assert!(!memory_admitted(Some(512), 1536));
        assert!(memory_admitted(Some(2048), 1536));
    }
}
