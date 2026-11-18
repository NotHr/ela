# Offline school workflow

## Roles and data flow

**The phone collects; the school desktop computes.** No OCR, speech recognition, NLP or scoring runs on the phone. There is no required cloud account, hosted API, analytics service or internet connection during assessment processing.

```text
Student's phone / shared school phone
  stable student ID + class + activity
  paragraph photograph + voice recording
  optional student-only voice reference for conversations
        ↓ finalized ODK submission XML + media
  USB / Bluetooth / local-network file transfer
        ↓ collection ZIP
School desktop
  safe ZIP import + instance-ID deduplication
  local photo OCR → local audio conversion
  student separation → bilingual ASR → local UD NLP
  provisional reading screen → required local SLM explanation/practice
  SQLite assessment history + per-student progress
```

Keep the phone's originals until the desktop has completed the jobs and its database has been backed up. File transfer is not a processing acknowledgement. An imported job can fail because a photo is unreadable, a model is missing, or the desktop is interrupted.

## OSS mobile collector recommendation

Use **ELA Collect** initially. It is the Apache-2.0 ODK Collect fork already adapted for local form import and ZIP/XML/media export, avoiding a new app with fragile offline storage. Its README describes forms under `Download/ela/forms` and export via the Android sharesheet. Audit/build a current APK and test it on the devices used by the school; this repository does not distribute an APK or claim device-level validation.

The supplied **`forms/ela-reading.xml`** collects:

- a stable, pseudonymous `student_id` (school-issued; ASCII letters/digits/`-_.`);
- optional display name, class, activity and reference language;
- paragraph photo and reading audio;
- student-only/conversation mode, automatic/fixed language processing;
- a student-only reference recording when conversation mode is chosen;
- ODK `instanceID` and phone capture/start timestamp.

Download it from the desktop's Import dialog or `GET /api/collection/form`; copy it to the phone using the collector's supported local-form installation process. The form has no external media dependencies or remote submission URL. Student-only is its default recording mode. There is no typing of the paragraph or manual scoring on the phone.

**Stock ODK Collect** is also OSS and records forms offline, but this Rust service is **not an OpenRosa/ODK Central server**. A deployment using stock Collect needs a tested way to export finalized instances and media or an additional local server bridge. Do not assume pointing Collect at localhost will sync to this application. KoboCollect does not remove that integration requirement. ELA Collect's existing ZIP export is the simpler fit here; a custom Kotlin/Flutter app would create additional offline storage, migration and maintenance work without improving assessment models.

USB/Bluetooth transfers need no network. [LocalSend](https://localsend.org/) is an OSS file-transfer option on a school LAN with no internet, but is not required. Do not upload children's packages to a public transfer/cloud service. Keep the desktop assessment server loopback-only; LAN file transfer does not require exposing the assessment API.

## Import contract and safety

Use the desktop **Import** button and choose the ZIP containing finalized XML and referenced media. The importer supports the supplied `ela_reading_v1` schema, not every historical ELA/ODK form. It resolves media relative to each submission XML; a flattened fallback is accepted only for a unique filename. Missing or ambiguous attachments are rejected, never assigned to the wrong child.

Limits: 128 MiB compressed ZIP, 256 MiB total decoded members, 512 files, 64 submissions, 64 MiB per member, 12 MiB per paragraph photo. No filesystem extraction occurs. Traversal/absolute paths, symlinks, duplicate archive members, DTD/entity XML and invalid identifiers are rejected. Large class collections should be split into smaller ZIPs.

The desktop serially processes imported jobs using one shared inference slot. A required SLM stage writes a plain-language, evidence-constrained English/Telugu summary even without a reference passage. It explains measurements and limits instead of presenting raw grammar tags as a diagnosis. If summary generation is unavailable, saved measurements remain accessible but processing is not marked complete; the report's **Generate summary** button retries without the original audio. Older reports can use that button too. The Import dialog shows accepted, duplicate and invalid entries; all accepted jobs appear in Assessments with current status. A single failed item does not prevent subsequent queued items from being processed.

`instanceID` deduplicates retransfers. The same instance cannot be reassigned to another student. A failed job can be deleted and its original package re-imported to retry. Creating a new form instance is appropriate for a new assessment attempt. Never reuse the same school student ID for different children, and do not generate a new student ID for each assessment.

Raw photos, audio and reference voice clips are staged in private disk-backed job directories rather than held in the in-memory queue during model processing; uploaded/decompressed ZIP data still has a bounded transient RAM footprint. These job files are temporary under normal completion. A restart marks unfinished jobs failed; re-import the original package to retry. Reports, extracted reference text, student transcript, NLP observations, identifiers and provenance remain in SQLite. The collector's source package is the recovery copy until processing and backup are confirmed.

## Desktop OCR

Tesseract's local LSTM models run with `tel+eng` for Telugu or `eng+tel` for English (target language first), not an online OCR endpoint. The flake includes Tesseract plus Telugu/English traineddata. Outside Nix, install them locally; configure `ELA_TESSERACT_BIN` and optionally `ELA_TESSDATA_DIR`.

JPEG, PNG, WebP and TIFF photos are decoded with memory/dimension limits, EXIF orientation is applied, and a private normalized PNG is passed to Tesseract. The current OCR layout assumes **one cropped printed paragraph**. Handwriting, multiple columns, perspective distortion, low light and unusual textbook layouts are not robustly handled. Use a clear, upright, cropped photo; do not mistake an OCR error for a reading error.

The extracted text and word-confidence summary are stored in the report. A starter quality gate requires average word confidence ≥85 and no word below 70 before enabling the reading-match screen. Otherwise processing still produces a report but withholds the reading score and recommends another photo. Confidence is not calibrated probability, and high-confidence OCR can still be wrong; screening remains provisional. Exact recovered reference text is part of progress grouping.

## Advanced local NLP, with open model licensing

The deployment now uses actual **spaCy `en_core_web_trf` for English** and **UDPipe for Telugu**, locally, for sentence segmentation, lemmas, POS, morphology and dependencies. Native Rust/Candle T5 proposes English transcript corrections, then official ERRANT classifies edits in a separate spaCy worker. The SLM summarizes their coverage/counts without receiving raw sentences. See [English grammar setup and resource requirements](english-grammar.md). The report includes POS/dependency counts, content-word lexical density, lemma diversity, morphology and mean dependency distance. When a suitable reference analysis is available it also measures reference content-lemma coverage. This goes beyond raw text matching. The corresponding English model/tool families are integrated, but Rust greedy decoding, bounded coverage and unpinned upstream artifacts mean exact ELA output/scoring parity is not claimed. These are correction proposals to recognized text, not confirmed child mistakes or a comprehension measure.

Telugu/English ASR segments are routed to their corresponding NLP models; mixed-script reference passages are not dependency-parsed as if they were monolingual. ASR errors and inferred punctuation affect parsing. Read-aloud syntax largely belongs to the printed passage, not to the child's authored grammar. These observations must not be turned into an invented grammar grade. For assessing independent language production, collect a separate retelling/conversation activity and validate a language-specific rubric.

The **UDPipe engine is MPL-2.0**, but many official pretrained model bundles use **CC BY-NC-SA** terms. They are not unrestricted open models and are not bundled in this source repository. A noncommercial school deployment can explicitly choose compatible pretrained models under those terms, retaining attribution/share-alike notices; compatible UD 2.5 English-EWT and Telugu-MTG weights were installed locally for this deployment with source/checksum/licence manifests under `models/nlp/`. Training is not technically required to run them. For deployments that need a licence without the noncommercial restriction, train models from appropriately licensed treebanks, for example **UD English-EWT** and **UD Telugu-MTG**, both carrying **CC BY-SA 4.0** licenses in the inspected repositories. Keep their attribution and share-alike notices with your artifacts. Data/model licenses are separate from software licenses.

A completely local training helper is supplied:

```sh
nix develop
scripts/train-nlp-model.sh te /path/to/te_mtg-ud-train.conllu models/nlp/te.udpipe
scripts/train-nlp-model.sh en /path/to/en_ewt-ud-train.conllu models/nlp/en.udpipe
export ELA_NLP_TE_MODEL="$PWD/models/nlp/te.udpipe"
export ELA_NLP_EN_MODEL="$PWD/models/nlp/en.udpipe"
export ELA_NLP_MODEL_REVISION=your-treebank-and-model-manifest
cargo build --bins
cargo run
```

The helper downloads nothing. Obtain source treebanks and licenses during provisioning or copy them by USB. Inspected source revisions: Telugu-MTG `72b7063462be67d41861992b7a322830af69428e`, English-EWT `491be069d36e378f67c29f183699eea88308ea42`. The starter training settings are not tuned or validated for child speech. The helper records source/model SHA-256 and runtime provenance. Set `ELA_UDPIPE_BIN` if necessary. The Nix shell includes UDPipe on supported platforms; the pinned package is marked unsupported on aarch64 Linux, where it needs a separately built compatible executable. Missing/failed NLP is reported explicitly, never replaced with fabricated tags.

## Individual progress

The **Students** view groups all jobs by school-issued student ID, not by a child's name. Each profile contains complete assessment status history, scored and unscored attempts, and passage-specific reading-match trends. Capture time (including timezone) is used for ordering, not the order in which offline ZIPs reach the desktop. Keep phone clocks reasonably correct; this is collector metadata, not a verified timestamp authority.

A trend series groups only the same reference text, reference language, recording mode, language-routing setting, analysis version, scoring policy, backend and model fingerprint. Native inference automatically SHA-256 fingerprints the actual ASR/LID/diarization model files, token tables and relevant inference settings. Per-file digests are cached using trusted local file identity/size/mtime to avoid repeated gigabyte-scale hashing; replacing artifacts invalidates them. Administrators must not modify models while jobs run. A model replacement therefore starts a separate series. External adapters with missing revision metadata can show historical scores but do not get an improvement delta.

The displayed change is an **observed reading-match percentage-point change**, not demonstrated literacy gain or a standardized proficiency improvement. Pace is shown separately; faster reading with more errors is not automatically called improvement. Incomplete speech, uncertain OCR and failed/pending jobs remain visible and do not become zero scores. A changed/easier passage is not compared against the old passage as if it proved progress.

Older records that lack a stable student ID are retained as distinct `legacy-assessment-*` identities. They remain available in Assessments but are not auto-merged into child profiles by matching names. A reviewed roster-linking migration is needed before those records can contribute to longitudinal reporting.

## Offline installation versus offline operation

At runtime all processing is local; missing local models produce setup errors, not cloud fallback. First-time `nix develop`, Cargo builds, APK installation and model/data provisioning can still require internet **unless their dependencies are already cached/copied**. This repository is not yet a complete air-gapped installer. A deployment administrator must prepare the application binaries, native libraries, FFmpeg, Tesseract/traineddata, UDPipe/models, local Python/spaCy/ERRANT packages, T5 weights and model/license manifests on a connected machine or USB package in advance.

Test a complete school workflow with internet physically disconnected, target Android devices, target desktop hardware and representative child speech/photos before rollout. Protect children's voice references, images and reports with consent, pseudonymous IDs, filesystem permissions, encrypted storage and backup/retention policies. Do not claim measurable learning gains until interventions and repeated comparable assessments have actually been evaluated.
