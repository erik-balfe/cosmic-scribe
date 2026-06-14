#!/usr/bin/env bash
# Re-apply com.cosmic-scribe.service login autostart. Safe to re-run.
set -euo pipefail

BIN="${HOME}/.local/bin/cosmic-scribe"
if [[ ! -x "$BIN" ]]; then
  echo "error: cosmic-scribe not installed at $BIN" >&2
  exit 1
fi

"$BIN" --autostart
systemctl --user restart com.cosmic-scribe.service

echo ""
echo "Autostart: com.cosmic-scribe.service (graphical-session.target)"
systemctl --user status com.cosmic-scribe.service --no-pager 2>/dev/null | head -12 || true