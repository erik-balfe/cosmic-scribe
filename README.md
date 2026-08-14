<p align="center">
  <img src="assets/logo.png" width="96" height="96" alt="Cosmic Scribe">
</p>

# Cosmic Scribe

**Speak anywhere on your Linux desktop.** Mail, Telegram, the browser, a notes app — one shortcut.

**Sign in with SuperGrok or X Premium+.** Same account you already use on [grok.com](https://grok.com) or X. No API keys, no console.x.ai, no prepaid token balance.

```bash
cosmic-scribe --login
```

Then: press the shortcut, talk, press again. Text lands in the field you were already using.

This is **not** voice-inside-an-AI-IDE. Agent harnesses (Grok Build, Codex, Claude, Hermes) often have their own mic. Cosmic Scribe is a **system keyboard for speech** — COSMIC / Pop!_OS / Fedora (Wayland).

The app is free ([MIT](LICENSE)). Speech uses **your** SuperGrok or X plan (or an API key if you prefer). Independent project — not System76, Pop!_OS, or xAI. [Branding](docs/BRANDING.md).

## How it feels

| You see | Meaning |
|---------|---------|
| Tray mic, idle | Ready |
| Capsule **red** | Recording — speak |
| Capsule **blue** | Transcribing — typically **under a second** after you stop |
| Text in the focused app | Ready to edit or send |

Started by mistake? **Cancel** (suggested **Ctrl+Shift+Space**) — nothing pasted.

| Idle | Recording | Recognizing |
|:---:|:---:|:---:|
| ![Idle](screenshots/tray-idle.png) | ![Recording](screenshots/tray-recording.png) | ![Recognizing](screenshots/tray-transcribing.png) |

## Why sign in (not an API key)

| Ordinary path | Power-user path |
|---------------|-----------------|
| **`cosmic-scribe --login`** — SuperGrok or X Premium+ | Paste an [xAI API key](https://console.x.ai/) in Settings |
| Uses the same plan access as grok.com | Pay-per-use on the console |
| No key files to invent | Env / `--set-key` still work |

Most people who just want to **type by voice in the browser** should sign in.

## Why it’s fast

Audio is encoded **while you speak**. After stop we upload and wait on speech-to-text. On a home connection (**138** real takes):

| Take length | Stop → text (median) |
|-------------|----------------------|
| Under 5 s | **~0.3 s** |
| 5–20 s | **~0.6 s** |
| 20–60 s | **~1.0 s** |
| Over 60 s | **~1.8 s** |
| All takes | **~0.8 s** (p90 ~1.7 s; encode after stop ≈ 0–4 ms) |

Typical compression ~10× vs raw PCM. One final paste — not live captions.

## Quick start

**[Full install](docs/INSTALL.md)** · **[Shortcuts](docs/SHORTCUT.md)** · **[Output modes](docs/OUTPUT.md)**

```bash
sudo dnf install alsa-utils wl-clipboard wtype libnotify ffmpeg
git clone https://github.com/erik-balfe/cosmic-scribe.git
cd cosmic-scribe
./scripts/install-prod.sh
./scripts/install-gui-native-prod.sh
cosmic-scribe --login
```

COSMIC → **Settings → Keyboard → Custom shortcuts:**

| Command | Suggested keys |
|---------|----------------|
| `~/.local/bin/cosmic-scribe --trigger` | **Ctrl+Space** — start / stop |
| `~/.local/bin/cosmic-scribe --cancel` | **Ctrl+Shift+Space** — abort |

Focus a text field → shortcut → **red** → speak → shortcut → **blue** → text.

## Output

| Mode | Behavior |
|------|----------|
| **wtype** (default) | Clipboard + type into the focused field |
| **clipboard** | Clipboard only (better in some terminals) |

## Status

| | |
|--|--|
| **Shipped** | Sign-in (OAuth), API key, tray, trigger + cancel, progressive encode, local history, native Settings, opt-in usage numbers |
| **Planned** | Silence cutting on upload (F13); other speech APIs (F7); optional punctuation-only tidy (F14) |
| **Not this app** | Live captions; a pause button; silent “Grok rewrite” of your words |

Punctuation comes from the speech model. We do **not** rewrite your transcript through a chat model (that often changes rare words). [Backlog](docs/BACKLOG.md).

## Privacy

Audio goes to xAI for transcription. Sign-in tokens, keys, and history stay **on this computer**.

**Usage numbers** are **off by default**. If you turn them on in Settings, Cosmic Scribe stores only anonymous counts (how long takes are, how fast stop→text was, sign-in vs API key, which actions fired). No transcript text, no audio, no account id. You can see the same numbers in Settings. Nothing leaves the machine unless you opt in **and** a telemetry URL is configured (maintainers only; not a third party).

## Requirements

COSMIC (Pop!_OS / Fedora), Wayland. SuperGrok / X Premium+ **or** an API key. Runtime: `arecord`, `ffmpeg`, `wl-clipboard`, `wtype`, `libnotify`.

## Uninstall

```bash
cosmic-scribe --uninstall
./scripts/uninstall-gui-prod.sh
cosmic-scribe --uninstall --purge   # also delete local data
```

## License

[MIT](LICENSE) · [Contributing](CONTRIBUTING.md) · [Docs](docs/README.md)
