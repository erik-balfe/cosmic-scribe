# Cosmic Scribe

Typing breaks your flow when you already know what you want to say. **Cosmic Scribe** is voice dictation for the [COSMIC desktop](https://github.com/pop-os/cosmic-epoch) on Pop!_OS and Fedora (Wayland): press a shortcut or use the tray, speak, and get recognized text in your app—or on the clipboard. Transcription uses [xAI Grok speech-to-text](https://docs.x.ai/developers/models/speech-to-text) (paid API key required).

## Who it's for

Linux users on **COSMIC** who want reliable **speech-to-text on Wayland** with a visible tray status (idle / recording / transcribing). Built for daily dictation, not as a generic cross-desktop suite.

## What you get

### Dictation and recognition

- **Global shortcut** — bind `voice-input --trigger` in COSMIC keyboard settings; press to start/stop recording.
- **Tray mic** — left-click to record when idle; icon shows **white mic → red dot (recording) → gray mic (transcribing)**.
- **Cloud STT** — after you stop, audio is sent to xAI; the transcript is what you get for pasting or review.
- **Language** — default **en**; set any code xAI accepts (Settings or `voice-input --configure` / `--set-lang`).

### Text output (clipboard is always updated)

Every successful transcription is **copied to the clipboard**. On top of that:

| Mode | What happens |
|------|----------------|
| **wtype** (default) | Clipboard + simulated typing into the focused field (`wtype`, no per-key delay). Best for editors and browsers. |
| **clipboard** | Clipboard only + notification; you paste yourself (good for **terminals**). |

Change mode in **Settings** (tray → Settings).

### Cancel mistakes

- **While recording** — tray right-click → **Cancel recording**, or stop via shortcut.
- **While transcribing** — tray right-click → **Cancel transcription** (in-flight STT is dropped).
- **While transcribing** — new shortcut/tray toggles are **ignored** so you do not accidentally start another take on top of a running job.

### History and Settings (tray right-click)

| Menu item | Opens |
|-----------|--------|
| **History** | Web UI — list of past recordings, open any item |
| **Settings** | Web UI — API key, language, output mode |
| **Quit** | Stops the daemon |

**History** keeps local files under `~/.local/share/voice-input/recordings/` (audio `.raw`, transcript `.txt`, optional word timings `.stt.json`). Use it to:

- **Copy** an older transcript if you started a new recording and did not paste the previous one
- **Listen** and check whether recognition was correct
- **Edit** text and keep versions

**Settings** also offers experimental OpenRouter “AI correction” (beta; often unreliable).

### Service commands

| Command | Purpose |
|---------|---------|
| `voice-input --install` | Install daemon binary, autostart, start tray |
| `voice-input --update` | Replace installed binary and restart |
| `voice-input --status` | Daemon running? paths? |
| `voice-input --stop` | Stop daemon |
| `voice-input --uninstall` | Remove user install (keep data) |
| `voice-input --uninstall --purge` | Remove user install and all local data |

## Screenshots

| Idle (white mic) | Recording (red dot) | Processing (gray mic) |
|:---:|:---:|:---:|
| ![Idle](screenshots/tray-idle.png) | ![Recording](screenshots/tray-recording.png) | ![Processing](screenshots/tray-processing.png) |

## Quick start

**Dependencies (Fedora):**

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify
```

**From source:**

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe   # clone directory name may vary
cargo build --release
./target/release/voice-input --install
./target/release/voice-input --configure
```

**Homebrew (Linux):**

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install erik-balfe/cosmic-scribe/voice-input
$(brew --prefix)/bin/voice-input --install
$(brew --prefix)/bin/voice-input --configure
```

If `voice-input` fails after removing an old `~/.local/bin` install, run `hash -r` or use `$(brew --prefix)/bin/voice-input`.

**Shortcut** — COSMIC **Settings → Keyboard → Custom shortcuts**:

```text
voice-input --trigger
```

## Uninstall

```bash
voice-input --uninstall          # keep API key, recordings, settings
voice-input --uninstall --purge  # delete ~/.local/share/voice-input/ too
brew uninstall voice-input       # if installed via Homebrew
```

See `scripts/uninstall.sh` if the shell still points at a removed `~/.local/bin/voice-input`.

## Requirements

| Item | Notes |
|------|--------|
| Desktop | COSMIC on Pop!_OS or Fedora, Wayland |
| API | [xAI API key](https://console.x.ai/) |
| Tools | `arecord`, `wl-clipboard`, `wtype` (default output mode), `libnotify` |
| Command | `voice-input` |

## Privacy

Microphone audio is sent to **xAI** for transcription. API keys and recording history stay **on your machine** (encrypted key storage). See [xAI's terms](https://x.ai/).

## License

[MIT](LICENSE)

---

Developers: [CONTRIBUTING.md](CONTRIBUTING.md).