#!/usr/bin/env python3
"""Optional local inference bridge. Rust owns HTTP, validation, storage and workflow.

Contract: local-asr.py /absolute/audio.wav te|en
stdout: one JSON object {"text": "..."}. Diagnostics go to stderr.
Only pre-downloaded models are used; this script never downloads model weights.
"""

import contextlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def telugu(audio):
    import torch
    import torchaudio
    from transformers import AutoModel

    model_path = os.environ.get("ELA_TELUGU_MODEL")
    if not model_path or not Path(model_path).is_dir():
        raise RuntimeError(
            "Set ELA_TELUGU_MODEL to a downloaded AI4Bharat IndicConformer model directory"
        )
    # The model's custom code must be reviewed before installation. Pin its revision.
    model = AutoModel.from_pretrained(
        model_path, trust_remote_code=True, local_files_only=True
    )
    model.eval()
    wav, sample_rate = torchaudio.load(audio)
    if sample_rate != 16000:
        wav = torchaudio.functional.resample(wav, sample_rate, 16000)
    wav = wav.mean(dim=0, keepdim=True)
    with torch.inference_mode():
        result = model(wav, "te", "ctc")
    if not isinstance(result, str):
        raise RuntimeError(
            "Unexpected model output; check the pinned model's inference API"
        )
    return result


def english(audio):
    binary = os.environ.get("ELA_WHISPER_BIN", "whisper-cli")
    model = os.environ.get("ELA_ENGLISH_MODEL")
    if not model or not Path(model).is_file():
        raise RuntimeError(
            "Set ELA_ENGLISH_MODEL to a local whisper.cpp GGML model file"
        )
    with tempfile.TemporaryDirectory(prefix="ela-whisper-") as directory:
        output = str(Path(directory) / "transcript")
        subprocess.run(
            [
                binary,
                "-m",
                model,
                "-f",
                audio,
                "-l",
                "en",
                "-otxt",
                "-of",
                output,
                "-nt",
            ],
            stdout=sys.stderr,
            stderr=sys.stderr,
            check=True,
            timeout=170,
        )
        return Path(output + ".txt").read_text(encoding="utf-8").strip()


def main():
    if len(sys.argv) != 3 or sys.argv[2] not in ("te", "en"):
        raise RuntimeError("Usage: local-asr.py AUDIO.wav te|en")
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    with contextlib.redirect_stdout(sys.stderr):
        text = telugu(sys.argv[1]) if sys.argv[2] == "te" else english(sys.argv[1])
    print(json.dumps({"text": text}, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"Local ASR error: {error}", file=sys.stderr)
        sys.exit(1)
