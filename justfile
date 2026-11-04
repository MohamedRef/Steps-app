# Steps task runner. Install `just` first: `brew install just`.

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true

server := "crates/server"

app := "crates/app"
dist := "dist"
keystore := env_var("HOME") / ".steps/steps-release.jks"
key_alias := "steps"

default:
    @just --list

# fmt + clippy (zero warnings) + tests + content checks: run before every commit
check:
    scripts/check-tracked-files.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo run -q -p steps-tools -- validate
    cargo run -q -p steps-tools -- packs build --check

fmt:
    cargo fmt --all

# Check the content files (docs/CONTENT.md)
validate:
    cargo run -q -p steps-tools -- validate

# Authoring (docs/AUTHORING.md): the next free item and lesson ids of a lecture
content-next-id lecture:
    cargo run -q -p steps-tools -- content next-id {{lecture}}

# A ready-to-paste item of a kind with the next free id
content-new-item lecture kind:
    cargo run -q -p steps-tools -- content new-item {{lecture}} {{kind}}

# What a change adds, edits or removes per lecture
content-diff base="origin/main":
    cargo run -q -p steps-tools -- content diff --base {{base}}

# Every check a content change must pass, in one table
content-preflight base="origin/main":
    cargo run -q -p steps-tools -- content preflight --base {{base}}

# Dry run of a content-only update on a channel: every publishing step,
# signed with a throwaway key, nothing uploaded (docs/RELEASE.md)
publish-update-dry-run channel="preview":
    scripts/update-preflight.sh {{channel}}

# Show which changed files can be published without a new APK (base commit or ref)
content-impact base="HEAD^1":
    python3 scripts/content_impact.py --base "{{base}}" --head HEAD

# Test content-only classification independently of Rust
content-impact-test:
    python3 -m unittest discover -s scripts/tests -p 'test_content_impact.py' -v

# One Anki file with every written lecture: dist/anki/steps.apkg (docs/ANKI.md)
anki:
    cargo run -q -p steps-tools -- anki

# Run the desktop app (STEPS_WINDOW_SIZE=412x900 previews the phone layout)
run:
    cargo run -p steps-app

# The web app: builds target/web/steps and serves it on http://localhost:8080 (docs/WEB.md)
web:
    scripts/web/build.sh --dev
    python3 -m http.server 8080 --bind 127.0.0.1 --directory target/web/steps

# The web app, optimised, as Cloudflare serves it (docs/WEB.md)
web-release:
    scripts/web/build.sh

# Android debug APK on a connected device (needs dx 0.7.10, Android SDK + NDK)
android:
    cd crates/app && dx serve --platform android --no-default-features --features mobile

# --- release builds (docs/RELEASE.md; nothing is released before Mohamed says go) ---

# Re-export the launcher and Mac icons from assets/src/icon/*.svg (pip install cairosvg pillow)
icons:
    python3 scripts/assets/export_icons.py

# One-time: create the release signing key (~/.steps, password in the Keychain)
android-keystore:
    scripts/android-keystore.sh "{{keystore}}" "{{key_alias}}"

# The SHA-256 of the content pack this tree builds: a build that carries it
# (STEPS_BUILTIN_CONTENT) doesn't download the same pack again (docs/RELEASE.md)
content-digest:
    @cargo run -q -p steps-tools -- packs build --out {{dist}}/packs > /dev/null
    @shasum -a 256 {{dist}}/packs/steps-content.pack | cut -d' ' -f1

# Build a signed release APK into dist/android/steps.apk
apk:
    rm -rf {{dist}}/android && mkdir -p {{dist}}/android
    export STEPS_BUILTIN_CONTENT="$({{just_executable()}} content-digest)" && cd {{app}} && STEPS_COMMIT="$(git rev-parse --short HEAD)" dx bundle --platform android --release --package-types apk --target aarch64-linux-android --no-default-features --features mobile --out-dir ../../{{dist}}/android/raw
    # dx leaves a debug build; rebuild it as a release (not debuggable, R8, versionCode).
    apk="$(scripts/android-release-apk.sh)" && \
      { [ ! -x scripts/android-check-apk.sh ] || scripts/android-check-apk.sh "$apk"; } && \
      scripts/android-sign.sh "$apk" "{{dist}}/android/steps.apk" "{{keystore}}" "{{key_alias}}"

# List connected Android devices
devices:
    adb devices -l

# Install the signed APK: `just install-android` (one device connected) or `just install-android <serial>`
install-android device="":
    adb {{ if device != "" { "-s " + device } else { "" } }} install -r {{dist}}/android/steps.apk

# Build a release Steps.app for the Mac into dist/mac/Steps.app
bundle-mac:
    rm -rf {{dist}}/mac/raw && export STEPS_BUILTIN_CONTENT="$({{just_executable()}} content-digest)" && cd {{app}} && STEPS_COMMIT="$(git rev-parse --short HEAD)" dx bundle --platform macos --release --package-types macos --out-dir ../../{{dist}}/mac/raw
    app="$(find {{dist}}/mac/raw -maxdepth 4 -name '*.app' | head -1)"; \
      rm -rf {{dist}}/mac/Steps.app && cp -R "$app" {{dist}}/mac/Steps.app
    # The same code identity on every build, once `just mac-codesign-cert` made it.
    if security find-identity -v -p codesigning | grep -q '"Steps Code Signing"'; then \
      codesign --force --deep --sign "Steps Code Signing" {{dist}}/mac/Steps.app && codesign --verify --deep --strict {{dist}}/mac/Steps.app && echo "signed {{dist}}/mac/Steps.app"; \
    else echo "Steps.app is ad-hoc signed; run 'just mac-codesign-cert' once (docs/RELEASE.md)"; fi

# One-time: a free self-signed code-signing certificate for Steps.app
mac-codesign-cert:
    scripts/mac-codesign-cert.sh

# One-time: the update signing key (public half → crates/app/update-key.pub, secret → Keychain)
update-keygen:
    scripts/update-keygen.sh

# Publish `just apk` + `just bundle-mac` as an update, only once Mohamed says go (docs/RELEASE.md §5)
publish-update notes_en="" notes_ar_eg="":
    scripts/update-publish.sh "{{notes_en}}" "{{notes_ar_eg}}"

# --- Steps Cloud (docs/CLOUD.md) ----------------------------------------------

# Install the Worker tools once: wrangler 4 + worker-build
cloud-tools:
    npm install -g wrangler@4
    cargo install worker-build --version ^0.8 --locked

# One-time, after `wrangler login`: token signing key and your Gmail on the allowlist
cloud-setup:
    scripts/cloud-setup.sh

# Run Steps Cloud locally on http://127.0.0.1:8787 (local workerd)
cloud-dev:
    cd {{server}} && wrangler dev

# Deploy to steps-cloud.<your-subdomain>.workers.dev
cloud-deploy:
    cd {{server}} && wrangler deploy

# Stream live logs from the deployed Worker
cloud-logs:
    cd {{server}} && wrangler tail
