#!/usr/bin/env bash
# Seed demo History data, export tray icons, capture app UI via headless Chromium.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
SCREENSHOTS="$ROOT/screenshots"
BIN="$ROOT/target/release/cosmic-scribe"

mkdir -p "$SCREENSHOTS"

echo "==> Seeding demo recordings"
"$ROOT/scripts/seed-demo-recordings.sh"

echo "==> Generating tray icon masks"
"$ROOT/scripts/generate-gui-icons.sh"

echo "==> Building release binary"
cargo build --release --bin cosmic-scribe --bin export-tray-icons

echo "==> Exporting tray state PNGs"
"$ROOT/target/release/export-tray-icons"
if command -v magick >/dev/null 2>&1; then
  for f in "$SCREENSHOTS"/tray-*.png; do
    [[ -f "$f" ]] || continue
    magick "$f" -filter point -resize 200% "$f"
  done
fi

if [[ ! -f web/dist/index.html ]]; then
  echo "==> Building web UI"
  (cd web && npm run build)
fi

echo "==> Starting UI server"
pkill -f "cosmic-scribe --ui-server" 2>/dev/null || true
sleep 0.5
COSMIC_SCRIBE_NO_BROWSER=1 "$BIN" --ui-server >"$SCREENSHOTS/.ui-server.log" 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

BASE_URL=""
for _ in $(seq 1 40); do
  BASE_URL="$(grep -oE 'UI: http://127\.0\.0\.1:[0-9]+' "$SCREENSHOTS/.ui-server.log" 2>/dev/null | head -1 | sed 's/UI: //' || true)"
  if [[ -n "$BASE_URL" ]]; then
    break
  fi
  sleep 0.25
done
if [[ -z "$BASE_URL" ]]; then
  echo "error: UI server did not start" >&2
  cat "$SCREENSHOTS/.ui-server.log" >&2 || true
  exit 1
fi
echo "UI server: $BASE_URL"

echo "==> Capturing app window screenshots (Playwright)"
PW_DIR="$(mktemp -d)"
trap 'rm -rf "$PW_DIR"; kill "$SERVER_PID" 2>/dev/null || true' EXIT
(
  cd "$PW_DIR"
  npm init -y >/dev/null 2>&1
  npm install playwright@1.58.0 >/dev/null 2>&1
  node "$ROOT/scripts/capture-ui-playwright.mjs" "$BASE_URL" "$SCREENSHOTS"
)

rm -f "$SCREENSHOTS/.ui-server.log"
echo ""
echo "Screenshots ready in: $SCREENSHOTS"