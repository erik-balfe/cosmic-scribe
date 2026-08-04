#!/usr/bin/env bash
# Run all local quality checks (same as CI).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> rustfmt"
cargo fmt --all -- --check

echo "==> clippy (daemon; matches CI)"
cargo clippy -p cosmic-scribe --all-targets -- -D warnings

echo "==> cargo test (daemon; matches CI)"
cargo test -p cosmic-scribe

echo "==> web lint"
cd web
npm ci
npm run lint
npm run build
cd ..

echo "==> cargo build --release (daemon; matches CI)"
cargo build --release -p cosmic-scribe

# Optional local GUIs (not in GitHub CI — need GTK/WebKit/COSMIC).
if [[ "${COSMIC_SCRIBE_CHECK_GUI:-}" == "1" ]]; then
  echo "==> clippy/test GUI crates (COSMIC_SCRIBE_CHECK_GUI=1)"
  cargo clippy -p cosmic-scribe-gui --all-targets -- -D warnings
  cargo test -p cosmic-scribe-gui
fi

echo "All checks passed."