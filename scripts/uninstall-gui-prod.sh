#!/usr/bin/env bash
# Remove prod Tauri GUI launcher, binary, desktop entry, dbus service, icons (keeps daemon + data).
set -euo pipefail
APP_ID="com.cosmic-scribe.gui"
pgrep -f "${HOME}/.local/(bin/cosmic-scribe-gui|share/cosmic-scribe/cosmic-scribe-gui)" 2>/dev/null \
  | xargs -r kill 2>/dev/null || true
rm -f "${HOME}/.local/bin/cosmic-scribe-gui"
rm -f "${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui"
rm -f "${HOME}/.local/share/applications/${APP_ID}.desktop"
rm -f "${HOME}/.local/share/dbus-1/services/${APP_ID}.service"
for size in 32 128 256; do
  rm -f "${HOME}/.local/share/icons/hicolor/${size}x${size}/apps/${APP_ID}.png"
done
update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
echo "Removed: cosmic-scribe-gui launcher, binary, desktop entry, dbus service, and icons"