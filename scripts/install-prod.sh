#!/usr/bin/env bash
# Full prod install: release daemon + Tauri GUI. Keeps existing data/settings.
# Replaces ~/.local install; does not --purge data.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BIN="${HOME}/.local/bin/cosmic-scribe"
DAEMON="${HOME}/.local/share/cosmic-scribe/cosmic-scribe"

echo "==> Stopping daemon and stray UI processes"
if [[ -x "$BIN" ]]; then
  "$BIN" --stop 2>/dev/null || true
fi
pkill -f "cosmic-scribe --settings" 2>/dev/null || true
pkill -f "cosmic-scribe --history" 2>/dev/null || true
pkill -f "voice-input --settings" 2>/dev/null || true
pkill -f "voice-input --history" 2>/dev/null || true
pkill -f "cosmic-scribe-gui" 2>/dev/null || true
"$ROOT/scripts/uninstall-gui-debug.sh" 2>/dev/null || true
sleep 1

echo "==> Removing previous user install (data preserved)"
if [[ -x "$BIN" ]]; then
  "$BIN" --uninstall 2>/dev/null || true
elif [[ -x "$DAEMON" ]]; then
  "$DAEMON" --uninstall 2>/dev/null || true
fi

if [[ ! -f web/dist/index.html ]]; then
  echo "==> Building web UI"
  (cd web && npm run build)
fi

echo "==> Building release daemon"
cargo build --release

echo "==> Installing daemon"
"$ROOT/target/release/cosmic-scribe" --install

echo "==> Installing Tauri GUI"
"$ROOT/scripts/install-gui-prod.sh"

echo ""
echo "Prod install complete (daemon + Tauri GUI)."
echo "Optional native COSMIC UI:  ./scripts/install-gui-native-prod.sh"
echo "Auth:  cosmic-scribe --login   # or set API key in Settings"
echo "Data:  ${HOME}/.local/share/cosmic-scribe/"
"$BIN" --status