//! Keep whisper.cpp and sherpa-onnx in separate processes: their native FFT
//! libraries export overlapping C symbols and must not be linked into one image.
use ela_local::language_policy::{Window, select_language};
use whisper_rs::{WhisperContext, WhisperContextParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The inference parent holds a pipe open. If it is killed/times out, EOF
    // terminates this helper too rather than leaving model work orphaned.
    ela_local::process_control::watch_parent_pipe();
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("Usage: ela-language-id MODEL AUDIO.wav WINDOWS.json".into());
    }
    let windows: Vec<Window> = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    if windows.len() > 300 {
        return Err("Too many language windows".into());
    }
    let mut wav = hound::WavReader::open(&args[2])?;
    let spec = wav.spec();
    if spec.channels != 1
        || spec.sample_rate != 16000
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
        || wav.duration() > 16000 * 300
    {
        return Err("Expected bounded mono 16 kHz PCM WAV".into());
    }
    let samples = wav
        .samples::<i16>()
        .map(|v| v.map(|v| v as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    whisper_rs::install_logging_hooks();
    let context = WhisperContext::new_with_params(&args[1], WhisperContextParameters::default())?;
    if !context.is_multilingual() {
        return Err("Use a multilingual Whisper GGML model, not .en".into());
    }
    let threads = std::env::var("ELA_ASR_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2)
        .clamp(1, 16);
    let mut languages = Vec::new();
    for window in windows {
        if window.start >= window.end || window.end > samples.len() {
            return Err("Invalid language window".into());
        }
        if window.end - window.start < 24000 {
            languages.push("und");
            continue;
        }
        let mut state = context.create_state()?;
        state.pcm_to_mel(&samples[window.start..window.end], threads)?;
        let (_, probabilities) = state.lang_detect(0, threads)?;
        let probability = |language| {
            whisper_rs::get_lang_id(language)
                .and_then(|id| probabilities.get(id as usize))
                .copied()
                .unwrap_or(0.0)
        };
        languages.push(select_language(probability("te"), probability("en")).unwrap_or("und"));
    }
    println!("{}", serde_json::to_string(&languages)?);
    Ok(())
}
