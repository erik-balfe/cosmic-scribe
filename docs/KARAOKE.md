# Karaoke playback & word navigation (planned)

Last updated: 2026-06-05

Cosmic Scribe uses **batch REST** xAI STT (`POST /v1/stt`), not streaming. The API returns the full transcript plus optional **`words[]`** with `start` / `end` times in seconds. We now persist that payload per recording — see **Storage** below.

This document is the source of truth for the feature until it ships in the web UI.

## Why batch REST (not streaming)

| | REST batch | WebSocket streaming |
|---|------------|---------------------|
| Price | $0.10 / hr | $0.20 / hr |
| Use case | Record → stop → paste | Live captions, assistants |
| Timestamps | `words[]` in final JSON | `words` on partial/done events |
| Our daemon | **Uses this** | Not used (no fake “typing” effect) |

Text output: see **`docs/OUTPUT.md`** (clipboard default; optional wtype typing).

## Storage (implemented)

For each recording stem `YYYY-MM-DD_HH-MM-SS_<ms>ms`:

| File | Purpose |
|------|---------|
| `.raw` | PCM audio |
| `.txt` | Plain transcript (history list, inject, copy) |
| `.stt.json` | Schema v1: `text`, `language`, `duration_secs`, `words[]`, `api_response` |
| `.json` | User edits / AI correction versions (existing) |

`SttResult` is defined in `src/traits.rs`; written in `save_stt_artifacts()` in `src/app.rs`.

Web API: `GET /api/recording/:id` includes `has_stt` and `stt` when present.

## UI plan (not implemented)

### Modes in `Detail.svelte`

1. **Navigate** (default) — click word → `audio.currentTime = word.start`; during playback highlight active word by time.
2. **Mark wrong** — red tool; click toggles wrong marks (existing correction flow).
3. **Mark correct** — green tool; click toggles keep marks.
4. **Esc / “Done”** — clears tool back to Navigate.

Rules: marking tools do not seek; Navigate does not change correction marks.

### Playback highlight

- Load `stt.words` from API.
- On `timeupdate`, binary-search / scan for word where `start <= t < end`.
- CSS class `word-active` on current span; optional auto-scroll into view.

### Click-to-seek

- Render transcript from `words[].text` (or split `text` if `words` empty).
- `onclick` on word span → set `audio.currentTime = word.start` (Navigate mode only).

### Coexistence with waveform

- Waveform seek stays as today.
- Text bar is a second “progress” view tied to the same `audio` element.

## Implementation order

1. [x] Persist `.stt.json` from REST response
2. [x] Expose `stt` in recording detail API
3. [ ] Render word spans from `stt.words` in Detail (read-only highlight during play)
4. [ ] Tool mode state machine (Navigate / Wrong / Correct)
5. [ ] Click-to-seek in Navigate mode
6. [ ] Optional: list badge “timed transcript” when `has_stt`

## References

- [xAI Speech to Text](https://docs.x.ai/developers/model-capabilities/audio/speech-to-text)
- Code: `src/stt.rs`, `src/injector.rs`, `web/src/Detail.svelte`