#!/usr/bin/env bash
# Train local UD tagging/parsing weights from an appropriately licensed treebank.
# This script does not download anything or use student recordings/transcripts.
set -euo pipefail
if [ "$#" -ne 3 ]; then
  printf 'Usage: %s te|en TRAIN.conllu OUTPUT.udpipe\n' "$0" >&2
  exit 2
fi
language=$1
training=$2
output=$3
case "$language" in te|en) ;; *) printf 'Language must be te or en\n' >&2; exit 2 ;; esac
[ -f "$training" ] || { printf 'Training CoNLL-U file not found\n' >&2; exit 2; }
[ ! -e "$output" ] || { printf 'Refusing to overwrite an existing model\n' >&2; exit 2; }
mkdir -p "$(dirname "$output")"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"${ELA_UDPIPE_BIN:-udpipe}" --train --tokenizer=epochs=10 --tagger=iterations=5 --parser=iterations=5 "$work/model.udpipe" "$training"
[ -s "$work/model.udpipe" ]
mv "$work/model.udpipe" "$output"
{
  printf 'language=%s\ntraining_policy=starter-10-tokenizer-5-tagger-5-parser\n' "$language"
  printf 'model_sha256='; sha256sum "$output" | cut -d ' ' -f 1
  printf 'treebank_sha256='; sha256sum "$training" | cut -d ' ' -f 1
  printf 'runtime='; "${ELA_UDPIPE_BIN:-udpipe}" --version 2>&1
  printf 'NOTICE: retain the source treebank attribution and licence. This model is unvalidated for child speech.\n'
} > "$output.manifest.txt"
printf 'Model and provenance manifest: %s\n' "$output"
