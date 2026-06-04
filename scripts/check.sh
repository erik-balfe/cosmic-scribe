#!/usr/bin/env bash
# Run all local quality checks (same as CI).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> rustfmt"
cargo fmt --all -- --check

echo "==> clippy"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test"
cargo test

echo "==> web lint"
cd web
npm ci
npm run lint
npm run build
cd ..

echo "==> cargo build --release"
cargo build --release

echo "All checks passed."