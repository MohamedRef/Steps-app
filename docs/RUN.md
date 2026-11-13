# Run Steps

## Mac (desktop)
1. Install Rust (`curl https://sh.rustup.rs -sSf | sh`) and `just` (`brew install just`).
2. Clone the repo: `git clone https://github.com/MohamedRef/Steps-app && cd Steps-app` (Steps's code is on `main`).
3. `just run` opens the app. `STEPS_WINDOW_SIZE=412x900 just run` previews the phone layout. `STEPS_OPEN_SCREEN=study just run` opens a screen directly. `STEPS_OPEN_LECTURE=cvs.L01 just run` opens that lecture's page over the first screen. `STEPS_OPEN_LECTURE=references` opens the References list instead. `STEPS_COURSE=cvs just run` puts one course in view on Lessons, Review and Questions (`all` for every course), and `STEPS_COURSE_SHEET=1` opens the course sheet at launch. `STEPS_OPEN_SETTINGS=look just run` opens Settings on a section (`study`, `reminders`, `sync`, `look`, `tablet`, `mac`, `data`, `updates`, `about`; empty for the list). Theme and interface size (`text_size`) are kept per device in `device-prefs.txt` in the data folder (Settings › Your data shows where).
   - Progress screen previews: `STEPS_OPEN_SCREEN=stats STEPS_STATS_TAB=plan STEPS_PLAN_EXAM=2027-01-20 STEPS_STATS_DEMO=1 just run` opens the exam planner with a preset exam day and demo numbers (labelled as demo on screen). `STEPS_STATS_TAB` is `progress`, `mistakes` or `plan`.

## S24 Ultra and Tab S11 (Android)
1. One-time setup: install the Dioxus CLI 0.7.10 (`cargo install dioxus-cli --version 0.7.10 --locked`), Java 17, and Android Studio's SDK plus NDK r27c. Then run `rustup target add aarch64-linux-android`.
2. Turn on USB debugging on the phone or tablet (Settings › About phone › Software information › tap Build number 7 times, then Developer options › USB debugging), and connect it with a cable.
3. `just android` builds and installs a debug build. The app id is `app.steps`, so it installs next to any other app without touching its data.

### Kotlin glue (`android/`)
`android/` at the repo root is a small Gradle library module with the Kotlin that Rust calls through JNI (screen-off oral mode, vibration, study reminders, the system Back key and bars, sharing text in, saving and opening backups, and installing updates). Nothing to configure: the `#[manganis::ffi("../../android")]` marker in `crates/app/src/platform/background.rs` makes `dx` 0.7.10 copy the folder into its generated Gradle project, add it as an app dependency, and merge `android/src/main/AndroidManifest.xml` (permissions and receivers the glue needs) into the app's manifest.

Study reminders on the Mac come from UNUserNotificationCenter and only work from the bundled Steps.app (`cargo run` / `just run` has no bundle, so Settings › Reminders says reminders need the installed app). macOS asks for permission the first time the Mac is chosen as the device that rings.

`scripts/android-check-apk.sh <apk>` fails if the Kotlin classes or their permissions are missing from an APK; CI runs it on every change to `android/`. When you add a Kotlin class that Rust looks up by name, add it to `CLASSES` in that script. `adb logcat` shows Kotlin errors on the device.

The Mac build (clippy, tests and `dx bundle`) runs in `.github/workflows/mac.yml` nightly and by hand (Actions › Mac › Run workflow), never on pull requests or merges: hosted Mac runners are slow and sometimes unavailable for most of an hour. No PR waits for it. Running `just check` plus `just bundle-mac` on the Mac checks the same things.

Release builds (`just apk`, `just install-android`, `just bundle-mac`), their one-time keys, the CI preview APK and updates over the air are described in [RELEASE.md](RELEASE.md). The permanent signing keys are made once on the Mac with `just release-keys` and backed up with `just release-keys-backup` (RELEASE.md §2).

Releases are on GitHub as `steps-v<version>` ([releases](https://github.com/MohamedRef/Steps-app/releases); the README names the current one).

- **Android:** install `steps-<version>.apk`. It installs over any Steps build signed with the permanent key (preview or release) and keeps the data. Export a backup before installing over an older build.
- **Mac:** unzip `Steps-<version>-mac.zip`, move `Steps.app` to Applications, then run `xattr -dr com.apple.quarantine /Applications/Steps.app` once (the app is self-signed).

After the first install, later versions arrive over the air.

**Alpha** is the default update channel for every build, including preview APKs, unless a channel has already been chosen in Settings. It follows published releases through the `steps-alpha` feed. **Preview** follows the CI builds of `main` on `steps-preview`, built nightly when `main` moved or on demand from Actions › Android preview APK › Run workflow. To receive those builds and their content updates, open الإعدادات › التحديثات and pick «معاينة» (preview); installing a preview APK alone does not select the preview channel. A preview left on alpha follows the alpha feed instead. Settings › التحديثات switches between preview, alpha, beta or stable without reinstalling (RELEASE.md §5).

Each update says what changed (from `changes/*.yaml`, RELEASE.md §5): before installing, once after, and in Settings › التحديثات › سجل التغييرات.

## Web (any browser)
The same app, compiled to WebAssembly (the Crafting Apps way: one Rust codebase for every platform). It is deployed by `.github/workflows/web-deploy.yml` after CI passes on `main`, when the app or the content changed since the last deploy (Steps Cloud's `cloud-deploy.yml` does the same for the hub), at `https://steps-web.hamorefaat85.workers.dev` (open it, then “Add to Home Screen” or “Install app” if you like). After the first visit it opens from the device, and a new version downloads in the background and shows from the next open. Your progress is kept in that browser (IndexedDB), so take a backup (Settings › بياناتك) now and then. Sync with Steps Cloud works in the browser too (Settings › المزامنة) once the Google Web client is set up ([WEB.md](WEB.md) § Sync). Locally: `cargo install wasm-bindgen-cli@<the version in Cargo.lock> --locked` once, then `just web` and open http://localhost:8080. Details in [WEB.md](WEB.md).

## Study in Anki now
One file, `steps/anki/steps.apkg` in the project files folder (HANDBOOK §13), holds every written lecture as a subdeck of `Steps`. Open it with AnkiDroid on the phone or tablet, or with Anki on the Mac. Importing a newer `steps.apkg` updates the cards in place and keeps your progress. `just anki` builds it into `dist/anki/`; details in [ANKI.md](ANKI.md).

## Your data and your content checks
- Reviews, your ✓ / ✎ / ✗ checks and review settings live in one SQLite file on each device: `~/Library/Application Support/app.Steps.Steps/steps.db` on the Mac, and the app's private storage on Android. Settings › مذاكرتك shows the exact path. `STEPS_DATA_DIR=/some/folder just run` uses a separate profile (handy for previews).
- Settings › صدّر مراجعاتك للمحتوى writes `steps-verdicts-YYYY-MM-DD.yaml` (format `steps.verdicts/v1`) to `~/Downloads` on the Mac, or to `Android/data/app.steps/files` on the phone and tablet (open it with the Files app or over USB). The same text is shown on screen, so you can copy it. `steps-tools verdicts FILE` turns it into a fix list per lecture, and `--approve` adds your ✓ to `content/approved.txt`.
- Settings › نسخة احتياطية من كل حاجة writes `steps-export-YYYY-MM-DD.json` (every review, check and setting) to `~/Downloads` on the Mac. On the phone and tablet it is written to the app's folder, then the system "Save to" dialog offers Downloads or Drive. Settings › استرجع نسخة merges such a file back (nothing is deleted; the newer row wins). On Android its button opens the system file picker, so a copy on Drive restores a new phone without any cloud.
- Settings also sets the desired retention (85, 90 or 95%) and new items a day (15, 30, 45 or 60). After 400 reviews the schedule is fitted to your own memory, and it is refitted once a week.

## Counts
- The content numbers in `README.md` ("What is inside") and `docs/HANDBOOK.md` §12 are generated: `just counts` (`steps-tools counts --write`) rewrites the text between `<!-- counts -->` and `<!-- /counts -->` from `content/`. Never edit them by hand. `steps-tools counts` prints them exactly.
- They are rounded ("about"), so a content PR leaves them alone and stays inside its course folders. The Nightly checks run `steps-tools counts --check`, which fails and opens "Nightly: README and handbook counts out of date" once any number is more than 5% away; then run `just counts` in a small PR of its own.

## UWorld QIDs and B&B videos per lecture
Each lecture lists its Boards and Beyond videos with their UWorld question IDs to paste into a custom test. `scripts/qbank-sync.sh` refreshes them from the public doc in one command, and a weekly GitHub workflow opens a PR when they change (close and reopen it to run CI). Details in [QBANK.md](QBANK.md).

## Sync between your devices (Steps Cloud)
Steps works fully offline. To keep the Mac, phone and tablet in step, deploy Steps's own free Cloudflare Worker once and sign in under Settings › المزامنة; the steps are in [CLOUD.md](CLOUD.md). The cloud URL and Google client come from `.env` (copy `.env.example`), which `just` loads at build time. Without them, the Sync section says sync isn't set up and nothing leaves the device.

On a device without your Gmail (a borrowed laptop, a library computer), open the web app and sign in under Settings › المزامنة with the username and password you set from a signed-in device ([CLOUD.md §4.1](CLOUD.md#41-username-and-password)).

## Your own cards
Study › كروتك holds cards you write yourself: «＋ كارت جديد» opens a short form (front, back, and optionally the lecture, book page and where it came from). On the phone and tablet, select text in any app and pick «Steps» from the menu, or share text to Steps: it opens the same form with the text on the back, ready for you to write the question. Your cards study like the book's, as their own deck («ذاكر كروتك بس», or the ✎ chip with the lectures), skip the ✓ / ✎ / ✗ check, sync with Steps Cloud, and go into Anki with `--own` ([ANKI.md](ANKI.md)).

## Tests, CI and merging
How to test, what CI runs and when, and how a PR is merged: [HANDBOOK.md](HANDBOOK.md) §5 to §7. In short, run `just check` before every push; it runs what CI's per-PR job runs.
