//! Bounded ELA Collect/ODK submission ZIP reader for our shipped XForm schema.
//! No archive path is ever extracted to the filesystem.
use crate::Submission;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

pub struct Item {
    pub filename: String,
    pub submission: Result<Submission, String>,
}
#[derive(Serialize)]
pub struct ImportItem {
    pub filename: String,
    pub id: Option<i64>,
    pub status: String,
    pub error: Option<String>,
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && !path.starts_with('/')
        && !path.contains(['\\', ':', '\0'])
        && path
            .trim_end_matches('/')
            .split('/')
            .all(|part| !matches!(part, ".." | "." | ""))
}
fn media(files: &BTreeMap<String, Vec<u8>>, xml: &str, name: &str) -> Result<Vec<u8>, String> {
    if !safe_path(name) {
        return Err("Invalid media filename in submission".into());
    }
    let directory = xml
        .rsplit_once('/')
        .map(|(directory, _)| format!("{directory}/"))
        .unwrap_or_default();
    if let Some(bytes) = files.get(&format!("{directory}{name}")) {
        return Ok(bytes.clone());
    }
    let matches = files
        .iter()
        .filter(|(path, _)| path.rsplit('/').next() == Some(name))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Missing or ambiguous submission media".into());
    }
    Ok(matches[0].1.clone())
}
fn submission(
    files: &BTreeMap<String, Vec<u8>>,
    filename: &str,
    bytes: &[u8],
) -> Result<Submission, String> {
    if bytes.len() > 128 * 1024 {
        return Err("Submission XML exceeds limit".into());
    }
    let xml = std::str::from_utf8(bytes).map_err(|_| "Submission XML must be UTF-8")?;
    let document = roxmltree::Document::parse(xml)
        .map_err(|_| "Invalid XML (DTDs/entities are not supported)")?;
    let root = document.root_element();
    if root.attribute("id") != Some("ela_reading_v1") {
        return Err("Unsupported form; use forms/ela-reading.xml".into());
    }
    let field = |name: &str| {
        root.children()
            .find(|node| node.is_element() && node.tag_name().name() == name)
            .and_then(|node| node.text())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let instance = root
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "instanceID")
        .and_then(|node| node.text())
        .unwrap_or("")
        .trim();
    if instance.is_empty() || instance.len() > 200 {
        return Err("Missing or invalid ODK instanceID".into());
    }
    let id = field("student_id");
    if id.is_empty() {
        return Err("Student ID is required".into());
    }
    let learner = field("student_name");
    let mode = field("audio_mode");
    let switching = field("detect_language");
    let mut input = Submission {
        student_id: id.clone(),
        learner: if learner.is_empty() { id } else { learner },
        classroom: field("classroom"),
        recorded_at: field("recorded_at"),
        language: field("language"),
        prompt: field("activity"),
        task: "read_aloud".into(),
        audio_mode: if mode.is_empty() {
            "learner_only".into()
        } else {
            mode
        },
        detect_language: if switching.is_empty() {
            "true".into()
        } else {
            switching
        },
        reference_text: field("reference_text"),
        source_id: format!("ela_reading_v1:{instance}"),
        audio: media(files, filename, &field("recording"))?,
        ..Default::default()
    };
    let photo = field("paragraph_photo");
    if !photo.is_empty() {
        input.paragraph_photo = media(files, filename, &photo)?;
    }
    let reference = field("student_reference");
    if !reference.is_empty() {
        input.student_reference = media(files, filename, &reference)?;
    }
    input.validate().map_err(|error| error.1)?;
    Ok(input)
}
pub fn parse(bytes: Vec<u8>) -> Result<Vec<Item>, String> {
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("Collection ZIP exceeds 128 MiB".into());
    }
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "Invalid ZIP archive")?;
    if archive.len() > 512 {
        return Err("Too many files in collection ZIP".into());
    }
    let mut files = BTreeMap::new();
    let mut total = 0_usize;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| "Cannot read ZIP member")?;
        if !safe_path(file.name())
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Unsafe archive member".into());
        }
        if file.is_dir() {
            continue;
        }
        if file.size() > 64 * 1024 * 1024 {
            return Err("Archive member exceeds 64 MiB".into());
        }
        let name = file.name().to_owned();
        if files.contains_key(&name) {
            return Err("Duplicate archive member".into());
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|_| "Cannot decompress ZIP member")?;
        total += data.len();
        if data.len() > 64 * 1024 * 1024 || total > 256 * 1024 * 1024 {
            return Err("Decompressed ZIP exceeds limits".into());
        }
        files.insert(name, data);
    }
    let xmls = files
        .iter()
        .filter(|(name, _)| name.to_ascii_lowercase().ends_with(".xml"))
        .collect::<Vec<_>>();
    if xmls.is_empty() || xmls.len() > 64 {
        return Err("ZIP must contain 1–64 submission XML files".into());
    }
    Ok(xmls
        .into_iter()
        .map(|(name, bytes)| Item {
            filename: name.clone(),
            submission: submission(&files, name, bytes),
        })
        .collect())
}

pub async fn import(
    axum::extract::State(app): axum::extract::State<crate::App>,
    mut multipart: axum::extract::Multipart,
) -> crate::Result<impl axum::response::IntoResponse> {
    use axum::{Json, http::StatusCode};
    let permit = app.inference.clone().try_acquire_owned().map_err(|_| {
        crate::Error(
            StatusCode::TOO_MANY_REQUESTS,
            "Desktop is processing a recording or collection".into(),
        )
    })?;
    let field = multipart
        .next_field()
        .await
        .map_err(|_| crate::bad("Invalid collection upload"))?
        .ok_or_else(|| crate::bad("Choose a collection ZIP"))?;
    if field.name() != Some("archive") {
        return Err(crate::bad("Expected archive field"));
    }
    let bytes = field
        .bytes()
        .await
        .map_err(|_| crate::bad("Cannot read collection ZIP"))?
        .to_vec();
    if multipart
        .next_field()
        .await
        .map_err(|_| crate::bad("Invalid multipart upload"))?
        .is_some()
    {
        return Err(crate::bad("Only one collection ZIP is accepted"));
    }
    let items = tokio::task::spawn_blocking(move || parse(bytes))
        .await
        .map_err(|_| crate::bad("Collection parsing failed"))?
        .map_err(|message| crate::bad(&message))?;
    let mut results = Vec::new();
    let mut queued = Vec::new();
    for item in items {
        match item.submission {
            Err(error) => results.push(ImportItem {
                filename: item.filename,
                id: None,
                status: "invalid".into(),
                error: Some(error),
            }),
            Ok(input) => {
                let prepared = match crate::staging::Prepared::new(input) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        results.push(ImportItem {
                            filename: item.filename,
                            id: None,
                            status: "failed".into(),
                            error: Some(error),
                        });
                        continue;
                    }
                };
                match crate::persist_job(&app.db, &prepared.input).await {
                    Ok((id, duplicate)) => {
                        results.push(ImportItem {
                            filename: item.filename,
                            id: Some(id),
                            status: if duplicate { "duplicate" } else { "accepted" }.into(),
                            error: None,
                        });
                        if !duplicate {
                            queued.push((id, prepared));
                        }
                    }
                    Err(error) => results.push(ImportItem {
                        filename: item.filename,
                        id: None,
                        status: "failed".into(),
                        error: Some(error.1),
                    }),
                }
            }
        }
    }
    tokio::spawn(async move {
        let _permit = permit;
        for (id, prepared) in queued {
            crate::prepared_worker(&app, id, prepared).await;
        }
    });
    Ok((StatusCode::ACCEPTED, Json(results)))
}

pub async fn form() -> impl axum::response::IntoResponse {
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=ela-reading.xml",
            ),
        ],
        include_str!("../forms/ela-reading.xml"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn archive(path: &str, xml: &str) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file(path, options).unwrap();
        archive.write_all(xml.as_bytes()).unwrap();
        archive.start_file("a/voice.wav", options).unwrap();
        archive
            .write_all(&crate::audio::fixture(16000, 1, 0))
            .unwrap();
        archive.finish().unwrap().into_inner()
    }
    #[test]
    fn parses_stable_identity_and_relative_media() {
        let xml = "<data id='ela_reading_v1'><student_id>S-001</student_id><student_name>లక్ష్మి</student_name><classroom>3</classroom><language>te</language><activity>పఠనం</activity><detect_language>false</detect_language><reference_text>నా పేరు లక్ష్మి</reference_text><recording>voice.wav</recording><meta><instanceID>uuid:one</instanceID></meta></data>";
        let mut items = parse(archive("a/submission.xml", xml)).unwrap();
        let input = items.remove(0).submission.ok().unwrap();
        assert_eq!(input.student_id, "S-001");
        assert_eq!(input.learner, "లక్ష్మి");
        assert!(!input.audio.is_empty());
        assert!(input.source_id.ends_with("uuid:one"));
    }
    #[test]
    fn rejects_path_traversal_and_wrong_forms() {
        assert!(parse(archive("../bad.xml", "<data/>")).is_err());
        assert!(
            parse(archive("a/submission.xml", "<data id='another'/>"))
                .unwrap()
                .remove(0)
                .submission
                .is_err()
        );
    }
}
