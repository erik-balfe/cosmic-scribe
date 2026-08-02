# Backlog — Cosmic Scribe

Task tracker for bugs, features, and research. Update status as work progresses.

**Statuses:** `open` · `in_progress` · `done` · `deferred`

Last updated: 2026-08-03

**Doc index:** [docs/README.md](README.md)

**Priority:** ship polish. F8 OAuth, F10 native UI, F11 progressive Opus **done** on master tip (unpushed until release).

---

## P0 shipped — xAI OAuth

| ID | Status | Summary | Notes |
|----|--------|---------|-------|
| **F8** | **done** | **xAI OAuth for STT (SuperGrok / Premium+)** | Own device-code login; encrypted `xai-oauth.json`; API key fallback. CLI: `--login` / `--logout` / `--no-browser`. |
| F9 | open | Local usage meter + soft/hard budgets | Complements OAuth rate limits. |
| R3 | open | Other providers’ OAuth | Mirror F8 when a second backend offers subscription OAuth. |

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
| F1 | **done** | Re-transcribe failed recordings from History | `POST /api/recording/:id/transcribe` |
| F2 | **done** | Rename binary/paths `voice-input` → `cosmic-scribe` | Code + user install migrated |
| F3 | open | Karaoke UI (word-timed playback, click-to-seek) | Plan: [KARAOKE.md](KARAOKE.md) |
| F4 | **done** | Tray History vs Settings open correct routes | `--history` and `--settings` |
| F6 | **done** | README + shortcut setup docs for users | [SHORTCUT.md](SHORTCUT.md) |
| F5 | open | Fedora COPR / packaged binary | `docs/DISTRIBUTION.md` |
| F7 | open | Pluggable STT **dialects** (OpenAI Whisper etc.) | Endpoint URL done (xAI dialect). OpenAI needs `/v1/audio/transcriptions` + `model` — [STT_PROVIDERS.md](STT_PROVIDERS.md) |
| F8 | **done** | xAI OAuth (subscription quota) | on master |
| F9 | open | Local usage meter + budgets | |
| F10 | **done** | **libcosmic native History/Settings UI** | UX pass + progressive STT; prefer native when installed. See **Native UI fix list**. |
| F11 | **done** | Progressive Opus encode during capture | Finalize ~ms; REST upload OGG; `.raw` kept |
| F12 | **wontfix** | Streaming STT (WebSocket) as core path | Not the product — PTT wants final paste; REST is the right (and cheaper list) path |
| F13 | open | **Silence cutting (VAD)** on upload path only | **Not shipped.** No RMS hard-reject (false-positive safe). Future: conservative cut before STT; full `.raw` kept. Design: `docs/LOCAL/VAD-SILENCE.md`. No pause button. |

---

## Native UI fix list (F10 — done)

Historical checklist from UX pass. Match real COSMIC apps.

### Settings layout

| ID | Status | Issue | Desired |
|----|--------|-------|---------|
| N1 | **done** | Rows look cramped / shrunk while window has free space | Full-width `view_column` + Fill containers |
| N2 | **done** | Captions / notes as fake empty-title rows look wrong | `builder(title).description(…).control(…)` only |
| N3 | **done** | Save button placement weird | Header Save only |
| N4 | **done** | Segmented controls / notes limited component feel | segmented_control + theme spacing |
| N12 | **done** | All history times “just now” | Parse local wall-clock timestamps (not UTC) |
| N13 | **done** | No stop; bad progress on 2nd play; no seek | Stop + slider seek (ffmpeg offset when available) |
| N14 | **done** | Edit: no cancel; same-text creates version | Cancel; skip no-op save |
| N15 | **done** | View transcript scroll glitches | Scrollable `text::body` instead of text_editor |
| N16 | **done** | Tray legend text-only | System icons per state |
| N17 | **done** | xAI section missing plan links | Links to x.ai/grok + console; subscription copy |

### History list

| ID | Status | Issue | Desired |
|----|--------|-------|---------|
| N5 | **done** | **Items can’t be opened** to see full transcript | Row click + Open (`go-next`) → detail; view uses scrollable body text |
| N6 | **done** | Third action icon is vague | Open uses `go-next-symbolic` + tooltip “Open full transcript” |
| N7 | **done** | No tooltips on row actions | Open / Copy / Delete tooltips |
| N8 | **done** | **Show more** placement looks wrong | Centered footer control; “End of history” when exhausted |
| N9 | **done** | **Show more** gives no feedback | Toast “Loaded N more” / “No more recordings” |

### General

| ID | Status | Issue | Desired |
|----|--------|-------|---------|
| N10 | open | History detail / Settings polish vs cosmic-term | Ongoing polish; not a ship blocker |
| N11 | open | Desktop/tray dual-stack (Tauri vs native) | Native preferred when installed; Tauri remains fallback |

**Third button today (code order):** Open (`document-open-symbolic`) → Copy (`edit-copy`) → Delete (`edit-delete`). User-visible confusion is almost certainly **Open**, not export.

---

## Research / architecture

| ID | Status | Summary | Notes |
|----|--------|---------|-------|
| R1 | **done** | **Native GUI** (libcosmic + shared api) | Prefer native when installed; Tauri remains parallel path |
| R2 | deferred | WebSocket live STT | 2× API cost |
| R3 | open | Multi-provider OAuth patterns | Feeds F7/F8 |

---

## How to use this file

1. Add new rows with the next ID in each section.
2. Move status when picking up or shipping work.
3. Link to design docs instead of duplicating specs.
4. Keep `docs/STATE.md` as the high-level engineering snapshot.
