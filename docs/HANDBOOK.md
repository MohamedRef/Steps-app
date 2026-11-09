# Steps handbook

This is the one page to read before working on Steps. It says what the app is, how the repo is laid out, how to build, test, release and deploy it, the rules I work by, and exactly where things stand today. Someone who has never seen the project should be able to pick it up from here, using only this repo and my Mac.

The other docs go deeper on each part; this page links to them instead of repeating them.

**Keep this page true.** Every pull request that changes something described here updates it in the same PR, and §12 (state and roadmap) is updated whenever a release ships, an item from the roadmap lands, or a new known issue appears.

## Contents

1. [What Steps is](#1-what-steps-is)
2. [Rules that never bend](#2-rules-that-never-bend)
3. [Repo map and architecture](#3-repo-map-and-architecture)
4. [Build and run on every platform](#4-build-and-run-on-every-platform)
5. [Tests and checks](#5-tests-and-checks)
6. [CI, nightly runs and deploys](#6-ci-nightly-runs-and-deploys)
7. [How a change gets merged](#7-how-a-change-gets-merged)
8. [Content pipeline and schema](#8-content-pipeline-and-schema)
9. [Steps Cloud and Google sign-in](#9-steps-cloud-and-google-sign-in)
10. [Releases, signing and updates](#10-releases-signing-and-updates)
11. [Identity: icon, font, themes and styles](#11-identity-icon-font-themes-and-styles)
12. [Current state, roadmap and known issues](#12-current-state-roadmap-and-known-issues)
13. [Where things live outside the repo](#13-where-things-live-outside-the-repo)

## 1. What Steps is

Steps is my personal USMLE Step 1 study app, for one learner (me). It is private and never goes to a store. Repo: `MohamedRef/Steps-app`, branch `main`.

- **Backbone:** First Aid for the USMLE Step 1 2026. One course per First Aid discipline or organ system (16) plus Rapid Review (`rr`), one lecture per First Aid subsection (84 lectures, e.g. `cvs.L04` = Cardiovascular › Pathology). Pages are the **printed** page numbers (printed = PDF page − 22).
- **What it does:** FSRS spaced repetition, study sessions, Step 1 style single-best-answer vignettes with a `why` for every option, clozes, tables, cases, lessons in Egyptian Arabic with drawings and try-it steps, timed blocks in the 2026 exam format (20 questions / 30 minutes), lab values sheet, go-to-page, exam planner with phases, catch-up and rebalancing, self-assessment score log (NBME/UWSA/Free 120) with readiness, pages-read gate for new cards, confidence and error reasons, a UWorld mistake log by question id with its top reasons and lectures, personal notes, own cards, weak spots, oral mode with the screen off, reminders, Boards & Beyond videos and UWorld question ids per lecture with an index and a custom-test maker (QBANK.md), Anki export, backup and restore, and sync between devices.
- **Devices:** Galaxy S24 Ultra and Tab S11 (Android APK), the Mac (Steps.app), and any browser (web app). One Rust codebase for all of them.
- **Language:** the UI is Egyptian Arabic, right to left. Medical content is English, wrapped in `<bdi>` inside Arabic text. Code, comments, commits and docs are English.
- **Origin:** Steps started on 2026-11-04 as a copy of my internal-medicine app Batna (`MohamedRef/Batna-app`) and is now a separate app with its own identity, keys, cloud and look (§2).

## 2. Rules that never bend

These come from my own decisions. Follow them even when a default tool or habit says otherwise.

**Money and privacy**
- $0 a month. Ask me before adding anything that could cost money.
- No secrets in the repo, ever: not in code, docs, workflow files or commit messages. Only names of variables and secrets are written down.
- No real patient data to any outside service. Tests never call live APIs and never go online.

**Content**
- The textbook never goes in git: no PDF, no extracted text. Cropped book figures for lessons are the one exception. Items paraphrase the book, cite `src.page`, and quote only what proves the fact. `scripts/check-tracked-files.sh` enforces this in CI.
- **There is no maximum length on First Aid quotes.** Never shorten a quote to fit a limit (decided 2026-11-05).
- **Content is never shortened or deduplicated.** First Aid is already condensed to the high-yield facts, so nothing is removed from a lecture, a lesson or a course glossary because it also appears elsewhere. Every course glossary lists every term its lectures use, even if another course lists it too (validate accepts that; it only refuses a term twice in one file).
- Every content item ships `status: unverified`. Only I approve items, through my in-app export (`content/approved.txt`).
- The book's answer is the exam answer. A newer guideline or another source's different view goes in `update:` with its name and year, never as a silent change.
- No generated images. Lesson figures are inline SVG drawings, crops of the book's own figures (`book: true`, cited by page), or open-licence images with credit, licence and source.

**Code and tests**
- **Tests never depend on today's date.** Build dates from "now" (`now - 5 days`) or pass a fixed clock. A test pinned to a date once turned every PR red the next morning. The nightly run moves the clock 37 hours and 400 days forward to catch this.
- **Tests never depend on the size of the content.** A test that grows with the bank (a paper of every MCQ, every item) slows every run as lectures are written. Use a small paper or one topic. Keep each test well under a minute; CI warns about any test over 60 seconds.
- Keep CI fast. A slow run is a bug to fix, not to wait out. Don't add new per-PR jobs without asking me.
- UI uses logical CSS properties only (RTL). Layout follows window width (compact under 600dp, medium under 840dp, expanded from 840dp), never the device name. From 840dp the whole window is the canvas (centred past 100rem); reading screens keep their own line length inside it. From 1100dp an info sheet (book page, abbreviations, labs, go to page) is a full-height side panel at the inline end and the content makes room for it; a lesson turns into two columns when the content area itself is 54rem or wider (a container query on `.content`).
- Review events are append-only; statuses and settings are last-writer-wins; synced tables carry `account_id`, `device_id`, an HLC timestamp and a tombstone.

**Merging**
- Branch from `origin/main`, open PRs against `main`.
- **Merge `origin/main` into the branch to update it. Never rebase, never force-push.**
- **A PR with every check green and no conflicts is squash-merged at once.** Never leave a green PR waiting.
- A blocker that would take long (a red `main`, a big fix) is split into smaller parallel pieces so it's fixed fast.

**Separate from Batna**
- Steps is its own app. Never push to or open PRs on Batna for Steps work, and never depend on Batna crates.
- Steps never shares a key, an update feed, a Google client, a Worker or a secret with Batna or any other app.
- Porting a Batna improvement is fine: copy the module, adapt it, and credit it in a comment ("adapted from batna-app main: <path>"). How: [BATNA-SYNC.md](BATNA-SYNC.md). Batna's content and calculators are never ported.
- The look is Steps' own (§11). Nothing Batna-branded (liver icon, amber, Batna theme or style names) belongs in Steps.

**GitHub hygiene**
- Everything on GitHub reads as written by me: commits, branch names, PR titles and bodies, issues, reviews, docs and code name no tools the work was written with. No "generated with" footers, co-author trailers or session links. CI checks the tracked files and warns on PR text.
- Squash-merge with the PR's own title and body as the commit message, so branch commit trailers never reach `main`.
- No GitHub comments as a workflow: close issues with "Fixes #N" in the PR or a close reason.

**Tone**
- Warm, specific, encouraging. No guilt, no fear, no fake urgency, no dark patterns.

## 3. Repo map and architecture

Rust stable (edition 2024, minimum 1.90), Dioxus 0.7 with HTML/CSS for the UI. A Cargo workspace:

| Path | What it is |
|---|---|
| `crates/core` | Pure logic, no UI and no IO; builds for wasm32. Content schema and validation (`content.rs`, `validate.rs`, `lesson.rs` with `lesson/markup.rs` and `lesson/check.rs`, `glossary.rs`, `quote.rs`), FSRS and scheduling (`srs.rs`, `review/`, `session.rs`), planner (`plan.rs`), timed exam (`exam.rs`), self-assessment (`selfassess.rs`), pages read (`pages.rs`), own lesson highlights (`highlight.rs`), stats, packs (`pack.rs`), update feed (`update.rs`), themes (`theme.rs`). |
| `crates/app` | The app (Dioxus). One module per feature (`today`, `study`, `bank`, `case`, `learn`, `lecture`, `plan`, `stats`, `oral`, `notes`, `own`, `qbank`, `labs`, `settings`, `update`, `cloud`, …). `store/` is SQLite and sync outbox; `platform/` is device glue (Android JNI, Mac, browser); `i18n/` holds all Arabic UI text, one file per feature (`i18n/today.rs`, …) declared in `i18n/mod.rs`; `prefs.rs` and `look.rs` hold appearance; `render_tests/` are the screen tests. |
| `crates/protocol` | Wire types shared by app and server: sync ops and tables (`sync.rs`, `TABLES`), cloud API (`cloud.rs`), merge rules, client state machine. |
| `crates/server` | Steps Cloud, the Cloudflare Worker (Durable Objects, no D1/R2). `wrangler.toml`. |
| `crates/tools` | `steps-tools` CLI: `validate`, `refs`, `packs`, `ids`, `content` (next-id, new-item, diff, preflight), `anki`, `qbank sync`, `update` (changes, guard, publish), `verdicts`. |
| `crates/anki` | Builds the `.apkg`. |
| `content/` | All study content (YAML), §8. |
| `changes/` | One YAML per user-visible change; becomes the in-app "What's new". |
| `crates/app/assets/` | The app's CSS and small scripts (`main.css` root tokens, `parts.css`, `appearance.css`, `shape.css`, `nav.css`, …). |
| `assets/` | Fonts (`assets/fonts/readex-pro`), sounds, icon sources (`assets/src/icon/*.svg`) and exported icons (`assets/export/`). |
| `android/` | Kotlin glue called from Rust through JNI (screen-off oral mode, reminders, back key, sharing, backups, installing updates, Google sign-in return). |
| `web/` | Web shell: `index.html`, service worker `sw.js`, manifest, `wrangler.toml` for the static site. |
| `scripts/` | Build, signing, release, check and sync scripts (`release-secrets.sh`, `android-*.sh`, `update-*.sh`, `test-later-dates.sh`, `qbank-sync.sh`, `check-tracked-files.sh`, `web/build.sh`). |
| `fixtures/` | Sample content (`fixtures/content/sample.yaml` shows every item kind). |
| `.github/workflows/` | CI, nightly, deploys, release (§6). |
| `justfile` | Every common command. `just` with no argument lists them. |
| `vendor/` | Dioxus 0.7.10 `dioxus-desktop` and `dioxus-interpreter-js`, patched (via `[patch.crates-io]` in the root `Cargo.toml`) so the screen keeps updating after Android destroys the WebView's edit socket while the phone is locked. Same patch as Batna. Read `vendor/README.md` before any Dioxus upgrade and drop the folder once upstream (DioxusLabs/dioxus#5812) ships the fix. Test: `cargo test -p dioxus-desktop --lib reconnect` (needs the desktop WebView libraries). |

**File layout rule (2026-11-05):** a file holds one concern, and work that many people change in parallel lives in many small files rather than one shared one, so parallel PRs rarely touch the same lines. A new feature adds its own folder (`crates/app/src/<feature>/`), its own text file (`i18n/<feature>.rs`), its own CSS and its own `changes/<topic>.yaml`, and adds only one declaration line to a shared list. Content is already split per course (§8). Rust modules group a concern rather than one type per file. Unit tests longer than a screen live in a sibling file (`plan.rs` declares `#[cfg(test)] mod tests;` and the tests sit in `plan/tests.rs`), so a test change and a code change rarely meet. Splits are behavior-preserving (same module paths and test names) and land one area per PR.

How the pieces talk: content YAML → `steps-tools packs build` → one signed content pack. Apps carry a built-in pack and download newer ones from the update feed without a new APK when only content changed. Each device keeps its reviews, checks and settings in SQLite (IndexedDB in the browser) and pushes an outbox of ops to Steps Cloud over a WebSocket; other devices get them within seconds. Everything works offline.

More: [UI-MAINTENANCE.md](UI-MAINTENANCE.md) (UI architecture), [COMPATIBILITY.md](COMPATIBILITY.md) (versions, pack and data compatibility), [CONTENT-DELIVERY.md](CONTENT-DELIVERY.md) (pack decisions).

## 4. Build and run on every platform

Full detail: [RUN.md](RUN.md). The short version:

| Platform | One-time setup | Command |
|---|---|---|
| Mac desktop | Rust (`rustup`), `brew install just` | `just run` (`STEPS_WINDOW_SIZE=412x900` previews the phone; `STEPS_OPEN_SCREEN`, `STEPS_OPEN_LECTURE`, `STEPS_OPEN_SETTINGS` open a screen directly; `STEPS_DATA_DIR` uses a separate profile) |
| Android (debug, on a cable) | Dioxus CLI `cargo install dioxus-cli --version 0.7.10 --locked`, Java 17, Android SDK + NDK r27c, `rustup target add aarch64-linux-android` | `just android` (app id `app.steps`) |
| Android release APK | the permanent keys (§10) | `just apk`, then `just install-android <serial>` |
| Mac release app | the keys (§10) | `just bundle-mac` → `dist/mac/Steps.app` |
| Web | `cargo install wasm-bindgen-cli@<version in Cargo.lock> --locked` | `just web` → http://localhost:8080 ([WEB.md](WEB.md)) |
| Steps Cloud locally | `just cloud-tools` | `just cloud-dev` → http://127.0.0.1:8787 |
| Anki file | — | `just anki` → `dist/anki/steps.apkg` ([ANKI.md](ANKI.md)) |

Sync in local builds needs `.env` (copy `.env.example`; git-ignored): `STEPS_CLOUD_URL`, `STEPS_GOOGLE_CLIENT_ID`, `STEPS_GOOGLE_CLIENT_SECRET`. Without it the app runs fully offline and says sync isn't set up.

On Linux, building the desktop app needs `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libsoup-3.0-dev`. Without them, check and test with `--no-default-features` or with `-p steps-core -p steps-tools`.

## 5. Tests and checks

- **`just check`** runs everything CI's per-PR `check` job runs, against `origin/main`: tracked-files check, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the Node and Python tests, `steps-tools validate`, `packs build --check`, `ids check` (ids my records point at must survive) and the What's new gate (`changes/<topic>.yaml`; commit it first). Needs `node` and `python3`. Without `just`, run those lines from the `justfile` by hand.
- **Screen tests** (`crates/app/src/render_tests/`) open the whole app with the real content, tap through it and read each screen. Each starts from empty data and never touches a real profile or the network. Opening a second `App` in the same test keeps the data, which is how a restart is tested (`render_tests/study.rs`, session restore).
- **Browser checks:** `crates/app/tests/screens-browser.cjs` (lesson and study, per PR) and `settings-browser.cjs` (nightly). See [SCREENS-BROWSER-QA.md](SCREENS-BROWSER-QA.md) and [SETTINGS-BROWSER-QA.md](SETTINGS-BROWSER-QA.md).
- **Coverage:** at least 90% of lines, checked nightly. `cargo llvm-cov --workspace --summary-only --ignore-filename-regex '(browser\.rs|browser_fixture\.rs|crates/app/src/platform/)'`.
- **Later dates:** `scripts/test-later-dates.sh` runs every test binary under `faketime` at +37 hours and +400 days.

## 6. CI, nightly runs and deploys

| Workflow | When | What |
|---|---|---|
| `ci.yml` (CI) | every PR and push to `main` | job "fmt, clippy, tests, content" (about 5 to 13 minutes); the app is checked with `--no-default-features` so no WebKitGTK install is needed |
| `screens-browser.yml` | every PR | lesson and study browser check |
| `nightly.yml` (Nightly checks) | nightly + by hand | Android arm64, web wasm32 and Worker compiles; 90% coverage gate (opens an issue when under); tests on later dates |
| `mac.yml` (Mac) | nightly + by hand | Mac clippy, tests and `dx bundle`. Never on PRs: Mac runners are slow |
| `steps-android-preview.yml` | nightly when `main` moved + by hand | signed preview APK on the `steps-preview` prerelease; PRs touching `android/` build it as a check |
| `settings-browser.yml`, `android-webview-regression.yml` | nightly + by hand | Settings browser check; Android WebView renderer-loss tests |
| `daily.yml` | after every CI run (`main` or any pull request) | starts any platform workflow that hasn't run on `main` for 20 hours (QBank sync: a week), and opens "Nightly checks have not run for 36 hours" while Nightly hasn't run on `main` for 36 hours |
| `alerts.yml` | after nightly/deploy runs | opens "<workflow> failed on main" issues and closes them when green again |
| `web-deploy.yml` | after green CI on `main`, when app or content changed | deploys the web app to `https://steps-web.hamorefaat85.workers.dev` |
| `cloud-deploy.yml` | after green CI on `main`, when `crates/server` or `crates/protocol` changed; or by hand | deploys Steps Cloud and smoke-tests sign-in |
| `qbank-sync.yml` | weekly | refreshes B&B videos and UWorld ids; opens a PR when they change ([QBANK.md](QBANK.md)) |
| `release.yml` (Release) | by hand only | §10 |

Things to know:
- A PR waits only on CI and the screens browser check. Everything platform-specific runs nightly.
- A newer push to a branch cancels its older run, on `main` too.
- A run that still says "in progress" after its jobs finished, and answers "run not running" to Cancel, is GitHub recording the end late (5 to 30 minutes). Don't re-run or push for it.
- GitHub creates no `schedule:` events for this repo (#340, #614), even after #552 and #612 (merged by me) moved each cron line and every scheduled workflow was disabled and enabled again on 2026-11-06. So `daily.yml` starts the platform runs after every CI run, without cron; the `schedule:` lines stay and run too if GitHub ever fires them. Before any release, check that Nightly checks, Mac and Android preview have run green on the release commit.

## 7. How a change gets merged

1. `git fetch origin && git checkout -b <short-topic> origin/main`. Name the branch after the topic.
2. Make the change. Content changes stay inside their own course's folders (§8) so parallel work never conflicts; changes to `crates/`, the schema or shared files are kept in their own PRs.
3. Add `changes/<topic>.yaml` (or append to one): one line per change I can see, with `kind` (`new`, `improved`, `fixed`), `ar_eg` (at most 160 characters) and optional `en`. A housekeeping PR adds a file holding `[]`. CI fails without one. Details: [RELEASE.md](RELEASE.md) §5.
4. **Update this handbook** if the change touches anything it describes, and §12 if it changes the state, the roadmap or the known issues.
5. Keep [RUN.md](RUN.md) current when a command or setup step changes.
6. `git merge origin/main` (no rebase), `just check`, push, open the PR against `main`.
7. When CI and the screens check are green and there are no conflicts, squash-merge at once with the GitHub MCP merge tool (`merge_pull_request`, `merge_method: squash`, `expectedHeadSha`), giving `commit_title` = "<PR title> (#N)" and a short `commit_message` you write yourself (what changed, `Fixes #N`). **Never merge with `gh api .../merge` or the `gh` CLI, and never with the plain "merge" method:** those paths add a generated-by line or the branch's co-author trailers to the commit on `main` (#616–#625), and `main`'s history can't be rewritten. CI warns on `main` when a merge commit's message names a tool.
8. If `main` goes red, fixing it comes before anything else.

## 8. Content pipeline and schema

Rules for writing: [CONTENT.md](CONTENT.md) (items), [LESSONS.md](LESSONS.md) (lessons), [SOURCES.md](SOURCES.md) (other sources), [AUTHORING.md](AUTHORING.md) (commands), [CONTENT-IDS.md](CONTENT-IDS.md) (ids).

**Layout**
- `content/program.yaml`: the 16 courses + `rr` (group `general` = First Aid Section II, `systems` = Section III).
- `content/index.yaml`: every lecture with its printed page range and its `chunk` (the workstream, [WORKSTREAMS.md](WORKSTREAMS.md)).
- `content/<course>/LNN.yaml`: the items of one lecture. `content/lessons/<course>/LNN.yaml`: its lessons (`steps.lesson/v1`).
- `content/book-refs/<course>/LNN.tsv`: locked page references (written by `steps-tools refs --write`).
- `content/glossary/<course>.yaml`: the course's terms. `content/glossary.yaml`: shared terms.
- `content/sources.yaml`: sources besides First Aid (`fa2026` is First Aid). An item from another source carries `src.book: <id>` and goes under the same First Aid lecture and topic, never a parallel tree.
- `content/qbank/`: `uworld.yaml` (generated) and `bb-map.yaml` (B&B section → lecture). `content/labs.yaml`: lab values.
- `content/approved.txt`: items I approved. `content/ids-retired.txt`: ids never to reuse.

**Items:** every item has `id` (`cvs.L04.001`, never reused), `kind` (`card`, `cloze`, `mcq`, `enumerate`, `essay`, `case`, `table`, `order`, `calc`), `topic` (a slug of First Aid's own heading, in book order), `src.page`, `src.quote` (the page's own words that prove the fact), `status: unverified`. One fact per card or cloze; lists become `enumerate` (optionally with a `hook`, a تحشيشة), and a lesson step that teaches a list again lists that تحشيشة in `remind:` so it shows under the step with a link back to the lesson that taught it (LESSONS.md rule 13; `pack::SCHEMA` 3); comparisons become `table`; each topic gets at least one Step 1 vignette MCQ (A–E, one best answer, a `why` per option, testing one step past the diagnosis). The book's own emphasis is tagged `high-yield`.

**Pipeline:**
1. Book text per lecture lives outside git (§13).
2. Write items and lessons; `just content-next-id <lecture>` and `just content-new-item <lecture> <kind>` help.
3. `cargo run -p steps-tools -- refs --lecture <id> --write` checks and locks page references against the book text. A reference it can't confirm is read by eye and recorded in the verdicts file outside git.
4. `just content-preflight` runs validate, the pack check and the id check in one table. `just content-diff` shows what changed per lecture; `just content-impact` says whether it ships without a new APK.
5. Merge. The web app redeploys by itself; installed apps get the new content pack from the update feed with the next release or content-only update ([RELEASE.md](RELEASE.md) § Content packs).

**Approvals:** I mark ✓ / ✎ / ✗ in the app, export from Settings (`steps-verdicts-YYYY-MM-DD.yaml`), and `steps-tools verdicts` turns it into a fix list per lecture and adds my ✓ to `content/approved.txt`. Nothing else ever sets an item to approved.

## 9. Steps Cloud and Google sign-in

Full detail: [CLOUD.md](CLOUD.md) and [WEB.md](WEB.md) § Sync.

- **What:** a Rust Cloudflare Worker (`crates/server`) on the free Workers plan. Worker name `steps-cloud`, live at `https://steps-cloud.hamorefaat85.workers.dev`. One Durable Object per account is the sync hub; one per username handles password sign-in. Sign-in with Google (allowlisted to my Gmail) or username + password (Argon2id on the device; the server stores only a salted hash). Steps issues its own 15-minute access and 60-day rotating refresh tokens.
- **Synced tables:** `review_events`, `item_status`, `settings`, `own_cards`, `self_assessments` (`steps_protocol::sync::TABLES`; a new synced table must be added there). Whenever `steps_protocol::sync` changes, deploy the hub before installing the new app.
- **Google Cloud:** project "Steps" (`steps-513803`), OAuth consent in Testing mode with my Gmail as test user. Two OAuth clients: "Steps desktop" (Desktop app type: Mac and Android, PKCE + loopback) and a Web client for the web app. On Android the sign-in page returns through `app.steps://signed-in`.
- **Cloudflare:** my free account; `workers.dev` subdomain `hamorefaat85`.

GitHub Actions settings (Settings › Secrets and variables › Actions). Names only; the values are in GitHub, Cloudflare and Google Cloud, never in the repo:

| Kind | Name | Used for |
|---|---|---|
| Secret | `CLOUDFLARE_API_TOKEN` | deploying Steps Cloud and the web app ("Edit Cloudflare Workers" token) |
| Secret | `STEPS_ALLOWED_EMAILS` | the Gmail allowlist for sign-in |
| Variable | `STEPS_GOOGLE_CLIENT_IDS` | every Google client id the Worker accepts (desktop, then web, comma-separated). The deploy fails without it |
| Variable | `STEPS_GOOGLE_CLIENT_ID` | the Desktop client id built into the APK and Mac app |
| Secret | `STEPS_GOOGLE_CLIENT_SECRET` | the Desktop client's secret (an installed-app secret, non-confidential by Google's rules, but still kept out of git) |
| Variable | `STEPS_WEB_GOOGLE_CLIENT_ID` | the Web client id for the web app |
| Variable | `STEPS_CLOUD_URL` | the Worker URL built into the apps |

The Worker's own secrets (`TOKEN_SIGNING_KEY`, `ALLOWED_EMAILS`) live in Cloudflare; the first deploy makes the signing key. `crates/server/wrangler.toml` keeps `GOOGLE_CLIENT_IDS` empty on purpose; the deploy passes it from the variable, so a dashboard edit would be overwritten.

If sign-in fails: check `STEPS_GOOGLE_CLIENT_IDS`, then run Actions › Deploy Steps Cloud › Run workflow, then `curl https://steps-cloud.hamorefaat85.workers.dev/v1/health`. Android sign-in waiting and retry logic is in `crates/app/src/cloud/oauth.rs` and `engine.rs`; the return activity is `android/src/main/kotlin/app/steps/SignInReturn.kt`. On Android the single-use code goes to Google only while Steps is in front (`SignInBridge.inForeground`): Android can abort a backgrounded app's request after Google took the code, and the retry then gets `invalid_grant` (#629). If that still happens, the app says to sign in again.

## 10. Releases, signing and updates

Full detail: [RELEASE.md](RELEASE.md) and [COMPATIBILITY.md](COMPATIBILITY.md).

**Versions:** `version` in the root `Cargo.toml`. Stages: `1.0.0-alpha.N` → `1.0.0-beta.N` → `1.0.0`; I decide when to move stage. The APK `versionCode` comes from the version and may never go down. Build number = commit count on `main` (`scripts/build-number.sh`). `steps-tools update guard` refuses `-alpha.0` and versions that aren't newer.

**Permanent keys (never regenerate them):**

| Key | Where it lives | GitHub secret / variable |
|---|---|---|
| Android release keystore (RSA 4096, alias `steps`) | my Mac: `~/.steps/steps-release.jks`, password in the Keychain item `steps-android` | `STEPS_ANDROID_KEYSTORE`, `STEPS_ANDROID_KEYSTORE_PASSWORD`; public fingerprint in variable `STEPS_ANDROID_CERT_SHA256` (workflows refuse an APK signed with anything else) |
| Update signing key (Ed25519) | secret half in the Keychain item `steps-update`; public half committed at `crates/app/update-key.pub` | `STEPS_UPDATE_SECRET` |
| Mac code-signing identity "Steps Code Signing" | my Mac's Keychain, also `~/.steps/steps-codesign.p12` (password: Keychain `steps-codesign`) | `STEPS_MAC_CERT_P12`, `STEPS_MAC_CERT_PASSWORD` |

`just release-keys` (`scripts/release-secrets.sh`) made them once and refreshes the GitHub secrets on later runs without changing a key. **Backup:** `just release-keys-backup` made an encrypted `Steps-release-keys.dmg` (keys, Keychain passwords, `RESTORE.txt`); copies are kept in two places and the passphrase is in my password manager. On a new Mac: restore from that image, then run `just release-keys` again. A new keystore would force uninstalling Steps on every device; a new update key would strand every installed build.

**Release day (from CI):**
1. A PR sets the new `version` in `Cargo.toml` with its `changes/` line; merge it.
2. On that commit of `main`: Nightly checks, Mac and Android preview APK are green (start them by hand if needed), and Actions › Release with "Dry run" ticked is green.
3. Actions › Release with "Dry run" unticked. It builds the content pack, the signed APK and Steps.app, and publishes the `steps-v<version>` GitHub release (`steps-<version>.apk`, `Steps-<version>-mac.zip`) plus the signed channel feed (`steps-alpha`, …).
4. Web and Cloud deploy by themselves after green CI on `main`.
5. Update the README's version and install links and §12 here.

A check shows the app's answer as soon as the feed is read; a newer content pack downloads after it (Settings shows «بيجيب الدروس والأسئلة الجديدة…»), then an automatic build download. Every request through `update::agent` (updates, Steps Cloud, Google sign-in) has bounded timeouts (20 s maximum per stalled lookup/connect/read/write, and a tighter total budget for small update-feed files), so a connection Android cut while the phone was locked doesn't hold anything for minutes. When Steps comes back to the front on Android, a check, content pack or build download that failed on the network is tried again at once (`Updater::resumed`); sync does the same with its `Kick`.

**Android update connectivity (2026-11-08):** A phone may reach the GitHub API but fail the usual GitHub Releases download host. The updater first tries the normal release URL; on a network failure it resolves the exact asset name through the public GitHub release API and downloads the binary with `application/octet-stream`. It still verifies the Ed25519-signed manifest, each downloaded file's SHA-256 and size, and the decoded content pack; API metadata never supplies a trusted digest. The next files of the same updater prefer the working route. Content packs now retain verified-identity `content.download.part` and resume with HTTP Range after a cut, like APKs. Content-Length mismatches fail promptly, and a fresh check clears an old content failure from Settings. The phone network itself is not yet verified; keep this as a tested-code change until a device confirms it.

Installed apps update themselves from their channel's feed; Settings › التحديثات switches between preview, alpha, beta and stable without reinstalling. Content-only changes ship as a new content pack without a new APK. The first install of the Mac app needs `xattr -dr com.apple.quarantine /Applications/Steps.app` once (it is self-signed).

## 11. Identity: icon, font, themes and styles

Steps has its own look, chosen by me on 2026-11-05 (#604, #609). Full design system: [DESIGN.md](DESIGN.md).

- **Icon:** a bold petrol **S** that turns like a stair, with a gold summit dot (the option I picked, "F"). Sources: `assets/src/icon/*.svg` and `web/icon.svg`; `just icons` re-exports the PNGs (needs `pip install cairosvg pillow`). The same mark is in the update sheet and the CSS `--mark`.
- **Font:** **Readex Pro** (from Fontsource, bundled in `assets/fonts/readex-pro`, loaded by `crates/app/src/fonts.rs`). It runs about 3% wider than the previous font, so lesson figure text has `letter-spacing: -0.03em` to fit.
- **Shape language:** rising-step minute chips, a hero with a staircase and a gold summit, cards on a solid "ledge", a docked bottom bar with a top marker, the mark before the "Steps" wordmark.
- **Themes (10 + three that follow the device):** صبح (Paper), بترولي (Night), أسود AMOLED (True black), سهر (Night shift, amber, no blue), سكرَب (Scrubs), سيبيا (Sepia), نيلي (Indigo), تباين عالي (High contrast), مذاكرة (Study, calm sage paper) and مذاكرة خافت (Study dim, calm slate), plus زي الجهاز (Paper/Night), زي الجهاز وبالليل أسود (Paper/True black) and مذاكرة زي الجهاز (Study/Study dim). Code: `crates/core/src/theme.rs` (every colour pair is tested for WCAG AA 4.5:1). Theme ids never change. Each theme also has a key-term pill colour (`--key`, `--key-ink`) in its opposite hue, which lessons use for `==key==` ([LESSONS.md](LESSONS.md) § Text by importance); `--mark` is the logo, not this.
- **Styles (10):** درجات (Stepped, the default), خط رفيع, من غير حدود, فواصل, خط جانبي, رمادي فاتح, أبيض وأسود, خطوط, ملوّن, زجاج. Code: `crates/app/src/prefs.rs` (`Appearance`) and `crates/app/assets/appearance.css`. A device preference, never synced.
- **Lesson reading (2026-11-08):** text size (4 steps), line spacing (3) and line width (narrow, book, wide; tablet and Mac only), from «Aa» in the lesson bar or Settings › Look. Device preferences like the theme (`Reading` in `crates/app/src/prefs.rs`, `read_size`/`read_spacing`/`read_width` in the device file); UI in `crates/app/src/learn/reading.rs`; the player carries `data-read-*` and `learn.css` scales the step text with `--ls-scale`, `--ls-space` and the column with `--ls-one`/`--ls-col`. Idea from Kabar's Reader.
- Labels are in `crates/app/src/i18n/` (`THEME_*`, `APPEARANCE_*`). I prefer minimal looks, and alternatives offered as options rather than a single pick.
- Feature stylesheets use tokens only (`--bg`, `--surface`, `--accent`, …), never their own hex colours or copies of shared parts.

## 12. Current state, roadmap and known issues

*Last updated: 2026-11-09.*

### State
- **Released:** `1.0.0-alpha.1` and `1.0.0-alpha.2` on 2026-11-05 (#611, #641), then `1.0.0-alpha.3` on 2026-11-06: APK, Mac app, web and Steps Cloud. Latest release: <https://github.com/MohamedRef/Steps-app/releases/tag/steps-v1.0.0-alpha.3>. Installed alpha builds update to it over the air.
- **Reminders (2026-11-08):** lesson steps that repeat a list show its تحشيشة again (`remind:`, 140 reminders on 121 steps, «من درس …» opens the lesson that taught it), ported from Batna #771. `pack::SCHEMA` is 3, so this ships with an app build, not as a content-only update; older apps keep their built-in content until they update.
- **Content:** all 84 lectures written (16 First Aid courses + Rapid Review), about 9,400 items: about 5,100 cards, 1,700 MCQs, 1,050 clozes, 1,500 tables and lists, and 360 lessons. All `unverified` until I approve them.
- **Quality:** two full review rounds (code, platform, UX, and content per course group) were run on 2026-11-04 and 2026-11-05; every finding of every severity was fixed and closed before alpha.1.
- **Sync:** Steps Cloud is deployed with Google sign-in working (GCP project `steps-513803`) and password sign-in.
- **Only other source so far:** First Aid 2026. B&B videos and UWorld ids per lecture come from a public mapping document (QBANK.md).
- **UWorld index (2026-11-05):** More › أسئلة UWorld is a tree of every course, lecture and B&B video with a copy button on each and a basket that cuts any mix (and pasted ids) into custom tests of up to 40; each lesson opens with its lecture's B&B videos and UWorld ids. It follows the public "Exam Maker Tool" (uworld-tool.streamlit.app) and fixes its gaps (search, counts, done videos, a stable shuffle).

### Roadmap
- **1.0.0-alpha.2 (released 2026-11-05):** First Aid coverage fixes, Android Google sign-in without `invalid_grant` when Chrome is in front (#629), the Today first-launch fixes (#637), the UWorld tree, basket and lesson strip (#638) and the UWorld mistake log (#640).
- **1.0.0-alpha.3 (released 2026-11-06):** the Android screen no longer freezes after lock and unlock, and the update check is faster (#643).
- **1.0.0-alpha.4:** whatever daily use of the alpha turns up. So far: lesson text weighted by importance with a key-term pill per theme, and the calm Study themes (from Batna #767, 2026-11-08); the Mac and landscape-tablet layout (whole-window canvas, two-column lessons, side-panel sheets) and safer keyboard review keys (arrows in lessons, Space/Enter rates Good, Z undo), ideas taken from Batna #739 and #740, and my own highlights in lessons (select words → five colours, fill or underline, pill or rectangle; one synced setting `hl.<lesson>.<id>` each, `steps_core::highlight`, LESSONS.md § Own highlights), ported from Batna #770, and lesson reading settings (text size, line spacing, line width), an idea from Kabar's Reader. Batna's resizable study panes and focus mode (#741) and ⌘K palette (#743), in Steps' own look: a draggable question/answer divider saved per device (`desk_split`), focus mode (F) in lessons and review (`desk_focus`), and «دوّر بسرعة» (Ctrl/⌘+K, `shortcuts/palette.rs`) for screens, tools, lectures, lessons and First Aid pages, and since 2026-11-09 Settings sections and a lecture's questions ("questions <lecture>" or «أسئلة …» opens Questions narrowed to it), from Batna #744. Also from Batna (2026-11-09): after a key reveals, grades, undoes or turns a lesson step, focus moves to the new card or step (`shortcuts/focus.js`, mouse clicks untouched, tested in Chromium by `crates/app/tests/keyboard-focus-browser.cjs` inside the screens browser job), from #745; and on windows 1100px wide and 740px tall or more, Today gives the plan the wide column and the lecture list scrolls on its own beside it, from #742.
- **Research gaps (2026-11-05):** of the three gaps the research reports named, the NBME/UWSA score tracker (Progress › اختباراتي: chart, trend and readiness verdict) and new cards only from pages read (the pages-read gate) were already in the app. The third, a UWorld mistake log by question id, is on Progress › الغلطات: type a QID and the reason (`steps_core::uwmiss`, one synced setting `uw.miss.<qid>` each), see the most common reason with what to change, and the lectures with most misses with their QIDs to copy into a UWorld custom test. Features that copy a commercial question bank stay low priority, since I already have the banks.
- **More sources:** Pathoma, Sketchy, UWorld notes and my own materials merge into the First Aid lectures and topics as they arrive ([SOURCES.md](SOURCES.md)). When the first non-First-Aid source arrives, `steps-tools refs` must learn to read its text.
- **Approvals:** I check items against the book in the app and export my verdicts; fixes follow per lecture.
- **Beta, then stable:** after daily use of the alpha; I decide when.
- **Exam:** Step 1 around September 2027 (date not fixed yet). Since May 2026 the exam is 14 blocks of 30 minutes, up to 20 questions each.
- **Study sessions survive leaving and restarts (2026-11-08, ideas from Batna #753, #755, #756):** Today shows "كمّل الجلسة" first while a Study session is open, and returns to the same item without starting another. After every step the queue is saved on this device only (local setting `study.session.v1`, `steps_core::session::SessionCheckpoint`) with the review log's revision (`Store::study_review_marker`, `v2:` + a random token in `meta` key `study.review_revision.v2` that SQLite triggers replace in the same transaction as every insert, update or delete in `review_events`, local or synced; one row read however long the log grows, 2026-11-09 from Batna #763, about 25 µs instead of 24 ms at 3,000 answers on CI's runner). At the next start it comes back only when that marker and every saved grade still match the log, otherwise it is dropped, so no answer is ever replayed. A checkpoint saved by an older build (`v1:` marker, a fingerprint of the whole log) is checked once with `Store::study_review_marker_legacy` and saved as v2 at the next step. Tested against the real SQLite store (`study/checkpoint.rs`, `store/revision_tests.rs`), by leaving Study and coming back in one process (`study/tests.rs`), and in the browser check, which runs `foreground.js` on a scrolled card and expects the same card, once, at the same place (Batna #754, #757). CI's summary shows what a Study step costs at 100, 1,000 and 3,000 answers (`store::marker_perf`, `store::backend::replay_perf`; informational, no timing gate; Batna #759, #761). Ending a session or finishing it clears the save. Time spent in another app no longer counts as answer time (`study/foreground.rs` + `foreground.js`, tested by `crates/app/tests/study-visibility.test.cjs`). Not yet tried on a real phone after Android kills the app.
- **Smaller files, fewer merge conflicts:** split the busiest shared files one area per PR, with no behavior change (§3 file layout rule). Done: UI text (`i18n.rs` into `i18n/`, one file per feature); unit tests of every crate in sibling files; `lesson.rs` split into schema, `lesson/markup.rs` and `lesson/check.rs`.

### Known issues
- The 2026-11-08 GitHub Releases/API fallback has not yet been verified on a real phone/network. If both GitHub hosts are blocked, Settings still reports a failed update check; test on Wi-Fi and mobile data before a release.
- **GitHub schedules don't fire** (2026-11-06). GitHub has never created a `schedule` run for this repo. `daily.yml` starts the platform runs after every CI run instead (#614 closed), and opens an issue if Nightly hasn't run for 36 hours. Check Actions for runs with event `schedule` now and then.
- A workflow run can stay "in progress" for up to about 30 minutes after it finished. Ignore it (§6).
- The lock/unlock screen-freeze fix (`vendor/`) is tested on desktop sockets only, not yet on a real phone.
- Restoring a Study session after Android ends the app is covered by screen tests (a second app over the same data), not yet checked on a real phone.
- The lowercase clone URL `github.com/mohamedref/steps-app` once served a stale `main`; use `https://github.com/MohamedRef/Steps-app.git`.
- Nightly coverage, later-date tests and platform builds only alert through issues; no alert issue doesn't prove they ran, so look at the runs before a release.

## 13. Where things live outside the repo

| What | Where |
|---|---|
| First Aid 2026 PDF | my Mac: `~/Shared Folder/USMLE/Books`; also my project files folder |
| Book text per lecture (reading order with `=== book page N` markers, and a column-preserving layout copy) | project files: `steps/courses/<course>/text/LNN.txt` and `LNN.layout.txt`; whole pages by PDF number in `steps/book/pNNN.txt`. Regenerate from the PDF with `pdftotext` if lost (printed page = PDF page − 22) |
| Reference verdicts read by eye | project files: `steps/courses/<course>/verdicts/LNN.tsv` |
| Research reports (apps, question banks, study methods) | project files: `steps/research/` |
| Review round briefs and results | project files: `critique/round-2026-11-04/`, `critique/round-2026-11-05/` |
| Batna (the app Steps came from) | `MohamedRef/Batna-app`; on my Mac in `~/Documents/projects apps` |
| Release keys | my Mac (`~/.steps/`, Keychain) and the encrypted backup image (§10) |
| Cloud and sign-in consoles | Cloudflare dashboard (Workers & Pages), Google Cloud Console project "Steps" |

— Mohamed Refaat
