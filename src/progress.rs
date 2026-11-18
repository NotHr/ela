//! Longitudinal evidence grouped by stable student ID and comparable conditions.
//! Never infer improvement from an easier passage or an unrecorded model change.
use crate::{App, Job, Result};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;

#[derive(Serialize)]
pub struct Student {
    pub student_id: String,
    pub name: String,
    pub classroom: String,
    pub assessments: usize,
    pub completed: usize,
    pub processing: usize,
    pub failed: usize,
    pub last_assessed_at: String,
}
#[derive(Serialize)]
pub struct History {
    pub id: i64,
    pub activity: String,
    pub language: String,
    pub recorded_at: String,
    pub status: String,
    pub outcome: Option<String>,
    pub reading_match: Option<f64>,
    pub matched_words_per_minute: Option<f64>,
}
#[derive(Serialize)]
pub struct Point {
    pub id: i64,
    pub recorded_at: String,
    pub reading_match: f64,
    pub matched_words_per_minute: f64,
}
#[derive(Serialize)]
pub struct Series {
    pub key: String,
    pub passage: String,
    pub language: String,
    pub model_revision: Option<String>,
    pub analysis_version: String,
    pub policy: String,
    pub comparable: bool,
    pub points: Vec<Point>,
    pub reading_match_change_pp: Option<f64>,
    pub pace_change: Option<f64>,
}
#[derive(Serialize)]
pub struct Profile {
    pub student: Student,
    pub series: Vec<Series>,
    pub history: Vec<History>,
}
fn recorded_key(job: &Job) -> (i128, i64) {
    (
        time::OffsetDateTime::parse(
            &job.recorded_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map(|date| date.unix_timestamp_nanos())
        .unwrap_or(job.id as i128),
        job.id,
    )
}
fn summary(jobs: &[&Job]) -> Student {
    let latest = jobs.iter().max_by_key(|job| recorded_key(job)).unwrap();
    Student {
        student_id: latest.student_id.clone(),
        name: latest.learner.clone(),
        classroom: latest.classroom.clone(),
        assessments: jobs.len(),
        completed: jobs.iter().filter(|job| job.status == "complete").count(),
        processing: jobs
            .iter()
            .filter(|job| !matches!(job.status.as_str(), "complete" | "failed"))
            .count(),
        failed: jobs.iter().filter(|job| job.status == "failed").count(),
        last_assessed_at: latest.recorded_at.clone(),
    }
}
fn profile(jobs: &[Job]) -> Profile {
    let mut ordered = jobs.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|job| recorded_key(job));
    let student = summary(&ordered);
    let jobs = &ordered;
    let history = jobs
        .iter()
        .rev()
        .map(|job| History {
            id: job.id,
            activity: job.prompt.clone(),
            language: job.language.clone(),
            recorded_at: job.recorded_at.clone(),
            status: job.status.clone(),
            outcome: job.report.as_ref().map(|r| r.outcome.clone()),
            reading_match: job
                .report
                .as_ref()
                .and_then(|r| r.reading_screen.as_ref().map(|s| s.score)),
            matched_words_per_minute: job.report.as_ref().and_then(|r| {
                r.reading_screen
                    .as_ref()
                    .map(|s| s.matched_words_per_minute)
            }),
        })
        .collect();
    let mut groups: BTreeMap<String, Series> = BTreeMap::new();
    for job in jobs {
        let Some(report) = &job.report else { continue };
        let Some(screen) = &report.reading_screen else {
            continue;
        };
        let reference = job
            .reference_text
            .nfc()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if reference.is_empty() {
            continue;
        }
        let condition = serde_json::to_vec(&(
            &reference,
            &job.language,
            &job.audio_mode,
            job.detect_language,
            &report.analysis_version,
            &screen.policy,
            &report.backend,
            &report.model_revision,
            report
                .ocr
                .as_ref()
                .and_then(|ocr| ocr.model_revision.as_ref()),
        ))
        .expect("serializable comparison conditions");
        let key = format!("{:x}", Sha256::digest(condition));
        let group = groups.entry(key.clone()).or_insert_with(|| Series {
            key,
            passage: reference.chars().take(100).collect(),
            language: job.language.clone(),
            model_revision: report.model_revision.clone(),
            analysis_version: report.analysis_version.clone(),
            policy: screen.policy.clone(),
            comparable: report
                .model_revision
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty()),
            points: Vec::new(),
            reading_match_change_pp: None,
            pace_change: None,
        });
        group.points.push(Point {
            id: job.id,
            recorded_at: job.recorded_at.clone(),
            reading_match: screen.score,
            matched_words_per_minute: screen.matched_words_per_minute,
        });
    }
    for group in groups.values_mut() {
        if group.comparable && group.points.len() > 1 {
            let first = group.points.first().unwrap();
            let last = group.points.last().unwrap();
            group.reading_match_change_pp = Some(last.reading_match - first.reading_match);
            group.pace_change =
                Some(last.matched_words_per_minute - first.matched_words_per_minute);
        }
    }
    Profile {
        student,
        series: groups.into_values().collect(),
        history,
    }
}
pub async fn students(State(app): State<App>) -> Result<Json<Vec<Student>>> {
    let jobs = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs ORDER BY id")
        .fetch_all(&app.db)
        .await?;
    let mut grouped: BTreeMap<&str, Vec<&Job>> = BTreeMap::new();
    for job in &jobs {
        if !job.student_id.starts_with("legacy-assessment-") {
            grouped.entry(&job.student_id).or_default().push(job);
        }
    }
    Ok(Json(grouped.values().map(|jobs| summary(jobs)).collect()))
}
pub async fn student(State(app): State<App>, Path(id): Path<String>) -> Result<Json<Profile>> {
    let jobs =
        sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE student_id=? ORDER BY id")
            .bind(id)
            .fetch_all(&app.db)
            .await?;
    if jobs.is_empty() {
        return Err(crate::Error(
            StatusCode::NOT_FOUND,
            "Student not found".into(),
        ));
    }
    let jobs = jobs
        .into_iter()
        .map(Job::with_report)
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(profile(&jobs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(id: i64, passage: &str, revision: Option<&str>) -> Job {
        let (_, audio) = crate::audio::decode(&crate::audio::fixture(16000, 1, 2000)).unwrap();
        let report = crate::analysis::analyze(
            crate::inference::Transcript {
                text: passage.into(),
                model_revision: revision.map(str::to_owned),
                backend: "test".into(),
                ..Default::default()
            },
            audio,
            passage,
        );
        Job {
            id,
            learner: "లక్ష్మి".into(),
            student_id: "S-1".into(),
            source_id: None,
            recorded_at: format!("2026-01-{id:02}T00:00:00Z"),
            classroom: "3".into(),
            language: "te".into(),
            prompt: "Reading".into(),
            task: "read_aloud".into(),
            reference_text: passage.into(),
            audio_mode: "learner_only".into(),
            detect_language: false,
            status: "complete".into(),
            error: None,
            created_at: format!("2026-01-{id:02}T00:00:00Z"),
            report_json: None,
            report: Some(report),
        }
    }
    #[test]
    fn compares_only_same_passage_and_model_conditions() {
        let result = profile(&[
            job(1, "నా పేరు లక్ష్మి", Some("v1")),
            job(2, "నా పేరు లక్ష్మి", Some("v1")),
            job(3, "వేరే పాఠం", Some("v1")),
            job(4, "నా పేరు లక్ష్మి", Some("v2")),
        ]);
        assert_eq!(result.series.len(), 3);
        assert_eq!(
            result
                .series
                .iter()
                .filter(|s| s.reading_match_change_pp.is_some())
                .count(),
            1
        );
        assert_eq!(result.history.len(), 4);
    }
    #[test]
    fn unknown_model_versions_do_not_claim_improvement() {
        let result = profile(&[job(1, "నా పేరు లక్ష్మి", None), job(2, "నా పేరు లక్ష్మి", None)]);
        assert!(!result.series[0].comparable);
        assert!(result.series[0].reading_match_change_pp.is_none());
    }
}
