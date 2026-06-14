# Branding & naming

Last updated: 2026-06-07

## Name disclaimer

**Cosmic Scribe** is an independent open-source project by its maintainer. It is **not** affiliated with, endorsed by, or owned by **System76**, **Pop!_OS**, or the **COSMIC** desktop project.

“Cosmic” in the name means the app was built for and daily-driven on the COSMIC desktop (Wayland). It is a descriptive target platform, not a trademark claim.

## Official icon

| Asset | Path | Use |
|-------|------|-----|
| Source SVG | `gui/icons/icon.svg` | Edit here; run `./scripts/generate-gui-icons.sh` |
| App / dock | `gui/icons/icon.png`, `128x128.png`, `256x256.png` | Tauri, `.desktop` files |
| Tray masks | `gui/icons/icon-tray-*.svg` | Panel mic (separate from app icon) |
| Repo / README | `assets/logo.png`, `assets/logo.svg` | Copied by icon script — do not edit by hand |

**GitHub:** upload `assets/logo-256.png` (or `gui/icons/256x256.png`) as the repository **Social preview** image (Settings → General).

## Speech-to-text providers

**Today:** transcription uses [xAI Grok STT](https://docs.x.ai/developers/models/speech-to-text) only (`SttClient` trait in `src/traits.rs`).

**Planned:** user-selectable backends (e.g. other cloud APIs or local engines) without rebinding the app to one vendor. Tracked as **F7** in [BACKLOG.md](BACKLOG.md).

Correction (“Fix with AI”) already uses OpenRouter separately from STT.