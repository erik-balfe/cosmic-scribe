# Contributing to Cosmic Scribe

Thank you for your interest! This project is a personal daily driver that I'm opening up. Contributions, feedback, and packaging help are very welcome.

## Development setup

### Build from source

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
cargo build --release
```

The binary is at `target/release/cosmic-scribe`.

## Quality checks

Same as CI (requires `rustfmt` + `clippy` — on Fedora: `sudo dnf install rustfmt rust-clippy`):

```bash
./scripts/check.sh
```

Or individually:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cd web && npm ci && npm run lint && npm run build
```

### Install for local testing

```bash
./scripts/install-prod.sh                 # daemon + Tauri GUI
./scripts/install-gui-native-prod.sh      # optional: libcosmic UI (preferred on COSMIC)
```

Auth: API key in Settings / `cosmic-scribe --set-key` / `COSMIC_SCRIBE_API_KEY`, **or** `cosmic-scribe --login` (SuperGrok plan OAuth).

**STT providers:** default dialect is xAI (`POST …/v1/stt`). Endpoint is configurable. OpenAI Whisper and other dialects need a client adapter — see **[docs/STT_PROVIDERS.md](docs/STT_PROVIDERS.md)** (welcome contributions).

Runtime deps include **ffmpeg** (progressive Opus). See [docs/INSTALL.md](docs/INSTALL.md).

Run the daemon for development:

```bash
./target/release/cosmic-scribe --daemon
```

Trigger recording:

```bash
./target/release/cosmic-scribe --trigger
```

### GUIs

| Path | Notes |
|------|--------|
| **libcosmic** `gui-native/` | Native COSMIC look; `cargo build -p cosmic-scribe-gui-native` (needs system libs; CI may exclude it) |
| **Tauri** `gui/` + `web/` | WebKit window; build `web` then Tauri package |

Svelte UI (embedded in daemon / Tauri):

```bash
cd web && npm install && npm run build
```

### Running tests

```bash
cargo test
```

There are ~39 unit/integration tests covering the state machine, audio validation, IPC, recordings API, etc. They use WireMock for STT.

### Requirements for development

Same as runtime + build tools:

Fedora example:
```bash
sudo dnf install alsa-utils wl-clipboard libnotify rust cargo nodejs npm
```

## Architecture overview (for contributors)

The core is deliberately simple and testable:

- `src/state.rs` — pure state machine (5 states, 8 events). No IO.
- `src/traits.rs` — traits for `AudioCapture`, `SttClient`, `TextInjector`, `KeyringStore`, etc. Easy to mock.
- `src/app.rs` — main event loop tying everything together.
- `src/web.rs` — embedded HTTP + WebSocket server + Svelte assets. Also handles OpenRouter correction calls.
- `src/main.rs`, `src/tray.rs`, `src/ipc.rs`, etc. — platform glue, tray (ksni), Unix socket IPC.

Key design goals:
- Everything important is behind traits → high test coverage without real hardware or network.
- Single binary, no external services at runtime besides the STT API.
- Low resource use on Linux laptops.

See also [docs/README.md](docs/README.md) (doc index), [docs/STATE.md](docs/STATE.md), and [docs/BACKLOG.md](docs/BACKLOG.md).

### Tauri GUI (`gui/`)

Production app window for History/Settings (`cosmic-scribe-gui`):

```bash
./scripts/install-gui-prod.sh    # build + install to ~/.local/bin
cosmic-scribe-gui                # History
cosmic-scribe-gui --settings     # Settings
```

Build deps: [docs/INSTALL.md](docs/INSTALL.md). Architecture notes: [docs/TAURI.md](docs/TAURI.md).

Debug binary (does not replace prod): `./scripts/install-gui-debug.sh`

## Using jj (Jujutsu)

The repo uses jj (colocated with git). Common commands:

```bash
jj status
jj log
jj commit -m "message"
jj bookmark list
jj git push --bookmark master
```

The default branch is `master` (traditional naming respected here).

When pushing after local changes, `jj git push --bookmark master` is usually sufficient.

## Making changes

1. Create a branch or use a change (jj style).
2. Make focused commits with clear messages.
3. Run `cargo test` and manually test the flow (recording → transcription → text output + history UI).

After changing anything under `web/src/`, run `cd web && npm run build` and commit updated `web/dist/` (embedded in the Rust binary).
4. Update docs/README if user-facing behavior changes.
5. Open a PR against `master`.

Areas where help is especially appreciated:
- Packaging (COPR, AUR, Flatpak, proper Homebrew tap for Linux)
- Better error handling / user messages
- Wayland/DE compatibility (especially tray and injection)
- Documentation improvements
- Reliability of the daemon (auto-restart, logging)

## Beta / experimental areas (as of now)

- LLM word correction via OpenRouter + model picker
- These are recent additions and quality/reliability is not yet production-ready. Focus PRs on the stable core unless discussing in an issue first.

## Questions / feedback

Open an issue or start a discussion. Real usage reports (especially on Cosmic or other compositors) are extremely valuable.

Thanks for helping make Cosmic Scribe better!