#!/usr/bin/env bash
# Render gui/icons/icon.svg → PNG sizes for Tauri + freedesktop icon theme.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON_DIR="$ROOT/gui/icons"
SVG="$ICON_DIR/icon.svg"

if [[ ! -f "$SVG" ]]; then
  echo "error: missing $SVG" >&2
  exit 1
fi

for size in 32 128 256; do
  magick -background none -density 384 "$SVG" -type TrueColorAlpha -depth 8 \
    -resize "${size}x${size}" "PNG32:${ICON_DIR}/${size}x${size}.png"
  echo "wrote $ICON_DIR/${size}x${size}.png"
done

TRAY_SVG="$ICON_DIR/icon-tray.svg"
if [[ ! -f "$TRAY_SVG" ]]; then
  echo "error: missing $TRAY_SVG" >&2
  exit 1
fi
for size in 22 44; do
  magick -background none -density 384 "$TRAY_SVG" -type TrueColorAlpha -depth 8 \
    -resize "${size}x${size}" "PNG32:${ICON_DIR}/tray-${size}.png"
  echo "wrote $ICON_DIR/tray-${size}.png"
  for part in capsule body; do
    magick -background none -density 384 "$ICON_DIR/icon-tray-${part}.svg" \
      -type TrueColorAlpha -depth 8 -resize "${size}x${size}" \
      "PNG32:${ICON_DIR}/tray-${part}-${size}.png"
    echo "wrote $ICON_DIR/tray-${part}-${size}.png"
  done
done

cp -f "$ICON_DIR/128x128.png" "$ICON_DIR/icon.png"
echo "wrote $ICON_DIR/icon.png"

ASSETS="$ROOT/assets"
mkdir -p "$ASSETS"
cp -f "$ICON_DIR/icon.svg" "$ASSETS/logo.svg"
cp -f "$ICON_DIR/128x128.png" "$ASSETS/logo.png"
cp -f "$ICON_DIR/256x256.png" "$ASSETS/logo-256.png"
echo "wrote $ASSETS/logo.{svg,png} and logo-256.png"