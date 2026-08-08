# Global keyboard shortcuts

Cosmic Scribe is meant to be used with **system-wide shortcuts**. Commands talk to the tray daemon over a local socket. The daemon must be running (`cosmic-scribe --install` installs the binary and enables `com.cosmic-scribe.service` on login).

| Action | Command | Suggested keys |
|--------|---------|----------------|
| **Start / stop** recording | `cosmic-scribe --trigger` | **Ctrl+Space** (or Super+Shift+Space) |
| **Cancel** recording or transcription | `cosmic-scribe --cancel` | **Ctrl+Shift+Space** |

- **Trigger** once starts (tray mic **red**); trigger again stops and sends audio for STT (**blue** briefly).
- **Cancel** aborts the current take or in-flight transcription — same as tray → **Cancel recording** / **Cancel transcription**. Does **not** paste; discards the take (no STT / removes incomplete history file). Idle: no-op.

See also: [README Quick start](../README.md#quick-start) · [INSTALL.md](INSTALL.md) · [OUTPUT.md](OUTPUT.md)

---

## 1. Install and start the daemon

If you have not already:

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install erik-balfe/cosmic-scribe/cosmic-scribe
$(brew --prefix)/bin/cosmic-scribe --install
```

Check status:

```bash
cosmic-scribe --status
```

You should see `daemon: running` and the tray mic icon in the panel.

---

## 2. Use the full command path (recommended)

COSMIC runs shortcuts in a minimal environment. Prefer the **full path** to the binary you installed:

```bash
# Homebrew
BIN="$(brew --prefix)/bin/cosmic-scribe"

# User install (after --install)
BIN=~/.local/bin/cosmic-scribe
```

Copy the path that works in your terminal:

```bash
which cosmic-scribe
# or
readlink -f ~/.local/bin/cosmic-scribe
```

---

## 3. Add shortcuts on COSMIC

Steps may vary slightly by COSMIC version; names are usually **Settings → Keyboard**.

1. Open **Settings** (Super key → “Settings”, or tray).
2. Go to **Keyboard** (or **Input devices → Keyboard**).
3. Open **Custom shortcuts** / **Custom commands** / **Add shortcut**.
4. Add **two** shortcuts (recommended):

| Name | Command | Keys |
|------|---------|------|
| Cosmic Scribe | `$BIN --trigger` | **Ctrl+Space** |
| Cosmic Scribe cancel | `$BIN --cancel` | **Ctrl+Shift+Space** |

Use the **full path** to the binary (COSMIC shortcuts often have a minimal `PATH`), e.g.

```text
/home/you/.local/bin/cosmic-scribe --trigger
/home/you/.local/bin/cosmic-scribe --cancel
```

5. Click each shortcut field and press the key combo.
6. Save / Apply.

Avoid combos already used by the desktop or other apps. If **Ctrl+Space** is taken (IME / launcher), pick another pair and keep cancel as “trigger + Shift”.

---

## 4. Test

1. Focus any app with a text field (browser, editor, terminal).
2. Press **trigger** once → tray capsule turns **red** (recording).
3. Speak a short phrase.
4. Press **trigger** again → capsule turns **blue** until text is ready, then idle (or lands on clipboard per [OUTPUT.md](OUTPUT.md)).
5. **Cancel:** start recording, then press **cancel** → capsule returns to **idle** without pasting text. You can also cancel while the capsule is **blue** (transcription).

If nothing happens:

| Check | Command |
|-------|---------|
| Daemon running? | `cosmic-scribe --status` |
| Trigger works? | `cosmic-scribe --trigger` |
| Cancel works? | start recording, then `cosmic-scribe --cancel` |
| API key set? | Tray → **Settings**, or `cosmic-scribe --configure` |
| Wrong binary on PATH? | `hash -r` after install; use full path in shortcut |

---

## 5. Upgrading from `voice-input`

Update the shortcut **command** to `cosmic-scribe --trigger` (and the new binary path). Old `voice-input` shortcuts will not talk to the new daemon.

```bash
brew uninstall voice-input   # if still installed
cosmic-scribe --install
```

---

## Why not a built-in shortcut?

Cosmic Scribe does not register global shortcuts itself. COSMIC (and most Wayland desktops) require **you** to bind a key in system settings. That way you choose the combo that fits your workflow and avoid conflicts with other tools.