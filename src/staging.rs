//! Queue media on private disk, not in RAM alongside native models. A normal
//! completion/failure drops the directory; crash leftovers require retention cleanup.
use crate::{App, Error, Result, Submission};
use axum::http::StatusCode;
use std::io::Write;

pub struct Prepared {
    pub input: Submission,
    audio: tempfile::NamedTempFile,
    photo: Option<tempfile::NamedTempFile>,
    reference: Option<tempfile::NamedTempFile>,
    _directory: tempfile::TempDir,
}
fn file(
    directory: &std::path::Path,
    bytes: &[u8],
) -> std::result::Result<tempfile::NamedTempFile, String> {
    let mut file = tempfile::Builder::new()
        .suffix(".upload")
        .tempfile_in(directory)
        .map_err(|_| "Cannot stage media")?;
    file.write_all(bytes)
        .map_err(|_| "Cannot write staged media")?;
    Ok(file)
}
impl Prepared {
    pub fn new(mut input: Submission) -> std::result::Result<Self, String> {
        let base = std::env::var_os("ELA_WORK_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ".ela-work".into());
        std::fs::create_dir_all(&base).map_err(|_| "Cannot create local work directory")?;
        let directory = tempfile::Builder::new()
            .prefix("job-")
            .tempdir_in(base)
            .map_err(|_| "Cannot create private job directory")?;
        let audio = file(directory.path(), &std::mem::take(&mut input.audio))?;
        let photo = if input.paragraph_photo.is_empty() {
            None
        } else {
            Some(file(
                directory.path(),
                &std::mem::take(&mut input.paragraph_photo),
            )?)
        };
        let reference = if input.student_reference.is_empty() {
            None
        } else {
            Some(file(
                directory.path(),
                &std::mem::take(&mut input.student_reference),
            )?)
        };
        Ok(Self {
            input,
            audio,
            photo,
            reference,
            _directory: directory,
        })
    }
    pub async fn execute(mut self, app: &App, id: i64) -> Result<()> {
        let error = |_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot read staged media".into(),
            )
        };
        if let Some(file) = &self.photo {
            self.input.paragraph_photo = std::fs::read(file.path()).map_err(error)?;
        }
        if let Some(file) = &self.reference {
            self.input.student_reference = std::fs::read(file.path()).map_err(error)?;
        }
        crate::process(app, id, self.audio, self.input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_media_does_not_remain_in_the_in_memory_queue() {
        let input = Submission {
            audio: vec![1; 1024],
            paragraph_photo: vec![2; 1024],
            student_reference: vec![3; 1024],
            ..Default::default()
        };
        let prepared = Prepared::new(input).unwrap();
        let path = prepared._directory.path().to_owned();
        assert!(
            prepared.input.audio.is_empty()
                && prepared.input.paragraph_photo.is_empty()
                && prepared.input.student_reference.is_empty()
        );
        assert_eq!(
            std::fs::metadata(prepared.audio.path()).unwrap().len(),
            1024
        );
        drop(prepared);
        assert!(!path.exists());
    }
}
