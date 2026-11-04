#!/usr/bin/env bash
# Publishes the current release builds as an over-the-air update
# (docs/RELEASE.md §5): one signed feed per channel the version belongs to
# (an alpha → steps-alpha; a beta → alpha + beta; a release → all three),
# each on its own GitHub release. The content pack rides along, so lectures
# reach devices whose app updates are off.
#
# Prepared, not run: only once Mohamed says go. Needs `gh auth login`, the
# Keychain key from `just update-keygen`, and `just apk` + `just bundle-mac`.
# Usage: scripts/update-publish.sh "<notes en>" "<notes ar-EG>"
# Adapted from batna-app main: scripts/update-publish.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

notes_en="${1:-}"
notes_ar_eg="${2:-}"
repo="${STEPS_UPDATE_REPO:-MohamedRef/Steps-app}"
v="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
apk="dist/android/steps.apk"
app="dist/mac/Steps.app"
pack="dist/packs/steps-content.pack"
tools="cargo run -q --release -p steps-tools --"

[[ -f "$apk" ]] || { echo "no $apk: run 'just apk' first" >&2; exit 1; }
[[ -d "$app" ]] || { echo "no $app: run 'just bundle-mac' first" >&2; exit 1; }
command -v gh >/dev/null || { echo "install the GitHub CLI: brew install gh && gh auth login" >&2; exit 1; }
grep -q '^[A-Za-z0-9+/]\{43\}=$' crates/app/update-key.pub || { echo "no update key yet: run 'just update-keygen' (and rebuild)" >&2; exit 1; }
secret="$(security find-generic-password -s steps-update -w)" || { echo "no update key: run 'just update-keygen'" >&2; exit 1; }

case "$(uname -m)" in
  arm64) mac=macos-arm64 ;;
  x86_64) mac=macos-x86_64 ;;
  *) echo "unknown Mac architecture $(uname -m)" >&2; exit 1 ;;
esac
# ditto keeps the bundle's symlinks, modes and signature; the app unzips with ditto too.
rm -f dist/mac/Steps.app.zip
ditto -c -k --keepParent "$app" dist/mac/Steps.app.zip
# The pack reaches phones without an install: check the content first (#332).
$tools validate
$tools packs build --out dist/packs

# The tool says which channels this version goes on, and fails loudly on a
# version it can't read, so a broken build never publishes nowhere quietly.
channels="$($tools update channels --version "$v")"
for channel in $channels; do
  out="dist/update/$channel"
  # What's new: changes/ since the version this channel carries now, on top
  # of its history (none yet on a first publish).
  previous="$(mktemp)"
  gh release download "steps-$channel" --repo "$repo" --pattern update.json --output "$previous" --clobber \
    2>/dev/null || rm -f "$previous"
  $tools update manifest --channel "$channel" --version "$v" --commit "$(git rev-parse --short HEAD)" \
    --notes-en "$notes_en" --notes-ar-eg "$notes_ar_eg" --previous "$previous" --changes \
    --asset "android-arm64=$apk" --asset "$mac=dist/mac/Steps.app.zip" --asset "content=$pack" \
    --out "$out"
  STEPS_UPDATE_SECRET="$secret" $tools update sign "$out"
  $tools update verify "$out"
  tag="steps-$channel"
  if ! gh release view "$tag" --repo "$repo" >/dev/null 2>&1; then
    gh release create "$tag" --repo "$repo" --prerelease --latest=false --title "Steps updates ($channel)" \
      --target main \
      --notes "Steps's $channel update feed. The app reads update.json, checks its signature and each file's SHA-256. Published with 'just publish-update'; never edit by hand."
  fi
  # Files first, the signed manifest last: a device that reads the new
  # manifest always finds the files it names.
  shopt -s nullglob
  gh release upload "$tag" "$out"/steps-* "$out"/Steps-* --repo "$repo" --clobber
  gh release upload "$tag" "$out/update.json.sig" "$out/update.json" --repo "$repo" --clobber
  echo "published $v on $channel: https://github.com/$repo/releases/tag/$tag"
done
unset secret
