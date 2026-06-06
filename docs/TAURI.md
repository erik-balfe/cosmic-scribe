# Tauri — native UI option for Cosmic Scribe

Last updated: 2026-06-06

Research note for backlog item **R1** ([BACKLOG.md](BACKLOG.md)). Approved for step-by-step migration.

**Doc index:** [README.md](README.md)

## Problem today

Tray → **History** or **Settings** runs:

1. `cosmic-scribe --history` or `--settings`
2. Binds `127.0.0.1:0` (random port)
3. `xdg-open http://127.0.0.1:PORT/...` → **browser tab**

Downsides:

- Opens a tab in whatever browser is default (feels non-native)
- No dedicated window (resize/minimize/alt-tab as “Cosmic Scribe”)
- Random port each launch
- Two processes (daemon + short-lived HTTP server)

## What Tauri is

[Tauri 2](https://v2.tauri.app/) — Rust backend + system **webview** (not Chromium/Electron).

| Layer | Cosmic Scribe today | With Tauri |
|-------|---------------------|------------|
| Frontend | Svelte 5 in `web/` | **Same Svelte** (reuse) |
| Backend | `src/web.rs` HTTP API | Rust `#[tauri::command]` or shared lib |
| Shell | Browser via `xdg-open` | Native window (GTK on Linux) |
| Tray daemon | `cosmic-scribe --daemon` | **Unchanged** |

Tauri is closer to “native webview + Rust” than Electron. Binary stays smaller than bundling Chromium.

## Linux / Fedora / COSMIC

On Fedora, Tauri uses **WebKitGTK 4.1** (`webkit2gtk4.1-devel`).

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel librsvg2-devel
```

Wayland + COSMIC: generally works (GTK app + WebKit). **Spike must verify:**

- `<audio>` playback for history detail (waveform + listen)
- Clipboard “Copy transcript” from webview
- Window focus when opened from tray while daemon runs

WebKit lacks some APIs vs Chrome; our UI is forms + lists + audio — low risk, but **audio is the main unknown**.

## Recommended architecture (if we proceed)

```
┌─────────────────────────────────────┐
│  cosmic-scribe --daemon             │  ← keep as today (tray, STT, inject)
│  ksni tray, IPC, arecord, xAI       │
└─────────────────────────────────────┘
              │ spawn
              ▼
┌─────────────────────────────────────┐
│  cosmic-scribe-gui (Tauri)          │  ← new crate or feature
│  Window: History | Settings         │
│  Svelte from web/dist               │
│  invoke() → shared Rust lib       │
└─────────────────────────────────────┘
```

**Do not** merge GUI into the daemon process — tray daemon should stay lightweight.

### Code migration path

1. Extract `web.rs` API logic into reusable functions (e.g. `list_recordings()`, `transcribe_recording(id)`).
2. Add workspace crate `gui/` with Tauri 2 + same `web/dist` assets.
3. Replace `fetch('/api/...')` in Svelte with `@tauri-apps/api` `invoke()` (thin wrapper keeps both paths during transition).
4. Tray spawns `cosmic-scribe-gui --view history|settings` instead of `--history` / `--settings` HTTP modes.
5. Deprecate `web::run_at` + `xdg-open` once GUI is stable.

## Pros

- Dedicated, resizable window — feels like a real app
- No browser tab clutter
- Direct Rust IPC (`invoke`) — no localhost HTTP, no random ports
- Same Svelte UI and Rust STT/keyring code
- Aligns with “COSMIC desktop app” positioning

## Cons

- **Extra system dep:** `webkit2gtk` (document in README/Homebrew caveats)
- **Build complexity:** `tauri-cli`, platform-specific CI
- **WebKit quirks** on some distros (macOS WKWebView stricter than Linux for some APIs)
- **Two artifacts** unless we ship one binary with optional GUI feature flag
- Homebrew/Flatpak packaging gets heavier

## Effort estimate

| Phase | Work |
|-------|------|
| Spike | 1–2 days: empty Tauri window + load Svelte + test audio on COSMIC |
| Migrate API | 2–3 days: lib extraction + `invoke` commands |
| Polish | 1–2 days: single instance, window reuse, tray integration |
| Packaging | 1–2 days: deps in formula, CI |

## Decision gate

Proceed with full migration **only if spike passes:**

- [ ] History list renders
- [ ] Detail view plays `.raw` audio
- [ ] Transcribe button works
- [ ] Settings save API key
- [ ] Window opens from tray without opening Firefox/Chrome

If audio fails in WebKitGTK, alternatives:

- Keep browser flow; improve with fixed port + `COSMIC_SCRIBE_NO_BROWSER` for users who prefer their browser
- **wry** (Tauri’s webview layer) without full Tauri — more work, less polish
- **slint** / **iced** — rewrite UI (reject: throws away Svelte)

## Spike (phase 1)

| Step | Status | What |
|------|--------|------|
| 1 | **done** | `web::spawn_server()` — API on background thread, returns URL |
| 2 | in_progress | `gui/` Tauri 2 crate — webview loads that URL |
| 3 | open | You test: History list, audio playback, Settings save |
| 4 | open | Replace `xdg-open` tray paths with `cosmic-scribe-gui` |

Run spike (isolated from prod):

```bash
cd web && npm run build
./scripts/install-gui-debug.sh
cosmic-scribe-gui-debug              # History
cosmic-scribe-gui-debug --settings   # Settings
```

Uses **same config/data** as prod (`~/.local/share/cosmic-scribe/`). Prod daemon untouched.

Remove GUI binary only: `./scripts/uninstall-gui-debug.sh`

**Build note:** if `pkg-config` is from Homebrew, the install script sets `PKG_CONFIG_PATH` to include `/usr/lib64/pkgconfig` (Fedora system -devel packages).

Fedora build deps (install once):

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel glib2-devel openssl-devel \
  libappindicator-gtk3-devel librsvg2-devel
```

See also [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## Status

**R1 — in_progress (spike).** Phase 1 GUI crate; full switch only after COSMIC UI test passes.