#!/usr/bin/env bash
# Once, on the Mac, before the first release from CI (docs/RELEASE.md §4):
# makes Steps's release keys where the other scripts keep them (or reuses
# them), then copies them into this repository's Actions secrets so
# .github/workflows/release.yml can build, sign and publish. Run it again
# any time: it reuses every key that exists and only refreshes the secrets.
#
# | Secret                          | From                                            |
# |---------------------------------|-------------------------------------------------|
# | STEPS_UPDATE_SECRET             | Keychain steps-update (just update-keygen)      |
# | STEPS_ANDROID_KEYSTORE          | ~/.steps/steps-release.jks, base64 (just android-keystore) |
# | STEPS_ANDROID_KEYSTORE_PASSWORD | Keychain steps-android                          |
# | STEPS_MAC_CERT_P12              | ~/.steps/steps-codesign.p12, base64 (just mac-codesign-cert) |
# | STEPS_MAC_CERT_PASSWORD         | Keychain steps-codesign                         |
#
# The values go from the Keychain straight into `gh secret set` on stdin:
# never into a file in the repo, an argument list or the terminal.
# Needs gh (logged in with access to the repo), a JDK (keytool) and cargo.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="${STEPS_REPO:-MohamedRef/Steps-app}"

command -v gh >/dev/null || { echo "install the GitHub CLI: brew install gh && gh auth login" >&2; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "log in first: gh auth login" >&2; exit 1; }

if ! security find-generic-password -s steps-update >/dev/null 2>&1; then
  scripts/update-keygen.sh
fi
grep -q '^[A-Za-z0-9+/]\{43\}=$' crates/app/update-key.pub \
  || { echo "crates/app/update-key.pub holds no key, but the Keychain already has steps-update: put its public half back first" >&2; exit 1; }
scripts/android-keystore.sh "$HOME/.steps/steps-release.jks" steps
if [ ! -f "$HOME/.steps/steps-codesign.p12" ]; then
  if security find-identity -v -p codesigning | grep -q '"Steps Code Signing"'; then
    echo "\"Steps Code Signing\" is in the Keychain but ~/.steps/steps-codesign.p12 is missing: export it there (Keychain Access › Export) and store its password as steps-codesign" >&2
    exit 1
  fi
  scripts/mac-codesign-cert.sh
fi

put() { gh secret set "$1" --repo "$repo" >/dev/null && echo "set $1"; }
printf %s "$(security find-generic-password -s steps-update -w)" | put STEPS_UPDATE_SECRET
base64 < "$HOME/.steps/steps-release.jks" | tr -d '\n' | put STEPS_ANDROID_KEYSTORE
printf %s "$(security find-generic-password -s steps-android -w)" | put STEPS_ANDROID_KEYSTORE_PASSWORD
base64 < "$HOME/.steps/steps-codesign.p12" | tr -d '\n' | put STEPS_MAC_CERT_P12
printf %s "$(security find-generic-password -s steps-codesign -w)" | put STEPS_MAC_CERT_PASSWORD
gh secret list --repo "$repo"

echo
echo "Done. Commit crates/app/update-key.pub if it changed:"
grep -v '^#' crates/app/update-key.pub
echo "Back up ~/.steps/ and the Keychain items steps-update, steps-android, steps-codesign together."
