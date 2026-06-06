#!/bin/bash
# Record test audio for cosmic-scribe.
# Usage: ./test-assets/record.sh <name>
set -euo pipefail

NAME="${1:-test}"
OUT="test-assets/${NAME}.raw"
DUR="${2:-5}"  # default 5s, override with 2nd arg

echo "=== Recording $DUR seconds to $OUT ==="
echo "Recording starts NOW — speak!"
echo ""

# Try arecord first (ALSA), with fixed duration so no Ctrl+C needed
if command -v arecord &>/dev/null; then
    echo "[arecord] recording for ${DUR}s..."
    arecord -r 16000 -c 1 -f S16_LE -t raw -d "$DUR" "$OUT"
    SIZE=$(stat -c%s "$OUT" 2>/dev/null || echo 0)
    echo "[arecord] done: $SIZE bytes (~$(echo "scale=1; $SIZE / 32000" | bc)s)"

# Fallback: pw-record (PipeWire native)
elif command -v pw-record &>/dev/null; then
    echo "[pw-record] recording for ${DUR}s..."
    pw-record --rate 16000 --channels 1 --format s16 "$OUT" &
    PID=$!
    sleep "$DUR"
    kill $PID 2>/dev/null || true
    wait $PID 2>/dev/null || true
    SIZE=$(stat -c%s "$OUT" 2>/dev/null || echo 0)
    echo "[pw-record] done: $SIZE bytes"

# Fallback: ffmpeg
elif command -v ffmpeg &>/dev/null; then
    echo "[ffmpeg] recording for ${DUR}s..."
    ffmpeg -f alsa -ac 1 -ar 16000 -t "$DUR" -f s16le "$OUT" -y -loglevel error
    SIZE=$(stat -c%s "$OUT" 2>/dev/null || echo 0)
    echo "[ffmpeg] done: $SIZE bytes"

else
    echo "ERROR: no recording tool found (arecord, pw-record, ffmpeg)" >&2
    exit 1
fi

echo ""
echo "To test: source .env && cargo run -- --file-input=$OUT"
