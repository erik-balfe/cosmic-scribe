#!/usr/bin/env bash
# Add demo History entries for screenshots / README (does not touch API keys or settings).
# Safe to re-run: skips stems that already exist.
set -euo pipefail

RECORDINGS="${COSMIC_SCRIBE_RECORDINGS_DIR:-${HOME}/.local/share/cosmic-scribe/recordings}"
mkdir -p "$RECORDINGS"

# 5s clip @ 16 kHz mono s16le → 160000 bytes; enough for waveform bars.
write_raw() {
  local path="$1"
  local bytes="${2:-160000}"
  if [[ -f "$path" ]]; then
    return 0
  fi
  python3 - "$path" "$bytes" <<'PY'
import struct, sys
path, nbytes = sys.argv[1], int(sys.argv[2])
samples = nbytes // 2
with open(path, "wb") as f:
    for i in range(samples):
        # gentle wave so the UI waveform is not flat
        v = int(4000 * (0.5 + 0.5 * ((i % 800) / 800)))
        f.write(struct.pack("<h", v))
PY
}

write_entry() {
  local stem="$1"
  local text="$2"
  local raw="${RECORDINGS}/${stem}.raw"
  local txt="${RECORDINGS}/${stem}.txt"
  if [[ -f "$raw" ]]; then
    echo "skip (exists): $stem"
    return 0
  fi
  write_raw "$raw"
  printf '%s' "$text" >"$txt"
  echo "wrote: $stem"
}

write_entry "2026-06-07_09-15-00_5200ms" "Quick note for the meeting tomorrow."
write_entry "2026-06-07_10-30-00_4800ms" "Remind me to buy coffee filters."
write_entry "2026-06-07_11-05-00_6100ms" "The API key lives in Settings, not a config file."
write_entry "2026-06-07_14-20-00_5500ms" "Send the draft when you are done reviewing."

# Long entry for History detail screenshot (replace via dictation — see docs/SCREENSHOTS.md)
write_entry "2026-06-07_15-00-00_12000ms" "Cosmic Scribe lets you dictate anywhere on the COSMIC desktop. Press your shortcut, speak naturally, and the transcript lands in your editor or on the clipboard. Open History to review past recordings, copy older text, or edit a transcript before pasting it elsewhere."

echo ""
echo "Demo recordings in: $RECORDINGS"
echo "Open: cosmic-scribe-gui   or tray → History"
echo "Optional: dictate docs/SCREENSHOTS.md script to replace the long entry with your voice."