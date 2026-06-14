#!/usr/bin/env bash
# Launcher wrapper — COSMIC uses argv[0] for dock/switcher label when not DBus-activated.
set -euo pipefail
UNIT="${HOME}/.config/systemd/user/com.cosmic-scribe.service"
if [[ -f "$UNIT" ]]; then
  systemctl --user start com.cosmic-scribe.service
fi
REAL="${COSMIC_SCRIBE_GUI_REAL:-${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui}"
export GTK_APPLICATION_ID="${GTK_APPLICATION_ID:-com.cosmic-scribe.gui}"
export GDK_APPLICATION_NAME="${GDK_APPLICATION_NAME:-Cosmic Scribe}"
exec -a "Cosmic Scribe" "$REAL" "$@"