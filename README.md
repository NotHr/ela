# ELA Local

Offline Telugu/English reading assessment for school desktops, inspired by [ELA](https://github.com/IT-for-Change/ela).

**Phones collect. The desktop analyzes. No cloud processing.**

## What it does

- Import student details, paragraph photos and recordings from ELA Collect/ODK submission ZIPs.
- Run local OCR, speech recognition, speaker separation and Telugu/English language routing.
- Analyze English with spaCy, T5 and ERRANT; use UDPipe for Telugu.
- Produce readable English/Telugu summaries and practice suggestions using a required local SLM.
- Keep assessment history and comparable reading trends for each student ID.

Heavy model stages run sequentially. The full English stack is best tested on **8 GiB RAM or more**; spaCy loading alone was observed near 2.7 GiB RSS.

## Run

Install [Nix](https://nixos.org/download/) and configure model paths in a local `.env` using the [setup guide](docs/setup.md). Then run from the project root:

```sh
nix develop
cargo build --release --bins -j2
source .env
./target/release/ela-local
```

Open **http://127.0.0.1:3000**. Stop with **Ctrl+C**.

**Configure local models first.** `.env` is your local shell configuration, not a bundled file or an automatically loaded application setting. See [setup and model downloads](docs/setup.md) and [English model provisioning](docs/english-grammar.md). First-time installation needs internet unless dependencies and weights are already cached. Runtime processing is offline.

## Phone collection

Use the supplied [`forms/ela-reading.xml`](forms/ela-reading.xml). Finalize forms on the phone, transfer their XML/media ZIP to the desktop, then click **Import**.

ELA Collect's ZIP export is the simplest fit. Stock ODK Collect needs manual export or a separate server bridge: **this application is not ODK Central**. See the [collector workflow](docs/offline-workflow.md).

## Documentation

- [Setup, model paths and commands](docs/setup.md)
- [Phone collection and student progress](docs/offline-workflow.md)
- [English spaCy/T5/ERRANT setup and limitations](docs/english-grammar.md)
- [Memory policy and required SLM summaries](docs/low-memory.md)
- [Tests and validation](docs/testing.md)
- [Upstream ELA comparison](docs/upstream-review.md)

## Important limits

This is a prototype, not a validated proficiency test or full ELA replacement. OCR/ASR can be wrong. Grammar changes are proposals about recognized text, not confirmed child errors. Reading-match scores are provisional; no educational gains have been demonstrated.

Keep student IDs pseudonymous, protect recordings/reports, and back up the database. `.env`, databases, models, temporary media and build files are excluded from Git. Models retain their own licences, including noncommercial restrictions where applicable.

## Development

```sh
nix develop
cargo test --locked
cargo fmt --check
```

## License

[AGPL-3.0-or-later](LICENSE). Third-party code, fonts and models retain their respective licences. This is an independent project, not an official IT for Change release.
