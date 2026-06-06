# Text output on Linux / Wayland

Last updated: 2026-06-05

## There is no universal “insert text” API

On Wayland, compositors do not expose “put this string in the focused text field” for arbitrary apps. Unlike Windows `SendInput` with some bulk paths, Linux dictation tools can only:

1. **Clipboard** — `wl-copy` / `xclip`; the user pastes with whatever binding the app uses.
2. **Simulated keys** — `wtype` (virtual keyboard), `ydotool` (uinput), etc. Each key is a separate event.

Protocols like `text-input-unstable-v3` are for **IME/compositors**, not for random daemons to push transcript text into any focused window.

So Cosmic Scribe does **not** try multiple paste fallbacks (Ctrl+V, Shift+Insert, ydotool, …). That looked successful in logs but failed in terminals and COSMIC apps in practice.

## Settings → Text output

| Mode | Behavior |
|------|----------|
| **wtype** (default) | Copy + `wtype -d 0 -- <text>` (no delay between keys). Still one event per character; long text takes a moment. Often **does not work in terminals** (different paste/shortcut model). |
| **clipboard** | Copy transcript to clipboard only; `notify-send` reminds you to paste. |

Legacy config values `auto`, `always`, `never` map to `wtype` / `clipboard`.

### Environment

- `COSMIC_SCRIBE_WTYPE_DELAY_MS` — delay between keys when using wtype mode (default `0`; legacy `VOICE_INPUT_WTYPE_DELAY_MS` still works).

## Recommendations

- **Terminal / IDE console** — use **clipboard**; paste with Ctrl+Shift+V (or your terminal’s binding).
- **Text editor / browser** — clipboard + Ctrl+V is usually enough.
- **wtype** — only if you tested it in your target app and accept per-character injection.

## References

- [wtype](https://github.com/atx/wtype) — Wayland virtual keyboard
- [xAI STT](https://docs.x.ai/developers/model-capabilities/audio/speech-to-text) — batch REST (not related to output)
- Wayland text-input protocol: compositor/IME only, not generic injection