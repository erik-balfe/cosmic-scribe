#!/usr/bin/env bash
# Capture Cosmic Scribe autostart / daemon state for login debugging.
# Usage:
#   ./scripts/capture-autostart-debug.sh              # print to stdout
#   ./scripts/capture-autostart-debug.sh pre-login    # save snapshot before logout
#   ./scripts/capture-autostart-debug.sh post-login   # save snapshot after login

set -euo pipefail

LABEL="${1:-snapshot}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/cosmic-scribe"
OUT_DIR="$DATA_DIR/autostart-debug"
OUT="$OUT_DIR/${LABEL}-${STAMP}.txt"
BOOT_ID="$(cat /proc/sys/kernel/random/boot_id 2>/dev/null || echo unknown)"

mkdir -p "$OUT_DIR"

{
  echo "=== Cosmic Scribe autostart debug: $LABEL ==="
  echo "timestamp_utc: $STAMP"
  echo "boot_id: $BOOT_ID"
  echo "user: $(whoami)"
  echo "hostname: $(hostname)"
  echo

  echo "--- environment ---"
  echo "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-<unset>}"
  echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
  echo "DESKTOP_SESSION=${DESKTOP_SESSION:-<unset>}"
  echo "XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>}"
  echo

  echo "--- cosmic-scribe --status ---"
  if command -v cosmic-scribe >/dev/null 2>&1; then
    cosmic-scribe --status 2>&1 || true
  else
    echo "cosmic-scribe: not in PATH"
    if [[ -x "$HOME/.local/bin/cosmic-scribe" ]]; then
      "$HOME/.local/bin/cosmic-scribe" --status 2>&1 || true
    fi
  fi
  echo

  echo "--- daemon processes (pgrep) ---"
  pgrep -af 'cosmic-scribe.*--daemon' 2>/dev/null || echo "(none)"
  pgrep -af 'voice-input.*--daemon' 2>/dev/null || true
  echo

  echo "--- lock + socket ---"
  RUNTIME="${XDG_RUNTIME_DIR:-/tmp}"
  for f in "$RUNTIME/cosmic-scribe-daemon.lock" "$RUNTIME/cosmic-scribe.sock" \
           "$RUNTIME/voice-input-daemon.lock" "$RUNTIME/voice-input.sock"; do
    if [[ -e "$f" ]]; then
      echo "$f:"
      ls -la "$f" 2>/dev/null || true
      if [[ -f "$f" && "$f" == *lock ]]; then
        echo "  contents: $(tr -d '\n' < "$f" 2>/dev/null || echo ?)"
      fi
    else
      echo "$f: absent"
    fi
  done
  echo

  echo "--- legacy autostart desktop (should be absent) ---"
  for DESKTOP in \
    "$HOME/.config/autostart/cosmic-scribe.desktop" \
    "$HOME/.config/autostart/voice-input.desktop"; do
    if [[ -f "$DESKTOP" ]]; then
      echo "WARNING: stale desktop autostart: $DESKTOP"
      cat "$DESKTOP"
    else
      echo "absent (expected): $DESKTOP"
    fi
  done
  echo

  echo "--- legacy COSMIC autostart drop-in (should be absent) ---"
  DROPIN="$HOME/.config/systemd/user/app-cosmic\\x2dscribe@autostart.service.d"
  if [[ -d "$DROPIN" ]]; then
    echo "WARNING: stale drop-in: $DROPIN"
    ls -la "$DROPIN" 2>/dev/null || true
  else
    echo "absent (expected): $DROPIN"
  fi
  echo

  echo "--- systemd user unit (com.cosmic-scribe.service) ---"
  UNIT_FILE="$HOME/.config/systemd/user/com.cosmic-scribe.service"
  if [[ -f "$UNIT_FILE" ]]; then
    cat "$UNIT_FILE"
  else
    echo "absent: $UNIT_FILE"
  fi
  echo
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user show com.cosmic-scribe.service \
      -p ActiveState -p SubState -p Result -p ExecMainPID -p ExecMainStatus \
      -p ExecMainCode -p NRestarts -p Restart -p RestartUSec 2>/dev/null || \
      echo "(unit not found or systemctl failed)"
    systemctl --user is-enabled com.cosmic-scribe.service 2>/dev/null || true
  fi
  echo

  echo "--- journal (current boot, com.cosmic-scribe.service) ---"
  if command -v journalctl >/dev/null 2>&1; then
    journalctl --user -b -u com.cosmic-scribe.service --no-pager -n 80 2>/dev/null || \
      journalctl --user -b --no-pager -g 'cosmic-scribe' -n 40 2>/dev/null || \
      echo "(no journal matches)"
  fi
  echo

  echo "--- daemon.log (last 60 lines) ---"
  LOG="$DATA_DIR/daemon.log"
  if [[ -f "$LOG" ]]; then
    echo "path: $LOG ($(wc -c < "$LOG") bytes)"
    tail -n 60 "$LOG"
  else
    echo "absent: $LOG"
  fi
  echo

  echo "=== end $LABEL ==="
} | tee "$OUT"

echo "Wrote: $OUT"