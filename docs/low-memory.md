# Low-memory execution and required SLM summaries

## Execution policy

The phone only collects; the desktop computes. One global inference/job permit serializes heavy stages:

```text
OCR process → exit
speaker segmentation + embeddings → release
language-ID helper → exit
Telugu ASR worker → exit
English ASR worker → exit
spaCy English / UDPipe Telugu process → exit
English T5 Rust worker → exit
spaCy + ERRANT worker → exit
required local SLM summary → exit
save completed report
```

Diarization needs its small segmentation/embedding models together. Native allocator arenas may persist in the speech coordinator until it exits. The large ASR models, language detector and SLM live in short-lived processes so their model heaps are reclaimed before the next heavy model starts. Root model commands have dedicated Unix process groups: timeout/cancellation kills the group; a parent-pipe watchdog covers abrupt desktop termination. Explicitly daemonizing custom adapters are not covered.

Bilingual ASR processes one language at a time and restores results to chronological segment positions. Adjacent verified windows with the same student/language are coalesced up to 25 seconds; no merging across excluded spans, speaker gaps or detected switches. The async runtime, default ASR/LID compute and FFmpeg use two threads; OCR is capped at two OpenMP threads. `ELA_ASR_THREADS` tunes native speech compute independently.

## Queue media and model hashes

Queued media is staged in private per-job directories under `.ela-work/` (`ELA_WORK_DIR` overrides). Choose an encrypted, disk-backed directory rather than tmpfs. Uploaded/decompressed ZIPs still have a bounded transient RAM peak (128 MiB compressed, 256 MiB decoded); split large collections on weak PCs.

Normal completion/failure removes staged files. Hard crashes can leave private directories; clean these under school retention procedures after stopping the service. Raw media is not a resumable persistent queue: keep the original collector package. Reports and measurements are in SQLite.

Actual model/token SHA-256 digests are cached by canonical file path, identity, size and modification time. Replacements/metadata changes invalidate them. Combined inference fingerprints also include worker/configuration versions and relevant explicit GGML build flags. Model files are trusted/admin-managed: use read-only directories and do not change them during batches. `ELA_DISABLE_MODEL_CACHE=1` forces complete rehashing for audits. The digest cache is not a tamper-proof signature system.

## What the required SLM does

A report now needs a successful local SLM summary before it is marked **complete**, including recordings without an expected paragraph. The stage is `writing_summary` and runs after all other heavy models have exited. `ELA_ENABLE_SLM` no longer disables it.

The tiny model **selects and orders reviewed evidence-based explanations**, rather than generating unconstrained claims. Its grammar-constrained JSON selects explanation IDs and approved practice/intervention IDs. The application renders the selected explanations in English and Telugu, interpolating only measured values. This is deliberately not free-form diagnosis or a claim that the model authored novel Telugu prose.

The readable report explains:

- recording duration and recognized words—not necessarily correctly read words;
- whether a provisional reference-match score exists, and its limits;
- sentence/base-word observations when NLP is available—not overall vocabulary or grammar ability;
- recognition uncertainty and unmeasured comprehension/proficiency;
- appropriate next steps for reading, spontaneous speech or insufficient evidence.

All core limitations must appear; unavailable facts, invented IDs, duplicate selections, extra score fields and invalid cardinality fail validation. Technical POS/morphology/dependency information remains available in collapsed sections. No names/IDs, raw transcripts, reference paragraphs or images enter the SLM prompt. It sees only already-grounded explanation candidates and action IDs.

Scores remain calculated by the existing deterministic analysis. This summary layer is separate from the newly integrated English spaCy/T5/ERRANT stage; it explains proposed-edit counts and coverage, not confirmed learner mistakes. Neither layer assesses comprehension or proves educational improvement. See `docs/english-grammar.md` for the actual packages, native T5 engine, decoding differences and memory requirements.

## Configure

The installed candidate is Apache-2.0 **Qwen3-0.6B Q4_K_M**, using the community [unsloth GGUF export](https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/tree/50968a4468ef4233ed78cd7c3de230dd1d61a56b). Review upstream provenance/licences. Provision weights and runtime once; the assessment service downloads nothing.

```sh
export ELA_SLM_MODEL=/absolute/path/qwen3-0.6b-q4_k_m.gguf
export ELA_LLAMA_BIN=/absolute/path/to/llama-completion
export ELA_SLM_MIN_FREE_MIB=1536 # optional; minimum allowed is 1024
cargo build --release --bins -j2
./target/release/ela-local
```

Use llama.cpp's **`llama-completion`**, tested with 0.9.0, not the newer conversational `llama-cli`. Install/cache that OSS executable separately; it is not a mandatory Nix dev-shell dependency. The prompt targets Qwen3/ChatML with thinking disabled.

Limits: GGUF ≤600 MiB; CPU only; two threads; 2048-token context; 320-token output limit; batch 128/microbatch 64; input evidence ≤4000 characters; 90-second timeout. Validated selections are cached by model/evidence/policy fingerprint. No network endpoints or automatic weight downloads are configured.

The current memory admission check uses Linux `MemAvailable`; unknown availability blocks generation conservatively. The reserve is a heuristic, **not a hard RSS limit or guarantee against OOM**. Model file size is not total inference RAM.

## If the required summary cannot run

The application preserves completed measurements, records a processing failure and **does not silently substitute a generic/rule-based summary or mark the assessment complete**. Free memory/fix model setup, then choose **Generate summary** in the report. This retries only the summary under the same global permit; the original recording is not needed. Existing older reports without summaries expose the same button.

API: `POST /api/assessments/{id}/summary` returns 202, or 429 if the desktop is busy. Missing saved measurements require re-uploading audio. Summary retries do not recompute scores, transcripts or model provenance.

Diagnostic smoke check (does not modify the assessment database):

```sh
./target/release/ela-local --slm /path/to/exported-assessment.json
```

Do not run diagnostic model CLIs alongside active assessment jobs on a weak PC. They are separate processes, not clients of the desktop's global inference permit.

## Hardware and latency

A 2 GiB machine may not have enough headroom for the required summary plus OS/GUI. The earlier ASR/UDPipe/SLM-only profile was tested near 1.1 GiB process-family RSS, but the newly requested full spaCy transformer load peaks near 2.7 GiB by itself. It needs 3 GiB available memory; native T5 needs 2 GiB available. **8 GiB is a safer starting target for the full English stack**; some 4 GiB desktops will be blocked by admission checks. This is guidance, not certification. Do not bypass memory safety to force completion. More RAM does not validate recognition/assessment accuracy.

Use release binaries. The portable whisper-rs development build has AVX2/native CPU kernels disabled; short-window LID can dominate CPU time. Administrators compiling on the actual target CPU can evaluate `GGML_NATIVE=ON cargo build --release --bins -j2` with low CMake build parallelism. Do not ship a machine-specific binary to incompatible/older CPUs. Portable or CPU-specific fleet packages need their own validation.

Process isolation trades cold-loading time for bounded live model memory. A future same-language multi-student batch scheduler could reduce repeated model loads; it is not implemented. See `docs/testing.md` for developer-machine observations, including which measurements predate the required summary. Validate target Android devices, desktop RAM/latency, representative child speech and educational outcomes before rollout.
