#!/usr/bin/env bash
# Build and install cosmic-scribe-gui-native-debug for local testing.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PKG_CONFIG_PATH="/usr/lib64/pkgconfig:/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

echo "==> Building cosmic-scribe-gui-native-debug (release + debug-gui feature)"
cargo build --release -p cosmic-scribe-gui-native --bin cosmic-scribe-gui-native-debug --features debug-gui

REAL="${HOME}/.local/share/cosmic-scribe/cosmic-scribe-gui-native-debug"
WRAPPER="${HOME}/.local/bin/cosmic-scribe-gui-native-debug"
mkdir -p "${HOME}/.local/share/cosmic-scribe" "${HOME}/.local/bin"
cp -f "$ROOT/target/release/cosmic-scribe-gui-native-debug" "$REAL"
chmod 755 "$REAL"

cat >"$WRAPPER" <<EOF
#!/usr/bin/env bash
set -euo pipefail
export COSMIC_SCRIBE_GUI=native
export COSMIC_SCRIBE_GUI_REAL="${REAL}"
export GTK_APPLICATION_ID="\${GTK_APPLICATION_ID:-com.cosmic-scribe.gui}"
export GDK_APPLICATION_NAME="\${GDK_APPLICATION_NAME:-Cosmic Scribe}"
REAL="\${COSMIC_SCRIBE_GUI_REAL}"
exec -a "Cosmic Scribe (debug)" "\$REAL" "\$@"
EOF
chmod 755 "$WRAPPER"

echo "Installed: $WRAPPER"
echo "Run: cosmic-scribe-gui-native-debug [--settings]"