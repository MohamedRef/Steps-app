#!/usr/bin/env bash
# Runs every test with the wall clock moved forward (needs libfaketime), so a
# test pinned to today's date fails here instead of on tomorrow's pull
# requests (2026-11-05: the changelog test). nightly.yml runs it; locally:
#   scripts/test-later-dates.sh '+37h' '+400d'
# Offsets are libfaketime's: '+37h', '+400d'. The tests are built with the
# real clock and then run straight from their binaries: cargo itself under a
# moved clock rebuilds or hangs. Only the wall clock moves; the monotonic
# clock (timeouts) does not.
set -euo pipefail
cd "$(dirname "$0")/.."
[ $# -gt 0 ] || set -- '+37h' '+400d'
bins=$(mktemp)
build() { cargo test --no-run --message-format=json "$@" \
  | jq -r 'select(.profile.test == true and .executable != null) | "\(.manifest_path) \(.executable)"'; }
{ build --workspace --exclude steps-app; build -p steps-app --no-default-features; } > "$bins"
failed=0
for offset in "$@"; do
  echo "== clock $offset: $(faketime -f "$offset" date -u)"
  while read -r manifest exe; do
    # cargo runs each test binary from its crate's folder; so do we.
    if ! (cd "$(dirname "$manifest")" && FAKETIME_DONT_FAKE_MONOTONIC=1 faketime -f "$offset" "$exe" -q); then
      echo "::error title=Test depends on the date::$(basename "$exe") failed with the clock at $offset"
      failed=1
    fi
  done < "$bins"
done
rm -f "$bins"
exit "$failed"
