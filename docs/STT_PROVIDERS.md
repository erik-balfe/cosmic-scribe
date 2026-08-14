# Speech providers (STT)

Cosmic Scribe is a **system dictation** app: push-to-talk → cloud speech-to-text → paste.

Auth is **Bearer API keys** (OpenAI-style `Authorization: Bearer …`). Plan **sign-in** (SuperGrok / X Premium+) is an optional path for xAI when you prefer that over a key.

## What works today

| Piece | Today |
|-------|--------|
| Credential | API key file / env, or SuperGrok OAuth |
| Request dialect | **xAI REST** — `POST {endpoint}` multipart: `format`, `language`, `file` |
| Default endpoint | `https://api.x.ai/v1/stt` |
| Configurable | Full **STT endpoint URL** (Settings, or `COSMIC_SCRIBE_STT_URL`) |

Changing the endpoint is enough for:

- Same-dialect proxies
- Self-hosted mirrors of the xAI `/v1/stt` shape
- Local gateways that already speak that multipart protocol

It is **not** enough for OpenAI Whisper / GPT transcriptions.

## Why not “just base URL” for OpenAI?

OpenAI-compatible speech is a **different dialect**:

| | xAI (current) | OpenAI audio transcriptions |
|--|---------------|------------------------------|
| Path | `/v1/stt` | `/v1/audio/transcriptions` |
| Required form | `file` (+ `format`, `language`) | `file` + **`model`** (e.g. `whisper-1`) |
| Response | `{ text, words[{text,start,end}], … }` | `{ text }` or `verbose_json` with different word fields |
| Plan OAuth | SuperGrok device-code (xAI only) | N/A here |

So contributors need a **request/response adapter**, not only a host string.

## Env / Settings

| Setting | How |
|---------|-----|
| API key | Settings, `--set-key`, `COSMIC_SCRIBE_API_KEY` (or `COSMIC_SCRIBE_XAI_API_KEY`) |
| STT endpoint | Settings → **STT endpoint**, or `COSMIC_SCRIBE_STT_URL` |
| Language | Settings / `--set-lang` / `COSMIC_SCRIBE_LANG` |

## Contributing: OpenAI (or other) dialect

Goal (backlog **F7**): pluggable STT providers so Cosmic Scribe stays provider-neutral in the UI.

Suggested approach:

1. Keep `SttClient` in `src/traits.rs` as the trait.
2. Add something like `SttDialect { XaiRest, OpenAiTranscriptions, … }` or separate client types.
3. For OpenAI dialect:
   - `POST {base}/v1/audio/transcriptions`
   - multipart: `model`, optional `language`, `response_format=verbose_json` (if word timings matter), `file`
   - Map JSON into `SttResult` (especially `words` — OpenAI often uses `word` not `text`)
4. Settings: dialect picker + endpoint (or base URL) + model name when needed.
5. Tests: WireMock fixtures per dialect (see existing mock in `src/stt.rs`).

Do **not** break the default xAI path or SuperGrok OAuth while adding dialects.

## `format=true` is not punctuation

xAI REST: `format=true` + `language` enables **inverse text normalization** (spoken “one hundred dollars” → `$100`). It does **not** turn on a separate punctuation / sentence-restore API. Docs: [Speech to Text](https://docs.x.ai/developers/model-capabilities/audio/speech-to-text) — `format` “converts spoken numbers/currency to written form.” No `punctuate` flag.

Punctuation and capitals come from the **ASR model itself** (they show up *inside* `words[].text`, e.g. `"Okay,"` / `"понимаю,"`). Quality varies by language. Local history (2026-08-15): **~30% of Russian takes have no `.?!`** vs **~6% of English**; RU often starts lowercase. Streaming STT has Smart Turn / endpointing for *when* a phrase ends — that is turn-taking, not written punctuation.

**Do not** add a default LLM “tidy / full rewrite” of the transcript. That is how Grok web chat post-processes voice (off / tidy up / full rewrite); it often swaps rare-but-correct words for common ones. Cosmic Scribe still has a leftover OpenRouter History “AI fix” API (`correct_recording`) — same rewrite class; **not** on the paste path; do not revive as default. If we add post-process later (F14), keep it **optional**, **punctuation-only**, and never replace content by default.

## Product boundary

Cosmic Scribe is **speech input**, not a multi-model chat shell. LLM “OpenAI-compatible” chat endpoints are out of scope unless they also expose a speech-to-text path you implement behind `SttClient`.
