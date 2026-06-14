#!/usr/bin/env bash
# Build, test, commit, tag, push, and install v0.3.2.
# Run: bash scripts/release-v0.3.2.sh
# Log: ~/.local/share/cosmic-scribe/release-v0.3.2.log
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${XDG_DATA_HOME:-$HOME/.local/share}/cosmic-scribe/release-v0.3.2.log"
mkdir -p "$(dirname "$LOG")"
exec > >(tee -a "$LOG") 2>&1

cd "$ROOT"
echo "=== release v0.3.2 $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

echo "==> cargo fmt --all"
cargo fmt --all

echo "==> cargo test"
cargo test

echo "==> cargo build --release"
cargo build --release

echo "==> jj status"
jj status

echo "==> jj diff --stat"
jj diff --stat

if jj log -r @ -T description 2>/dev/null | grep -q 'v0.3.2'; then
  echo "==> working copy already describes v0.3.2, skipping jj commit"
elif jj diff --stat 2>/dev/null | grep -q .; then
  echo "==> jj commit"
  jj commit -m "fix(lifecycle): systemd user service autostart for cold boot

Replace COSMIC desktop autostart with com.cosmic-scribe.service on
graphical-session.target (same pattern as cosmic-paste). Migrate on
--install, --autostart, and --update when autostart was configured.

v0.3.2"
else
  echo "==> no diff, skipping jj commit"
fi

echo "==> jj tag"
jj git tag set v0.3.2 -r @ 2>/dev/null || jj git tag set v0.3.2 -r @ --ignore-immutable

echo "==> jj push"
jj git push --bookmark master
jj git push --tags

echo "==> install release binary"
"$ROOT/target/release/cosmic-scribe" --install-from="$ROOT/target/release/cosmic-scribe"

echo "==> verify"
cosmic-scribe --status
systemctl --user is-enabled com.cosmic-scribe.service
test ! -e "${HOME}/.config/autostart/cosmic-scribe.desktop"

echo "OK — v0.3.2 built, pushed, installed"