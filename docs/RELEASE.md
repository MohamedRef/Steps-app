# Steps releases

Steps is for one learner, on his own devices: the Galaxy S24 Ultra, the Galaxy
Tab S11 and the Mac. A release is a signed APK sideloaded onto both Galaxy
devices, plus a Steps.app built on the Mac. There are no store listings and no
paid developer accounts. Everything here costs $0.

No release yet. The first **alpha** will be `1.0.0-alpha.1`, when Mohamed
says go. Releases are built and published by the **Release** workflow
(`.github/workflows/release.yml`, run by hand; §4). The CI *preview* APKs
(§3) keep coming on every merge.

Steps has its own keys, its own feed and its own package (`app.steps`).
It never shares a key, a feed or a client with Batna or Kabar.

## 1. Stages

| Stage | Version | When |
|---|---|---|
| **Alpha** (first) | `1.0.0-alpha.N` | All workstreams in docs/WORKSTREAMS.md are merged, the critic rounds are clean, and Mohamed says go. |
| **Beta** | `1.0.0-beta.N` | Later, after daily use of the alpha. Mohamed decides. |
| **Stable** | `1.0.0` | Later still. Mohamed decides. |

The version is `version` in the root `Cargo.toml`. The APK's `versionCode` is
derived from it (`scripts/android-release-apk.sh`). An alpha always sorts
before its beta, and a beta before the release, so each stage installs over
the previous one and keeps your data.

Until the first alpha, the version stays `1.0.0-alpha.0` (versionCode
10000100). The first alpha is `1.0.0-alpha.1` (10000101), then
`1.0.0-alpha.2`…, `1.0.0-beta.1` (10000501)…, and `1.0.0` (10000999). Every
step sorts above the one before, and every build is signed with the same
permanent keys (§2), so each installs over the last and keeps the data: no
uninstall between alpha, beta and stable. The versionCode may never go
down: the preview workflow refuses to publish a build below the live
feed's. Only move to a bare version (`1.0.0`) for a real stable release.

**The keys are permanent** (Mohamed, 2026-11-04). They are made once, before
the first alpha, by `scripts/release-secrets.sh` (§4), and never replaced:
the Android keystore decides which APKs install over Steps, and the update
key decides which feeds it trusts. Back them up the day they are made.

## 2. One-time setup on the Mac

### The permanent keys (once, before the first alpha)

Mohamed makes the keys himself on his Mac. Nothing here costs money, and no
key or password ever goes into the repo, a file in it, or the chat.

1. Tools, once: `brew install just gh` and `brew install --cask temurin`
   (it brings `keytool`), Rust (`rustup`), then `gh auth login` (GitHub.com,
   HTTPS, log in with the browser).
2. In the repo on `main`: `git pull`, then **`just release-keys`**. It runs
   `scripts/release-secrets.sh`, which makes each key once and reuses it on
   every later run:
   - the Android release keystore `~/.steps/steps-release.jks` (RSA 4096,
     alias `steps`, valid about 27 years), its password in the Keychain
     (`steps-android`);
   - the update key (Ed25519): its public half in `crates/app/update-key.pub`,
     its secret half only in the Keychain (`steps-update`);
   - the "Steps Code Signing" identity for Steps.app, also kept as
     `~/.steps/steps-codesign.p12` (password: Keychain `steps-codesign`).

   It then fills the repo's Actions secrets with `gh secret set`
   (`STEPS_UPDATE_SECRET`, `STEPS_ANDROID_KEYSTORE`,
   `STEPS_ANDROID_KEYSTORE_PASSWORD`, `STEPS_MAC_CERT_P12`,
   `STEPS_MAC_CERT_PASSWORD`) and the repo variable
   `STEPS_ANDROID_CERT_SHA256` (the certificate's public fingerprint: both
   Android workflows refuse to publish an APK signed with any other key).
   macOS may ask for the login password to trust the code-signing
   certificate.
3. The script prints the new public update key (one line ending in `=`).
   Paste that line in the Steps chat, and it lands in a PR that commits
   `crates/app/update-key.pub` (it is public). Until that PR merges, the
   "Android preview APK" job on `main` fails on purpose with "holds no key
   yet".
4. **`just release-keys-backup`** (or `just release-keys-backup /Volumes/USB`):
   one encrypted `Steps-release-keys.dmg` with both key files, the three
   Keychain passwords, the public key and `RESTORE.txt`. Pick a passphrase
   and keep it in your password manager. Copy the image to a second place
   (iCloud Drive and a USB stick).

From the next merge on, every preview APK on `steps-preview` is signed with
the permanent key, so it installs over any later alpha, beta or stable
build and the other way round, with the data kept.

**Never make the keys again.** Android installs an update only when it is
signed with the same key, so a new keystore would mean uninstalling Steps;
a new update key would strand every installed build. On a new Mac, restore
from the backup (`RESTORE.txt`) and run `just release-keys` again: it
reuses everything and only refreshes the GitHub secrets.
`just android-keystore` on its own only checks that the keystore and its
Keychain password still match, and prints the certificate's SHA-256.

### Other one-time steps

| Step | Command | Notes |
|---|---|---|
| Tools | `brew install just`, Rust, `cargo install dioxus-cli --version 0.7.10`, Android SDK + NDK | Same as `docs/RUN.md` |
| Icons | `just icons` (only after editing `assets/src/icon/*.svg`) | Needs `pip install cairosvg pillow`. The exported PNGs are committed. |

## 3. Builds

| What | Command | Output |
|---|---|---|
| Signed release APK | `just apk` | `dist/android/steps.apk` (arm64, not debuggable, R8, versionCode from the version) |
| Install on a device | `just devices`, then `just install-android <serial>` | Installs over the previous Steps signed with the same key |
| Mac app | `just bundle-mac` | `dist/mac/Steps.app`, signed "Steps Code Signing" once that exists |
| CI preview APK | Every push to `main` (`.github/workflows/steps-android-preview.yml`) | `steps-preview.apk` on the `steps-preview` prerelease, signed with the permanent release keys, so it installs over a release build and the other way round. Until the keys exist, main only proves the APK builds and publishes nothing. It updates itself from its feed once Settings › Updates picks preview (§5). |

Preview and release APKs carry the same keys, so either installs over the
other and keeps the data.

## 4. Release day (only once Mohamed says go)

### From CI (the usual way)

1. Once, on the Mac: the permanent keys (§2: `just release-keys`, the
   public update key committed, `just release-keys-backup`).
2. Set `version` in `Cargo.toml` in a PR (with its `changes/` line) and merge it.
3. Actions › Release › Run workflow. It builds the content pack, the
   release APK (Linux) and Steps.app (macOS runner), signs them with the
   keys above, bakes in Steps Cloud and the Google client (sync), and
   publishes:
   - `steps-v<version>`: `steps-<version>.apk` and `Steps-<version>-mac.zip`
     for installing by hand;
   - the channel feed(s) (`steps-alpha`, …): the signed `update.json`
     that installed release builds read (§5).
4. The first time only, install by hand: install the APK on both Galaxy
   devices (over any preview build signed with the permanent key: same
   keys; a preview from before the keys existed was signed with a one-off
   key and must be uninstalled first), and on the Mac
   move Steps.app to Applications and run
   `xattr -dr com.apple.quarantine /Applications/Steps.app` once (it is
   self-signed). Every later release reaches them over the air.

### By hand on the Mac (fallback)

1. Set `version` in `Cargo.toml` (`1.0.0-alpha.1` the first time).
2. Run `just check`, then `just apk` and `just bundle-mac`.
3. Install by hand the first time. Copy `Steps.app` to Applications, and run
   `just install-android <serial>` for the phone and the tablet.
4. Walk the device checks: study a session on each device, check that
   reminders ring on one device only, and check that export and import
   round-trip.
5. Tag it, e.g. `git tag steps-v1.0.0-alpha.1`.
6. From the second alpha on, `just publish-update` sends it over the air
   instead of step 3 (§5).

Before installing anything, **Settings → Your data → Export everything** is a
cheap backup.

## 5. Updates over the air

Steps has its own key, its own feeds, and one asset besides the builds: the
content pack.

### The feed

Each channel is one GitHub release of this repository (`MohamedRef/Steps-app`). Downloading from a
public release needs no token and costs nothing.

| Channel | Release tag | Who follows it |
|---|---|---|
| Preview | `steps-preview` | Every push to `main`, signed with the release keys, for whoever picks it in Settings › Updates. |
| Alpha | `steps-alpha` | Every build by default (Mohamed, 2026-11-04), from the first alpha on. |
| Beta / Stable | `steps-beta`, `steps-stable` | Later. A beta also reaches alpha followers; a release reaches everyone. |

A feed holds `update.json`, `update.json.sig` (Ed25519 over the exact bytes),
and the files the manifest names: the APK, the zipped Steps.app, and
`steps-content-….pack`. The types and checks are `steps_core::update`.

What a device checks before it uses anything:

1. The signature, against the public key built into the app
   (`crates/app/update-key.pub` for release builds; previews carry the CI
   preview key). Nothing in an unsigned or changed manifest is read.
2. The channel inside the manifest is the one asked for, the format is one
   this build understands, and every file name is a plain name (no paths,
   no URLs).
3. The version and build are newer than what is installed.
4. Each download's size and SHA-256 match the signed manifest. The file is
   hashed again just before it is installed.

Size caps: 256 KB for `update.json` and its signature, 400 MB for a build.

A build download that stops (the line drops, Steps is closed) keeps its
`<file>.part`; the next try, a few seconds later or at the next check or tap,
asks only for the rest with an HTTP `Range` request (GitHub's release CDN
serves ranges). The whole file is still hashed at the end, and a resumed
file that doesn't match is fetched once more from the start. Content packs
are a few MB and download whole.

### Content packs

A new content pack is offered whenever its SHA-256 differs from the pack in
use, even when the app build itself is unchanged, and even with app updates
off. A pack from an older build than the one installed is never taken, and
the app decodes it with `steps_core::pack` before switching to it, so a pack
this build can't read is never used.

**Pack integrity.** `Pack::problems` (`crates/core/src/pack.rs`) checks
what a device must not open: index and lecture schema tags, lecture ids
listed once and only lectures the index lists, item and lesson ids used
once and under their own lecture, lesson `practice` and `try` items and
`topics` that exist in the pack, and the program's own rules. It runs in
memory with no YAML, so it is cheap: `steps-tools packs build` refuses to
build a pack with problems, the app refuses (and remembers) a downloaded
pack with problems, and a pack already on disk or a last-good backup with
problems is dropped at start in favour of the built-in content. Authoring
rules that need the YAML (approvals) stay in `steps-tools
validate`.

A pack also carries the app's stylesheets (`Pack::look`, every `.css` file
under `crates/app`), so a change to the look needs no install either. The
manifest's `ui_build` names the app build they were made with, and only a
running build with exactly that number uses them (`crates/app/src/look.rs`);
any other build takes the lectures and keeps its own look. Strings, icons,
layout and logic are code and still come with an APK.

**Publishing preflight.** `just publish-update-dry-run [channel]`
(`scripts/update-preflight.sh`, default `preview`) runs every publishing
step for a content-only update without uploading anything, on any machine
and with no secrets: validate the source; build the pack (`Pack::problems`
and the round-trip); download the channel's live `update.json` and pack,
run `ids check` against that pack and write the per-lecture diff
(`dist/preflight/impact.md`); write the manifest carrying the live feed's
builds (so no APK is made); sign it with a throwaway key; `update verify`
it like a device (signature, sizes, SHA-256, then the pack checks); and
print each file's size and digest (`dist/preflight/report.md`). The
preview workflow runs the same id check against the live preview pack
before it signs and puts the diff in the job summary. `update verify` now
also decodes the content pack and runs `Pack::problems`, so a feed devices
would refuse fails before upload.

**Content impact preflight.** To see whether a change needs a new APK,
run `just content-impact BASE` (for example, `just content-impact HEAD^1`).
Pull-request CI writes the same classification and affected paths to the
GitHub Actions job summary; `just content-impact-test` tests the rules.
The preview publisher uses `scripts/content_impact.py --check-content-only`
instead of its own separate path filter. Any unknown path, including
`content/glossary.yaml` and `content/glossary/` (compiled into the app), forces a full
build. Content-only eligibility never bypasses validation, signing, or feed
verification. Changing these rules requires adjusting the classifier tests.

**Content-only feeds.** When everything merged since the build on the
preview feed is under `content/index.yaml`, `content/program.yaml`, a course
folder such as `content/cvs/`, `content/lessons/`,
`changes/`, `docs/`, a top-level `.md` or a `crates/app/**/*.css` file, CI
builds no APK: `steps-tools update manifest --carry android-arm64` keeps the
feed's APK entry as it is (same file, its own `build`), and only the pack is
new; `steps-tools validate` runs before the pack is built, and
`update verify --carried android-arm64` checks the feed without the APK file,
which stays on the release. Apps compare an asset's own `build`, so nothing is offered for install;
the pack is downloaded in the background and switched to live once Mohamed
is between sessions (the calm moments a quiet install waits for). The sheet
doesn't open by itself for it: a quiet card on Today names the first change
and opens "What's new" (titled "تحديث محتوى") on a tap. Settings and the
sheet show the app build and the content apart ("بناء 40 · المحتوى من 45").
Open screens and the review catalog follow the new pack. A pack that
downloads whole but fails its checks, won't decode, or carries a stylesheet
that reaches outside the app (`@import`, a remote `url(…)`, script) is
refused and remembered in `content.unreadable.json` with the app build that
refused it, so it isn't fetched again until the app changes; a cut or
mismatched download is not remembered and the next check tries again. A
feed naming the pack in use downloads nothing, and one naming the last good
pack takes it from disk. The preview workflow builds the pack before the APK and bakes its
digest into the app (`STEPS_BUILTIN_CONTENT`; `just apk` and `just bundle-mac` do
the same through `just content-digest`), so a phone that has just
installed that APK doesn't download the same content again. A pack that is opened three times in a row without
Steps getting past its first screen is rejected for good (`content.rejected`)
and the last settled, verified content pack is restored if compatible; otherwise the built-in content comes back (`crates/app/src/update/recovery.rs`). The last-good pack is saved only after a calm screen has run on it, never immediately after download. Backup files are content-addressed and the backup pointer is published last so interrupted writes preserve the older backup. The glossary is built into the app, so a glossary change is
a full build.

### Keys

| Key | Where it lives | Made by |
|---|---|---|
| Release update key (secret) | Mac Keychain, service `steps-update` | `just update-keygen`, once. The public half is committed as `crates/app/update-key.pub`. |
| Preview update key | GitHub Actions cache of the preview workflow | The workflow, on first run. Never the release key. |

`crates/app/update-key.pub` holds no key until `just update-keygen` runs, so
until then a release build trusts no feed at all. Back the secret up with the
Android keystore: losing it means every installed build has to be updated by
hand once.

### Publishing (prepared, not run)

Only once Mohamed says go:

1. `just apk` and `just bundle-mac` (§3).
2. `just publish-update "notes in English" "ملاحظات بالمصري"`.

`scripts/update-publish.sh` zips Steps.app with `ditto`, builds the content
pack, writes one manifest per channel the version belongs to, signs it with
the Keychain key, verifies it with `steps-tools update verify`, and uploads
the files first and the signed manifest last, so a device never reads a
manifest whose files aren't there yet.

By hand, the same steps are:

```sh
cargo run -q -p steps-tools -- update manifest --channel alpha \
  --asset android-arm64=dist/android/steps.apk \
  --asset macos-arm64=dist/mac/Steps.app.zip \
  --asset content=dist/packs/steps-content.pack --out dist/update/alpha
STEPS_UPDATE_SECRET="$(security find-generic-password -s steps-update -w)" \
  cargo run -q -p steps-tools -- update sign dist/update/alpha
cargo run -q -p steps-tools -- update verify dist/update/alpha
```

### On the device

`crates/app/src/update/` checks the feed 20 seconds after launch and then
every 6 hours, in the background. **Settings › التحديثات** opens with what is
on this device: the app (version, CI build number, commit) and, apart from
it, the lessons, questions and look (the content-only build in use and when
it was published, or "came with the app"), marked "جديد عليك" until its
"What's new" is seen; each row opens the changes. Then one "دوّر على تحديث"
looks for both at once and says what it found for each half (the app's
phase, and for the content: up to date, new and waiting for a calm moment,
or not reachable), with when it last looked. Then two per-device choices:

| Choice | Options |
|---|---|
| How it updates | **لوحده** (automatic, the default): downloads on Wi-Fi only and installs at a calm moment. **قولّي الأول** (tell me first): says so, and downloads and installs on a tap. **لأ** (off): only on "دوّر على تحديث". |
| Channel | Preview, alpha, beta or stable, on any build (every build starts on alpha, Mohamed 2026-11-04; all are signed with the same permanent keys, so switching never needs a reinstall). Preview brings every merge; on the Mac it brings content only (no Mac preview build). Going back from preview to alpha offers the next alpha version. |

When something is ready, a card above the current screen says so, with the
first two lines of what's new (below) and "إيه الجديد؟". "بعدين" hides it
until the next version.

**"A new Steps is out" notification.** On Android a JobScheduler job
(`UpdateCheckJob` in `android/…/UpdateNotice.kt`) runs about every four hours,
even while Steps is closed and across reboots. It loads Steps's native
library and calls Rust's `update::background_check` over JNI, so the feed is
checked with the same signature and hash checks, and in automatic mode on
Wi-Fi the build is downloaded too. The app's own six-hourly checks send the
notification as well, when Steps is in the background. There is one per
version and step (out, then downloaded and ready), with the first three lines
of what's new and a **نزّلها** or **ثبّتها** button. A tap opens the "what's
new" sheet and the button starts the download or the install. A tap while
Steps is already open is picked up within two seconds (the tap's activity is
translucent, so the page may never hide), and a tap left over a minute is
dropped. The job marks a notification as sent only after Android showed it
(`UpdateCheckJob.shown`), so one Android refused goes out at the next run
(Steps-app#165). Nothing is sent while Steps is in front
(the card covers that), for a version whose card was closed, in "لأ" mode, with
Settings › Updates' switch off, or when Android's notification switch is off.
The Mac gets the same notification once UNUserNotificationCenter lands
(Steps-app#50).

**What's new.** Every pull request adds or extends
`changes/<topic>.yaml` (CI fails one that adds no line: `steps-tools update
changes --gate HEAD^1` compares the merge commit with its own base, and a
comment or whitespace edit doesn't count), one line per change, in Egyptian Arabic (`en`
optional), each `new`, `improved` or `fixed`:

```yaml
- kind: fixed
  ar_eg: المؤقت في الامتحان التجريبي مبقاش بيقف
  en: The mock timer no longer stops
```

A file holding `[]` says there's nothing to tell (CI, refactors, docs). When
a feed is built (CI for `steps-preview`, `just publish-update` for releases),
`steps-tools update manifest --previous <current update.json> --changes`
gathers the files added since the commit the feed carries now, one pull
request at a time, plus the lines a pull request appended to an existing
file (a fragment that is moved or renamed keeps its old lines old). A commit with no file (pushed straight to main, or merged before the
check) is looked up by title in `changes/retro/titles.yaml`, which also
rewrites the English-only lines older feeds carry; failing that it falls
back to its title in English, and titles starting with CI, docs, refactor,
test and the like are skipped. That build's entry goes on top of the feed's `history` (40
builds, inside the signed `update.json`, so nothing extra to download or
trust), and the notes older apps read are its first three lines.
`steps-tools update changes
[--since <commit>]` previews what the next build will say.

The app keeps every history it verifies (`updates/history.json`) and shows
it in one sheet, laid out like Morphe Manager's update dialog (ideas only,
none of its code): the installed build → the new one, how many builds that
is and a count per kind, then a timeline with the newest build open and a
card per kind ("اعرض الكل" past six lines). On a phone it fills the screen under Steps's icon; GitHub (the channel's releases page) and the download or install button stay pinned at the bottom. It opens from the banner's "إيه
الجديد؟", from the notification, and from Settings › التحديثات (also "سجل
التغييرات": every build so far); downloading and installing happen in it
too. At the first launch after an update it opens once by itself, listing
everything since the build that ran before.

| Device | How the new build goes in |
|---|---|
| Android | `android/…/UpdateInstaller.kt` checks the APK (same package, same signing key, a versionCode no lower than the installed one) and hands it to PackageInstaller. Android asks once for "Install unknown apps", then shows its own confirmation. On Android 12+, once Steps installed itself, automatic mode updates quietly while Steps is in the background, and never during a study session, bank block, case or oral round. When notifications are allowed, one says the new version is in. |
| Mac | The verified `Steps.app.zip` is unzipped with `ditto` next to the running app, its code signature is checked against the running one ("Steps Code Signing"), and the two are swapped with renames, so a failure leaves the old app in place. In automatic mode it goes in at the next launch; otherwise "ثبّتها وافتح تاني". |
| Content pack | Kept in `<data>/updates/content.pack`, and switched to live once he is between sessions (else at the next start). No install. |

Automatic downloads wait while Android says the network is metered (mobile
data or Data Saver). Study data never moves: it stays in the data folder
across every update.

Trying it without GitHub (development builds only): build `steps-tools update
manifest` and `sign` into a folder with a test key, serve it with
`python3 -m http.server`, and run a debug build with
`STEPS_UPDATE_URL=http://127.0.0.1:8000 STEPS_UPDATE_KEY=<public key>
STEPS_UPDATE_PLATFORM=android-arm64 STEPS_BUILD=1`.
