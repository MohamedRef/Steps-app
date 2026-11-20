#!/usr/bin/env bash
# Builds the Android APK with dx, from crates/app, into <out-dir>.
#
# Gradle fetches its plugins and libraries from Maven while it builds, and
# Maven Central sometimes answers "Could not find" for files it has (2026-11-20:
# a release, a pull request and a release dry run in one day). So:
#
# - Gradle also searches Google's mirror of Maven Central (an init script in
#   ~/.gradle/init.d, which the release APK's own Gradle run uses too);
# - when the failure is still Gradle not fetching, this waits and runs Gradle
#   again in the project dx generated, with --refresh-dependencies: Gradle
#   remembers a miss, so a retry without it fails at once, and running dx again
#   would rebuild the Rust library for nothing (it is built by then, and the
#   APK that ships comes from scripts/android-release-apk.sh's own Gradle run).
#
# Any other failure stops at once.
set -uo pipefail

out="${1:?usage: android-bundle.sh <out-dir>}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
waits=(60 180)
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

init="${GRADLE_USER_HOME:-$HOME/.gradle}/init.d"
mkdir -p "$init"
cat >"$init/steps-maven-mirror.gradle" <<'EOF'
// Written by scripts/android-bundle.sh: Google's mirror of Maven Central,
// searched first for everything but Google's own Android libraries, so a
// "not found" from Maven Central doesn't fail the build.
def mirror = 'https://maven-central.storage-download.googleapis.com/maven2/'
def central = { handler ->
    handler.maven {
        url = mirror
        content {
            excludeGroupByRegex 'androidx\\..*'
            excludeGroupByRegex 'com\\.android\\..*'
            excludeGroupByRegex 'com\\.google\\.android\\..*'
        }
    }
}
allprojects {
    central(buildscript.repositories)
    central(repositories)
}
EOF

fetch_failed() {
  grep -qE "Could not resolve all (artifacts|dependencies|files)|Could not (GET|HEAD) 'https://" "$log"
}

dx bundle --platform android --release --package-types apk \
  --target aarch64-linux-android --no-default-features --features mobile \
  --out-dir "$out" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
[ "$status" -eq 0 ] && exit 0
fetch_failed || exit "$status"

project="$(find "$repo/target/dx" -path '*/release/android/app/gradlew' -print -quit 2>/dev/null | xargs -r dirname)"
if [ -z "$project" ]; then
  echo "::error::Gradle couldn't fetch from Maven, and dx left no Gradle project to try again in."
  exit "$status"
fi

for attempt in 1 2; do
  wait="${waits[$((attempt - 1))]}"
  echo "::warning::Gradle couldn't fetch from Maven (attempt $attempt); asking again in ${wait}s."
  sleep "$wait"
  (cd "$project" && ./gradlew --refresh-dependencies assembleDebug) 2>&1 | tee "$log"
  status=${PIPESTATUS[0]}
  [ "$status" -eq 0 ] && exit 0
  fetch_failed || exit "$status"
done
echo "::error::Gradle couldn't fetch from Maven three times."
exit "$status"
