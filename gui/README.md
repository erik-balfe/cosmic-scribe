# Cosmic Scribe GUI (Tauri spike)

Native window for History and Settings. See [docs/TAURI.md](../docs/TAURI.md).

## Build

```bash
cd ../web && npm run build
cd ../gui
sudo dnf install webkit2gtk4.1-devel gtk3-devel glib2-devel   # Fedora, once
cargo build
```

## Run

```bash
cargo run -p cosmic-scribe-gui              # History
cargo run -p cosmic-scribe-gui -- --settings # Settings
```

Phase 1 loads the same Svelte UI via a local HTTP server inside the WebKit webview. Phase 2 will replace HTTP with Tauri `invoke()` commands.