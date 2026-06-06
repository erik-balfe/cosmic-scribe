#!/usr/bin/env bash
# Build and install cosmic-scribe-gui (release): real binary + wrapper launcher + desktop/dbus.
# Does not replace the prod daemon — pair with cosmic-scribe --install/--update.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PKG_CONFIG_PATH="/usr/lib64/pkgconfig:/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

if ! pkg-config --exists glib-2.0 2>/dev/null; then
  echo "error: glib-2.0 not found via pkg-config (PKG_CONFIG_PATH=$PKG_CONFIG_PATH)" >&2
  echo "Fedora: sudo dnf install glib2-devel webkit2gtk4.1-devel gtk3-devel openssl-devel \\" >&2
  echo "              libappindicator-gtk3-devel librsvg2-devel" >&2
  exit 1
fi

if [[ ! -f web/dist/index.html ]]; then
  echo "==> Building web UI (web/dist missing)"
  (cd web && npm run build)
fi

if [[ ! -f gui/icons/128x128.png ]]; then
  echo "==> Generating GUI icons"
  "$ROOT/scripts/generate-gui-icons.sh"
fi

echo "==> Stopping any open GUI window"
pgrep -f "${HOME}/.local/(bin/cosmic-scribe-gui|share/cosmic-scribe/cosmic-scribe-gui)" 2>/dev/null \
  | xargs -r kill 2>/dev/null || true
sleep 0.5

echo "==> Building cosmic-scribe-gui (release)"
cargo build --release -p cosmic-scribe-gui --bin cosmic-scribe-gui

REAL="${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui"
WRAPPER="${HOME}/.local/bin/cosmic-scribe-gui"
mkdir -p "${HOME}/.local/share/cosmic-scribe" "${HOME}/.local/bin"
cp -f "$ROOT/target/release/cosmic-scribe-gui" "$REAL"
chmod 755 "$REAL"
cp -f "$ROOT/scripts/cosmic-scribe-gui.sh" "$WRAPPER"
chmod 755 "$WRAPPER"

APP_ID="com.cosmic-scribe.gui"
ICON_SRC="$ROOT/gui/icons"
for size in 32 128 256; do
  theme_dir="${HOME}/.local/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$theme_dir"
  cp -f "$ICON_SRC/${size}x${size}.png" "$theme_dir/${APP_ID}.png"
done

DESKTOP="${HOME}/.local/share/applications/${APP_ID}.desktop"
mkdir -p "${HOME}/.local/share/applications"
sed "s|^Exec=.*|Exec=${WRAPPER} %U|" "$ROOT/gui/com.cosmic-scribe.gui.desktop" >"$DESKTOP"
chmod 644 "$DESKTOP"

DBUS_SERVICE="${HOME}/.local/share/dbus-1/services/${APP_ID}.service"
mkdir -p "${HOME}/.local/share/dbus-1/services"
sed -e "s|^Exec=.*|Exec=${WRAPPER} %U|" "$ROOT/gui/com.cosmic-scribe.gui.service" >"$DBUS_SERVICE"
chmod 644 "$DBUS_SERVICE"

update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true

echo "Installed binary: $REAL"
echo "Launcher:         $WRAPPER  (argv[0] = Cosmic Scribe for COSMIC dock)"
echo "Desktop:          $DESKTOP"
echo "DBus service:     $DBUS_SERVICE"
echo ""
echo "Close any old window, then tray → History or Settings."
echo "Remove GUI only: scripts/uninstall-gui-prod.sh"