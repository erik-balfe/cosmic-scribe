# Installation

End-to-end setup for **Cosmic Scribe** on Fedora / Pop!_OS with the COSMIC desktop (Wayland).

## What you get

| Component | After install |
|-----------|----------------|
| **Tray daemon** | `cosmic-scribe` — mic in the panel; records + transcribes; login autostart via `com.cosmic-scribe.service` |
| **App window** | History + Settings — **native** libcosmic UI recommended (`cosmic-scribe-gui-native`), or Tauri (`cosmic-scribe-gui`) |

Daily use only needs the **daemon + shortcut**. The app window is for history, re-transcribe, and settings.

## 1. Runtime dependencies (required)

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify ffmpeg
```

| Package | Provides | Used for |
|---------|----------|----------|
| `alsa-utils` | `arecord` | Microphone capture |
| `ffmpeg` | `ffmpeg` | Progressive Opus encode during recording |
| `wl-clipboard` | `wl-copy` | Clipboard |
| `wtype` | `wtype` | Type into focused field (default mode) |
| `libnotify` | `notify-send` | Notifications |

`wtype` is optional if you use **clipboard-only** output mode.

## 2. Build dependencies (from source)

**Daemon** needs Rust. **Tauri GUI** also needs Node + WebKitGTK. **Native GUI** needs extra system libs (libcosmic / Wayland stack).

Fedora (daemon + Tauri GUI baseline):

```bash
sudo dnf install rust cargo rustfmt rust-clippy nodejs npm \
  glib2-devel webkit2gtk4.1-devel gtk3-devel openssl-devel \
  libappindicator-gtk3-devel librsvg2-devel
```

Native GUI may need additional packages (e.g. `libxkbcommon-devel`, Wayland/COSMIC-related devel libs). If `cargo build -p cosmic-scribe-gui-native` fails on a missing `.pc` file, install the matching `-devel` package and ensure `PKG_CONFIG_PATH` includes `/usr/lib64/pkgconfig`.

## 3. Install paths

### A. From git clone (recommended)

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
./scripts/install-prod.sh              # daemon + Tauri GUI
./scripts/install-gui-native-prod.sh   # native History/Settings (recommended on COSMIC)
```

`install-prod.sh` builds the release daemon, runs `cosmic-scribe --install`, and installs the Tauri GUI.  
`install-gui-native-prod.sh` installs the libcosmic UI; when present, the daemon **prefers** it over Tauri.

Verify:

```bash
cosmic-scribe --status
systemctl --user is-enabled com.cosmic-scribe.service
test -x ~/.local/bin/cosmic-scribe-gui-native && echo "native GUI OK"
```

### B. Homebrew (Linux) — daemon binary

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install erik-balfe/cosmic-scribe/cosmic-scribe
$(brew --prefix)/bin/cosmic-scribe --install
```

Then install a GUI from a clone (`install-gui-prod.sh` and/or `install-gui-native-prod.sh`). Homebrew does not ship the GUI yet.

### C. Cargo install — daemon only

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
cargo install --path . --locked
cosmic-scribe --install
./scripts/install-gui-native-prod.sh   # or install-gui-prod.sh
```

## 4. First-time configuration

### Auth (pick one)

**API key** (usual path):

1. Open **Cosmic Scribe → Settings**, or  
2. `cosmic-scribe --set-key '…'` / env `COSMIC_SCRIBE_API_KEY`

Default STT endpoint is `https://api.x.ai/v1/stt` (change under Settings → **STT endpoint**).  
Provider notes: [STT_PROVIDERS.md](STT_PROVIDERS.md).

**Optional — plan sign-in** (SuperGrok / X Premium+):

```bash
cosmic-scribe --login
```

Browser device-code flow. Cosmic Scribe stores **its own** session on this machine.

Recording is blocked until an API key **or** sign-in is set up.

### Global shortcut

See [SHORTCUT.md](SHORTCUT.md). Examples:

```text
~/.local/bin/cosmic-scribe --trigger    # start / stop (e.g. Ctrl+Space)
~/.local/bin/cosmic-scribe --cancel     # abort take (e.g. Ctrl+Shift+Space)
```

### Output mode

Default **wtype** (clipboard + type into focus). Use **clipboard** only for terminals that mishandle synthetic typing — [OUTPUT.md](OUTPUT.md).

## 5. Upgrade

```bash
cd cosmic-scribe
git pull
./scripts/install-prod.sh
./scripts/install-gui-native-prod.sh   # if you use native UI
```

Data under `~/.local/share/cosmic-scribe/` is preserved.

## 6. Uninstall

```bash
cosmic-scribe --uninstall
./scripts/uninstall-gui-prod.sh
# native wrapper if installed:
rm -f ~/.local/bin/cosmic-scribe-gui-native \
      ~/.local/share/cosmic-scribe/cosmic-scribe-gui-native
cosmic-scribe --uninstall --purge   # optional: delete all local data
```

## Troubleshooting

| Symptom | Check |
|---------|--------|
| No tray | `cosmic-scribe --status`; session has StatusNotifier; restart daemon |
| Record blocked | `--login` or set API key |
| Slow after stop | Ensure `ffmpeg` installed (progressive Opus); see `~/.local/share/cosmic-scribe/daemon.log` |
| Wrong GUI | Install native script, or `COSMIC_SCRIBE_GUI=tauri` / `native` |
