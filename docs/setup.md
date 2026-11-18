# Detailed setup and reference

Run commands from the project root. See the [short project introduction](../README.md).

An independent Rust implementation in progress, inspired by [IT for Change’s ELA](https://github.com/IT-for-Change/ela). **Mobile form + photo + recording → offline transfer → school desktop OCR, speech/NLP analysis and longitudinal reports.** All assessment computation happens on the desktop; the phone is only an offline collector. No teacher-entered transcript, ratings, or review checkbox. Plain HTML/CSS/JavaScript with bundled [Oat](https://oat.ink); Telugu and English interfaces. The compact sidebar/table interface takes design cues from listmonk’s admin UI, without copying its branding.

See **[the upstream repository review and implementation map](upstream-review.md)** for what ELA Web, AI, Middleware, Collect, and ECUBE do, the source revisions inspected, and the remaining gaps. This is not full ELA feature parity.

## Offline collection and student progress

Use **ELA Collect/ODK Collect** as the OSS mobile collector. `forms/ela-reading.xml` captures a stable student ID, class, activity, paragraph photo, recording and timestamp; it does not run any models. ELA Collect's ZIP export can be transferred by USB/Bluetooth or a local file-transfer tool and imported through the desktop's **Import** button. The importer supports this supplied schema, not arbitrary historical ELA forms or the OpenRosa server API.

The desktop performs **local Tesseract OCR**, audio conversion, student separation, bilingual ASR, **spaCy `en_core_web_trf` English analysis**, native Rust **T5** correction proposals and official **ERRANT** classification. Telugu structural analysis retains UDPipe. See [English model setup and memory requirements](english-grammar.md). It queues all valid items, deduplicates ODK instance IDs, and saves individual processing outcomes. The **Students** view shows assessment history and same-passage/model trends, without treating failed/uncertain attempts as zero scores or comparing an easier passage as proof of improvement.

See **[offline workflow, collector setup, open NLP model licensing, and progress semantics](offline-workflow.md)** and **[low-memory execution and required local SLM summaries](low-memory.md)**. First-time dependencies/models must be preinstalled or copied from USB for an air-gapped school. A complete offline installer and a device-tested mobile APK are not bundled here.

## Run

```sh
nix develop
cargo build --bins
cargo run
# Open http://127.0.0.1:3000
```

Or `nix develop --command bash -c 'cargo build --bins && cargo run'`. Without Nix, install stable Rust, a C/C++ compiler, CMake and FFmpeg, set `WHISPER_DONT_GENERATE_BINDINGS=1` to use the shipped FFI bindings (or install libclang), and run `cargo build --bins` followed by `cargo run`. Run from the project root so the application can serve `web/`. For deployment, build `cargo build --release --bins` and run `target/release/ela-local`. The flake supports x86_64/aarch64 Linux and Apple Silicon macOS, and includes Rust tooling, FFmpeg, Telugu/English Tesseract data, Node/Prettier and Python/Ruff for development, plus UDPipe where the pinned package supports the platform. **Speech and T5 inference are Rust/native. The official spaCy/ERRANT stages require the separately provisioned local Python 3.12 environment.**

The default `native-asr` Cargo feature links [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)'s Rust bindings. Its version-pinned build downloads native runtime libraries on the first build. For an offline build, pre-populate the Cargo cache and provide compatible libraries using `SHERPA_ONNX_LIB_DIR` (or archives using `SHERPA_ONNX_ARCHIVE_DIR`), as documented by `sherpa-onnx-sys`. This is not a sandboxed `nix build` package; the flake currently provides a development shell.

The service binds only to `127.0.0.1:3000` (or an explicit loopback-only `ELA_PORT` from 1024–65535). Use only one service instance per database. SQLite creates `ela.db`; override using `DATABASE_URL`, for example `sqlite:///secure/location/ela.db?mode=rwc`.

## Native local inference

The same Rust executable is spawned in `--infer AUDIO.wav te|en` mode. It calls sherpa-onnx directly for diarization, speaker embeddings and ASR. Language identification uses a separate Rust `ela-language-id` helper linked to whisper.cpp through `whisper-rs`, **not Python or a cloud service**. Build/install all Rust binaries together (`cargo build --bins` or `cargo build --release --bins`), including `ela-language-id` and the new `ela-grammar` T5 worker. Keeping the native engines in separate processes avoids conflicting FFT symbols in their native libraries. The helper normally lives beside `ela-local`; override with `ELA_LID_EXECUTABLE` if needed. This is Rust-based inference integration, not a claim that the underlying engine is pure Rust.

Model files are installed separately and never downloaded by the running web service. Read model licensing/access terms and pin trusted revisions. Model paths must be local files. `/api/health` checks file presence, not whether a model will successfully load.

### Telugu: AI4Bharat-derived IndicConformer CTC

Use a **sherpa-onnx-compatible NeMo CTC export**, with the Telugu head/mask and matching tokens. The original AI4Bharat PyTorch checkpoint cannot be loaded directly.

One public community conversion is [Docglasses-AI/indic-conformer-600m-sherpa-onnx-int8](https://huggingface.co/Docglasses-AI/indic-conformer-600m-sherpa-onnx-int8), derived from [AI4Bharat IndicConformer](https://huggingface.co/ai4bharat/indic-conformer-600m-multilingual). This is not an official AI4Bharat Rust model. Review its model card and provenance. Example download pinned to an inspected revision (~659 MB model):

```sh
mkdir -p models/te
base=https://huggingface.co/Docglasses-AI/indic-conformer-600m-sherpa-onnx-int8/resolve/9fa6d9a76a863c4449935086f56155d3bc3994c1
curl -fL "$base/indicconformer-te.int8.onnx" -o models/te/model.onnx
curl -fL "$base/indicconformer-tokens.txt" -o models/te/tokens.txt

export ELA_TE_MODEL="$PWD/models/te/model.onnx"
export ELA_TE_TOKENS="$PWD/models/te/tokens.txt"
```

### English: Whisper ONNX

Use a sherpa-onnx Whisper export, **not** a whisper.cpp GGML file. For example, download and inspect the [Whisper tiny.en release archive](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-tiny.en.tar.bz2), extract it under `models/`, and point these variables at the matching files:

```sh
export ELA_EN_ENCODER="$PWD/models/sherpa-onnx-whisper-tiny.en/tiny.en-encoder.int8.onnx"
export ELA_EN_DECODER="$PWD/models/sherpa-onnx-whisper-tiny.en/tiny.en-decoder.int8.onnx"
export ELA_EN_TOKENS="$PWD/models/sherpa-onnx-whisper-tiny.en/tiny.en-tokens.txt"
```

Choose an appropriate model size for accuracy and hardware; tiny models are primarily useful for testing. Check the extracted filenames; archives may contain both float and quantized weights. Record source revisions/checksums alongside your local files rather than silently replacing them.

Then start the server in the same shell:

```sh
export ELA_ASR_THREADS=2       # optional, 1–16, defaults to 2
cargo run
```

Native inference automatically fingerprints the actual model/token files and relevant settings using SHA-256. Those fingerprints are stored with reports and used to separate progress series after model changes; older/external results without provenance do not receive improvement deltas. Keep source/licence manifests with model artifacts too. Native inference uses CPU. Fixed-language ASR uses 25-second windows; automatic bilingual routing uses short energy-gap-aware windows (roughly 2–5 seconds). Boundaries can affect words. This is **window-level switching, not word-level code-switch alignment**. The inference child is killed after ten minutes. Native libraries/model loading may use substantial RAM.

### Student-only conversation extraction and language switching

The new assessment dialog defaults to **Teacher + student** and **Auto · Telugu + English**. For an already-isolated learner recording, choose **Student only**; for known monolingual speech, choose **Fixed language**. Enhanced modes require the additional models below; they never silently fall back to unfiltered conversation audio.

For conversations, upload a **3–30-second student-only voice reference**, with at least two seconds of active signal. It can be in any supported upload format and is converted locally. The app cannot infer student/teacher roles from anonymous speaker IDs: it never assumes the first, last, quietest or longest-speaking person is the student.

Configure compatible models, for example:

```sh
# sherpa-onnx-compatible pyannote segmentation export and speaker embedding model:
export ELA_DIARIZATION_MODEL="$PWD/models/speech/segmentation/model.onnx"
export ELA_SPEAKER_MODEL="$PWD/models/speech/wespeaker_en_voxceleb_resnet34.onnx"
# A MULTILINGUAL whisper.cpp GGML model, not an English-only .en model:
export ELA_LID_MODEL="$PWD/models/speech/ggml-base.bin"
cargo build --bins
cargo run
```

Example sources: [pyannote segmentation export archive](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2), [speaker embedding model](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34.onnx), and [multilingual GGML Whisper base](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin). Inspect upstream model cards/licenses and pin/checksum downloads. Larger multilingual models may improve LID but need more memory; these examples are not validated on Telugu/English child speech.

Pipeline:

1. Diarize without assuming a fixed number of speakers.
2. Match speaker embeddings to the student reference. Default cosine threshold: `0.65`, with `0.10` separation from the runner-up (`ELA_SPEAKER_MATCH_THRESHOLD`, `ELA_SPEAKER_MATCH_MARGIN`, each 0–1). These are uncalibrated starter thresholds, **not identity probabilities**.
3. Remove all model-detected overlaps with other speakers and independently verify each retained turn. Short/unverifiable turns are excluded. Teacher/other-speaker transcripts are never generated or saved by this path.
4. In automatic mode, use actual Whisper language posteriors for Telugu/English routing per student window. Require at least 5% joint Telugu/English posterior mass and 80% of that mass for the winner. Ambiguous/low-support windows are `und` and excluded; never force them into the chosen assessment language. These are heuristic, uncalibrated gates for a bilingual setting, **not a universal language detector or a guarantee that other languages are recognized**.
5. Route Telugu windows to IndicConformer and English windows to Whisper, preserving original timestamps and language labels in the report.

Diarization and voice matching can be wrong, especially with children, noise, similar voices and language changes. They do not guarantee perfect separation. A voice reference must genuinely contain the student alone. No speaker age, gender or role is inferred. Missing student matches produce insufficient evidence. Any material exclusion (>50 ms of overlap/untranscribed student speech) withholds the reading/quantity scores rather than treating excluded audio as mistakes. Reference matching across bilingual speech requires the corresponding expected passage; speech is transcribed, not translated.

Raw/reference recordings and embeddings are transient. Only student transcript/segments, aggregate durations, speaker counts and match similarity are stored. Do not use this as an authentication or surveillance system; obtain appropriate consent for children’s voice processing and validate false-match rates before school deployment. In student-only mode the app trusts that the input is already isolated.

CLI equivalent:

```sh
cargo run -- --infer conversation.wav te --student-reference student.wav --detect-language
```

`te`/`en` remains the assessment/reference language; automatic mode detects and routes both languages. The language helper batches windows, and a pipe watchdog terminates it if its inference parent dies. Student audio duration—not teacher talk time—is used for per-minute report metrics after separation. This differs from whole-recording timing in student-only mode; compare like-for-like activities/modes.

### Optional legacy/external adapter

`ELA_ASR_COMMAND=/absolute/path/to/executable` overrides native inference. The executable receives `AUDIO.wav te|en`, returns `{"text":"...","backend":"...","model_revision":"..."}` on stdout and exits nonzero on failure. Only `text` is required for fixed-language, student-only mode. Enhanced requests append `--student-reference PATH` and/or `--detect-language`; an adapter must implement those options and return the requested `speech` evidence or the job fails closed. The bundled Python compatibility script does not implement enhanced modes. It runs without a shell; diagnostics belong on stderr. Treat adapters as trusted code. Rust's offline environment flags are not a network sandbox.

`scripts/local-asr.py` remains an optional compatibility bridge for the original PyTorch AI4Bharat model or whisper.cpp. It is **not the default**. Its model/runtime setup differs from native inference: `ELA_TELUGU_MODEL` for a local Hugging Face snapshot, or `ELA_WHISPER_BIN` and `ELA_ENGLISH_MODEL` for whisper.cpp. Do not confuse those with the native ONNX configuration above. For a lighter adapter-only build: `cargo run --no-default-features`.

## Automatic workflow

1. Enter learner ID/class/activity, choose the reference language, and select speaking or read-aloud. Choose conversation/student-only and automatic/fixed language processing; conversation mode requires a student voice sample.
2. For read-aloud only, provide the expected passage. It is task configuration, not manual grading.
3. Upload learner-only audio in a common audio format: WAV, MP3, M4A/AAC, Ogg/Opus, FLAC, WebM, AMR, WMA, AIFF, CAF, and other enabled containers. Audio tracks in supported video containers are also accepted. **Conversion is automatic**, on the server, using FFmpeg. The request limit is **64 MiB** and decoded audio is limited to **five minutes**; longer files are rejected rather than silently assessed as shorter recordings. Already valid mono 16 kHz PCM WAV bypasses conversion. Unsupported/corrupt files get a saved conversion failure.

   FFmpeg must be on `PATH` (provided by `nix develop`), or set `ELA_FFMPEG_BIN` to its executable path. Conversion runs without a shell, disables network protocols/playlists, strips metadata, downmixes/resamples to 16 kHz mono PCM, and has a 90-second timeout. No codec stack can guarantee support for every possible audio file; actual codecs depend on the installed FFmpeg build.

4. Submit once. Rust automatically checks audio, transcribes, analyzes, and saves the report. UI status updates without stage-launch buttons. Closing the page does not cancel processing; stopping the server does.
5. View/search reports or export UTF-8 JSON. No teacher transcription, score entry, or approval is required.

There is one active inference slot to control RAM usage. Additional uploads receive HTTP 429; wait and submit again. Each recording has durable stage events and a saved failure if inference fails. Unfinished jobs are marked failed after restart because raw audio is intentionally temporary; re-upload to retry. The list/export is unpaginated and intended for small classroom datasets.

## What the report means

Computed automatically in Rust:

- Unicode-aware recognized word count and distinct word forms for Telugu/English;
- type-token ratio, words per **recording** minute (including pauses), duration, simple energy-active time and clipping warnings;
- an **ELA-inspired speech quantity band**: 1 for 1–10 recognized words, 2 for 11–25, 3 for >25; withheld for insufficient/suspicious evidence;
- reference-based word/grapheme edit distances and word-level omissions, substitutions and insertions for read-aloud;
- a **provisional reading-match screen** (0–100), target practice words and suggested teacher interventions;
- optional local UD POS/dependency, lemma and morphological analysis, with explicit model availability;
- photo OCR provenance/quality gates and deterministic next-activity suggestions;
- per-student assessment history and comparable-condition progress series.

### Foundational reading screening

The primary activity is Telugu/English read-aloud, aimed at timely foundational-literacy feedback in Telangana classrooms. The reference passage must be known; free speaking does not receive a reading-match score. The reading score is `max(0, 100 − word_error_rate_percent)`. A transparent starter policy labels <60 as `needs_support`, 60–<85 as `developing`, and ≥85 as `on_track`. **These thresholds are provisional and unvalidated**, not Telangana grade-level norms, the original ELA scale, or a diagnosis. They must be calibrated with educators and representative child speech before consequential deployment. Scores are withheld for empty/low-energy/strongly repetitive recognition or substantial clipping.

Word alignment selects up to 12 expected words that were omitted/substituted for personalised practice. Rule-based activities include listening/repetition, sound/syllable blending and passage rereading. Suggested teacher interventions vary by provisional band (individual verification/model-and-echo, guided small groups, or another passage). ASR cannot reliably identify a child’s underlying phonics gap, so these are actionable starting suggestions, not inferred diagnoses. Matched words/minute counts reference words matched by alignment divided by whole-recording time, not a standardized oral-reading-fluency score. Repeated comparable activities can support progress tracking; controlled educational evaluation is still needed to demonstrate literacy gains.

**These are automatic speech measurements, not validated comprehension, pronunciation, grammar, vocabulary proficiency, or overall fluency grades.** Surface word-form diversity is not lexical density and varies with sample length and morphology. Read-aloud differences include ASR errors; WER can exceed 100%. Signal energy is not a trained speech detector and can include noise. Silence/empty recognition/repetitive output yields insufficient or uncertain evidence, not a failure grade.

The original ELA report has **16 categories (0–15)** combining language-ID and speech-quantity evidence. Its language decision CSV/complete scoring rules are not integrated here. English now uses the corresponding spaCy/T5/ERRANT components, but Rust T5 uses greedy decoding rather than ELA's beam search and their proposed edits are not converted into an invented proficiency score. The new Whisper LID/pyannote separation path is not a reproduction of ELA’s Vaani decision rules. Therefore `proficiency_score` is explicitly null. We do not invent an equivalent score or infer language quality from script, accent, selected language, or recording length. Telugu uses recognized words rather than ELA's non-English duration estimate. See the review document for the full gap list.

### English grammar components

Use [the provisioning guide](english-grammar.md) for actual spaCy `en_core_web_trf` and ERRANT Python workers plus the same-family Vennify T5 checkpoint run by Rust/Candle. The report shows original/suggested text and human-readable edit explanations. **Update language analysis** reprocesses a saved transcript without re-uploading audio. Proposals do not overwrite ASR text, alter reading scores or establish confirmed child errors. Telugu is not passed through the English corrector.

Full spaCy transformer loading was observed near **2.7 GiB RSS**; it requires about 3 GiB available RAM before loading. **8 GiB is a safer target for this full English stack** than assuming all 4 GiB PCs will fit. The stages run separately, not simultaneously.

### Resource use and small language models

The default worker is serial: OCR → speaker/LID processing → isolated Telugu ASR process → isolated English ASR process → local NLP → English native T5 → spaCy/ERRANT → required SLM summary. ASR workers exit between languages; neighboring same-language windows are coalesced for fewer encoder runs. Queued collection media lives on private disk rather than alongside model weights in RAM. Model SHA-256 values are cached by file identity/size/mtime, with `ELA_DISABLE_MODEL_CACHE=1` available for full rehashing.

A required local **Qwen3-0.6B Q4** stage selects and organizes grounded, reviewed English/Telugu explanations and approved practice actions. The main report explains what was measured, what cannot be concluded and what to do next; technical POS/morphology data is collapsed. This also works for speech-only recordings without a reference paragraph. The model cannot change scores or invent ability/grammar diagnoses. It runs last with a 600 MiB model-file cap, RAM admission check, CPU/context/output limits and timeout. If the summary fails, measurements remain saved but the assessment is not marked complete; use **Generate summary** to retry without re-uploading audio. Older reports can generate summaries through the same button. `ELA_SLM_MODEL` and `ELA_LLAMA_BIN` are required; `ELA_ENABLE_SLM` is no longer used. See [configuration and limitations](low-memory.md).

### Existing records

The original manual-rubric table is left untouched. It is no longer used for new assessments and is not mixed with automatic reports. Use **Export previous manual assessments** or `GET /api/legacy-assessments` to retrieve it. Back up your database before upgrading.

## Privacy and deployment

- No CDN, analytics, cloud inference, or model downloads at runtime. Oat is bundled under `web/vendor/` with its MIT notice.
- Queued media is staged in private, disk-backed `.ela-work/` job directories (override with `ELA_WORK_DIR`), not retained as large RAM buffers while models run. Active audio is converted in private temporary files and discarded after processing. Reports/transcripts remain in SQLite. A hard process/machine crash may leave OS temporary files; protect temporary storage too.
- This is a loopback-only, single-user application without authentication. Host/Origin checks mitigate foreign browser requests but do not authenticate local users. Do not expose it via a tunnel/proxy/network bind without proper authorization, CSRF protection and TLS.
- For Nix development, use a Git checkout with runtime data ignored, and keep real pupil data outside the flake source. Non-Git path flakes can copy the working directory (including untracked data) into the world-readable Nix store. Do not provision/build from a workspace containing unprotected pupil data.
- Use pseudonymous learner IDs, appropriate consent, disk encryption and restricted database permissions. Exports contain learner data. Deletion removes logical records, not forensic copies or backups. Stop the server before backing up SQLite.
- Model access terms/licenses are separate. No original ELA source, branding, or model weights are distributed here. This is not an official IT for Change release.

## Development

```sh
nix develop
cargo test --locked
cargo fmt --check
prettier --check web/index.html web/style.css web/app.js web/students.js web/collection.js
ruff format --check scripts/local-asr.py scripts/ela-linguistics.py
ruff check scripts/ela-linguistics.py
nix fmt
nix flake check --all-systems
```

`nix flake update` explicitly updates pinned development tools. Default tests exercise native-linked builds, automatic jobs using clearly marked inference fixtures, silence handling, Telugu persistence/tokenization, reference comparisons, failures, concurrency limits, and restart handling. Model quality must be tested separately on representative child speech; passing a runtime smoke test is not validation of educational assessment.

API:

- `GET /api/health`: ASR, diarization, switching, conversion, OCR, per-language NLP, spaCy, English T5/ERRANT and required SLM configuration.
- `GET /api/collection/form`: downloadable offline XForm.
- `POST /api/collection/import`: multipart `archive` ZIP, returns per-submission accepted/duplicate/invalid results and queues valid jobs.
- `GET /api/students`, `GET /api/students/{student_id}`: stable-ID roster summaries and complete assessment/comparable-series history.
- `POST /api/assessments`: multipart `learner`, `student_id`, `classroom`, `recorded_at` (ISO timestamp with timezone), optional `source_id`, `paragraph_photo` (instead of typed reference), `language`, `prompt`, `task` (`speaking`/`read_aloud`), `reference_text`, `audio`, optional `audio_mode` (`learner_only`/`conversation`), `detect_language` (`true`/`false`) and `student_reference` file; returns HTTP 202 with job ID. Legacy API requests default to student-only/fixed mode; the new UI explicitly requests conversation/automatic mode.
- `GET /api/assessments`, `GET /api/assessments/{id}`: metadata/status/report.
- `GET /api/assessments/{id}/events`: stage history.
- `POST /api/assessments/{id}/summary`: generate/retry the required summary from saved measurements.
- `POST /api/assessments/{id}/linguistics`: rerun language analysis, English corrections and summary from the saved transcript.
- `DELETE /api/assessments/{id}`: delete a completed/failed job and its stage history.
- `GET /api/legacy-assessments`: previous manual records, read-only.

See [validation performed](testing.md) for real English/Telugu inference and browser smoke checks, model hashes, and the limits of those checks.

Project code: AGPL-3.0-or-later (`LICENSE`). Third-party dependencies and models retain their own licenses.
