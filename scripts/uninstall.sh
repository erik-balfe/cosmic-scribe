#!/usr/bin/env bash
# Uninstall Cosmic Scribe user install (tray daemon, wrapper, autostart).
# Prefer the real binary (Homebrew/cargo), not a stale bash hash to ~/.local/bin/cosmic-scribe.
set -euo pipefail

resolve_binary() {
  if command -v brew >/dev/null 2>&1 && brew --prefix cosmic-scribe >/dev/null 2>&1; then
    echo "$(brew --prefix cosmic-scribe)/bin/cosmic-scribe"
    return
  fi
  if command -v brew >/dev/null 2>&1; then
    prefix="$(brew --prefix 2>/dev/null)" && [ -x "${prefix}/bin/cosmic-scribe" ] && {
      echo "${prefix}/bin/cosmic-scribe"
      return
    }
  fi
  type -P cosmic-scribe 2>/dev/null || true
}

BIN="$(resolve_binary || true)"
PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
"$ROOT/scripts/uninstall-gui-prod.sh" 2>/dev/null || true
"$ROOT/scripts/uninstall-gui-debug.sh" 2>/dev/null || true

if [ -n "$BIN" ] && [ -x "$BIN" ]; then
  if [ "$PURGE" -eq 1 ]; then
    exec "$BIN" --uninstall --purge
  else
    exec "$BIN" --uninstall
  fi
fi

# Fallback if no binary on PATH (broken symlink): same paths as cosmic-scribe --uninstall
pkill -f 'cosmic-scribe.*--daemon' 2>/dev/null || true
pkill -f 'voice-input.*--daemon' 2>/dev/null || true

rm -f "${HOME}/.local/bin/cosmic-scribe"
rm -f "${HOME}/.local/bin/voice-input"
rm -f "${HOME}/.local/bin/cosmic-scribe-gui"
rm -f "${HOME}/.local/bin/cosmic-scribe-gui-debug"
rm -f "${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui"
rm -f "${HOME}/.local/share/cosmic-scribe/cosmic-scribe"
rm -f "${HOME}/.local/share/cosmic-scribe/voice-input"
rm -f "${HOME}/.local/share/voice-input/voice-input"
rm -f "${HOME}/.local/share/cosmic-scribe/ui-browser.lock"
rm -f "${HOME}/.local/share/cosmic-scribe/gui-debug.lock"
systemctl --user disable --now com.cosmic-scribe.service 2>/dev/null || true
rm -f "${HOME}/.config/systemd/user/com.cosmic-scribe.service"
rm -rf "${HOME}/.config/systemd/user/app-cosmic\\x2dscribe@autostart.service.d"
rm -f "${HOME}/.config/autostart/cosmic-scribe.desktop"
rm -f "${HOME}/.config/autostart/voice-input.desktop"
rm -f "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/cosmic-scribe.sock"
rm -f "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/voice-input.sock"
if [ "$PURGE" -eq 1 ]; then
  rm -rf "${HOME}/.local/share/cosmic-scribe"
  rm -rf "${HOME}/.local/share/voice-input"
fi

echo "Run: hash -r   # if the shell still points at a deleted ~/.local/bin/cosmic-scribe"
echo "Homebrew: brew uninstall cosmic-scribe"