#!/usr/bin/env bash
# Refreshes content/qbank/uworld.yaml (UWorld QIDs per lecture, by Boards and
# Beyond video) from the public doc. One command on the Mac:
#   scripts/qbank-sync.sh            rewrite the file if the doc changed
#   scripts/qbank-sync.sh --check    only say whether it is current
# A new B&B video the map doesn't cover stops it with its name: add it to
# content/qbank/bb-map.yaml and run again (docs/QBANK.md).
set -euo pipefail
cd "$(dirname "$0")/.."
DOC="https://docs.google.com/document/d/1LZweYMU5yXZIQtFl4AVpOo1-U9UQ6uRAo4mnu74Z2n4/export?format=txt"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fsSL --retry 3 "$DOC" -o "$tmp"
cargo run -q -p steps-tools -- qbank sync --from "$tmp" "$@"
