#!/usr/bin/env bash
# Fails loudly if an APK lacks Steps's Kotlin glue from android/, i.e. if dx
# did not pick up the `#[manganis::ffi]` module (crates/app/src/platform/background.rs).
# Adapted from batna-app main: scripts/android-check-apk.sh.
#
# Usage: scripts/android-check-apk.sh path/to/steps.apk
# When a chunk adds a Kotlin class that Rust calls by name, add it to CLASSES.
set -euo pipefail
APK="${1:-}"
CLASSES=(app/steps/Haptics app/steps/OralService app/steps/OralBridge app/steps/ReminderBridge app/steps/ReminderReceiver app/steps/UpdateInstaller app/steps/UpdateInstallReceiver app/steps/Documents app/steps/DocumentActivity app/steps/ShareActivity app/steps/PendingShare app/steps/UpdateNotice app/steps/UpdateCheckJob app/steps/UpdateTapActivity app/steps/SignInReturnActivity)

# Failures become annotations on GitHub, where job logs are hard to reach.
fail() {
  for line in "$@"; do
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error::$line"; else echo "$line"; fi
  done
  exit 1
}

[ -f "$APK" ] || fail "No APK at '${APK}'"

# Read everything first: `producer | grep -q` under pipefail fails whenever
# grep stops early and the producer gets SIGPIPE.
dex="$(mktemp)"
trap 'rm -f "$dex"' EXIT
unzip -p "$APK" 'classes*.dex' >"$dex"
missing=()
for cls in "${CLASSES[@]}"; do
  LC_ALL=C grep -aqF "L$cls;" "$dex" || missing+=("$cls")
done
if [ "${#missing[@]}" -ne 0 ]; then
  fail "Missing from the APK: ${missing[*]}" "dx did not build android/ into the APK. See docs/RUN.md, 'Kotlin glue'."
fi
echo "Kotlin glue present: ${CLASSES[*]}"

# Permissions the glue declares in android/src/main/AndroidManifest.xml must
# survive the manifest merger. aapt2 ships with the Android build-tools; skip
# quietly where it isn't on PATH.
if command -v aapt2 >/dev/null; then
  manifest="$(aapt2 dump xmltree --file AndroidManifest.xml "$APK")"
  for perm in VIBRATE POST_NOTIFICATIONS RECEIVE_BOOT_COMPLETED REQUEST_INSTALL_PACKAGES; do
    grep -q "android.permission.$perm" <<<"$manifest" || fail "The merged manifest lost $perm (android/src/main/AndroidManifest.xml)."
  done
  grep -q "app.steps.ReminderReceiver" <<<"$manifest" || fail "The merged manifest lost ReminderReceiver (android/src/main/AndroidManifest.xml)."
  grep -q "app.steps.ShareActivity" <<<"$manifest" || fail "The merged manifest lost ShareActivity (android/src/main/AndroidManifest.xml)."
  grep -q "app.steps.DocumentActivity" <<<"$manifest" || fail "The merged manifest lost DocumentActivity (android/src/main/AndroidManifest.xml)."
  grep -q "app.steps.SignInReturnActivity" <<<"$manifest" || fail "The merged manifest lost SignInReturnActivity (android/src/main/AndroidManifest.xml)."
  grep -q "app.steps.UpdateCheckJob" <<<"$manifest" || fail "The merged manifest lost UpdateCheckJob (android/src/main/AndroidManifest.xml)."
  echo "Merged manifest: glue permissions present"
fi

# The S24 Ultra and Tab S11 are arm64.
unzip -Z1 "$APK" | grep -q '^lib/arm64-v8a/' || fail "The APK has no arm64-v8a library."
echo "Native library: arm64-v8a present"

# The background update check (UpdateNotice.kt) calls into Rust by its JNI
# name: the library must export it, or the job quietly does nothing.
if command -v readelf >/dev/null; then
  libdir="$(mktemp -d)"
  unzip -q -o "$APK" 'lib/arm64-v8a/*.so' -d "$libdir"
  syms="$(for so in "$libdir"/lib/arm64-v8a/*.so; do readelf --dyn-syms -W "$so"; done)"
  rm -rf "$libdir"
  for export in Java_app_steps_UpdateCheckJob_check Java_app_steps_UpdateCheckJob_shown; do
    grep -q "$export" <<<"$syms" \
      || fail "The native library doesn't export $export: the background update check can't reach Rust."
  done
  echo "Native exports for the background update check present"
fi
