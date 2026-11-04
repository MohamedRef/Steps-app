#!/usr/bin/env bash
# Dry run of a content-only update (docs/RELEASE.md, "Publishing
# preflight"): everything publishing does, nothing it uploads. Runs on any
# machine with no secrets; signs with a throwaway key, never the release or
# preview key.
#
#   scripts/update-preflight.sh [channel]     (default: preview)
#
# 1. validate the source content            5. manifest (APK carried, not rebuilt)
# 2. build the pack (device checks)          6. sign (throwaway key) and verify
# 3. ids against the pack on the feed        7. report: sizes, digests, impact
# 4. what changed against the feed
#
# Exit status is non-zero when any step fails. Output: dist/preflight/.
set -euo pipefail
cd "$(dirname "$0")/.."

channel="${1:-preview}"
case "$channel" in preview | alpha | beta | stable) ;; *) echo "unknown channel $channel" >&2; exit 2 ;; esac
repo="${STEPS_UPDATE_REPO:-MohamedRef/Steps-app}"
tag="steps-$channel"
tools="cargo run -q -p steps-tools --"
out=dist/preflight
rm -rf "$out" && mkdir -p "$out/live" "$out/feed"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
step() { printf '\n== %s\n' "$*"; }

step "1. Source content"
$tools validate

step "2. Pack (the checks devices run before taking it)"
$tools packs build --out "$out/pack"

step "3–4. Against what the $channel feed publishes now"
live_manifest="$out/live/update.json"
base=()
carry=()
if curl -fsSL --retry 3 -o "$live_manifest" "https://github.com/$repo/releases/download/$tag/update.json"; then
  live_pack="$(jq -r '.assets[] | select(.platform == "content") | .file' "$live_manifest")"
  if [[ -n $live_pack ]] && curl -fsSL --retry 3 -o "$out/live/$live_pack" \
      "https://github.com/$repo/releases/download/$tag/$live_pack"; then
    base=(--base-pack "$out/live/$live_pack")
  fi
  # A content-only update keeps every build the feed has: no APK is made.
  while read -r p; do carry+=(--carry "$p"); done < <(jq -r '.assets[] | select(.platform != "content") | .platform' "$live_manifest")
else
  echo "no $channel feed yet: nothing to compare with"
  rm -f "$live_manifest"
fi
((${#base[@]})) || { echo "no published pack: ids and impact compare with origin/main instead"; base=(--base origin/main); }
$tools ids check "${base[@]}"
head=()
[[ ${base[0]} == --base-pack ]] && head=(--head-pack "$out/pack/steps-content.pack")
$tools content diff "${base[@]}" "${head[@]}" | tee "$out/impact.md"

step "5. Manifest (content-only: builds carried from the live feed)"
previous=()
[[ -f $live_manifest ]] && previous=(--previous "$live_manifest")
build="$( [[ -f $live_manifest ]] && jq -r '.build + 1' "$live_manifest" || echo 1)"
$tools update manifest --channel "$channel" --version "$version" --build "$build" \
  --commit "$(git rev-parse --short HEAD)" "${previous[@]}" --changes "${carry[@]}" \
  --asset "content=$out/pack/steps-content.pack" --out "$out/feed"

step "6. Sign with a throwaway key, verify like a device"
secret="$($tools update keygen --public-out "$out/throwaway.pub" --force 2>/dev/null)"
STEPS_UPDATE_SECRET="$secret" $tools update sign "$out/feed" --public-key "$out/throwaway.pub"
unset secret
carried=()
for c in "${carry[@]}"; do [[ $c == --carry ]] || carried+=(--carried "$c"); done
$tools update verify "$out/feed" --public-key "$out/throwaway.pub" "${carried[@]}"

step "7. Report"
jq -r '"| Field | Value |\n|---|---|",
  "| channel | \(.channel) |", "| version | \(.version) build \(.build) |", "| ui_build (stylesheets) | \(.ui_build) |",
  (.assets[] | "| \(.platform) | \(.file), \(.size) bytes, sha256 \(.sha256[0:16])…\(if .platform != "content" then " (carried, no APK built)" else "" end) |")' \
  "$out/feed/update.json" | tee "$out/report.md"
echo
echo "Dry run passed. Nothing was uploaded; the real feed is published by CI (preview) or 'just publish-update'."
