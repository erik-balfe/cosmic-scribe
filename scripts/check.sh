#!/usr/bin/env bash
# Run all local quality checks (same as CI).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> rustfmt"
cargo fmt --all -- --check

echo "==> clippy"
cargo clippy --workspace --exclude cosmic-scribe-gui-native --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace --exclude cosmic-scribe-gui-native

echo "==> web lint"
cd web
npm ci
npm run lint
npm run build
cd ..

echo "==> cargo build --release"
cargo build --release --workspace --exclude cosmic-scribe-gui-native

echo "All checks passed."