# Screenshots

Assets for the [README](../README.md) and GitHub release page.

## Automated capture (recommended)

```bash
./scripts/capture-screenshots.sh
```

This script:

1. Seeds demo History entries (`./scripts/seed-demo-recordings.sh`)
2. Regenerates tray icon masks and exports idle / recording / transcribing PNGs
3. Starts the embedded UI server (`cosmic-scribe --ui-server`)
4. Captures History, Settings, and detail views with headless Chromium (or Firefox)

Output: `screenshots/app-*.png`, `screenshots/tray-*.png`

Requires: release build, `web/dist`, ImageMagick (`magick`), and `chromium`, `google-chrome`, or `firefox` in PATH.

## Manual tray capture

Processing state lasts only a few seconds — use the panel screenshot tool while dictating, or rely on exported tray PNGs from the script above.

## After capture

1. Review PNGs — no real API keys visible (Settings should show `(stored)` placeholder).
2. Update [README.md](../README.md) if filenames change.
3. `jj commit` images + README when ready to publish.