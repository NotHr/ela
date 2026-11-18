# Validation performed

## Automated tests

```sh
nix develop --command cargo test --locked
nix develop --command cargo clippy --all-targets -- -D warnings
```

The tests cover automatic status transitions, native-linked builds, Telugu Unicode handling, reference word alignment, provisional screening/target words, withheld scores on insufficient evidence, persistence/deletion, restart recovery, inference failures, request origin/host checks and inference concurrency. Inference fixtures are explicitly marked and do not stand in for real model testing.

FFmpeg tests encode and normalize MP3, Ogg, M4A, FLAC and WebM, plus malformed file rejection. Runtime input remains limited to installed codecs and allowed non-playlist containers.

## Real native inference smoke tests

Run on x86_64 Linux through the default Rust executable, with CPU sherpa-onnx 2.0.1. No Python model inference:

- **English**: sherpa-onnx Whisper tiny.en int8 encoder/decoder on the release archive’s `test_wavs/0.wav`; produced an English transcript.
- **Telugu**: community AI4Bharat-derived Telugu IndicConformer CTC int8 export, revision `9fa6d9a76a863c4449935086f56155d3bc3994c1`, on public `google/fleurs`, `te_in`, validation row 0 (ID 1519). Produced a transcript matching that sample’s reference.
- **End to end**: uploaded the FLEURS float WAV through the HTTP submission API, automatically converted it, ran native Telugu inference, generated and saved a provisional reading screen, and retrieved the completed report. Reference match was 100 on this single sample. This is a pipeline smoke test, **not an accuracy benchmark or evidence of educational validity**. Original/converted files and models were kept outside the project, in temporary test storage.

Model SHA-256 values used:

| File | SHA-256 |
| --- | --- |
| Telugu ONNX | `c5e8e2bb70419fb5575b126480953661f9245b2b0c82924621c0e60c77e81958` |
| Telugu tokens | `d4130fcac3d8300b5c225070b01a23487087835465b6898ea5ebf7023e7e55b4` |
| Whisper tiny.en int8 encoder | `0ce578b827c94a961aacb8fa14b02f096504b337e5c94be37c36238cbe3e8bc6` |
| Whisper tiny.en int8 decoder | `06c0e6ff6348d427e51839219d1c886c18cfdf411e629e33f5e1679bff9c1527` |
| Whisper tiny.en tokens | `306cd27f03c1a714eca7108e03d66b7dc042abe8c258b44c199a7ed9838dd930` |

Public sample source: Google FLEURS, dataset revision `70bb2e84b976b7e960aa89f1c648e09c59f894dd`. No sample audio or model weights are bundled in this repository.

## Native diarization and switching checks

The enhanced pipeline was exercised on a synthetic assembly of **public adult recordings**, not classroom/student data:

- Conversation: the English bundled sample, a gap, the Telugu FLEURS sample, a gap and the English sample again. A 4-second crop of the Telugu speaker served as the reference. Native pyannote + WeSpeaker found two speakers and selected the reference-matching cluster; the output contained no English-speaker transcript. This tests wiring and discrimination on this assembly, **not identification accuracy on children or unseen utterances**. Some short Telugu LID windows remained unknown, so this example must withhold a reading score.
- Switching: concatenated Telugu and English public clips in declared student-only mode. The final native bilingual routing path produced ordered Telugu and English segments and both language labels. These are different source speakers and are a routing test, **not a benchmark of natural within-speaker code switching**. Boundary ASR errors remain.
- Early unrestricted top-language-only tests confused short Telugu windows with other languages. The final implementation uses Whisper GGML base posteriors with explicit bilingual support/dominance gates, retaining `und` for uncertain windows rather than silently forcing a language. This improves the tested routing but is not calibration or proof of general accuracy.
- Native whisper.cpp and sherpa-onnx must be kept in **separate executable images** because linking their FFT libraries into one image caused an allocator crash during development. `ela-language-id` batches language windows in a dedicated process; a stdin pipe watchdog stops it if its inference parent dies.

Additional automated tests cover speaker matching thresholds/margins, overlap subtraction, reference requirements, rejection of an external adapter that omits requested speaker evidence, and score withholding when student audio is incomplete. Model paths, licensing and build commands are in README. Re-test natural bilingual child speech and overlapping classroom conversations before deployment.

Additional model SHA-256 values used in those native checks:

| File | SHA-256 |
| --- | --- |
| Multilingual GGML Whisper base | `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe` |
| WeSpeaker English VoxCeleb ResNet34 | `5ef208a9da1453335308a6b6f4e6dfbd7e183a38b604de0a57664f45d257fe94` |
| Pyannote segmentation 3.0 ONNX | `220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079` |

## Browser smoke tests

Headless Chromium checks at desktop 1440×1000 and mobile 390×844:

- Assessment modal opens/closes; conversation mode requires a student reference, while student-only mode hides it.
- Report modal renders a real completed report.
- Telugu/English interface switching works.
- Overview route and mobile sidebar navigation work.
- No page-level horizontal overflow at the mobile viewport (wide tables scroll within their panel).
- No JavaScript runtime exceptions were observed during these interactions.

Screenshots were inspected for the table, submission form, report and mobile layout. This does not replace full accessibility, keyboard/screen-reader, cross-browser, microphone/recording-device, or classroom usability testing.

## Offline collection, OCR, NLP and progress checks

- Automated tests import the supplied ODK submission schema, verify ZIP path safety/relative attachment resolution, deduplicate instance IDs and retain capture-time history for one stable student ID.
- Real local OCR was run on generated printed English and Telugu paragraph images using the Nix-packaged Tesseract/traineddata. Target-language order matters: English uses `eng+tel`, Telugu `tel+eng`. Some Telugu OCR errors had high confidence; these tests do **not** establish reference correctness or photograph accuracy. Confidence gates alone are not enough to validate educational scoring.
- A local Telugu UDPipe model was trained from the inspected CC-BY-SA Telugu-MTG training treebank using the supplied offline helper; its native runtime produced POS/lemma/morphology/dependency observations. This tests training/inference wiring, not child-language parser quality. English model training remains a provisioning task.
- An end-to-end collection ZIP containing two copies of the public Telugu audio plus a generated paragraph image completed local OCR, native ASR, NLP, report persistence and a comparable two-point student series. Backdated phone capture timestamps were shown in the correct order rather than import order. These are labelled runtime samples, not real pupils or evidence of literacy gains.
- Headless Chromium checked the live student roster/profile/two-point chart, photo/text source switching, ZIP duplicate results, Telugu UI, and a 390-pixel mobile layout with no page-level overflow or JavaScript exceptions. The supplied XForm still needs testing in the target Android collector/APK; no Android device validation was performed here.

## Actual spaCy / native T5 / official ERRANT checks

The deployment now installs spaCy 3.8.18 with official `en_core_web_trf` 3.8.0 and ERRANT 3.0.0 in a local Python 3.12 venv. Unneeded Python HF transformer packages were removed; the packaged spaCy model uses its official curated-transformer backend. T5 remains native Rust/Candle 0.9.2 with pinned Vennify checkpoint revision `9e4a09d21dca1072a69302df9261289d03c3ed78`, losslessly repacked from PyTorch weights to safetensors by Rust. Greedy decoding differs from ELA's five-beam setting; exact upstream artifact/output parity is not claimed.

- Actual T5 corrected `He go ...` to `He goes ...` and `She do not ...` to `She does not ...`; an already-correct example remained unchanged. Three short sentences took about 2.4 seconds, with sampled process-family RSS about 1,683 MiB.
- Actual spaCy produced sentences/POS/lemmas/dependencies. After removing unused Python HF packages, outputs on the sample were unchanged; loading/processing took about 3 seconds and peaked around 2,686 MiB sampled RSS.
- Official ERRANT classified both explicit test corrections as `R:VERB:SVA`.
- A complete background-job integration test used **explicitly labelled ASR text fixtures and generated audio**, then real spaCy → Rust T5 → official ERRANT → required SLM. It saved two agreement proposals, preserved the original transcript, selected agreement practice and cleaned staged media. Roughly 42 seconds / 2,658 MiB sampled server/model-family RSS. This tests integration, not ASR accuracy, actual child errors or school-hardware suitability.
- Headless Chromium checked the real saved fixture report: original/suggested sentences, friendly English/Telugu edit explanations, required summary, collapsed raw tags, mobile width and no JavaScript exceptions.
- The running app's prior 27-word report was reprocessed from saved text. It used spaCy, completed the correction check and regenerated the bilingual summary; original ASR transcript, word count, reference-match score and audio measurements were unchanged. Linguistic counts may differ with the changed parser; that is not learning gain.

New tests cover English missing sentence analysis without falsely reporting correct grammar, exclusion of Telugu from the English corrector, required summary inclusion of the grammar observation, and saved-transcript reanalysis without changing primary reading measurements. Full-model admission gates require 3 GiB available for spaCy/ERRANT and 2 GiB for native T5; these are heuristics, not hard memory limits. See `docs/english-grammar.md` for scope, licences, partial-review limits and setup.

## Required plain-language summary checks

The SLM is now a required report stage, including recordings without a reference passage. Native Qwen3-0.6B Q4 generation was checked on an 11-second, 27-word English recording report with no reading score, and separately on a scored Telugu reading report. Both returned validated explanation/action selections and rendered English/Telugu summaries. The explanation clearly distinguishes recognized words from reading correctness and structural counts from demonstrated vocabulary/grammar ability.

The model selects/orders prewritten grounded explanations; it does not generate unrestricted prose or independently assess proficiency. POS/morphology/dependency data is collapsed in the main UI. Live headless Chromium checks passed for the saved English/Telugu summary, speaking activities, collapsed raw tags, 390-pixel layout and absence of JavaScript exceptions. A unit-test fixture verifies that unavailable summaries fail completion while preserving measurements, and that `POST /api/assessments/{id}/summary` can finish the report without audio. Test fixtures are explicit providers, not production fallback logic. Production missing-model/memory/output failures never silently substitute a summary.

Compatible pretrained UD 2.5 English-EWT and Telugu-MTG models were also installed locally under their CC BY-NC-SA licence, with notices/checksums retained, and both passed actual native parser checks. The earlier English training job was stopped when the noncommercial deployment licence choice was clarified. Neither parser installation nor these summary checks validates child-language accuracy or educational benefit.

## Earlier low-memory and optional-prioritizer checks

The following measurements predate making the explanation stage mandatory; they are historical resource observations, not a new benchmark of every final summary workload.

The latest desktop build runs one heavy model stage at a time, with per-language ASR processes, coalesced neighboring same-language windows, disk-staged media, cached model digests and a two-thread default. Unix process-group supervisors and pipe EOF watchdogs cover model timeouts/cancellation and abrupt parent exit. A separate smoke check closed a supervisor's parent pipe while its child was running and verified that no running model child remained. Unit-test harnesses use explicit fixture programs directly; the production CLI dispatcher is tested separately through these native checks.

A real **Qwen3-0.6B Q4_K_M** (`unsloth/Qwen3-0.6B-GGUF`, revision `50968a4468ef4233ed78cd7c3de230dd1d61a56b`, SHA-256 `ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a`) was run using llama.cpp 0.9.0 `llama-completion`. Its result contained only allowed practice/intervention IDs. The parser also handles the tool's known `[end of text]` display marker without accepting arbitrary prose or extra scoring fields. A forced insufficient-memory threshold skipped the SLM without loading it.

In a developer-machine **debug-build** smoke test, two copies of the same public sample went through OCR → fixed-language Telugu ASR → locally trained Telugu NLP → optional SLM. One SLM result was generated and the other reused its validated cache. Scores were unchanged between those identical attempts, and private staged job directories were removed afterward.

- Elapsed for the two-job sample: about **27 seconds**.
- Peak **sampled sum of RSS across all desktop/model descendant processes**, collecting children from every process thread: about **1,081 MiB**.
- The sample used the quantized Telugu recognizer and 0.6B Q4 SLM **serially**, not a fully bilingual classroom conversation. The ASR/LID switching path was smoke-tested separately, and can have substantially higher latency.

These are approximate observations at 100 ms sampling, not a hard memory cap or target-school benchmark. Summed RSS may double-count shared pages, misses sub-sample peaks, excludes OS/GUI/page-cache headroom, and does not prove safe operation on a 2 GiB/4 GiB school PC. Use release binaries and profile the actual workload on target hardware. No measured educational benefit or general accuracy advantage of SLM prioritization is claimed.

## Required before educational deployment

Validate Telugu/English child-speech recognition across grades, dialects, background noise and devices; establish grade-appropriate passages and teacher-labelled reference assessments; calibrate provisional thresholds and intervention policies with Telangana educators; measure latency and failure rates on target school hardware; evaluate repeated comparable reading measures and intervention outcomes. Do not report demonstrated literacy gains until they have actually been measured.
