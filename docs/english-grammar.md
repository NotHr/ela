# ELA-style English analysis: spaCy, T5 and ERRANT

## Installed components

- Official **spaCy 3.8.18 / en_core_web_trf 3.8.0**, using its packaged transformer, tagger, parser, lemmatizer and NER. English structural observations now come from this pipeline when `ELA_SPACY_MODEL` is configured; Telugu retains UDPipe.
- **vennify/t5-base-grammar-correction**, Hugging Face revision `9e4a09d21dca1072a69302df9261289d03c3ed78`, inferred with **Rust/Candle 0.9.2**, CPU float32. The original checkpoint was losslessly repacked to safetensors by `ela-grammar --convert`; no Python T5 inference or model training is needed.
- Official **ERRANT 3.0.0** with the same spaCy transformer pipeline, not a Rust approximation of its rules.
- Required **Qwen3-0.6B Q4** summary selection after these stages.

The inspected ELA code names `en_core_web_trf` and a local `vennify` T5 directory; it does not pin their exact artifact revisions. These are the corresponding model families, not proof of byte-for-byte equivalence with an unknown deployed ELA installation. Rust T5 currently uses **greedy decoding**, whereas ELA's HappyTransformer call uses five beams. Proposed corrections may differ. Full ELA scoring/matrix parity is still not implemented.

The T5 checkpoint is **CC BY-NC-SA 4.0**, installed for the explicitly noncommercial deployment. Retain its README/attribution/licence and observe redistribution/share-alike terms. spaCy model licences/source notices, ERRANT's licence and Python dependency versions are retained locally. Model weights and the venv are ignored by source control, not bundled into this source repository.

## Memory-aware execution

```text
ASR workers exit
 → spaCy English observations (Python worker exits)
 → T5 proposed corrections (Rust worker exits)
 → ERRANT classification + spaCy (Python worker exits)
 → required Qwen summary (worker exits)
```

The two Python invocations deliberately reload spaCy rather than keeping its model heap alive beside T5 or the SLM. Models are never downloaded at runtime. Python workers set offline flags, use CPU and two compute threads, and are supervised with the same Unix process-group/parent-pipe mechanism as other model stages.

Actual developer-machine observations on short public/example inputs:

- native T5: roughly **2.4 seconds / 1,683 MiB sampled process-family RSS** for three short sentences;
- spaCy: roughly **3 seconds / 2,686 MiB sampled RSS** while loading/processing two sentences;
- official ERRANT classified `go → goes` and `do → does` as `R:VERB:SVA` in explicit test sentences.

These are sampled observations, not maximum limits or school-hardware certification. Full spaCy transformer loading is substantially heavier than UDPipe. Admission requires **3 GiB available memory** for spaCy/ERRANT and **2 GiB** for T5, using Linux `MemAvailable`. Unknown counters block conservatively. A 4 GiB desktop may not have enough headroom with the OS/browser; **8 GiB is a safer starting point for this full English stack**, subject to profiling. Sequential loading avoids summing all model heaps, but cannot make the largest model's own loading peak disappear. Insufficient memory is surfaced, not silently replaced by another parser or treated as a learner deficit.

## Provision once while connected

Use Python 3.12 and uv, for example via `nix shell nixpkgs#uv nixpkgs#python312`, plus the Rust/CMake development environment. Stop the desktop while provisioning on a memory-constrained PC.

```sh
scripts/setup-ela-english.sh
source models/grammar/vennify-t5-base/english.env
# Also load the existing ASR/LID/SLM configuration.
cargo build --release --bins -j2
./target/release/ela-local
```

The script installs pinned CPU PyTorch/spaCy/ERRANT packages and the official spaCy model wheel, downloads the pinned T5 checkpoint, repacks its tensor weights using Rust, and writes model hashes/dependency versions. The generated `english.env` contains local paths; merge/source them with the existing `.env`. On Nix, its C++ runtime-library path lets binary Python wheels find `libstdc++.so.6`. An air-gapped deployment must pre-cache/copy Python packages, Nix/runtime libraries and weights; this is not yet a packaged offline installer.

Configuration:

```sh
export ELA_NLP_PYTHON=/absolute/project/path/scripts/ela-python
export ELA_SPACY_MODEL=/absolute/path/to/en_core_web_trf/en_core_web_trf-3.8.0
export ELA_T5_MODEL_DIR=/absolute/path/to/models/grammar/vennify-t5-base
# Nix/Linux wheel support, when necessary:
export ELA_CPP_RUNTIME_LIB=/nix/store/...-gcc-...-lib/lib
```

Keep model directories immutable while assessments run. T5/config/tokenizer and main spaCy artifacts are content-fingerprinted; those digests/settings are stored separately in grammar provenance. Never overwrite the original recognized transcript with corrected text.

## What teachers see

The readable summary includes the correction check's coverage and count of categorized proposed edits, with an explicit warning that they are **not confirmed mistakes by the child**. The report shows original and suggested wording plus friendly edit explanations. Raw POS/morphology/dependencies remain collapsed.

ERRANT labels and token-span indices are stored. Like inspected ELA code, `R:ORTH`, `R:SPELL` and `R:OTHER` are excluded from the main grammar-category count; those edits are retained for traceability and may still appear as proposed wording changes. No new grammar/proficiency grade is invented.

Only English ASR sentences from retained learner segments are eligible. The pipeline does not apply the English model to Telugu. Boundaries/gaps/exclusions are respected when constructing text units. At most eight detected English sentences are reviewed per attempt; each T5 input is capped at 256 tokens and generation at 128 tokens. Over-limit, empty or unfinished generations are not presented as completed corrections. Incomplete coverage is labelled partial, not "all grammar correct". Unchanged output or zero categorized edits does not establish grammatical correctness.

For read-aloud, grammar mainly belongs to the passage, and discrepancies may be recognition errors. For spontaneous speech, punctuation/disfluency handling and adult-trained model bias still matter. T5 can change meaning; these are proposals about the transcript, not diagnoses, pronunciation findings, comprehension results or validated child-language norms.

## Existing reports and diagnostics

Choose **Update language analysis** to run spaCy → native T5 → ERRANT → summary on a saved transcript, without the original recording. API: `POST /api/assessments/{id}/linguistics` (202; 429 when busy). This preserves original ASR words/audio metrics/reference-match score; linguistic counts may change because the parser changed. Do not interpret that model change as learning gain.

```sh
# Uses all local English stages, without changing the assessment database:
./target/release/ela-local --grammar /path/to/exported-assessment.json
```

Do not run diagnostic CLIs alongside active assessments on a low-memory PC. They are separate from the server's global permit. No representative child-speech or educational-outcome validation is claimed by these integration tests.
