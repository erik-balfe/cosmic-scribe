#!/usr/bin/env bash
# Build and install cosmic-scribe-gui-native (libcosmic): real binary + wrapper launcher.
# Does not replace the Tauri GUI or prod daemon.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PKG_CONFIG_PATH="/usr/lib64/pkgconfig:/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

echo "==> Stopping any open GUI window"
pgrep -f "${HOME}/.local/(bin/cosmic-scribe-gui|share/cosmic-scribe/cosmic-scribe-gui)" 2>/dev/null \
  | xargs -r kill 2>/dev/null || true
sleep 0.5

echo "==> Building cosmic-scribe-gui-native (release)"
cargo build --release -p cosmic-scribe-gui-native --bin cosmic-scribe-gui-native

REAL="${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui-native"
WRAPPER="${HOME}/.local/bin/cosmic-scribe-gui-native"
mkdir -p "${HOME}/.local/share/cosmic-scribe" "${HOME}/.local/bin"
cp -f "$ROOT/target/release/cosmic-scribe-gui-native" "$REAL"
chmod 755 "$REAL"

cat >"$WRAPPER" <<EOF
#!/usr/bin/env bash
set -euo pipefail
export COSMIC_SCRIBE_GUI=native
export COSMIC_SCRIBE_GUI_REAL="${REAL}"
export GTK_APPLICATION_ID="\${GTK_APPLICATION_ID:-com.cosmic-scribe.gui}"
export GDK_APPLICATION_NAME="\${GDK_APPLICATION_NAME:-Cosmic Scribe}"
REAL="\${COSMIC_SCRIBE_GUI_REAL}"
exec -a "Cosmic Scribe" "\$REAL" "\$@"
EOF
chmod 755 "$WRAPPER"

if [[ ! -f gui/icons/128x128.png ]]; then
  "$ROOT/scripts/generate-gui-icons.sh"
fi

APP_ID="com.cosmic-scribe.gui"
ICON_SRC="$ROOT/gui/icons"
for size in 32 128 256; do
  theme_dir="${HOME}/.local/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$theme_dir"
  cp -f "$ICON_SRC/${size}x${size}.png" "$theme_dir/${APP_ID}.png"
done
gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true

# App menu → native UI (same APP_ID as Tauri so icon/name stay Cosmic Scribe)
DESKTOP_DIR="${HOME}/.local/share/applications"
mkdir -p "$DESKTOP_DIR"
cat >"$DESKTOP_DIR/${APP_ID}.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Cosmic Scribe
GenericName=Voice dictation
Comment=Recording history and settings (libcosmic)
Exec=${WRAPPER} %U
Icon=${APP_ID}
StartupWMClass=Cosmic Scribe
Categories=Utility;
Terminal=false
SingleMainWindow=true
EOF
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo "Installed binary: $REAL"
echo "Launcher:         $WRAPPER"
echo "Desktop entry:    $DESKTOP_DIR/${APP_ID}.desktop"
echo ""
echo "Open:  cosmic-scribe-gui-native   or   cosmic-scribe-gui-native --settings"
echo "Tray:  prefers native when this binary is installed (override: COSMIC_SCRIBE_GUI=tauri)"