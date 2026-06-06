# Backlog — Cosmic Scribe

Task tracker for bugs, features, and research. Update status as work progresses.

**Statuses:** `open` · `in_progress` · `done` · `deferred`

Last updated: 2026-06-06

**Doc index:** [docs/README.md](README.md)

---

## Bugs

| ID | Status | Summary | Notes |
|----|--------|---------|-------|
| B1 | **done** | Recording starts without API key; error only after STT | Block at Idle→Recording; notify + open Settings |
| B2 | open | Stale `voice-input` install still running on dev machine | See migration notes in README; user must `--install` cosmic-scribe |

---

## Features

| ID | Status | Summary | Notes |
|----|--------|---------|-------|
| F1 | **done** | Re-transcribe failed recordings from History | `POST /api/recording/:id/transcribe`; Detail UI button when no transcript |
| F2 | **done** | Rename binary/paths `voice-input` → `cosmic-scribe` | Code + user install migrated |
| F3 | open | Karaoke UI (word-timed playback, click-to-seek) | Plan: [KARAOKE.md](KARAOKE.md) steps 3–6 |
| F4 | **done** | Tray History vs Settings open correct routes | `--history` and `--settings` |
| F6 | **done** | README + shortcut setup docs for users | [SHORTCUT.md](SHORTCUT.md) + README Quick start |
| F5 | open | Fedora COPR / packaged binary | `docs/DISTRIBUTION.md` |

---

## Research / architecture

| ID | Status | Summary | Notes |
|----|--------|---------|-------|
| R1 | in_progress | **Tauri 2** for native Settings/History windows | Spike: `gui/` crate loads UI in WebKit window. See [TAURI.md](TAURI.md). Daemon stays separate. |
| R2 | deferred | WebSocket live STT | 2× API cost; batch REST has timestamps — see `docs/KARAOKE.md` |

---

## How to use this file

1. Add new rows with the next ID in each section.
2. Move status when picking up or shipping work.
3. Link to design docs (`KARAOKE.md`, `OUTPUT.md`) instead of duplicating specs here.
4. Keep `docs/STATE.md` as the high-level engineering snapshot; this file is the **actionable task list**.