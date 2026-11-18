//! Native, local printed-text OCR. Never send images to an external service.
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Write},
    time::Duration,
};
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub mean_word_confidence: Option<f64>,
    pub low_confidence_words: usize,
    pub word_count: usize,
    pub reliable_for_scoring: bool,
    pub backend: String,
    pub model_revision: Option<String>,
}
fn command() -> Command {
    let mut cmd =
        Command::new(std::env::var("ELA_TESSERACT_BIN").unwrap_or_else(|_| "tesseract".into()));
    if let Ok(directory) = std::env::var("ELA_TESSDATA_DIR") {
        cmd.arg("--tessdata-dir").arg(directory);
    }
    cmd.env("OMP_THREAD_LIMIT", "2")
        .env("OMP_NUM_THREADS", "2")
        .kill_on_drop(true);
    cmd
}
pub async fn ready() -> bool {
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        command().arg("--list-langs").output(),
    )
    .await;
    result.is_ok_and(|result| {
        result.is_ok_and(|output| {
            let languages = String::from_utf8_lossy(&output.stdout);
            output.status.success()
                && ["tel", "eng"]
                    .iter()
                    .all(|lang| languages.lines().any(|line| line.trim() == *lang))
        })
    })
}
fn normalize(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Paragraph photo exceeds 12 MiB".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Invalid image")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(10000);
    limits.max_image_height = Some(10000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    use image::ImageDecoder;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| "Unsupported or corrupt paragraph photo")?;
    let orientation = decoder
        .orientation()
        .map_err(|_| "Cannot read photo orientation")?;
    let mut image =
        image::DynamicImage::from_decoder(decoder).map_err(|_| "Cannot decode paragraph photo")?;
    image.apply_orientation(orientation);
    let mut output = Cursor::new(Vec::new());
    image
        .grayscale()
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "Cannot prepare OCR photo")?;
    Ok(output.into_inner())
}
pub fn parse_tsv(tsv: &str) -> Result<OcrResult, String> {
    let mut lines: Vec<(String, Vec<String>)> = Vec::new();
    let mut confidence_sum = 0.0;
    let mut count = 0;
    let mut low = 0;
    for row in tsv.lines().skip(1) {
        let columns = row.splitn(12, '\t').collect::<Vec<_>>();
        if columns.len() != 12 || columns[0] != "5" || columns[11].trim().is_empty() {
            continue;
        }
        let confidence = columns[10]
            .parse::<f64>()
            .map_err(|_| "Invalid OCR confidence")?;
        if !confidence.is_finite() || !(0.0..=100.0).contains(&confidence) {
            return Err("Invalid OCR confidence".into());
        }
        let key = columns[1..5].join(":");
        if lines.last().is_none_or(|(last, _)| last != &key) {
            lines.push((key, Vec::new()));
        }
        lines.last_mut().unwrap().1.push(columns[11].trim().into());
        confidence_sum += confidence;
        count += 1;
        if confidence < 70.0 {
            low += 1;
        }
    }
    let text = lines
        .into_iter()
        .map(|(_, words)| words.join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() || text.chars().count() > 4000 {
        return Err("OCR found no paragraph or exceeded 4000 characters".into());
    }
    let mean = (count > 0).then(|| confidence_sum / count as f64);
    Ok(OcrResult {
        text,
        mean_word_confidence: mean,
        low_confidence_words: low,
        word_count: count,
        reliable_for_scoring: mean.is_some_and(|mean| mean >= 85.0) && low == 0,
        backend: "tesseract-lstm/tel+eng".into(),
        model_revision: std::env::var("ELA_OCR_MODEL_REVISION").ok(),
    })
}
pub async fn extract(bytes: Vec<u8>, language: &str) -> Result<OcrResult, String> {
    let languages = match language {
        "te" => "tel+eng",
        "en" => "eng+tel",
        _ => return Err("Unsupported OCR language".into()),
    };
    let png = tokio::task::spawn_blocking(move || normalize(&bytes))
        .await
        .map_err(|_| "Photo preparation failed")??;
    let mut photo = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|_| "Cannot create private OCR file")?;
    photo
        .write_all(&png)
        .map_err(|_| "Cannot write OCR photo")?;
    let output = crate::local_model(
        command()
            .arg(photo.path())
            .args(["stdout", "-l", languages, "--psm", "6", "tsv"]),
        Duration::from_secs(90),
    )
    .await
    .map_err(|_| "Tesseract unavailable or timed out; install local tel/eng OCR models")?;
    if !output.status.success() {
        return Err(
            "Local OCR failed; verify Telugu/English traineddata and paragraph photo".into(),
        );
    }
    if output.stdout.len() > 2_000_000 {
        return Err("OCR output exceeds limit".into());
    }
    let mut result =
        parse_tsv(&String::from_utf8(output.stdout).map_err(|_| "OCR output is not UTF-8")?)?;
    result.backend = format!("tesseract-lstm/{languages}");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconstructs_telugu_and_english_lines_and_gates_uncertain_photos() {
        let header = "level\tpage\tblock\tpar\tline\tword\tx\ty\tw\th\tconf\ttext\n";
        let good = format!(
            "{header}5\t1\t1\t1\t1\t1\t0\t0\t10\t10\t96\tనా\n5\t1\t1\t1\t1\t2\t0\t0\t10\t10\t95\tపేరు\n5\t1\t1\t1\t2\t1\t0\t0\t10\t10\t94\tHello\n"
        );
        let result = parse_tsv(&good).unwrap();
        assert_eq!(result.text, "నా పేరు\nHello");
        assert!(result.reliable_for_scoring);
        assert!(
            !parse_tsv(&good.replace("96\t", "40\t"))
                .unwrap()
                .reliable_for_scoring
        );
    }
    #[test]
    fn rejects_arbitrary_photo_bytes() {
        assert!(normalize(b"not an image").is_err());
    }
}
