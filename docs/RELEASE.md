# OSS release readiness

Last reviewed: 2026-08-03

**Docs:** [../README.md](../README.md) · [BACKLOG.md](BACKLOG.md)

## Product fit (stable)

| Area | Status | Notes |
|------|--------|-------|
| Core PTT loop | OK | Shortcut/tray → progressive Opus → batch REST STT → wtype/clipboard |
| Daemon + IPC | OK | Unix socket; login autostart unit |
| Tray (SNI) | OK | Idle / red / blue; cancel; History, Settings, Quit |
| OAuth | OK | SuperGrok path; own session; warm token |
| API key fallback | OK | Encrypted at rest |
| History | OK | `.raw`, `.txt`, `.stt.json`, edits; re-transcribe |
| libcosmic GUI | OK | Prefer when installed |
| Tauri GUI | OK | Parallel install path |
| Tests | OK | Core + gui-native unit tests |
| License | OK | MIT |

## Explicit non-goals (do not ship as “coming soon” hype)

| Item | Status |
|------|--------|
| Streaming / live STT | Not the product (PTT + final paste) |
| Pause button | Not planned — stop the take instead |
| Cloud sync | Out of scope |
| Silence cutting | Planned (F13); not shipped |

## Pre-public / pre-announce checklist

- [x] README: near-zero UI, OAuth + API key, how to use  
- [x] No internal roadmap essay on GitHub  
- [x] Tray screenshots in README (product identity)  
- [ ] Optional: app-window screenshots of native GUI later  
- [ ] `Cargo.toml` version + tag when cutting a release  
- [ ] Push `master` when ready (explicit)  
- [ ] GitHub topics: `cosmic-desktop`, `dictation`, `wayland`, `speech-to-text`, `pop-os`  

## Known limitations (honest)

- Linux + Wayland-oriented (`wl-copy`, `wtype`)  
- Cloud STT default dialect is **xAI** (`/v1/stt`); Bearer keys + optional plan OAuth  
- **OpenAI Whisper** (`/v1/audio/transcriptions`) is **not** a drop-in endpoint swap — see [STT_PROVIDERS.md](STT_PROVIDERS.md) (F7)  
- **No silence cutting / VAD upload trim** yet — long thinking pauses are still sent to the API (F13; design in maintainer LOCAL docs). We deliberately **do not** abort takes on RMS “silence” (false-positive risk).  
- No pause button (stop ends the take)  
- No Flatpak yet  
- `Formula/cosmic-scribe.rb` version is updated by CI on **tag** push (may lag `Cargo.toml` between tags)

## Release cut (when maintainer decides)

1. `./scripts/check.sh` green (core + web; gui-native excluded from clippy/test in CI)  
2. Smoke: sign-in or API key, one dictation, History re-transcribe, Settings save  
3. Bump `Cargo.toml` version if needed (already `0.3.3` on tip as of 2026-08-03)  
4. `jj` commit if dirty · push `master` · tag `vX.Y.Z` · push tags  
5. Verify CI release artifacts + formula  

## Pre-ship residual risks (accepted or deferred)

| Risk | Severity | Plan |
|------|----------|------|
| ffmpeg finalize hang (no timeout) | medium | Defer; rare; progressive path usually wins |
| STT multi-retry × long timeout | low | Defer; env overrides exist |
| Custom STT endpoint can exfiltrate key | medium | By design for proxies; docs warn |
| auth_mode vs env key priority | low | Fix if still wrong on tip |
| Dual GUI (Tauri + native) | low | Prefer native when installed |
