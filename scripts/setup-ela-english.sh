#!/usr/bin/env bash
# Connected provisioning only. Runtime workers never download packages/weights.
set -euo pipefail
cd "$(dirname "$0")/.."
python=${ELA_SETUP_PYTHON:-python3.12}
command -v uv >/dev/null || { echo 'Install uv and Python 3.12 first (e.g. nix shell nixpkgs#uv nixpkgs#python312).' >&2; exit 1; }
if [ ! -x .venv-ela/bin/python ]; then uv venv --python "$python" .venv-ela; fi
uv pip install --python .venv-ela/bin/python torch==2.10.0 --index-url https://download.pytorch.org/whl/cpu
uv pip install --python .venv-ela/bin/python -r scripts/requirements-ela-nlp.txt
uv pip install --python .venv-ela/bin/python https://github.com/explosion/spacy-models/releases/download/en_core_web_trf-3.8.0/en_core_web_trf-3.8.0-py3-none-any.whl
model="$PWD/models/grammar/vennify-t5-base"
mkdir -p "$model"
revision=9e4a09d21dca1072a69302df9261289d03c3ed78
for file in config.json tokenizer.json tokenizer_config.json special_tokens_map.json README.md pytorch_model.bin; do
  if [ ! -s "$model/$file" ]; then
    curl --fail --location --retry 3 "https://huggingface.co/vennify/t5-base-grammar-correction/resolve/$revision/$file" -o "$model/$file.download"
    mv "$model/$file.download" "$model/$file"
  fi
done
if [ ! -s "$model/LICENSE.txt" ]; then
  curl --fail --location --retry 3 https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.txt -o "$model/LICENSE.txt"
fi
cargo build --bins --locked -j2
if [ ! -s "$model/model.safetensors" ]; then
  target/debug/ela-grammar --convert "$model/pytorch_model.bin" "$model/model.safetensors.download"
  mv "$model/model.safetensors.download" "$model/model.safetensors"
fi
uv pip freeze --python .venv-ela/bin/python > "$model/python-freeze.txt"
spacy_model=$(.venv-ela/bin/python -c 'from importlib.metadata import distribution; print(distribution("en-core-web-trf").locate_file("en_core_web_trf/en_core_web_trf-3.8.0").resolve())')
{
  printf 'source=https://huggingface.co/vennify/t5-base-grammar-correction\nrevision=%s\nlicence=CC-BY-NC-SA-4.0\nconversion=lossless Rust/Candle PTH-to-safetensors, tied aliases omitted\ndecoding=CPU float32 greedy; not ELA beam-search parity\n' "$revision"
  sha256sum "$model/pytorch_model.bin" "$model/model.safetensors" "$model/config.json" "$model/tokenizer.json"
} > "$model/MANIFEST.txt"
{
  printf 'export ELA_NLP_PYTHON=%q\n' "$PWD/scripts/ela-python"
  printf 'export ELA_SPACY_MODEL=%q\n' "$spacy_model"
  printf 'export ELA_T5_MODEL_DIR=%q\n' "$model"
  if command -v nix >/dev/null; then
    runtime=$(nix eval --raw nixpkgs#stdenv.cc.cc.lib.outPath)
    printf 'export ELA_CPP_RUNTIME_LIB=%q\n' "$runtime/lib"
  fi
} > "$model/english.env"
printf 'Provisioned. Review licences, then source %s/english.env before starting the desktop.\n' "$model"
printf 'Full spaCy transformer loading needs about 3 GiB available RAM; profile the target PC.\n'
