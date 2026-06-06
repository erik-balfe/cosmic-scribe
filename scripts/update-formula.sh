#!/usr/bin/env bash
# Update Formula/cosmic-scribe.rb for a tagged release (used locally and in CI).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:?usage: $0 VERSION (e.g. 0.1.0)}"
FORMULA="Formula/cosmic-scribe.rb"
URL="https://github.com/erik-balfe/cosmic-scribe/archive/refs/tags/v${VERSION}.tar.gz"

echo "==> Fetching ${URL}"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT
curl -fsSL "$URL" -o "$TMP"
SHA256="$(sha256sum "$TMP" | awk '{print $1}')"
echo "==> sha256: ${SHA256}"

python3 - "$FORMULA" "$VERSION" "$URL" "$SHA256" <<'PY'
import re
import sys

path, version, url, sha = sys.argv[1:5]
text = open(path, encoding="utf-8").read()
text = re.sub(r'url "[^"]+"', f'url "{url}"', text, count=1)
text = re.sub(r'version "[^"]+"', f'version "{version}"', text, count=1)
text = re.sub(r"sha256 :no_check", f'sha256 "{sha}"', text, count=1)
text = re.sub(r'sha256 "[a-f0-9]{64}"', f'sha256 "{sha}"', text, count=1)
open(path, "w", encoding="utf-8").write(text)
PY

echo "==> Updated ${FORMULA}"
grep -E 'url |version |sha256 ' "$FORMULA"