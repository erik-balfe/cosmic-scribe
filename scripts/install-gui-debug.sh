#!/usr/bin/env bash
# Install cosmic-scribe-gui-debug to ~/.local/bin (does not touch prod cosmic-scribe).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Homebrew pkg-config often ignores /usr/lib64/pkgconfig (Fedora system -devel packages).
export PKG_CONFIG_PATH="/usr/lib64/pkgconfig:/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

if ! pkg-config --exists glib-2.0 2>/dev/null; then
  echo "error: glib-2.0 not found via pkg-config (PKG_CONFIG_PATH=$PKG_CONFIG_PATH)" >&2
  echo "Fedora: sudo dnf install glib2-devel webkit2gtk4.1-devel gtk3-devel openssl-devel \\" >&2
  echo "              libappindicator-gtk3-devel librsvg2-devel" >&2
  exit 1
fi

echo "==> Building cosmic-scribe-gui-debug (debug profile)"
cargo build -p cosmic-scribe-gui --bin cosmic-scribe-gui-debug

BIN="$ROOT/target/debug/cosmic-scribe-gui-debug"
DEST="${HOME}/.local/bin/cosmic-scribe-gui-debug"
mkdir -p "${HOME}/.local/bin"
cp -f "$BIN" "$DEST"
chmod 755 "$DEST"

echo "Installed: $DEST"
echo ""
echo "Run:"
echo "  cosmic-scribe-gui-debug              # History (same data as prod cosmic-scribe)"
echo "  cosmic-scribe-gui-debug --settings   # Settings"
echo ""
echo "Remove: scripts/uninstall-gui-debug.sh"