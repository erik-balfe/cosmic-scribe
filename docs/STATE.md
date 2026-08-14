# STATE — Cosmic Scribe

Last updated: 2026-08-15 (v0.5.0)

**Docs index:** [`docs/README.md`](README.md) · **Tasks:** [`docs/BACKLOG.md`](BACKLOG.md)

Internal engineering log (not the public README). Product / VAD notes for maintainers: **local only** — `docs/LOCAL/` (via `.git/info/exclude`).

## Snapshot

- **Product:** system speech input on COSMIC (near-zero UI: shortcut + tray).
- **Auth:** SuperGrok / X Premium+ sign-in is the ordinary path; API key is a fallback.
- **STT:** progressive Opus + batch REST (xAI dialect default; endpoint configurable). Streaming STT not a product goal.
- **VAD / silence cut:** **not shipped** — no RMS hard-reject (false-positive safe). Plan: F13, design in `docs/LOCAL/VAD-SILENCE.md`.
- **Public README:** everyday / OAuth-first landing (sign-in above the fold).
- **Analytics:** opt-in, off by default, aggregates only (F15).

## Shipped (master tip)

| Area | Notes |
|------|--------|
| PTT + tray | Red / blue / idle |
| Progressive Opus | Encode during capture; REST upload |
| OAuth + API key | Warm token; local-only credential gates |
| Native GUI | libcosmic History/Settings when installed |
| STT endpoint setting | Same dialect only; OpenAI Whisper = F7 |
| Path-safe recording IDs | Local API sanitization |
| Nested-runtime re-transcribe fix | Dedicated thread for STT from GUI |

## Next (post-ship or next minor)

1. **F13** silence cutting (upload path only; conservative VAD) — design locked in LOCAL docs  
2. **F7** OpenAI Whisper / multi-dialect STT  
3. **F14** optional punctuation-only pass (never default rewrite) — root cause measured 2026-08-15: in-model ASR, RU weaker than EN  
4. Optional: native GUI window screenshots for README  

## Release posture

See [RELEASE.md](RELEASE.md). Tip is **v0.5.0** (OAuth-first README + opt-in analytics). Do not retag older versions.
