# OSS release readiness

Last reviewed: 2026-06-14

**Docs:** [README.md](README.md) · [BACKLOG.md](BACKLOG.md)

## Stable (v0.3.x)

| Area | Status | Notes |
|------|--------|-------|
| Core loop | OK | Shortcut/tray → record → batch REST STT → wtype or clipboard (Settings) |
| Daemon + IPC | OK | Unix socket toggle; `com.cosmic-scribe.service` login autostart |
| Tray (SNI) | OK | Idle / red recording / blue processing; cancel; History, Settings, Quit |
| Tauri GUI | OK | `cosmic-scribe-gui` — app menu, History + Settings tabs |
| API key storage | OK | AES-256-GCM, env override |
| Recording history | OK | `.raw`, `.txt`, `.stt.json` (timestamps), `.json` (edits) under `~/.local/share/cosmic-scribe/recordings/` |
| Web UI — list/detail | OK | Waveform, playback, edit versions, delete |
| Tests | OK | 54 unit tests, recordings API regression |
| License | OK | MIT |

## Beta / WIP (label in UI + docs)

| Feature | Status | Blocker |
|---------|--------|---------|
| LLM correction (OpenRouter) | Beta / experimental | Works intermittently; not production quality yet. Marked as such in public README. |
| Model / provider choosing | Beta / experimental | Recent addition; not fully reliable for daily use yet. |
| Karaoke playback UI | Planned | `docs/KARAOKE.md` — storage done |
| Streaming STT (live) | Not planned for v0.1 | 2× price; batch REST has timestamps |
| Fedora RPM | Not started | — |

## Pre-public checklist

- [x] GitHub repo `erik-balfe/cosmic-scribe` (private); URLs point to cosmic-scribe
- [x] GitHub private repo created + pushed (default branch `master`, full history with `erik.balfe@proton.me`)
- [x] Tray screenshots in README (idle / recording / recognizing)
- [x] CI: `cargo fmt`, `clippy -D warnings`, `cargo test`, `cargo build --release`, `npm run lint`, `npm run build`
- [x] Single history store: recordings directory only (removed unused `history.db`)
- [x] Security note in README: API keys encrypted at rest, cloud STT sends audio to xAI (brief + honest)
- [x] README focused on users + Cosmic emphasis while noting broader Wayland support; dev content moved to CONTRIBUTING.md

**Note:** History viewing, copying, relistening, and re-transcribing work reliably. LLM correction + model choosing (recent additions) are still experimental — marked clearly in README and STATE.md.

## Known limitations (document, don’t hide)

- Linux + Wayland-oriented (`wl-copy`, `wtype` for optional typing — see `docs/OUTPUT.md`)
- Requires `arecord`, xAI API key (paid)
- Tray inject vs clipboard-only modes
- No Flatpak/AppImage yet