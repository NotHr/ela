use hound::{SampleFormat, WavReader};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

pub const SAMPLE_RATE: usize = 16_000;
pub const MAX_SECONDS: usize = 300;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioStats {
    pub duration_seconds: f64,
    /// A simple energy measurement, NOT a trained voice activity detector.
    pub energy_active_seconds: f64,
    pub rms: f64,
    pub clipped_sample_percent: f64,
}

pub fn decode(bytes: &[u8]) -> Result<(Vec<f32>, AudioStats), String> {
    let mut reader = WavReader::new(Cursor::new(bytes))
        .map_err(|_| "Upload a valid PCM WAV file".to_string())?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.sample_rate != SAMPLE_RATE as u32
        || spec.bits_per_sample != 16
        || spec.sample_format != SampleFormat::Int
    {
        return Err("Audio must be mono, 16 kHz, 16-bit PCM WAV".into());
    }
    if reader.duration() == 0 || reader.duration() as usize > SAMPLE_RATE * MAX_SECONDS {
        return Err("Audio must be nonempty and at most 300 seconds".into());
    }
    let samples = reader
        .samples::<i16>()
        .map(|sample| sample.map(|v| v as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Corrupt WAV samples".to_string())?;
    if samples.is_empty() {
        return Err("Audio has no samples".into());
    }
    let stats = sample_stats(&samples);
    Ok((samples, stats))
}

pub fn sample_stats(samples: &[f32]) -> AudioStats {
    if samples.is_empty() {
        return AudioStats {
            duration_seconds: 0.0,
            energy_active_seconds: 0.0,
            rms: 0.0,
            clipped_sample_percent: 0.0,
        };
    }
    let square_sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
    let active_samples: usize = samples
        .chunks(320) // 20 ms frames
        .filter(|frame| {
            let power: f64 = frame.iter().map(|s| (*s as f64).powi(2)).sum();
            (power / frame.len() as f64).sqrt() >= 0.01
        })
        .map(|frame| frame.len())
        .sum();
    let clipped = samples.iter().filter(|s| s.abs() >= 0.999).count();
    AudioStats {
        duration_seconds: samples.len() as f64 / SAMPLE_RATE as f64,
        energy_active_seconds: active_samples as f64 / SAMPLE_RATE as f64,
        rms: (square_sum / samples.len() as f64).sqrt(),
        clipped_sample_percent: 100.0 * clipped as f64 / samples.len() as f64,
    }
}

/// Normalize locally using a bounded, network-disabled FFmpeg process. Already
/// valid PCM WAV avoids a subprocess. Original filenames are never shell input.
pub async fn normalize(
    file: tempfile::NamedTempFile,
) -> Result<(tempfile::NamedTempFile, AudioStats), String> {
    let bytes = tokio::fs::read(file.path())
        .await
        .map_err(|_| "Cannot read uploaded audio")?;
    if let Ok((_, stats)) = decode(&bytes) {
        return Ok((file, stats));
    }
    let output = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|_| "Cannot create converted audio file")?;
    let binary = std::env::var("ELA_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".into());
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        tokio::process::Command::new(binary)
            .args([
                "-nostdin", "-hide_banner", "-loglevel", "error", "-y",
                "-max_alloc", "67108864",
                "-protocol_whitelist", "file,pipe",
                // Do not allow playlists, concat scripts, or remote sources.
                "-format_whitelist",
                "wav,mp3,flac,ogg,mov,mp4,m4a,3gp,3g2,mj2,matroska,webm,aac,amr,asf,aiff,caf,ac3,eac3,ape,au,mpc,mpc8,tta,wv,w64",
                "-i",
            ])
            .arg(file.path())
            .args([
                "-map", "0:a:0", "-vn", "-sn", "-dn", "-map_metadata", "-1",
                "-threads", "2", "-ac", "1", "-ar", "16000",
                "-c:a", "pcm_s16le", "-t", "301", "-f", "wav",
            ])
            .arg(output.path())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "Audio conversion timed out")?
    .map_err(|_| "FFmpeg is unavailable; start with nix develop or install FFmpeg")?;
    if !result.status.success() {
        return Err("Unsupported or corrupt audio file".into());
    }
    let bytes = tokio::fs::read(output.path())
        .await
        .map_err(|_| "Cannot read converted audio")?;
    let (_, stats) = decode(&bytes)?;
    Ok((output, stats))
}

#[cfg(test)]
pub fn fixture(sample_rate: u32, seconds: usize, amplitude: i16) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut bytes,
        hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    for i in 0..sample_rate as usize * seconds {
        let sample = if i % 40 < 20 { amplitude } else { -amplitude };
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn converts_common_formats_and_rejects_non_audio() {
        use std::io::Write;
        let mut source = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        source.write_all(&fixture(16000, 1, 2000)).unwrap();
        for extension in ["mp3", "ogg", "m4a", "flac", "webm"] {
            let encoded = tempfile::Builder::new()
                .suffix(&format!(".{extension}"))
                .tempfile()
                .unwrap();
            let status = tokio::process::Command::new("ffmpeg")
                .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-i"])
                .arg(source.path())
                .arg(encoded.path())
                .status()
                .await
                .unwrap();
            assert!(status.success(), "Could not encode {extension}");
            // Match HTTP uploads: sniff bytes, not the original filename.
            let mut upload = tempfile::Builder::new()
                .suffix(".upload")
                .tempfile()
                .unwrap();
            upload
                .write_all(&std::fs::read(encoded.path()).unwrap())
                .unwrap();
            let (_, stats) = normalize(upload).await.unwrap();
            assert!((stats.duration_seconds - 1.0).abs() < 0.2, "{extension}");
        }
        let mut invalid = tempfile::NamedTempFile::new().unwrap();
        invalid.write_all(b"not an audio file").unwrap();
        assert!(normalize(invalid).await.is_err());
    }

    #[test]
    fn validates_format_and_measures_energy() {
        assert!(decode(b"invalid").is_err());
        assert!(decode(&fixture(44100, 1, 2000)).is_err());
        let (_, silence) = decode(&fixture(16000, 1, 0)).unwrap();
        assert_eq!(silence.energy_active_seconds, 0.0);
        let (_, speech) = decode(&fixture(16000, 1, 2000)).unwrap();
        assert_eq!(speech.duration_seconds, 1.0);
        assert_eq!(speech.energy_active_seconds, 1.0);
    }
}
