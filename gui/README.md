# Cosmic Scribe GUI (Tauri spike, debug)

Native window for History and Settings. Binary `cosmic-scribe-gui-debug` only — **same data** as prod (`~/.local/share/cosmic-scribe/`). Does not install or replace the prod daemon.

See [docs/TAURI.md](../docs/TAURI.md).

## Install (recommended)

```bash
cd ../web && npm run build
./scripts/install-gui-debug.sh
```

## Run

```bash
cosmic-scribe-gui-debug              # History
cosmic-scribe-gui-debug --settings   # Settings
```

## Remove

```bash
./scripts/uninstall-gui-debug.sh
```

Phase 1 loads the Svelte UI via a local HTTP server in WebKit. Phase 2: Tauri `invoke()` + tray opens this instead of browser.