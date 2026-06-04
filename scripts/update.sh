#!/usr/bin/env bash
# Stop daemon, build release, install copy, start daemon.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release
exec "$ROOT/target/release/voice-input" --update