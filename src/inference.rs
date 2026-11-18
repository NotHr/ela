//! Native models run in a child Rust executable, allowing hard timeouts and
//! isolation from the web event loop. No Python is required.
use crate::{
    audio::AudioStats,
    speech::{Evidence, Segment},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    #[serde(default = "adapter_backend")]
    pub backend: String,
    #[serde(default)]
    pub model_revision: Option<String>,
    #[serde(default)]
    pub segments: Vec<Segment>,
    #[serde(default)]
    pub speech: Option<Evidence>,
    #[serde(default)]
    pub student_audio: Option<AudioStats>,
}

#[derive(Default)]
pub struct Options {
    pub student_reference: Option<PathBuf>,
    pub detect_language: bool,
}

fn adapter_backend() -> String {
    "external-local-adapter".into()
}

pub(crate) fn required_file(key: &str) -> Result<String, String> {
    let value = std::env::var(key).map_err(|_| format!("Set {key} to a local model file"))?;
    if !Path::new(&value).is_file() {
        return Err(format!("{key} does not point to a local file"));
    }
    Ok(value)
}

pub fn readiness(language: &str) -> Result<(), String> {
    if !cfg!(feature = "native-asr") {
        return Err("Build with native-asr or configure ELA_ASR_COMMAND".into());
    }
    let keys: &[&str] = match language {
        "te" => &["ELA_TE_MODEL", "ELA_TE_TOKENS"],
        "en" => &["ELA_EN_ENCODER", "ELA_EN_DECODER", "ELA_EN_TOKENS"],
        _ => return Err("Unsupported language".into()),
    };
    for key in keys {
        required_file(key)?;
    }
    Ok(())
}

pub fn diarization_readiness() -> Result<(), String> {
    if !cfg!(feature = "native-asr") {
        return Err("Native speech processing is disabled".into());
    }
    for key in ["ELA_DIARIZATION_MODEL", "ELA_SPEAKER_MODEL"] {
        required_file(key)?;
    }
    Ok(())
}

pub fn language_readiness() -> Result<(), String> {
    readiness("te")?;
    readiness("en")?;
    required_file("ELA_LID_MODEL")?;
    language_helper()?;
    Ok(())
}

pub(crate) fn language_helper() -> Result<PathBuf, String> {
    let path = if let Some(path) = std::env::var_os("ELA_LID_EXECUTABLE") {
        PathBuf::from(path)
    } else {
        std::env::current_exe()
            .map_err(|_| "Cannot locate language helper")?
            .with_file_name(format!("ela-language-id{}", std::env::consts::EXE_SUFFIX))
    };
    if !path.is_file() {
        return Err(
            "Language helper missing; run cargo build --bins, or set ELA_LID_EXECUTABLE".into(),
        );
    }
    Ok(path)
}

#[cfg(feature = "native-asr")]
pub(crate) fn fingerprint(language: &str, options: &Options) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut keys = if options.detect_language {
        vec![
            "ELA_TE_MODEL",
            "ELA_TE_TOKENS",
            "ELA_EN_ENCODER",
            "ELA_EN_DECODER",
            "ELA_EN_TOKENS",
            "ELA_LID_MODEL",
        ]
    } else if language == "te" {
        vec!["ELA_TE_MODEL", "ELA_TE_TOKENS"]
    } else {
        vec!["ELA_EN_ENCODER", "ELA_EN_DECODER", "ELA_EN_TOKENS"]
    };
    if options.student_reference.is_some() {
        keys.extend(["ELA_DIARIZATION_MODEL", "ELA_SPEAKER_MODEL"]);
    }
    keys.sort();
    let mut hash = Sha256::new();
    hash.update(b"ela-native-v2;per-file-sha256;asr-coalesce-25s;sherpa-2.0.1;whisper-rs-0.16");
    for flag in [
        option_env!("GGML_NATIVE").unwrap_or("OFF"),
        option_env!("GGML_AVX2").unwrap_or("OFF"),
        option_env!("GGML_FMA").unwrap_or("OFF"),
    ] {
        hash.update(flag.as_bytes());
    }
    for key in keys {
        let digest = ela_local::model_files::hash_file(Path::new(&required_file(key)?))
            .map_err(|_| "Cannot fingerprint model file")?;
        hash.update(key.as_bytes());
        hash.update(digest.as_bytes());
    }
    for (key, default) in [
        ("ELA_ASR_THREADS", "2"),
        ("ELA_SPEAKER_MATCH_THRESHOLD", "0.65"),
        ("ELA_SPEAKER_MATCH_MARGIN", "0.10"),
    ] {
        hash.update(key.as_bytes());
        hash.update(
            std::env::var(key)
                .unwrap_or_else(|_| default.into())
                .as_bytes(),
        );
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

#[cfg(feature = "native-asr")]
pub fn run(path: &Path, language: &str, options: Options) -> Result<Transcript, String> {
    crate::native_speech::run(path, language, options)
}

#[cfg(not(feature = "native-asr"))]
pub fn run(_path: &Path, _language: &str, _options: Options) -> Result<Transcript, String> {
    Err("Native ASR was disabled at build time".into())
}
