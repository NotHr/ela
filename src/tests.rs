use super::*;
use axum::{body::Body, http::Request};
use tower::ServiceExt;

async fn app(adapter: Option<String>) -> App {
    App {
        db: database("sqlite::memory:").await.unwrap(),
        port: 3000,
        adapter,
        inference: Arc::new(Semaphore::new(1)),
        summary_backend: slm::Backend::Fixture,
    }
}
fn upload(language: &str, amplitude: i16) -> Request<Body> {
    let mut body = Vec::new();
    for (name, value) in [
        ("learner", "లక్ష్మి"),
        ("classroom", "3 A"),
        ("language", language),
        ("prompt", "మీ గురించి చెప్పండి"),
        ("task", "read_aloud"),
        ("reference_text", "నా పేరు లక్ష్మి"),
    ] {
        body.extend(
            format!("--test\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend(b"--test\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"speech.wav\"\r\nContent-Type: audio/wav\r\n\r\n");
    body.extend(audio::fixture(16000, 1, amplitude));
    body.extend(b"\r\n--test--\r\n");
    Request::post("/api/assessments")
        .header("content-type", "multipart/form-data; boundary=test")
        .body(Body::from(body))
        .unwrap()
}
async fn conversation_upload(with_reference: bool) -> Request<Body> {
    let (parts, body) = upload("te", 2000).into_parts();
    let mut bytes = axum::body::to_bytes(body, 1_000_000)
        .await
        .unwrap()
        .to_vec();
    bytes.truncate(bytes.len() - b"--test--\r\n".len());
    for (name, value) in [("audio_mode", "conversation"), ("detect_language", "true")] {
        bytes.extend(
            format!("--test\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    if with_reference {
        bytes.extend(b"--test\r\nContent-Disposition: form-data; name=\"student_reference\"; filename=\"reference.wav\"\r\n\r\n");
        bytes.extend(audio::fixture(16000, 4, 2000));
        bytes.extend(b"\r\n");
    }
    bytes.extend(b"--test--\r\n");
    Request::from_parts(parts, Body::from(bytes))
}

async fn wait_for_job(db: &SqlitePool) -> Job {
    for _ in 0..200 {
        let job = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE id=1")
            .fetch_one(db)
            .await
            .unwrap();
        if matches!(job.status.as_str(), "complete" | "failed") {
            return job.with_report().ok().unwrap();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("Job did not complete");
}

#[tokio::test]
async fn required_summary_failure_preserves_measurements_and_can_retry_without_audio() {
    let mut state = app(None).await;
    state.summary_backend = slm::Backend::Unavailable;
    let response = router(state.clone())
        .oneshot(upload("te", 0))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.status, "failed");
    assert!(
        job.error
            .unwrap()
            .contains("measurements have been preserved")
    );
    let original = job.report.unwrap();
    assert_eq!(original.word_count, 0);
    assert!(!original.slm.unwrap().ready());
    state.summary_backend = slm::Backend::Fixture;
    let response = router(state.clone())
        .oneshot(
            Request::post("/api/assessments/1/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.status, "complete");
    assert!(job.error.is_none());
    let report = job.report.unwrap();
    assert_eq!(report.word_count, 0);
    assert!(report.slm.unwrap().ready());
}

#[tokio::test]
async fn silence_skips_asr_and_finishes_with_required_summary_provider() {
    let state = app(None).await;
    let response = router(state.clone())
        .oneshot(upload("te", 0))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.learner, "లక్ష్మి");
    assert_eq!(job.status, "complete");
    assert_eq!(job.report.unwrap().outcome, "insufficient_evidence");
    let events: Vec<String> = sqlx::query_scalar("SELECT stage FROM job_events ORDER BY id")
        .fetch_all(&state.db)
        .await
        .unwrap();
    assert_eq!(
        events,
        [
            "converting",
            "audio_checked",
            "analyzing",
            "linguistic_analysis",
            "grammar_analysis",
            "writing_summary",
            "reporting",
            "complete"
        ]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn autonomous_transcription_analysis_persistence_and_delete() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("adapter");
    // Explicit fixture only. No model output is fabricated in the application.
    std::fs::write(
        &script,
        "#!/bin/sh\nprintf '{\"text\":\"నా పేరు లక్ష్మి\",\"backend\":\"test-fixture\"}'\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let state = app(Some(script.to_str().unwrap().into())).await;
    let router = router(state.clone());
    let response = router.clone().oneshot(upload("te", 2000)).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.status, "complete");
    let report = job.report.unwrap();
    assert_eq!(report.word_count, 3);
    assert_eq!(report.reference_comparison.unwrap().word_edits, 0);
    assert_eq!(report.proficiency_score, None);
    assert_eq!(report.speech_quantity_band, Some(1));
    let response = router
        .clone()
        .oneshot(
            Request::get("/api/assessments/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 100000)
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["report"]["transcript"], "నా పేరు లక్ష్మి");
    let updated = router
        .clone()
        .oneshot(
            Request::post("/api/assessments/1/linguistics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::ACCEPTED);
    let updated = wait_for_job(&state.db).await;
    assert_eq!(updated.status, "complete");
    let updated = updated.report.unwrap();
    assert_eq!(updated.transcript, "నా పేరు లక్ష్మి");
    assert_eq!(updated.word_count, 3);
    assert_eq!(updated.reference_comparison.unwrap().word_edits, 0);
    let response = router
        .oneshot(
            Request::delete("/api/assessments/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM job_events")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[cfg(unix)]
#[tokio::test]
async fn inference_failure_is_saved_without_a_fabricated_report() {
    let state = app(Some("/nonexistent/adapter".into())).await;
    let response = router(state.clone())
        .oneshot(upload("en", 2000))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.status, "failed");
    assert!(job.report.is_none());
    assert!(job.error.is_some());
}

#[tokio::test]
async fn rejects_invalid_language_and_foreign_browser_requests() {
    let router = router(app(None).await);
    let response = router.clone().oneshot(upload("xx", 0)).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    for (header, value) in [
        ("host", "attacker.example:3000"),
        ("origin", "https://attacker.example"),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::get("/api/assessments")
                    .header(header, value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn conversation_requires_student_reference() {
    let response = router(app(None).await)
        .oneshot(conversation_upload(false).await)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[cfg(unix)]
#[tokio::test]
async fn conversation_passes_options_and_rejects_adapters_without_speaker_evidence() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("fixture");
    std::fs::write(&script, "#!/bin/sh\n[ \"$3\" = --student-reference ] && [ -f \"$4\" ] && [ \"$5\" = --detect-language ] || exit 1\nprintf '{\"text\":\"unfiltered teacher speech\"}'\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let state = app(Some(script.to_str().unwrap().into())).await;
    let response = router(state.clone())
        .oneshot(conversation_upload(true).await)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let job = wait_for_job(&state.db).await;
    assert_eq!(job.status, "failed");
    assert_eq!(job.audio_mode, "conversation");
    assert!(job.detect_language);
    assert!(job.report.is_none());
    assert!(job.error.unwrap().contains("speaker separation evidence"));
}

#[tokio::test]
async fn busy_worker_rejects_another_upload() {
    let state = app(None).await;
    let _permit = state.inference.clone().acquire_owned().await.unwrap();
    let response = router(state).oneshot(upload("en", 0)).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn upgrades_existing_jobs_without_changing_old_processing_modes() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("old.db").display());
    let old = SqlitePool::connect(&url).await.unwrap();
    sqlx::query(
        "CREATE TABLE assessment_jobs (
        id INTEGER PRIMARY KEY, learner TEXT, classroom TEXT, language TEXT,
        prompt TEXT, task TEXT, reference_text TEXT, status TEXT, error TEXT,
        report_json TEXT, created_at TEXT
    )",
    )
    .execute(&old)
    .await
    .unwrap();
    sqlx::query("INSERT INTO assessment_jobs VALUES (1,'లక్ష్మి','3','te','పాఠం','read_aloud','పాఠం','complete',NULL,NULL,'2026-01-01T00:00:00Z')").execute(&old).await.unwrap();
    old.close().await;
    let db = database(&url).await.unwrap();
    let job = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE id=1")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(job.learner, "లక్ష్మి");
    assert_eq!(job.audio_mode, "learner_only");
    assert!(!job.detect_language);
    assert_eq!(job.status, "complete");
}

#[tokio::test]
async fn imports_offline_collection_once_and_exposes_student_history() {
    use std::io::{Cursor, Write};
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (folder, instance, date) in [
        ("a", "one", "2026-02-02T10:00:00+05:30"),
        ("b", "two", "2026-01-01T10:00:00+05:30"),
    ] {
        zip.start_file(format!("{folder}/submission.xml"), options)
            .unwrap();
        write!(zip,"<data id='ela_reading_v1'><student_id>TS-001</student_id><student_name>లక్ష్మి</student_name><classroom>3</classroom><language>te</language><activity>పఠనం</activity><recorded_at>{date}</recorded_at><detect_language>false</detect_language><reference_text>నా పేరు లక్ష్మి</reference_text><recording>voice.wav</recording><meta><instanceID>uuid:{instance}</instanceID></meta></data>").unwrap();
        zip.start_file(format!("{folder}/voice.wav"), options)
            .unwrap();
        zip.write_all(&audio::fixture(16000, 1, 0)).unwrap();
    }
    let archive = zip.finish().unwrap().into_inner();
    let request = || {
        let mut bytes=b"--archive\r\nContent-Disposition: form-data; name=\"archive\"; filename=\"collection.zip\"\r\n\r\n".to_vec();
        bytes.extend(&archive);
        bytes.extend(b"\r\n--archive--\r\n");
        Request::post("/api/collection/import")
            .header("content-type", "multipart/form-data; boundary=archive")
            .body(Body::from(bytes))
            .unwrap()
    };
    let state = app(None).await;
    let router = router(state.clone());
    assert_eq!(
        router.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    for _ in 0..200 {
        let complete: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM assessment_jobs WHERE status='complete'")
                .fetch_one(&state.db)
                .await
                .unwrap();
        if complete == 2 && state.inference.available_permits() == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let response = router.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let data: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 100000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        data.as_array()
            .unwrap()
            .iter()
            .all(|item| item["status"] == "duplicate")
    );
    let response = router
        .oneshot(
            Request::get("/api/students/TS-001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let data: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 100000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(data["student"]["assessments"], 2);
    assert_eq!(data["student"]["completed"], 2);
    assert_eq!(
        data["history"][0]["recorded_at"],
        "2026-02-02T10:00:00+05:30"
    );
    assert_eq!(
        data["history"][1]["recorded_at"],
        "2026-01-01T10:00:00+05:30"
    );
}

#[tokio::test]
async fn interrupted_jobs_become_failed_on_restart() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("test.db").display());
    let db = database(&url).await.unwrap();
    sqlx::query("INSERT INTO assessment_jobs (learner,classroom,language,prompt,task,reference_text,status) VALUES ('id','3','en','prompt','speaking','','transcribing')").execute(&db).await.unwrap();
    let (_, stats) = audio::decode(&audio::fixture(16000, 1, 0)).unwrap();
    let report = analysis::analyze(inference::Transcript::default(), stats, "");
    sqlx::query("INSERT INTO assessment_jobs (learner,classroom,language,prompt,task,reference_text,status,report_json) VALUES ('id2','3','en','prompt','speaking','','writing_summary',?)").bind(serde_json::to_string(&report).unwrap()).execute(&db).await.unwrap();
    db.close().await;
    let db = database(&url).await.unwrap();
    let saved = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE id=2")
        .fetch_one(&db)
        .await
        .unwrap()
        .with_report()
        .unwrap();
    assert_eq!(saved.status, "failed");
    assert!(saved.error.unwrap().contains("retry summary"));
    assert!(saved.report.is_some());
    let status: String = sqlx::query_scalar("SELECT status FROM assessment_jobs WHERE id=1")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}
