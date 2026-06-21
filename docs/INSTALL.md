# Installation

End-to-end setup for **Cosmic Scribe** on Fedora / Pop!_OS with the COSMIC desktop (Wayland).

## What you get

| Component | After install |
|-----------|----------------|
| **Tray daemon** | `cosmic-scribe` — mic in the panel, records + transcribes; autostart via `com.cosmic-scribe.service` |
| **App window** | `cosmic-scribe-gui` — **Cosmic Scribe** in the app menu (History + Settings) |

Both are required for the intended workflow. Homebrew installs the **daemon binary only**; the GUI is installed from a git clone (see below).

## 1. Runtime dependencies (required)

Fedora / Pop!_OS:

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify
```

| Package | Provides | Used for |
|---------|----------|----------|
| `alsa-utils` | `arecord` | Microphone capture |
| `wl-clipboard` | `wl-copy` | Clipboard output |
| `wtype` | `wtype` | Typing into focused field (default output mode) |
| `libnotify` | `notify-send` | Tray notifications |

`wtype` is only needed if output mode is **wtype** (default). Clipboard-only mode still needs `wl-clipboard`.

The daemon warns at startup if any of these are missing (`cosmic-scribe --daemon` logs).

## 2. Build dependencies (from source only)

Needed to compile **`cosmic-scribe-gui`** (Tauri / WebKitGTK). Not required if you only use a pre-built binary.

Fedora:

```bash
sudo dnf install rust cargo rustfmt rust-clippy nodejs npm \
  glib2-devel webkit2gtk4.1-devel gtk3-devel openssl-devel \
  libappindicator-gtk3-devel librsvg2-devel
```

ImageMagick (`ImageMagick`) is optional — used by `scripts/generate-gui-icons.sh` when icon PNGs are missing.

## 3. Install paths

### A. From git clone (recommended — daemon + GUI)

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
./scripts/install-prod.sh
```

This builds the release daemon, runs `cosmic-scribe --install`, builds **cosmic-scribe-gui**, and registers the app menu entry. **Existing data is preserved** on reinstall.

Verify:

```bash
cosmic-scribe --status          # daemon: running, systemd unit present
systemctl --user is-enabled com.cosmic-scribe.service
test -x ~/.local/bin/cosmic-scribe-gui && echo "GUI OK"
```

### B. Homebrew (Linux) — daemon binary

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install erik-balfe/cosmic-scribe/cosmic-scribe
$(brew --prefix)/bin/cosmic-scribe --install
```

Then install the **GUI** from a clone (Homebrew does not ship `cosmic-scribe-gui` yet):

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
./scripts/install-gui-prod.sh
```

### C. Cargo install — daemon only

```bash
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
cargo install --path . --locked
cosmic-scribe --install
./scripts/install-gui-prod.sh
```

## 4. First-time configuration

1. Open **Cosmic Scribe** from the app menu → **Settings**.
2. Paste your [xAI API key](https://console.x.ai/) → **Save**.
3. Bind a global shortcut: [SHORTCUT.md](SHORTCUT.md) — command is `~/.local/bin/cosmic-scribe --trigger` (or `$(brew --prefix)/bin/cosmic-scribe --trigger`).

CLI alternative: `cosmic-scribe --configure` (terminal only).

## 5. Tray icon states

| State | Capsule color | Meaning |
|-------|---------------|---------|
| Idle | Theme (white/dark) | Ready — click tray or use shortcut |
| **Recording** | **Red** | Microphone is on — speak now |
| **Recognizing** | **Blue** | Transcribing and pasting — until done |

Also shown in **Settings** in the app window.

## 6. Upgrade

From clone:

```bash
cd cosmic-scribe
git pull
./scripts/install-prod.sh
```

Homebrew daemon:

```bash
brew upgrade cosmic-scribe
"$(brew --prefix)/bin/cosmic-scribe" --update
# Re-run install-gui-prod.sh from clone if the GUI changed
```

## 7. Uninstall

```bash
cosmic-scribe --uninstall              # daemon + autostart (keeps data)
./scripts/uninstall-gui-prod.sh        # app window only
cosmic-scribe --uninstall --purge      # also delete ~/.local/share/cosmic-scribe/
brew uninstall cosmic-scribe           # if installed via Homebrew
```

Or: `./scripts/uninstall.sh` (finds brew or `~/.local` binary).

## Troubleshooting

| Problem | Check |
|---------|--------|
| No tray icon after login | Daemon running? `cosmic-scribe --status` · Re-apply unit: `cosmic-scribe --autostart` · Tray registered? `gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems` (non-empty after login) |
| Daemon not running after reboot | Re-run `cosmic-scribe --install` or `--autostart` · `systemctl --user is-enabled com.cosmic-scribe.service` |
| No app in menu | `./scripts/install-gui-prod.sh` · log out/in or restart panel |
| GUI build fails | Tauri deps in §2 · `pkg-config --exists glib-2.0` |
| Recording silent | `arecord -l` · microphone permissions |
| No text in field | `wtype` installed · try clipboard mode in Settings |