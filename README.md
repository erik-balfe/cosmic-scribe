# Cosmic Scribe

Typing breaks your flow when you already know what you want to say. **Cosmic Scribe** is voice dictation for the [COSMIC desktop](https://github.com/pop-os/cosmic-epoch) on Pop!_OS and Fedora (Wayland): press a shortcut or use the tray, speak, and get recognized text in your app—or on the clipboard. Transcription uses [xAI Grok speech-to-text](https://docs.x.ai/developers/models/speech-to-text) (paid API key required).

## Who it's for

Linux users on **COSMIC** who want reliable **speech-to-text on Wayland** with a visible tray status (idle / recording / transcribing). Built for daily dictation, not as a generic cross-desktop suite.

## What you get

### Dictation and recognition

- **Global shortcut** — bind `cosmic-scribe --trigger` in COSMIC keyboard settings; press to start/stop recording. **Step-by-step:** [docs/SHORTCUT.md](docs/SHORTCUT.md).
- **Tray mic** — left-click to record when idle; icon shows **white mic → red dot (recording) → gray mic (transcribing)**.
- **Cloud STT** — after you stop, audio is sent to xAI; the transcript is what you get for pasting or review.
- **Language** — default **en**; set any code xAI accepts (Settings or `cosmic-scribe --configure` / `--set-lang`).

### Text output (clipboard is always updated)

Every successful transcription is **copied to the clipboard**. On top of that:

| Mode | What happens |
|------|----------------|
| **wtype** (default) | Clipboard + simulated typing into the focused field (`wtype`, no per-key delay). Best for editors and browsers. |
| **clipboard** | Clipboard only + notification; you paste yourself (good for **terminals**). |

Change mode in **Settings** (tray → Settings). Details: [docs/OUTPUT.md](docs/OUTPUT.md).

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

**History** keeps local files under `~/.local/share/cosmic-scribe/recordings/` (audio `.raw`, transcript `.txt`, optional word timings `.stt.json`). Use it to:

- **Copy** an older transcript if you started a new recording and did not paste the previous one
- **Listen** and check whether recognition was correct
- **Edit** text and keep versions
- **Transcribe** recordings that failed earlier (no internet, missing API key, etc.)

**Settings** also offers experimental OpenRouter “AI correction” (beta; often unreliable).

### Service commands

| Command | Purpose |
|---------|---------|
| `cosmic-scribe --install` | Install daemon binary, autostart, start tray |
| `cosmic-scribe --update` | Replace installed binary and restart |
| `cosmic-scribe --status` | Daemon running? paths? |
| `cosmic-scribe --stop` | Stop daemon |
| `cosmic-scribe --uninstall` | Remove user install (keep data) |
| `cosmic-scribe --uninstall --purge` | Remove user install and all local data |

## Screenshots

| Idle (white mic) | Recording (red dot) | Processing (gray mic) |
|:---:|:---:|:---:|
| ![Idle](screenshots/tray-idle.png) | ![Recording](screenshots/tray-recording.png) | ![Processing](screenshots/tray-processing.png) |

## Quick start (recommended)

End-to-end setup on Fedora/COSMIC: **install → API key → global shortcut → test**.

### 1. System dependencies

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify
```

### 2. Install via Homebrew (Linux)

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install erik-balfe/cosmic-scribe/cosmic-scribe
```

### 3. Install the tray daemon

Homebrew puts the binary in your brew prefix; `--install` adds `~/.local/bin/cosmic-scribe`, autostart, and starts the daemon:

```bash
$(brew --prefix)/bin/cosmic-scribe --install
cosmic-scribe --status    # daemon: running
```

You should see the **mic tray icon** in the panel.

### 4. Configure your xAI API key

Get a key from [console.x.ai](https://console.x.ai/), then:

```bash
cosmic-scribe --configure
```

Or tray → **Settings** → paste API key → Save.

Recording is blocked until a key is set (you’ll get a notification and Settings will open if you try without one).

### 5. Set your global shortcut (important)

This is the main way to dictate. Cosmic Scribe does **not** pick a key for you — you bind one in COSMIC:

1. **Settings → Keyboard → Custom shortcuts** (or **Custom commands**).
2. **Add** a new shortcut.
3. **Name:** `Cosmic Scribe`
4. **Command:** use the full path (most reliable):

   ```bash
   # copy yours:
   readlink -f "$(which cosmic-scribe)"
   ```

   Example command field:

   ```text
   /home/you/.local/bin/cosmic-scribe --trigger
   ```

5. Assign a key combo you like (e.g. **Super+Shift+Space**).
6. Save.

**Full guide with troubleshooting:** [docs/SHORTCUT.md](docs/SHORTCUT.md)

### 6. Test dictation

1. Open any app with a text field.
2. Press your shortcut → tray shows **red dot** → speak → press shortcut again.
3. Text should appear in the field (or on the clipboard if you use clipboard mode).

### From source (developers)

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
cargo build --release
./target/release/cosmic-scribe --install
./target/release/cosmic-scribe --configure
```

Then complete **steps 5–6** above. See [CONTRIBUTING.md](CONTRIBUTING.md).

### Upgrading from `voice-input`

```bash
cosmic-scribe --install    # migrates ~/.local/share/voice-input/ → cosmic-scribe/
```

Update your keyboard shortcut command to `cosmic-scribe --trigger`. Optional: `brew uninstall voice-input`.

## Uninstall

```bash
cosmic-scribe --uninstall          # keep API key, recordings, settings
cosmic-scribe --uninstall --purge  # delete ~/.local/share/cosmic-scribe/ too
brew uninstall cosmic-scribe       # if installed via Homebrew
```

See `scripts/uninstall.sh` if the shell still points at a removed `~/.local/bin/cosmic-scribe`.

## Requirements

| Item | Notes |
|------|--------|
| Desktop | COSMIC on Pop!_OS or Fedora, Wayland |
| API | [xAI API key](https://console.x.ai/) |
| Tools | `arecord`, `wl-clipboard`, `wtype` (default output mode), `libnotify` |
| Command | `cosmic-scribe` |

## Privacy

Microphone audio is sent to **xAI** for transcription. API keys and recording history stay **on your machine** (encrypted key storage). See [xAI's terms](https://x.ai/).

## License

[MIT](LICENSE)

---

**Developers:** [CONTRIBUTING.md](CONTRIBUTING.md) · **All docs:** [docs/README.md](docs/README.md)