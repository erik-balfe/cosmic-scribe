# STATE — Cosmic Scribe

Last updated: 2026-06-07

**Docs index:** [`docs/README.md`](README.md) · **Tasks:** [`docs/BACKLOG.md`](BACKLOG.md) · **Shortcut:** [`docs/SHORTCUT.md`](SHORTCUT.md) · **Tauri:** [`docs/TAURI.md`](TAURI.md)

Internal engineering log (not marketing copy). Keep in sync with `docs/KARAOKE.md` and `docs/RELEASE.md`.

## Done

- Architecture: single binary, CLI modes (`--daemon`, `--record-once`, `--trigger`, `--file-input`, `--settings`, `--configure`, `--install`, …)
- Pure state machine: Idle / Recording / Transcribing / Inserting / Error
- IO behind traits — **44+ tests**
- **Batch REST** xAI STT (`POST /v1/stt`) — full file after stop, not WebSocket streaming
- STT retry + timeout; tray capsule: red = recording, blue = transcribing; ignore tray click during STT
- **Text output**: wtype default (`docs/OUTPUT.md`); optional clipboard-only in Settings
- Per-recording artifacts: `.raw`, `.txt`, **`.stt.json`** (word timestamps + raw API JSON)
- Unix socket IPC, AES-256-GCM API keys, language config
- Tray SNI (Cosmic Scribe title); History + Settings → **cosmic-scribe-gui** (Tauri prod)
- Web UI: history list, detail (waveform, versions, user edit, Copy + toast)
- Lifecycle: `--install`, `--update`, `--start`, `--stop`, `--status`
- AI correction via OpenRouter (beta)

### Verified with real xAI API

| Test | Result |
|------|--------|
| `--record-once` lang=ru | Full Russian paragraph |
| `--file-input` | English, full cycle |
| Empty/short audio | Rejected |
| Daemon + trigger | IPC works |

## Beta / WIP

- LLM correction (mark words → Fix with AI): intermittent quality
- OpenRouter model list via models.dev + curl cache

## Next (release track)

See `docs/RELEASE.md`, `docs/DISTRIBUTION.md`, `docs/OUTREACH.md`.

- GitHub public when ready
- Fedora COPR / release binary
- **Karaoke UI** — plan in `docs/KARAOKE.md` (storage done, UI not started)

## Low

- VAD, configurable input device
- Re-transcribe from history — **done** (`POST /api/recording/:id/transcribe`)
- Optional: WebSocket STT only if we add *live* captions (not for dictation paste)

## Changed decisions

| Old | New | Why |
|-----|-----|-----|
| `wtype` with transcript text | Clipboard + Ctrl+V | Char-by-char looked like slow streaming |
| STT: `text` only | `.stt.json` with `words[]` | Karaoke + seek-by-word later |
| libsecret | Config file + AES-GCM | No D-Bus dep |
| cpal | arecord | No ALSA headers |
| Tray blink loop | Solid icons | System-wide lag on COSMIC |
| History viewer TODO | Web UI shipped | Done |