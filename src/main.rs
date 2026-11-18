mod analysis;
mod audio;
mod collection;
mod grammar;
mod inference;
#[cfg(feature = "native-asr")]
mod native_speech;
mod nlp;
mod ocr;
mod progress;
mod slm;
mod speech;
mod staging;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::{io::Write, sync::Arc, time::Duration};
use tokio::{process::Command, sync::Semaphore};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Clone)]
struct App {
    db: SqlitePool,
    port: u16,
    adapter: Option<String>,
    inference: Arc<Semaphore>,
    summary_backend: slm::Backend,
}

#[derive(Debug)]
struct Error(StatusCode, String);
type Result<T> = std::result::Result<T, Error>;
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error": self.1}))).into_response()
    }
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        eprintln!("Database error: {error}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database operation failed".into(),
        )
    }
}
fn bad(message: &str) -> Error {
    Error(StatusCode::BAD_REQUEST, message.into())
}
fn language(value: &str) -> Result<()> {
    if matches!(value, "te" | "en") {
        Ok(())
    } else {
        Err(bad("Language must be te or en"))
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct Job {
    id: i64,
    learner: String,
    student_id: String,
    source_id: Option<String>,
    recorded_at: String,
    classroom: String,
    language: String,
    prompt: String,
    task: String,
    reference_text: String,
    audio_mode: String,
    detect_language: bool,
    status: String,
    error: Option<String>,
    created_at: String,
    #[serde(skip)]
    report_json: Option<String>,
    #[sqlx(skip)]
    report: Option<analysis::Report>,
}
impl Job {
    fn with_report(mut self) -> Result<Self> {
        self.report = self
            .report_json
            .as_ref()
            .map(|json| serde_json::from_str(json))
            .transpose()
            .map_err(|_| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Stored report is invalid".into(),
                )
            })?;
        Ok(self)
    }
}

async fn database(url: &str) -> std::result::Result<SqlitePool, sqlx::Error> {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await?;
    // Keep the old manual-assessment table untouched. New automatic reports have
    // a distinct schema so old teacher scores never masquerade as AI output.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS assessment_jobs (
            id INTEGER PRIMARY KEY,
            learner TEXT NOT NULL,
            classroom TEXT NOT NULL,
            language TEXT NOT NULL CHECK(language IN ('te','en')),
            prompt TEXT NOT NULL,
            task TEXT NOT NULL CHECK(task IN ('speaking','read_aloud')),
            reference_text TEXT NOT NULL,
            audio_mode TEXT NOT NULL DEFAULT 'learner_only',
            detect_language INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL,
            error TEXT,
            report_json TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        )",
    )
    .execute(&db)
    .await?;
    let columns = sqlx::query("PRAGMA table_info(assessment_jobs)")
        .fetch_all(&db)
        .await?;
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "audio_mode")
    {
        sqlx::query("ALTER TABLE assessment_jobs ADD COLUMN audio_mode TEXT NOT NULL DEFAULT 'learner_only'").execute(&db).await?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "detect_language")
    {
        sqlx::query(
            "ALTER TABLE assessment_jobs ADD COLUMN detect_language INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&db)
        .await?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "student_id")
    {
        sqlx::query("ALTER TABLE assessment_jobs ADD COLUMN student_id TEXT NOT NULL DEFAULT ''")
            .execute(&db)
            .await?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "source_id")
    {
        sqlx::query("ALTER TABLE assessment_jobs ADD COLUMN source_id TEXT")
            .execute(&db)
            .await?;
    }
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "recorded_at")
    {
        sqlx::query("ALTER TABLE assessment_jobs ADD COLUMN recorded_at TEXT NOT NULL DEFAULT ''")
            .execute(&db)
            .await?;
    }
    sqlx::query("UPDATE assessment_jobs SET recorded_at=created_at WHERE recorded_at=''")
        .execute(&db)
        .await?;
    sqlx::query(
        "UPDATE assessment_jobs SET student_id='legacy-assessment-' || id WHERE student_id=''",
    )
    .execute(&db)
    .await?;
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS unique_source_id ON assessment_jobs(source_id) WHERE source_id IS NOT NULL").execute(&db).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS job_events (
            id INTEGER PRIMARY KEY,
            job_id INTEGER NOT NULL,
            stage TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        )",
    )
    .execute(&db)
    .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS suggestion_cache (cache_key TEXT PRIMARY KEY,plan_json TEXT NOT NULL)").execute(&db).await?;
    // Audio is intentionally temporary, so interrupted work cannot be resumed.
    sqlx::query(
        "UPDATE assessment_jobs SET status='failed', error=CASE WHEN report_json IS NOT NULL THEN 'Application restarted; retry summary' ELSE 'Application restarted; upload audio again' END
         WHERE status NOT IN ('complete','failed')",
    ).execute(&db).await?;
    Ok(db)
}

async fn list(State(app): State<App>) -> Result<Json<Vec<Job>>> {
    let rows = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs ORDER BY id DESC")
        .fetch_all(&app.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(Job::with_report)
            .collect::<Result<Vec<_>>>()?,
    ))
}
async fn detail(State(app): State<App>, Path(id): Path<i64>) -> Result<Json<Job>> {
    let job = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE id=?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| Error(StatusCode::NOT_FOUND, "Assessment not found".into()))?;
    Ok(Json(job.with_report()?))
}
async fn regenerate_summary(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    queue_report_update(app, id, false).await
}
async fn regenerate_language(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    queue_report_update(app, id, true).await
}
async fn queue_report_update(
    app: App,
    id: i64,
    reanalyze: bool,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let permit = app.inference.clone().try_acquire_owned().map_err(|_| {
        Error(
            StatusCode::TOO_MANY_REQUESTS,
            "Desktop is processing another assessment; retry shortly".into(),
        )
    })?;
    let job = sqlx::query_as::<_, Job>("SELECT * FROM assessment_jobs WHERE id=?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| Error(StatusCode::NOT_FOUND, "Assessment not found".into()))?
        .with_report()?;
    if !matches!(job.status.as_str(), "complete" | "failed") {
        return Err(bad("Wait until processing finishes"));
    }
    let mut report = job
        .report
        .ok_or_else(|| bad("No saved measurements; upload the recording again"))?;
    sqlx::query("UPDATE assessment_jobs SET status=?,error=NULL WHERE id=?")
        .bind(if reanalyze {
            "linguistic_analysis"
        } else {
            "writing_summary"
        })
        .bind(id)
        .execute(&app.db)
        .await?;
    tokio::spawn(async move {
        let _permit = permit;
        let result = async {
            if reanalyze {
                language_analysis(
                    &app,
                    id,
                    &mut report,
                    &job.reference_text,
                    &job.language,
                    &job.task,
                )
                .await?;
            }
            write_summary(&app, id, report, &job.task).await
        }
        .await;
        record_failure(&app, id, result).await;
    });
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({"id":id}))))
}
async fn legacy(State(app): State<App>) -> Result<Json<Vec<serde_json::Value>>> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='assessments'",
    )
    .fetch_one(&app.db)
    .await?;
    if exists == 0 {
        return Ok(Json(Vec::new()));
    }
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT json_object(
            'id',id,'learner',learner,'classroom',classroom,'language',language,
            'prompt',prompt,'transcript',transcript,'comprehension',comprehension,
            'expression',expression,'vocabulary',vocabulary,'fluency',fluency,
            'notes',notes,'created_at',created_at
        ) FROM assessments ORDER BY id DESC",
    )
    .fetch_all(&app.db)
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| serde_json::from_str(row).expect("SQLite generated JSON"))
            .collect(),
    ))
}
async fn remove(State(app): State<App>, Path(id): Path<i64>) -> Result<StatusCode> {
    let mut transaction = app.db.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM assessment_jobs WHERE id=?")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?;
    match status.as_deref() {
        None => return Err(Error(StatusCode::NOT_FOUND, "Assessment not found".into())),
        Some("complete" | "failed") => {}
        _ => return Err(bad("Wait until processing finishes before deleting")),
    }
    sqlx::query("DELETE FROM job_events WHERE job_id=?")
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM assessment_jobs WHERE id=?")
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Serialize, sqlx::FromRow)]
struct Event {
    stage: String,
    created_at: String,
}
async fn events(State(app): State<App>, Path(id): Path<i64>) -> Result<Json<Vec<Event>>> {
    Ok(Json(
        sqlx::query_as("SELECT stage,created_at FROM job_events WHERE job_id=? ORDER BY id")
            .bind(id)
            .fetch_all(&app.db)
            .await?,
    ))
}
async fn health(State(app): State<App>) -> Json<serde_json::Value> {
    let readiness = |lang| {
        if app.adapter.is_some() {
            serde_json::json!({"configured":true,"backend":"external-local-adapter","note":"Adapter readiness is not verified"})
        } else {
            let result = inference::readiness(lang);
            serde_json::json!({"configured":result.is_ok(),"backend":"sherpa-onnx","error":result.err()})
        }
    };
    let ffmpeg = std::env::var("ELA_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".into());
    let conversion_available = tokio::time::timeout(
        Duration::from_secs(2),
        Command::new(ffmpeg)
            .arg("-version")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .is_ok_and(|result| result.is_ok_and(|output| output.status.success()));
    Json(serde_json::json!({
        "languages": {"te": readiness("te"), "en": readiness("en")},
        "automatic": true,
        "conversion_available": conversion_available,
        "ocr_configured": ocr::ready().await,
        "nlp": {"te": nlp::configured("te"), "en": nlp::configured("en")},
        "collection": "ela_reading_v1",
        "slm_configured": slm::configured(),
        "slm_required": true,
        "english_spacy_configured": grammar::spacy_configured(),
        "english_grammar_configured": grammar::configured(),
        "resource_policy": "serial-model-processes/disk-queue",
        "diarization_configured": inference::diarization_readiness().is_ok(),
        "language_switching_configured": inference::language_readiness().is_ok(),
    }))
}

#[derive(Default)]
struct Submission {
    learner: String,
    student_id: String,
    source_id: String,
    recorded_at: String,
    paragraph_photo: Vec<u8>,
    classroom: String,
    language: String,
    prompt: String,
    task: String,
    reference_text: String,
    audio_mode: String,
    detect_language: String,
    student_reference: Vec<u8>,
    audio: Vec<u8>,
}
impl Submission {
    fn validate(&self) -> Result<()> {
        language(&self.language)?;
        for (value, max) in [
            (&self.learner, 120),
            (&self.classroom, 120),
            (&self.prompt, 4000),
        ] {
            if value.trim().is_empty() || value.chars().count() > max {
                return Err(bad(
                    "Learner, class and prompt are required within their length limits",
                ));
            }
        }
        if !matches!(self.task.as_str(), "speaking" | "read_aloud") {
            return Err(bad("Task must be speaking or read_aloud"));
        }
        if self.reference_text.chars().count() > 4000 {
            return Err(bad("Reference text is limited to 4000 characters"));
        }
        if self.task == "read_aloud"
            && self.reference_text.trim().is_empty()
            && self.paragraph_photo.is_empty()
        {
            return Err(bad(
                "Read-aloud tasks require a paragraph photo or reference text",
            ));
        }
        if !self.paragraph_photo.is_empty()
            && (!self.reference_text.trim().is_empty() || self.task != "read_aloud")
        {
            return Err(bad(
                "Choose a photo or typed passage for read-aloud, not both",
            ));
        }
        if self.paragraph_photo.len() > 12 * 1024 * 1024 {
            return Err(bad("Paragraph photo exceeds 12 MiB"));
        }
        if !self.recorded_at.is_empty()
            && time::OffsetDateTime::parse(
                &self.recorded_at,
                &time::format_description::well_known::Rfc3339,
            )
            .is_err()
        {
            return Err(bad(
                "recorded_at must be an ISO 8601 timestamp with timezone",
            ));
        }
        if self.student_id.chars().count() > 120
            || !self
                .student_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            || self.source_id.len() > 250
            || self.student_id.starts_with("legacy-assessment-")
        {
            return Err(bad("Invalid student or submission ID"));
        }
        if !self.source_id.is_empty() && self.student_id.trim().is_empty() {
            return Err(bad("A submission ID requires a stable student ID"));
        }
        if !matches!(self.audio_mode.as_str(), "learner_only" | "conversation") {
            return Err(bad("Audio mode must be learner_only or conversation"));
        }
        if !matches!(self.detect_language.as_str(), "true" | "false") {
            return Err(bad("detect_language must be true or false"));
        }
        if self.audio_mode == "conversation" && self.student_reference.is_empty() {
            return Err(bad(
                "Conversation mode requires a student-only reference recording",
            ));
        }
        if self.audio_mode == "learner_only" && !self.student_reference.is_empty() {
            return Err(bad("Student reference requires conversation mode"));
        }
        Ok(())
    }
}
async fn submit(State(app): State<App>, mut multipart: Multipart) -> Result<impl IntoResponse> {
    let permit = app.inference.clone().try_acquire_owned().map_err(|_| {
        Error(
            StatusCode::TOO_MANY_REQUESTS,
            "A recording is being assessed; wait for it to finish".into(),
        )
    })?;
    let mut input = Submission {
        audio_mode: "learner_only".into(),
        detect_language: "false".into(),
        ..Default::default()
    };
    let mut seen = std::collections::HashSet::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| bad("Invalid multipart upload"))?
    {
        let name = field.name().unwrap_or("").to_string();
        if !seen.insert(name.clone()) {
            return Err(bad("Duplicate upload field"));
        }
        if matches!(
            name.as_str(),
            "audio" | "student_reference" | "paragraph_photo"
        ) {
            let target = match name.as_str() {
                "audio" => &mut input.audio,
                "student_reference" => &mut input.student_reference,
                _ => &mut input.paragraph_photo,
            };
            *target = field
                .bytes()
                .await
                .map_err(|_| bad("Invalid audio upload"))?
                .to_vec();
        } else {
            let target = match name.as_str() {
                "learner" => &mut input.learner,
                "student_id" => &mut input.student_id,
                "source_id" => &mut input.source_id,
                "recorded_at" => &mut input.recorded_at,
                "classroom" => &mut input.classroom,
                "language" => &mut input.language,
                "prompt" => &mut input.prompt,
                "task" => &mut input.task,
                "reference_text" => &mut input.reference_text,
                "audio_mode" => &mut input.audio_mode,
                "detect_language" => &mut input.detect_language,
                _ => {
                    return Err(bad(
                        "Unknown upload field; transcripts and teacher scores are not accepted",
                    ));
                }
            };
            *target = field.text().await.map_err(|_| bad("Invalid text field"))?;
        }
    }
    input.validate()?;
    if input.audio.is_empty() {
        return Err(bad("Select an audio file"));
    }
    let known_silence =
        audio::decode(&input.audio).is_ok_and(|(_, stats)| stats.energy_active_seconds < 0.2);
    if app.adapter.is_none() {
        if !known_silence {
            inference::readiness(&input.language)
                .map_err(|message| Error(StatusCode::SERVICE_UNAVAILABLE, message))?;
        }
        if input.audio_mode == "conversation" {
            inference::diarization_readiness()
                .map_err(|message| Error(StatusCode::SERVICE_UNAVAILABLE, message))?;
        }
        if input.detect_language == "true" {
            inference::language_readiness()
                .map_err(|message| Error(StatusCode::SERVICE_UNAVAILABLE, message))?;
        }
    }
    let (id, duplicate) = persist_job(&app.db, &input).await?;
    if duplicate {
        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({"id":id,"status":"duplicate"})),
        ));
    }
    tokio::spawn(async move {
        let _permit = permit;
        worker(&app, id, input).await;
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"id":id,"status":"accepted"})),
    ))
}
async fn persist_job(db: &SqlitePool, input: &Submission) -> Result<(i64, bool)> {
    let mut tx = db.begin().await?;
    let source = (!input.source_id.is_empty()).then_some(input.source_id.as_str());
    if let Some(source) = source
        && let Some((id, student_id)) = sqlx::query_as::<_, (i64, String)>(
            "SELECT id,student_id FROM assessment_jobs WHERE source_id=?",
        )
        .bind(source)
        .fetch_optional(&mut *tx)
        .await?
    {
        if student_id != input.student_id {
            return Err(bad("Submission ID already belongs to another student"));
        }
        return Ok((id, true));
    }
    let result=sqlx::query("INSERT INTO assessment_jobs (learner,student_id,source_id,classroom,language,prompt,task,reference_text,audio_mode,detect_language,status) VALUES (?,?,?,?,?,?,?,?,?,?,'accepted')")
        .bind(input.learner.trim()).bind(input.student_id.trim()).bind(source).bind(input.classroom.trim())
        .bind(&input.language).bind(input.prompt.trim()).bind(&input.task).bind(input.reference_text.trim())
        .bind(&input.audio_mode).bind(input.detect_language=="true").execute(&mut *tx).await?;
    let id = result.last_insert_rowid();
    if input.student_id.trim().is_empty() {
        sqlx::query("UPDATE assessment_jobs SET student_id=? WHERE id=?")
            .bind(format!("legacy-assessment-{id}"))
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE assessment_jobs SET recorded_at=CASE WHEN ?='' THEN created_at ELSE ? END WHERE id=?").bind(&input.recorded_at).bind(&input.recorded_at).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((id, false))
}
async fn worker(app: &App, id: i64, input: Submission) {
    let result = match staging::Prepared::new(input) {
        Ok(prepared) => prepared.execute(app, id).await,
        Err(message) => Err(Error(StatusCode::INTERNAL_SERVER_ERROR, message)),
    };
    record_failure(app, id, result).await;
}
async fn prepared_worker(app: &App, id: i64, prepared: staging::Prepared) {
    record_failure(app, id, prepared.execute(app, id).await).await;
}
async fn record_failure(app: &App, id: i64, result: Result<()>) {
    if let Err(error) = result {
        eprintln!("Assessment {id} failed: {}", error.1);
        let _ = sqlx::query("UPDATE assessment_jobs SET status='failed',error=? WHERE id=?")
            .bind(error.1)
            .bind(id)
            .execute(&app.db)
            .await;
        let _ = sqlx::query("INSERT INTO job_events (job_id,stage) VALUES (?,'failed')")
            .bind(id)
            .execute(&app.db)
            .await;
    }
}
async fn stage(app: &App, id: i64, name: &str) -> Result<()> {
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE assessment_jobs SET status=? WHERE id=?")
        .bind(name)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO job_events (job_id,stage) VALUES (?,?)")
        .bind(id)
        .bind(name)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
async fn process(
    app: &App,
    id: i64,
    file: tempfile::NamedTempFile,
    mut input: Submission,
) -> Result<()> {
    let ocr_result = if !input.paragraph_photo.is_empty() {
        stage(app, id, "extracting_text").await?;
        let result = ocr::extract(std::mem::take(&mut input.paragraph_photo), &input.language)
            .await
            .map_err(|message| Error(StatusCode::UNPROCESSABLE_ENTITY, message))?;
        input.reference_text = result.text.clone();
        sqlx::query("UPDATE assessment_jobs SET reference_text=? WHERE id=?")
            .bind(&input.reference_text)
            .bind(id)
            .execute(&app.db)
            .await?;
        Some(result)
    } else {
        None
    };
    stage(app, id, "converting").await?;
    let (file, stats) = audio::normalize(file)
        .await
        .map_err(|message| Error(StatusCode::UNPROCESSABLE_ENTITY, message))?;
    let student_reference = if input.audio_mode == "conversation" {
        let mut reference = tempfile::Builder::new()
            .suffix(".upload")
            .tempfile()
            .map_err(|_| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cannot create student reference file".into(),
                )
            })?;
        reference.write_all(&input.student_reference).map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot write student reference".into(),
            )
        })?;
        let (reference, reference_stats) = audio::normalize(reference)
            .await
            .map_err(|message| Error(StatusCode::UNPROCESSABLE_ENTITY, message))?;
        if !(3.0..=30.0).contains(&reference_stats.duration_seconds)
            || reference_stats.energy_active_seconds < 2.0
        {
            return Err(bad(
                "Student reference must be 3–30 seconds with at least 2 seconds of active signal",
            ));
        }
        Some(reference)
    } else {
        None
    };
    stage(app, id, "audio_checked").await?;
    let enhanced = student_reference.is_some() || input.detect_language == "true";
    let transcript = if stats.energy_active_seconds < 0.2 && !enhanced {
        inference::Transcript {
            text: String::new(),
            backend: "energy-gate/no-asr".into(),
            ..Default::default()
        }
    } else {
        stage(
            app,
            id,
            if enhanced {
                "processing_speech"
            } else {
                "transcribing"
            },
        )
        .await?;
        recognize(
            app,
            file.path(),
            &input.language,
            student_reference.as_ref().map(|file| file.path()),
            input.detect_language == "true",
        )
        .await?
    };
    // Drop the temporary audio before generating and saving the report.
    drop(file);
    drop(student_reference);
    stage(app, id, "analyzing").await?;
    let reference = if input.task == "read_aloud" {
        input.reference_text
    } else {
        String::new()
    };
    let analysis_reference = reference.clone();
    let mut report = tokio::task::spawn_blocking(move || {
        analysis::analyze(transcript, stats, &analysis_reference)
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Analysis task failed".into(),
        )
    })?;
    language_analysis(
        app,
        id,
        &mut report,
        &reference,
        &input.language,
        &input.task,
    )
    .await?;
    if let Some(ocr) = &ocr_result
        && !ocr.reliable_for_scoring
    {
        report.reading_screen = None;
        report
            .warnings
            .push("uncertain_ocr_reference_score_withheld".into());
        report.next_steps = vec!["retake_paragraph_photo".into()];
        if report.outcome == "measured" {
            report.outcome = "uncertain_reference".into();
        }
    }
    report.ocr = ocr_result;
    write_summary(app, id, report, &input.task).await
}
async fn language_analysis(
    app: &App,
    id: i64,
    report: &mut analysis::Report,
    reference: &str,
    language: &str,
    task: &str,
) -> Result<()> {
    stage(app, id, "linguistic_analysis").await?;
    report.nlp = Some(nlp::analyze(report, reference, language, task).await);
    if report
        .nlp
        .as_ref()
        .is_some_and(|nlp| nlp.status == "complete")
    {
        report.unavailable.retain(|code| {
            code != "grammar_and_pos_models_not_integrated"
                && code != "grammar_error_model_not_integrated"
        });
        report
            .unavailable
            .push("grammar_error_model_not_integrated".into());
    }
    stage(app, id, "grammar_analysis").await?;
    report.grammar = Some(grammar::analyze(report, task, language).await);
    if report
        .grammar
        .as_ref()
        .is_some_and(|g| matches!(g.status.as_str(), "complete" | "partial"))
    {
        report
            .unavailable
            .retain(|code| code != "grammar_error_model_not_integrated");
    }
    Ok(())
}
async fn write_summary(app: &App, id: i64, mut report: analysis::Report, task: &str) -> Result<()> {
    // Commit measurements before loading the model, so a crash/OOM during the
    // required summary can be retried without retaining the child's raw audio.
    let mut pending = slm::SlmReport::unavailable("slm_summary_pending");
    pending.status = "pending".into();
    report.slm = Some(pending);
    let core = serde_json::to_string(&report).map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Report serialization failed".into(),
        )
    })?;
    let mut tx = app.db.begin().await?;
    sqlx::query(
        "UPDATE assessment_jobs SET status='writing_summary',report_json=?,error=NULL WHERE id=?",
    )
    .bind(core)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO job_events (job_id,stage) VALUES (?,'writing_summary')")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    report.slm = Some(slm::recommend(&report, &app.db, task, &app.summary_backend).await);
    let ready = report.slm.as_ref().is_some_and(slm::SlmReport::ready);
    let json = serde_json::to_string(&report).map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Report serialization failed".into(),
        )
    })?;
    // Preserve completed measurements even if the required summary is unavailable.
    // The summary-only retry endpoint does not need the original raw recording.
    sqlx::query("UPDATE assessment_jobs SET report_json=?,error=NULL WHERE id=?")
        .bind(&json)
        .bind(id)
        .execute(&app.db)
        .await?;
    if !ready {
        return Err(Error(StatusCode::SERVICE_UNAVAILABLE,"Required local summary unavailable. Check model/memory and retry summary; measurements have been preserved.".into()));
    }
    stage(app, id, "reporting").await?;
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE assessment_jobs SET status='complete',report_json=? WHERE id=?")
        .bind(json)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO job_events (job_id,stage) VALUES (?,'complete')")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
async fn recognize(
    app: &App,
    path: &std::path::Path,
    lang: &str,
    student_reference: Option<&std::path::Path>,
    detect_language: bool,
) -> Result<inference::Transcript> {
    let mut command = if let Some(adapter) = &app.adapter {
        Command::new(adapter)
    } else {
        let executable = std::env::current_exe().map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot locate inference executable".into(),
            )
        })?;
        let mut command = Command::new(executable);
        command.arg("--infer");
        command
    };
    command.arg(path).arg(lang);
    if let Some(reference) = student_reference {
        command.arg("--student-reference").arg(reference);
    }
    if detect_language {
        command.arg("--detect-language");
    }
    command
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1");
    let output = local_model(&mut command, Duration::from_secs(600))
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                Error(
                    StatusCode::GATEWAY_TIMEOUT,
                    "Local inference exceeded the 10-minute timeout".into(),
                )
            } else {
                Error(
                    StatusCode::BAD_GATEWAY,
                    "Cannot start local inference".into(),
                )
            }
        })?;
    if !output.status.success() {
        // Do not log model output or raw transcripts; stderr may contain learner information.
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "Local inference failed; verify model files and runtime compatibility".into(),
        ));
    }
    if output.stdout.len() > 256_000 {
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "Inference output exceeds limit".into(),
        ));
    }
    let transcript: inference::Transcript =
        serde_json::from_slice(&output.stdout).map_err(|_| {
            Error(
                StatusCode::BAD_GATEWAY,
                "Inference returned invalid JSON".into(),
            )
        })?;
    if transcript.text.chars().count() > 30000 {
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "Transcript exceeds limit".into(),
        ));
    }
    if student_reference.is_some()
        && !transcript
            .speech
            .as_ref()
            .is_some_and(|e| e.diarization_applied)
    {
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "Inference did not provide requested speaker separation evidence".into(),
        ));
    }
    if student_reference.is_some() && !transcript.text.trim().is_empty() {
        let evidence = transcript
            .speech
            .as_ref()
            .expect("speaker evidence checked above");
        if !evidence.student_identified
            || evidence.selected_speaker.is_none()
            || transcript.student_audio.is_none()
            || transcript.segments.is_empty()
            || transcript
                .segments
                .iter()
                .any(|segment| segment.speaker != evidence.selected_speaker)
        {
            return Err(Error(
                StatusCode::BAD_GATEWAY,
                "Unverified speaker transcript rejected".into(),
            ));
        }
    }
    if detect_language
        && !transcript.text.trim().is_empty()
        && !transcript
            .speech
            .as_ref()
            .is_some_and(|e| e.language_detection_applied)
    {
        return Err(Error(
            StatusCode::BAD_GATEWAY,
            "Inference did not provide requested language detection evidence".into(),
        ));
    }
    Ok(transcript)
}

async fn local_model(
    command: &mut Command,
    timeout: Duration,
) -> std::io::Result<std::process::Output> {
    // Unit-test harness binaries have no CLI dispatcher. Their explicit fixture
    // programs run directly; the real supervisor is exercised by native smoke tests.
    #[cfg(test)]
    {
        tokio::time::timeout(timeout, command.kill_on_drop(true).output())
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "Model timed out"))?
    }
    #[cfg(not(test))]
    {
        ela_local::process_control::run_model(command, timeout).await
    }
}

async fn local_browser_only(
    State(app): State<App>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let hosts = [
        format!("127.0.0.1:{}", app.port),
        format!("localhost:{}", app.port),
    ];
    if let Some(host) = request.headers().get(axum::http::header::HOST)
        && !host
            .to_str()
            .is_ok_and(|host| hosts.iter().any(|allowed| host == allowed))
    {
        return Error(
            StatusCode::FORBIDDEN,
            "Use the local application URL".into(),
        )
        .into_response();
    }
    if let Some(origin) = request.headers().get(axum::http::header::ORIGIN)
        && !origin
            .to_str()
            .is_ok_and(|origin| hosts.iter().any(|host| origin == format!("http://{host}")))
    {
        return Error(
            StatusCode::FORBIDDEN,
            "Cross-origin requests are not allowed".into(),
        )
        .into_response();
    }
    next.run(request).await
}
fn router(app: App) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/students", get(progress::students))
        .route("/api/students/{student_id}", get(progress::student))
        .route("/api/collection/form", get(collection::form))
        .route(
            "/api/collection/import",
            axum::routing::post(collection::import).layer(DefaultBodyLimit::max(128 * 1024 * 1024)),
        )
        .route("/api/assessments", get(list).post(submit))
        .route("/api/assessments/{id}", get(detail).delete(remove))
        .route("/api/assessments/{id}/events", get(events))
        .route(
            "/api/assessments/{id}/linguistics",
            axum::routing::post(regenerate_language),
        )
        .route(
            "/api/assessments/{id}/summary",
            axum::routing::post(regenerate_summary),
        )
        .route("/api/legacy-assessments", get(legacy))
        .fallback_service(ServeDir::new("web"))
        .layer(axum::middleware::from_fn_with_state(
            app.clone(),
            local_browser_only,
        ))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(concat!(
                "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; ",
                "img-src 'self' data:; object-src 'none'; frame-ancestors 'none'; ",
                "base-uri 'self'; form-action 'self'",
            )),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .with_state(app)
}
#[tokio::main(worker_threads = 2)]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--model-worker") {
        if args.len() != 3 {
            return Err("Internal model supervisor expects a request file".into());
        }
        ela_local::process_control::run_spec(std::path::Path::new(&args[2]))?;
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--asr-windows") {
        #[cfg(feature = "native-asr")]
        {
            ela_local::process_control::watch_parent_pipe();
            if args.len() != 5 {
                return Err("Internal ASR worker expects audio, language and window file".into());
            }
            let windows: Vec<ela_local::language_policy::Window> =
                serde_json::from_slice(&std::fs::read(&args[4])?)?;
            let result =
                native_speech::decode_windows(std::path::Path::new(&args[2]), &args[3], &windows)?;
            println!("{}", serde_json::to_string(&result)?);
            return Ok(());
        }
        #[cfg(not(feature = "native-asr"))]
        return Err("Native ASR is disabled".into());
    }
    if args.get(1).map(String::as_str) == Some("--slm") {
        if args.len() != 3 {
            return Err("Usage: ela-local --slm REPORT.json".into());
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[2])?)?;
        let task = value
            .get("task")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let report: analysis::Report =
            serde_json::from_value(value.get("report").cloned().unwrap_or(value))?;
        let db = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::query(
            "CREATE TABLE suggestion_cache(cache_key TEXT PRIMARY KEY,plan_json TEXT NOT NULL)",
        )
        .execute(&db)
        .await?;
        let result = slm::recommend(&report, &db, &task, &slm::Backend::Local).await;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--ocr") {
        if args.len() != 4 {
            return Err("Usage: ela-local --ocr PARAGRAPH_PHOTO te|en".into());
        }
        let result = ocr::extract(std::fs::read(&args[2])?, &args[3]).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--grammar") {
        if args.len() != 3 {
            return Err("Usage: ela-local --grammar EXPORTED_ASSESSMENT.json".into());
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[2])?)?;
        let task = value
            .get("task")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("speaking")
            .to_owned();
        let language = value
            .get("language")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("en")
            .to_owned();
        let mut report: analysis::Report =
            serde_json::from_value(value.get("report").cloned().unwrap_or(value))?;
        report.nlp = Some(nlp::analyze(&report, "", &language, &task).await);
        let result = grammar::analyze(&report, &task, &language).await;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--nlp") {
        if args.len() != 4 {
            return Err("Usage: ela-local --nlp TEXT_FILE te|en".into());
        }
        let result = nlp::run(&std::fs::read_to_string(&args[2])?, &args[3]).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--infer") {
        if args.len() < 4 {
            return Err("Usage: ela-local --infer audio.wav te|en [--student-reference sample.wav] [--detect-language]".into());
        }
        let mut options = inference::Options::default();
        let mut flags = args[4..].iter();
        while let Some(flag) = flags.next() {
            match flag.as_str() {
                "--student-reference" => {
                    options.student_reference =
                        Some(flags.next().ok_or("Missing student reference path")?.into())
                }
                "--detect-language" => options.detect_language = true,
                _ => return Err(format!("Unknown inference option: {flag}").into()),
            }
        }
        let transcript = inference::run(std::path::Path::new(&args[2]), &args[3], options)?;
        println!("{}", serde_json::to_string(&transcript)?);
        return Ok(());
    }
    if args.len() > 1 {
        return Err("Unknown command".into());
    }
    // Claim the single-instance listener before recovering interrupted jobs.
    let port = std::env::var("ELA_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse::<u16>()?;
    if port < 1024 {
        return Err("ELA_PORT must be 1024–65535".into());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let db = database(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://ela.db?mode=rwc".into()),
    )
    .await?;
    let app = App {
        db,
        port,
        adapter: std::env::var("ELA_ASR_COMMAND").ok(),
        inference: Arc::new(Semaphore::new(1)),
        summary_backend: slm::Backend::Local,
    };
    println!("ELA Local: http://127.0.0.1:{port}");
    axum::serve(listener, router(app)).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
