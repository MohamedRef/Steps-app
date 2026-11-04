#!/usr/bin/env bash
# Creates the permanent Android release signing key outside the repo
# (adapted from kabar-app main: scripts/android-keystore.sh). The password
# lives in the macOS Keychain (service "steps-android"), never in a file or
# an argument list. Run it again any time: an existing key is only checked,
# never replaced (docs/RELEASE.md §2).
set -euo pipefail
KEYSTORE="$1"
ALIAS="$2"

command -v keytool >/dev/null || { echo "keytool is missing: brew install --cask temurin" >&2; exit 1; }

if [ -f "$KEYSTORE" ]; then
  # The key is permanent: only make sure the Keychain password still opens it.
  if ! security find-generic-password -s steps-android >/dev/null 2>&1; then
    echo "$KEYSTORE exists but the Keychain has no steps-android password." >&2
    echo "Restore it from your backup (just release-keys-backup made one) before going on." >&2
    exit 1
  fi
  STEPS_KS_PASS="$(security find-generic-password -s steps-android -w)" \
    keytool -list -keystore "$KEYSTORE" -alias "$ALIAS" -storepass:env STEPS_KS_PASS >/dev/null \
    || { echo "The steps-android password does not open $KEYSTORE (alias $ALIAS)." >&2; exit 1; }
  echo "Keystore already exists and opens: $KEYSTORE. It is permanent: keep it and its backup."
else
  mkdir -p "$(dirname "$KEYSTORE")"
  pass="$(openssl rand -hex 24)"
  # Through security's own prompt (stdin), so the password never shows up in
  # another process's argument list.
  printf 'add-generic-password -U -s steps-android -a %s -w %s\n' "$USER" "$pass" | security -i >/dev/null
  STEPS_KS_PASS="$pass" keytool -genkeypair -keystore "$KEYSTORE" -storetype PKCS12 -alias "$ALIAS" \
    -keyalg RSA -keysize 4096 -validity 10000 -storepass:env STEPS_KS_PASS -keypass:env STEPS_KS_PASS \
    -dname "CN=Steps, O=Personal"
  unset pass
  chmod 600 "$KEYSTORE"
  echo "Created $KEYSTORE. Back it up now: just release-keys-backup"
fi
STEPS_KS_PASS="$(security find-generic-password -s steps-android -w)" \
  keytool -list -v -keystore "$KEYSTORE" -alias "$ALIAS" -storepass:env STEPS_KS_PASS \
  | sed -n 's/^[[:space:]]*SHA256: */Certificate SHA-256: /p'
