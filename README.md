<p align="center">
  <img src="assets/logo.png" width="96" height="96" alt="Cosmic Scribe">
</p>

# Cosmic Scribe

**Voice dictation for the COSMIC desktop** — a keyboard for speech.

One global shortcut. A tray mic. No window required.

Press the shortcut, speak, press again. The mic turns **red**, then **blue** (typically well under a second on a home connection), and the text is on your clipboard (and typed into the focused field if you want). Paste anywhere — notes, code, mail, chat, the browser.

Built for [COSMIC](https://github.com/pop-os/cosmic-epoch) on Pop!_OS and Fedora (Wayland). Transcription uses **cloud speech-to-text** with a Bearer **API key** (OpenAI-style auth). Default STT dialect is [xAI](https://docs.x.ai/developers/model-capabilities/audio/speech-to-text); you can set the **endpoint URL** in Settings. Optional **plan sign-in** for SuperGrok / X Premium+. Full OpenAI Whisper compatibility is a different API shape — see [docs/STT_PROVIDERS.md](docs/STT_PROVIDERS.md) (contributions welcome).

> **Independent project** — not affiliated with System76, Pop!_OS, or the COSMIC desktop. See [docs/BRANDING.md](docs/BRANDING.md).

## Why this instead of another STT app?

| Approach | Typical pain |
|----------|----------------|
| Type everything | Slow, especially long thoughts |
| Browser Grok / ChatGPT voice | Locked inside one tab — not your editor or terminal |
| Local-only tools (e.g. many Whisper UIs, Handy-style apps) | Quality/latency tradeoffs; often more UI |
| App-specific voice modes | Only where the vendor bothered to add them |

Cosmic Scribe is **system-level**: same shortcut everywhere, near-zero chrome, cloud speech quality — with an API key, or optional plan sign-in when your provider supports it (SuperGrok / X Premium+ today).

## Near-zero UI

| You see | Meaning |
|---------|---------|
| Tray mic, idle | Ready |
| Capsule **red** | Recording — speak |
| Capsule **blue** | Transcribing — typically **~0.3–1 s** after you stop (home internet; see below) |
| Notification / clipboard | Text ready — **Ctrl+V** (or already typed in **wtype** mode) |

Optional **History** and **Settings** windows (libcosmic / native COSMIC look when installed) for re-transcribe, edit, auth, and preferences — not required for the main loop.

| Idle | Recording | Recognizing |
|:---:|:---:|:---:|
| ![Idle](screenshots/tray-idle.png) | ![Recording](screenshots/tray-recording.png) | ![Recognizing](screenshots/tray-transcribing.png) |

## Features

- **Global shortcuts** — bind `cosmic-scribe --trigger` (start/stop) and `--cancel` ([guide](docs/SHORTCUT.md))
- **Tray mic** — red / blue status; left-click to record when idle
- **API key for speech** — Settings, `--set-key`, or `COSMIC_SCRIBE_API_KEY` (Bearer key)
- **Configurable STT endpoint** — Settings or `COSMIC_SCRIBE_STT_URL` (same dialect; default xAI)
- **Optional plan sign-in** — `cosmic-scribe --login` for SuperGrok / X Premium+
- **Fast after stop** — audio is encoded **while you speak** (progressive Opus); then one batch STT request
- **Clipboard + optional auto-type** — always copy; default mode also types into the focused field
- **Local history** — re-listen, copy, edit, re-transcribe without recording again
- **Cancel** — `cosmic-scribe --cancel` (e.g. Ctrl+Shift+Space) or tray; no paste, take discarded

## Why it feels instant

Encoding runs **during** the take, so after you stop we mostly **upload + wait on the speech API**. Measured on this project’s real daemon log (home connection, progressive Opus, 137 successful takes):

| Take length | Stop → text (median) |
|-------------|----------------------|
| Under 5 s | **~0.3 s** |
| 5–20 s | **~0.6 s** |
| 20–60 s | **~1.0 s** |
| Over 60 s | **~1.8 s** |
| All takes | **~0.8 s** median (encode after stop ≈ 0–4 ms) |

Typical compression ~10× vs raw PCM. No streaming STT: one final paste, not live captions.

## Status

| | |
|--|--|
| **Shipped** | Global trigger + cancel, tray mic, progressive Opus, batch xAI STT, OAuth or API key, native History/Settings, local history, re-transcribe |
| **In progress / next** | Silence cutting on upload only (F13); OpenAI-style STT dialect (F7) |
| **Not the product** | Streaming live captions; pause button; LLM “rewrite my dictation” (that invents words) |

Punctuation and sentence structure come from the **speech model**, not a second Grok rewrite. We send `format=true` (inverse text normalization: spoken numbers → `$100`). We do **not** run a hidden tidy/full-rewrite pass — that is what Grok web uses for chat, and it often replaces rare but correct words. Language hint is in Settings (`en`, `ru`, …). Tracker: [docs/BACKLOG.md](docs/BACKLOG.md).

## Quick start

Full install notes: **[docs/INSTALL.md](docs/INSTALL.md)**

### 1. Dependencies

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify ffmpeg
```

### 2. Install

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
./scripts/install-prod.sh
./scripts/install-gui-native-prod.sh   # native History/Settings (recommended on COSMIC)
```

### 3. Auth (pick one)

**API key** (works for everyone): open **Cosmic Scribe → Settings**, paste a speech API key, Save — or `cosmic-scribe --set-key …` / `COSMIC_SCRIBE_API_KEY`.  
Default STT endpoint: `https://api.x.ai/v1/stt` (changeable in Settings). Details: [docs/STT_PROVIDERS.md](docs/STT_PROVIDERS.md).

**Optional — plan sign-in** (SuperGrok / X Premium+):

```bash
cosmic-scribe --login
```

Recording is blocked until an API key or sign-in is set up.

### 4. Shortcut

**Settings → Keyboard → Custom shortcuts:**

| Command | Suggested keys |
|---------|----------------|
| `~/.local/bin/cosmic-scribe --trigger` | **Ctrl+Space** — start / stop |
| `~/.local/bin/cosmic-scribe --cancel` | **Ctrl+Shift+Space** — abort take |

Details: [docs/SHORTCUT.md](docs/SHORTCUT.md)

### 5. Dictate

1. Focus a text field (or plan to paste).  
2. Shortcut → **red** → speak → shortcut.  
3. **Blue** briefly → paste if needed.

### Tips

- **Started by mistake?** **Cancel** shortcut (`--cancel`, e.g. Ctrl+Shift+Space) or tray → Cancel — no paste, no STT.  
- **Long break mid-thought?** Stop recording (trigger again). The take is transcribed and saved; start a new one when you return. No separate pause mode.  
- **Thinking pauses still go to the API** for now (silence cutting is planned later). We do **not** discard takes for “quiet audio” — soft speech and pauses must not be treated as errors.  
- **Network glitch?** History → open the take → **Transcribe** again (audio stays on disk).  
- **Terminals:** Settings → output mode **clipboard** only, then paste yourself.

## Output modes

| Mode | Behavior |
|------|----------|
| **wtype** (default) | Clipboard + type into focused field |
| **clipboard** | Clipboard only |

More: [docs/OUTPUT.md](docs/OUTPUT.md)

## Privacy

Audio is sent to your configured **STT endpoint** for transcription (default xAI dialect). API keys, sign-in tokens, and history stay **on your machine** (encrypted credentials).

## Requirements

| Item | Notes |
|------|--------|
| Desktop | COSMIC (Pop!_OS / Fedora), Wayland |
| Auth | Speech **API key** (Bearer), or SuperGrok / X Premium+ sign-in |
| Runtime | `arecord`, `ffmpeg`, `wl-clipboard`, `wtype`, `libnotify` |
| Binaries | `cosmic-scribe` (daemon + tray); optional native or Tauri GUI |

## Speech providers

Default path uses the **xAI STT dialect**. The endpoint URL is user-configurable. **OpenAI Whisper** (`/v1/audio/transcriptions`) is not a drop-in base-URL change — contributors can add that dialect via `SttClient` (see [docs/STT_PROVIDERS.md](docs/STT_PROVIDERS.md)).

## Uninstall

```bash
cosmic-scribe --uninstall
./scripts/uninstall-gui-prod.sh
cosmic-scribe --uninstall --purge   # optional: delete all local data
```

## License

[MIT](LICENSE)

---

**Developers:** [CONTRIBUTING.md](CONTRIBUTING.md) · **Docs index:** [docs/README.md](docs/README.md)
