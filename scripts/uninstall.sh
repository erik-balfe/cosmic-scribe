#!/usr/bin/env bash
# Remove Cosmic Scribe user install (wrapper, daemon copy, autostart).
# Prefer the real binary (Homebrew/cargo), not a stale bash hash to ~/.local/bin/voice-input.
set -euo pipefail

resolve_binary() {
  if command -v brew >/dev/null 2>&1 && brew --prefix voice-input >/dev/null 2>&1; then
    echo "$(brew --prefix voice-input)/bin/voice-input"
    return
  fi
  if command -v brew >/dev/null 2>&1; then
    local prefix
    prefix="$(brew --prefix 2>/dev/null)" && [ -x "${prefix}/bin/voice-input" ] && {
      echo "${prefix}/bin/voice-input"
      return
    }
  fi
  type -P voice-input 2>/dev/null || true
}

BIN="$(resolve_binary || true)"
PURGE=0
for arg in "$@"; do
  if [ "$arg" = "--purge" ]; then
    PURGE=1
  fi
done

if [ -n "$BIN" ] && [ -x "$BIN" ]; then
  if [ "$PURGE" -eq 1 ]; then
    exec "$BIN" --uninstall --purge
  else
    exec "$BIN" --uninstall
  fi
fi

# Fallback if no binary on PATH (broken symlink): same paths as voice-input --uninstall
pkill -f 'voice-input.*--daemon' 2>/dev/null || true
sleep 1
rm -f "${HOME}/.local/bin/voice-input"
rm -f "${HOME}/.local/share/voice-input/voice-input"
rm -f "${HOME}/.config/autostart/voice-input.desktop"
rm -f "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/voice-input.sock"
if [ "$PURGE" -eq 1 ]; then
  rm -rf "${HOME}/.local/share/voice-input"
  echo "Removed data directory."
fi
echo "Removed install artifacts."
echo "Run: hash -r   # if the shell still points at a deleted ~/.local/bin/voice-input"
echo "Homebrew: brew uninstall voice-input"