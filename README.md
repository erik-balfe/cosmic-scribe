# Cosmic Scribe

Typing breaks your flow when you already know what you want to say. **Cosmic Scribe** is voice dictation for the [COSMIC desktop](https://github.com/pop-os/cosmic-epoch) on Pop!_OS and Fedora (Wayland): press a shortcut or use the tray, speak, and get text in the app you're using—or on the clipboard. Transcription runs in the cloud via [xAI Grok speech-to-text](https://docs.x.ai/developers/models/speech-to-text) (paid API key required).

## Who it's for

Linux users on **COSMIC** who want **speech-to-text / dictation on Wayland** without juggling a generic multi-desktop setup. The tray icon shows idle, recording, and transcribing so you always know what the app is doing.

## Quick start

**Dependencies (Fedora example):**

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify
```

**Build and install:**

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd voice-input
cargo build --release
./target/release/voice-input --install
./target/release/voice-input --configure
```

`--configure` sets your [xAI API key](https://console.x.ai/) and language.

**Global shortcut** — COSMIC **Settings → Keyboard → Custom shortcuts**:

```text
voice-input --trigger
```

## How to use it daily

1. **Shortcut** — `voice-input --trigger` starts or stops recording (same as tray when idle).
2. **Tray** — Left-click the mic to record (especially handy in clipboard mode). Right-click for **History**, **Settings**, **Quit**.
3. **While busy** — During recording (red dot) or transcribing (gray mic), you can't start another recording from the tray.
4. **Output** — Default pastes into the focused field; clipboard mode copies only and notifies you (good for terminals).

| Idle (white mic) | Recording (red dot) | Processing (gray mic) |
|:---:|:---:|:---:|
| ![Idle](screenshots/tray-idle.png) | ![Recording](screenshots/tray-recording.png) | ![Processing](screenshots/tray-processing.png) |

## Features

- Global shortcut or system tray to record from your microphone
- Cloud STT (xAI Grok) with text injected into the focused app or clipboard-only
- Tray states: idle → recording → transcribing → idle
- **Settings** output mode: `wtype` (default: copy + type keys) or `clipboard` (copy + notification; paste yourself)
- **History** — tray → History opens a local web UI; files under `~/.local/share/voice-input/recordings/` (`.raw`, `.txt`, `.stt.json`)
- API key stored locally with encryption (AES-256-GCM)
- Daemon install/update via `voice-input --install` and `voice-input --update`

**Experimental:** OpenRouter AI word correction in Settings — beta; results are often unreliable.

## Requirements

| Item | Notes |
|------|--------|
| Desktop | COSMIC on Pop!_OS or Fedora, Wayland |
| API | [xAI API key](https://console.x.ai/) (paid usage) |
| Tools | `arecord`, `wl-clipboard`, `wtype` (for default typing mode), `libnotify` |
| Command | `voice-input` (installed binary name) |

## Privacy

Your microphone audio is sent to **xAI** for transcription. API keys and recording history stay **on your machine**; keys use encrypted storage. Review [xAI's terms](https://x.ai/) before use.

## License

[MIT](LICENSE)

---

Developers: see [CONTRIBUTING.md](CONTRIBUTING.md).