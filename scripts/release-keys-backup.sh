#!/usr/bin/env bash
# Once, on the Mac, right after scripts/release-secrets.sh (docs/RELEASE.md §2):
# puts every permanent Steps release key into one encrypted disk image, so a
# lost or replaced Mac never means uninstalling Steps or losing his records.
#
# Steps-release-keys.dmg (AES-256, the passphrase you type twice) holds:
# - steps-release.jks and steps-codesign.p12 (from ~/.steps/);
# - passwords.txt: the Keychain items steps-android, steps-codesign and
#   steps-update (the update key's secret half);
# - update-key.pub and RESTORE.txt (how to put them back).
#
# Keep the image in two places (e.g. iCloud Drive and a USB stick) and the
# passphrase in your password manager. Never put it in the repo.
# Usage: scripts/release-keys-backup.sh [destination folder, default ~/Desktop]
set -euo pipefail
cd "$(dirname "$0")/.."
dest="${1:-$HOME/Desktop}"
out="$dest/Steps-release-keys.dmg"

[ -d "$dest" ] || { echo "No folder $dest" >&2; exit 1; }
[ ! -e "$out" ] || { echo "$out already exists: move it away first" >&2; exit 1; }
for f in steps-release.jks steps-codesign.p12; do
  [ -f "$HOME/.steps/$f" ] || { echo "~/.steps/$f is missing: run just release-keys first" >&2; exit 1; }
done
for s in steps-android steps-codesign steps-update; do
  security find-generic-password -s "$s" >/dev/null 2>&1 \
    || { echo "The Keychain has no $s: run just release-keys first" >&2; exit 1; }
done

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
chmod 700 "$stage"
mkdir "$stage/Steps release keys"
dir="$stage/Steps release keys"
cp "$HOME/.steps/steps-release.jks" "$HOME/.steps/steps-codesign.p12" "$dir/"
cp crates/app/update-key.pub "$dir/"
{
  echo "steps-android=$(security find-generic-password -s steps-android -w)"
  echo "steps-codesign=$(security find-generic-password -s steps-codesign -w)"
  echo "steps-update=$(security find-generic-password -s steps-update -w)"
} >"$dir/passwords.txt"
cat >"$dir/RESTORE.txt" <<'TXT'
Restoring Steps's release keys on a new Mac:

1. mkdir -p ~/.steps && cp steps-release.jks steps-codesign.p12 ~/.steps/ && chmod 600 ~/.steps/*
2. For each line name=value in passwords.txt, run (and paste the value when asked):
     security add-generic-password -U -s <name> -a "$USER" -w
3. security import ~/.steps/steps-codesign.p12 -k ~/Library/Keychains/login.keychain-db \
     -T /usr/bin/codesign -P "<the steps-codesign value>"
   then in Keychain Access set "Steps Code Signing" to Always Trust for code signing.
4. In the repo: scripts/release-secrets.sh (it reuses everything and refreshes the GitHub secrets).
These keys are permanent. Never make new ones: installed Steps builds only trust these.
TXT

echo "Choose a passphrase for the backup (it goes in your password manager):"
read -r -s -p "Passphrase: " p1; echo
read -r -s -p "Again: " p2; echo
[ -n "$p1" ] && [ "$p1" = "$p2" ] || { echo "The passphrases differ or are empty" >&2; exit 1; }
printf %s "$p1" | hdiutil create -quiet -encryption AES-256 -stdinpass -fs APFS \
  -volname "Steps release keys" -srcfolder "$dir" "$out"
unset p1 p2
echo "Wrote $out."
echo "Copy it to a second place (iCloud Drive or a USB stick) and keep the passphrase in your password manager."
