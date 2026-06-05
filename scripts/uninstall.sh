#!/usr/bin/env bash
# Remove Cosmic Scribe install (daemon, binaries, autostart). Keeps config under ~/.local/share/voice-input/.
set -euo pipefail

BIN="${HOME}/.local/bin/voice-input"
SHARE="${HOME}/.local/share/voice-input/voice-input"
AUTOSTART="${HOME}/.config/autostart/voice-input.desktop"
RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/voice-input.sock"

if command -v voice-input >/dev/null 2>&1; then
  voice-input --stop 2>/dev/null || true
else
  pkill -f 'voice-input.*--daemon' 2>/dev/null || true
fi
sleep 1

rm -f "$BIN" "$SHARE" "$AUTOSTART" "$RUNTIME"
echo "Removed install artifacts (config in ~/.local/share/voice-input/ kept)."
echo "  $BIN"
echo "  $SHARE"
echo "  $AUTOSTART"