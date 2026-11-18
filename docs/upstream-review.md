# ELA repository review and Rust implementation map

Reviewed source (not only the project description):

- **ELA Web**, commit [`bef5e7f`](https://github.com/IT-for-Change/ela/tree/bef5e7f58c43861838b7036549a8beeaa4dae575): `ela/ela/api/assessments.py`, `api/submissions.py`, `doctype/activity/activity.js`, activity/submission/speaking controllers, `assessment_package.py` XML/ZIP import, assessment-output schema and configuration schema.
- **ELA AI**, commit [`d403303`](https://github.com/IT-for-Change/elaai/tree/d4033036bc56971d18c4c83a71665fdb647d6095): `apps/sdz/sdz.py`, `apps/langid/langdetect2.py`, the alternative `langid.py`, `apps/stt/stt.py`, `apps/nlp/nlp.py`, `nlp_functions.py`, `util.py`, `apps/report/reporting.py`, API client and Dockerfile.
- **ELA Middleware**, commit [`3d1611c`](https://github.com/IT-for-Change/elamid/tree/3d1611c12f036d319f3a17d88e911075e1460a5b): `app.py` and deployment configuration.
- **[ELA Collect](https://github.com/IT-for-Change/ela_collect)**: README describing local XForms and ZIP export of submission XML plus media. Android implementation was not audited.
- **[ECUBE data collection](https://github.com/IT-for-Change/ecube-data-collection)**: README describing audio/metadata or database exports. The collector implementation was not audited.

## How upstream fits together

ELA Web is a Frappe application, not merely an upload screen. It manages learners, teachers, cohorts, learning spaces, activities, assessment forms/packages, questions, submissions and assessment outputs. Its activity controller has explicit stage-launch buttons and a refresh/status operation. Assessment outputs are computed by the AI services rather than teacher-entered scores.

The Web API chooses recordings by pending operation and supplies stage-specific inputs. `update_submissions` stores outputs and advances recording status. `refresh_submission_status` prepares the next stage, optionally skipping conversation separation or language detection according to the assessment definition:

```text
Collect / ECUBE package
    → learner submission + question media
    → [learner / teacher separation for conversations]
    → [language identification]
    → speech transcription
    → text analysis
    → report
    → ASSESSED
```

The middleware `/run` endpoint launches the requested AI operation in a Docker container named `myelaai`, replacing an existing container if necessary. It passes callback URLs and configuration in an environment variable, mounts the AI directory, and uses host networking. The AI client fetches pending inputs and posts stage outputs/files back to Web. This limits simultaneous resource-heavy model execution, but does not imply that all stages in the inspected Web UI start themselves.

The AI implementation includes:

- pyannote-based conversation separation using a teacher reference recording;
- Vaani audio language identification, chunk decisions, and an external `langidmatrix.csv`;
- Whisper transcription, including attempts to exclude non-English portions;
- spaCy lexical/syntactic/morphological analysis and T5/ERRANT grammar correction;
- a reporting matrix combining a five-level language score and three-level speech-quantity score into **16 categories, 0–15**. It is not four teacher ratings summed out of 16.

Speech-quantity thresholds in NLP utilities are 1–10, 11–25, and >25 words. Upstream estimates non-English word counts from duration rather than transcribing all home languages. Some inspected functions have inconsistent names/signatures; a port should test intended rules rather than mechanically copy apparent defects. The required language decision CSV is not present in the inspected repository tree, so it cannot be reconstructed faithfully from those files alone.

## What changed in this Rust project

The first implementation incorrectly substituted a manual four-dimension rubric. That workflow is removed. Existing manual records remain separately exportable and are never presented as autonomous scores.

| Upstream responsibility | Current Rust implementation |
| --- | --- |
| Web activity/submission entry | Compact listmonk-inspired bilingual HTML/Oat admin, learner/class/activity metadata, read-aloud as the default |
| Middleware orchestration | Rust background task automatically advances every implemented stage; one inference slot limits memory use |
| Model lifecycle | Child Rust executable loads one local model; timeout/crash cannot block the web event loop indefinitely |
| Audio ingestion | Server-side FFmpeg normalization of common audio/video containers, bounded duration/size/time, no network protocols or playlists |
| Telugu transcription | AI4Bharat-derived IndicConformer Telugu CTC ONNX export through sherpa-onnx Rust bindings |
| English transcription | Whisper ONNX encoder/decoder through the same Rust API |
| Conversation separation | Native pyannote diarization + student-reference embedding matching; other speakers and model-detected overlaps excluded |
| Bilingual switching | Separate Rust/whisper.cpp helper evaluates Telugu/English posteriors per student window, routes language-specific ASR and records timestamps |
| Recording status | SQLite job statuses, stage events, automatic UI polling, persisted failures |
| Text analysis | Unicode-aware word count/distinct forms/diversity, ELA-inspired quantity bands, optional read-aloud word/grapheme edit distances |
| Reporting | Automatic saved JSON report, provisional reference-match screening, word-specific practice, teacher intervention suggestions, provenance and evidence limits; required local Qwen summary selection rendered as grounded English/Telugu explanations |
| Offline collection | Supplied ODK XForm and bounded XML/media ZIP importer with stable student IDs, capture timestamps and instance deduplication |
| Photo reference | Local Telugu/English Tesseract LSTM OCR with uncertainty gates |
| Linguistic observations | Actual spaCy `en_core_web_trf` for English; UDPipe for Telugu; POS, lemma, morphology, dependency counts, lexical density and reference content-lemma coverage |
| English correction proposals | Corresponding Vennify T5 checkpoint through Rust/Candle, then official ERRANT 3.0.0 with spaCy; original/suggested text and categorized edits; greedy decoding differs from ELA's five beams |
| Longitudinal reporting | Student history and comparable passage/model fingerprint series; all processing remains on the desktop |
| Existing manual records | Preserved in the old table, downloadable through a separate legacy endpoint |

The speech path does not use Python or launch Docker. The English linguistic path now uses isolated official spaCy/ERRANT Python workers, with native Rust T5 between them; no Docker or cloud runtime is required. Rust calls sherpa-onnx for ASR/diarization/embeddings and uses an isolated whisper-rs/whisper.cpp language-ID helper (to avoid native FFT symbol collisions). Both wrap native engines: **Rust integration, not a pure-Rust neural-network engine**. Telugu requires a compatible export and token table, not arbitrary PyTorch weights or an arbitrary ONNX graph. The documented Telugu export is community-maintained and derived from AI4Bharat; it is not an official AI4Bharat Rust distribution.

## Plain-language summaries versus upstream ELA

The inspected upstream NLP code loads spaCy `en_core_web_trf`, uses HappyTransformer's T5 interface with a local `vennify` model directory for English grammar correction, and applies ERRANT to categorize correction edits. T5 is itself a text-to-text language model, but its role here is correction—not generating a parent-friendly assessment narrative. The inspected reporting path uses rules/matrices; no equivalent generative narrative-summary stage was identified in that revision.

This application now uses official spaCy for English structure, the corresponding Vennify T5 checkpoint via Rust/Candle, and actual ERRANT 3.0.0 for edit classification; Telugu retains UDPipe. A required, separate Qwen3-0.6B Q4 stage selects/orders grounded explanations and approved activities. It is intentionally evidence-constrained: prewritten English/Telugu explanations supply the prose, not unrestricted model-generated claims. The SLM does not replace the T5/ERRANT stage. Proposed transcript edits remain unconfirmed learner errors; zero edits does not establish correct grammar.

## Explicit remaining gaps

This is not yet an equivalent replacement for the whole ELA ecosystem:

1. **Conversation separation quality**: diarization/role matching now exists, but must be validated for child voices, cross-language identity consistency and similar speakers. It excludes overlap rather than reconstructing clean overlapping speech. A known student-only reference is required; speaker IDs alone cannot establish roles. Student-only mode still trusts the supplied audio.
2. **Original 16-category scoring**: window-level bilingual LID now exists, but is not ELA’s original Vaani model/decision table. `proficiency_score` remains null. Rapid within-window code switching and other languages are not reliably handled.
3. **Grammar/comprehension**: English spaCy/T5/ERRANT components are now integrated, but exact artifact versions in the inspected deployment were not pinned. Rust greedy T5 decoding, bounded sentence coverage and inference implementations differ from upstream; full output/scoring parity is not claimed. English correction is not equivalent Telugu correction, confirmed learner-error diagnosis or comprehension assessment. Read-aloud syntax is largely the passage author's language. See `docs/english-grammar.md` for model licences, resource observations and limits.
4. **Collection compatibility**: a supplied offline ODK form and matching Collect ZIP importer now exist. Arbitrary historical ELA forms/assessment definitions, ECUBE packages, full roster/activity-package administration and an OpenRosa server are not implemented. The Android collector implementation is reused externally; no new APK is distributed.
5. **Deployment**: local single-user service only; no Frappe compatibility layer, authenticated multi-user server, or public deployment support.
6. **Job recovery**: statuses persist, audio does not. After a restart unfinished jobs are marked failed and require re-upload. Busy submissions receive HTTP 429 rather than entering a persistent queue.

Automatic analysis is useful without inventing missing grammar/comprehension/proficiency judgments. Full ELA parity requires additional model ports, the original decision assets, language-specific validation with educators, and integration tests on real child speech in both languages.

The newer **reference-match reading screen** addresses the foundational-literacy use case without claiming to reproduce ELA’s score: word alignment → 0–100 reading-match score → provisional bands → target-word practice and intervention suggestions. Thresholds are explicit starter rules, not validated Telangana norms; a diagnostic grade is not inferred. See README for the exact policy.

## UI reference

Inspected `beta.listmonk.app` (public landing/login) and listmonk’s `static/admin/views/base.html`, `partials/sidebar.html`, `views/dashboard.html`, `views/subscribers.html`, and `assets/css/style.css`. The Rust app uses its own HTML/CSS/JS, taking layout cues: narrow sidebar, compact blue-accent navigation, muted table rows, filters, and modal forms. No listmonk logo or application code is copied. The bundled Geist font retains its upstream font license.
