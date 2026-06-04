# Distribution

## Presentability

**Positioning:** “Press shortcut or tray icon, speak, text appears in your focused field (or clipboard)” — single Rust binary, cloud STT (xAI) only for transcription. Built first for Cosmic desktop on Fedora/Wayland.

**Assets to add before public launch:**

1. Short screen recording: tray → speak → paste in editor
2. README hero / screenshots (tray icon + history detail)
3. User-focused README (done in initial public push)

**Repo hygiene:** README.md is now the user landing page. `docs/STATE.md` is maintainer truth. `CONTRIBUTING.md` for devs. This file is for packaging notes.

## Homebrew / brew tap (Linux)

Formula: `Formula/voice-input.rb` in this repo (self-tap).

```bash
brew tap erik-balfe/cosmic-scribe https://github.com/erik-balfe/cosmic-scribe
brew install voice-input
voice-input --install
voice-input --configure
```

Formula uses the public `master` tarball (`sha256 :no_check` until first release tag). Pin `url` + `sha256` when tagging `v0.1.0`.

Test locally (tap clones git — commit first):

```bash
brew tap erik-balfe/cosmic-scribe file:///path/to/cosmic-scribe
brew install voice-input
```

## macOS (future, not a priority)

Not applicable today. The project was built for Linux (Cosmic first). A macOS port would need native audio + paste stack.

## Linux — recommended paths (best experience on Cosmic desktop)

I built and daily-drive this on Cosmic (Wayland/Fedora). It works perfectly for me there. It should work on other Wayland compositors with a functional system tray.

### 1. Cargo install (current easiest / recommended)

See the main [README.md](../README.md) for the user-friendly quick start.

```bash
cargo install --path . --locked
voice-input --install
voice-input --configure
```

### 2. Homebrew tap (available)

See notes above. Ideal for users who prefer `brew` on Linux.

### 3. GitHub Releases binary

- `cargo build --release`
- Ship the binary + checksum
- Document deps (`alsa-utils`, `wl-clipboard`, `libnotify`)

### 4. Fedora COPR / RPM (high priority for broader adoption)

### 5. AUR, Nix, etc.

Same runtime deps as Fedora.

See README for the honest motivation around why xAI STT + this architecture makes sense on real Linux laptop hardware vs high-end local-only setups.

## Versioning

- `0.1.0` — first public: core STT + history UI stable, correction beta
- `0.2.0` — correction stable or removed; packaging (COPR minimum)

## Release cadence

Tag on jj commit → `jj git push` → GitHub Release with binary artifact until COPR exists.