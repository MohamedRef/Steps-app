#!/usr/bin/env bash
# zipalign + apksigner with the release key from ~/.steps and the Keychain.
# Adapted from kabar-app main: scripts/android-sign.sh.
set -euo pipefail
IN="$1"; OUT="$2"; KEYSTORE="$3"; ALIAS="$4"

[ -f "$IN" ] || { echo "No APK found from dx bundle"; exit 1; }
[ -f "$KEYSTORE" ] || { echo "No keystore. Run: just android-keystore"; exit 1; }
PASS="$(security find-generic-password -s steps-android -w)"

ALIGNED="$(mktemp -t steps).apk"
zipalign -f -p 4 "$IN" "$ALIGNED"
apksigner sign --ks "$KEYSTORE" --ks-key-alias "$ALIAS" --ks-pass "pass:$PASS" --key-pass "pass:$PASS" \
  --out "$OUT" "$ALIGNED"
apksigner verify "$OUT"
rm -f "$ALIGNED"
echo "Signed: $OUT"
