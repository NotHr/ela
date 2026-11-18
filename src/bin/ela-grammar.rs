//! Isolated CPU-only T5 inference. No Python, downloads, or remote model code.
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::t5::{Config, T5ForConditionalGeneration};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokenizers::Tokenizer;

#[derive(Deserialize)]
struct Request {
    model_dir: String,
    sentences: Vec<String>,
}
#[derive(Serialize)]
struct Suggestion {
    original: String,
    suggested: String,
    status: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--convert") {
        if args.len() != 4 {
            return Err("Usage: ela-grammar --convert INPUT.bin OUTPUT.safetensors".into());
        }
        let mut weights = candle_core::pickle::read_all(&args[2])?
            .into_iter()
            .collect::<HashMap<_, _>>();
        // T5's tied embeddings are read through shared.weight. Remove unused
        // aliases to avoid storing three redundant copies in the safe container.
        if weights.contains_key("shared.weight") {
            for key in [
                "encoder.embed_tokens.weight",
                "decoder.embed_tokens.weight",
                "lm_head.weight",
            ] {
                weights.remove(key);
            }
        }
        candle_core::safetensors::save(&weights, &args[3])?;
        return Ok(());
    }
    if args.len() != 2 {
        return Err("Usage: ela-grammar REQUEST.json".into());
    }
    let bytes = std::fs::read(&args[1])?;
    if bytes.len() > 128 * 1024 {
        return Err("Grammar request exceeds limit".into());
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.sentences.len() > 16 || request.sentences.iter().any(|s| s.len() > 8192) {
        return Err("Grammar sentence budget exceeded".into());
    }
    let directory = Path::new(&request.model_dir);
    let mut config: Config =
        serde_json::from_slice(&std::fs::read(directory.join("config.json"))?)?;
    config.use_cache = true;
    let tokenizer = Tokenizer::from_file(directory.join("tokenizer.json"))
        .map_err(|_| "Cannot load local T5 tokenizer")?;
    let device = Device::Cpu;
    // Administrator-managed, immutable local weights; never replace/truncate
    // this file while workers are using its mapping.
    let builder = unsafe {
        VarBuilder::from_mmaped_safetensors(
            &[directory.join("model.safetensors")],
            DType::F32,
            &device,
        )?
    };
    let mut model = T5ForConditionalGeneration::load(builder, &config)?;
    let mut results = Vec::new();
    for original in request.sentences {
        let encoding = tokenizer
            .encode(format!("grammar: {original}"), true)
            .map_err(|_| "Cannot tokenize grammar input")?;
        if encoding.len() > 256 {
            results.push(Suggestion {
                suggested: String::new(),
                original,
                status: "input_limit".into(),
            });
            continue;
        }
        model.clear_kv_cache();
        let input = Tensor::new(encoding.get_ids(), &device)?.unsqueeze(0)?;
        let encoded = model.encode(&input)?;
        let mut next = config.decoder_start_token_id.unwrap_or(config.pad_token_id) as u32;
        let mut output = Vec::new();
        let mut ended = false;
        for _ in 0..128 {
            let input = Tensor::new(&[next], &device)?.unsqueeze(0)?;
            let logits = model.decode(&input, &encoded)?.squeeze(0)?;
            next = logits.argmax(0)?.to_scalar::<u32>()?;
            if next == config.eos_token_id as u32 {
                ended = true;
                break;
            }
            output.push(next);
        }
        let suggested = if ended {
            tokenizer
                .decode(&output, true)
                .map_err(|_| "Cannot decode correction")?
        } else {
            String::new()
        };
        let status = if !ended {
            "output_limit"
        } else if suggested.trim().is_empty() {
            "empty_output"
        } else {
            "complete"
        };
        results.push(Suggestion {
            original,
            suggested,
            status: status.into(),
        });
    }
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
