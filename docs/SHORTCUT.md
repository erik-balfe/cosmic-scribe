# Global keyboard shortcut

Cosmic Scribe is meant to be used with a **system-wide shortcut**: press once to start recording, press again to stop. The command the desktop runs is:

```text
cosmic-scribe --trigger
```

That talks to the tray daemon over a local socket. The daemon must be running (`cosmic-scribe --install` installs the binary and enables `com.cosmic-scribe.service` on login).

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
$(brew --prefix)/bin/cosmic-scribe --trigger

# User install (after --install)
~/.local/bin/cosmic-scribe --trigger
```

Copy the path that works in your terminal:

```bash
which cosmic-scribe
# or
readlink -f ~/.local/bin/cosmic-scribe
```

---

## 3. Add the shortcut on COSMIC

Steps may vary slightly by COSMIC version; names are usually **Settings → Keyboard**.

1. Open **Settings** (Super key → “Settings”, or tray).
2. Go to **Keyboard** (or **Input devices → Keyboard**).
3. Open **Custom shortcuts** / **Custom commands** / **Add shortcut**.
4. Click **Add** (+).
5. Fill in:
   - **Name:** `Cosmic Scribe` (any label you like)
   - **Command:** full path from step 2, e.g.
     ```text
     /home/you/.linuxbrew/bin/cosmic-scribe --trigger
     ```
     or
     ```text
     /home/you/.local/bin/cosmic-scribe --trigger
     ```
6. Click the shortcut field and press your preferred key combo (e.g. **Super+Shift+Space** or **Ctrl+Alt+R**).
7. Save / Apply.

Avoid combos already used by the desktop or other apps.

---

## 4. Test

1. Focus any app with a text field (browser, editor, terminal).
2. Press your shortcut once → tray capsule turns **red** (recording).
3. Speak a short phrase.
4. Press the shortcut again → capsule turns **blue** until text is ready, then idle (or lands on clipboard per [OUTPUT.md](OUTPUT.md)).

If nothing happens:

| Check | Command |
|-------|---------|
| Daemon running? | `cosmic-scribe --status` |
| Command works in terminal? | `cosmic-scribe --trigger` (should toggle recording) |
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