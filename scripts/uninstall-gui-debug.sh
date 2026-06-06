#!/usr/bin/env bash
# Remove debug GUI binary only (prod data in ~/.local/share/cosmic-scribe/ is untouched).
set -euo pipefail

rm -f "${HOME}/.local/bin/cosmic-scribe-gui-debug"
echo "Removed: ${HOME}/.local/bin/cosmic-scribe-gui-debug"