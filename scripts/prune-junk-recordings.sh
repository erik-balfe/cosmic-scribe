#!/usr/bin/env bash
# Remove test/invalid recording artifacts (short clips, cargo test fixtures).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${COSMIC_SCRIBE_BIN:-$HOME/.local/bin/cosmic-scribe}"
if [[ -x "$BIN" ]]; then
  exec "$BIN" --prune-junk-recordings
fi
exec cargo run --manifest-path "$ROOT/Cargo.toml" --quiet -- --prune-junk-recordings