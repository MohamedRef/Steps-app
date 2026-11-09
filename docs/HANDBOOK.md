# Steps handbook

The one page to read before working on Steps: what the app is, the rules I work by, how the repo is laid out, how to build, test, merge, release and deploy it, and where things stand today. Someone who has never seen the project can pick it up from here with only this repo and my Mac. The other docs go deeper; this page links to them rather than repeating them.

**Keep this page true.** A pull request that changes something described here updates it in the same PR. §12 (state, roadmap, known issues) changes whenever a release ships, a roadmap item lands or a known issue appears or goes, and its date changes with it.

## Contents

1. [What Steps is](#1-what-steps-is)
2. [Rules that never bend](#2-rules-that-never-bend)
3. [Repo map and architecture](#3-repo-map-and-architecture)
4. [Build and run](#4-build-and-run)
5. [Tests and checks](#5-tests-and-checks)
6. [CI, platform runs and deploys](#6-ci-platform-runs-and-deploys)
7. [How a change gets merged](#7-how-a-change-gets-merged)
8. [Content](#8-content)
9. [Steps Cloud and Google sign-in](#9-steps-cloud-and-google-sign-in)
10. [Releases, signing and updates](#10-releases-signing-and-updates)
11. [Look: icon, font, themes and styles](#11-look-icon-font-themes-and-styles)
12. [State, roadmap and known issues](#12-state-roadmap-and-known-issues)
13. [Where things live outside the repo](#13-where-things-live-outside-the-repo)
14. [All docs](#14-all-docs)

## 1. What Steps is

My personal USMLE Step 1 study app, for one learner (me). Private, never in a store. Repo `MohamedRef/Steps-app`, branch `main`; clone with `https://github.com/MohamedRef/Steps-app.git` (the all-lowercase URL once served a stale `main`).

- **Backbone:** First Aid for the USMLE Step 1 2026. One course per First Aid discipline or organ system (16) plus Rapid Review (`rr`); one lecture per First Aid subsection, 84 in all (`cvs.L04` = Cardiovascular › Pathology). Pages are the **printed** numbers (printed = PDF page − 22).
- **Study:** FSRS spaced repetition and study sessions that survive leaving the app, cards, clozes, lists, tables, cases, Step 1 style single-best-answer vignettes with a `why` for every option, lessons in Egyptian Arabic with drawings, try-it steps, importance-weighted text, mnemonic reminders and my own highlights.
- **Exam tools:** timed blocks in the 2026 format (20 questions / 30 minutes), lab values, go to a book page, an exam planner with phases, catch-up and rebalancing, a self-assessment log (NBME, UWSA, Free 120) with a readiness verdict, new cards only from pages read, confidence and error reasons, weak spots, Boards & Beyond videos and UWorld question ids per lecture with a custom-test basket, and a UWorld mistake log by question id.
- **Also:** personal notes, my own cards, oral mode with the screen off, reminders, quick search (Ctrl/⌘+K), Anki export, backup and restore, and sync between devices.
- **Devices:** Galaxy S24 Ultra and Tab S11 (Android APK), the Mac (Steps.app) and any browser (web app), from one Rust codebase.
- **Language:** the UI is Egyptian Arabic, right to left. Medical content is English, wrapped in `<bdi>` inside Arabic text. Code, comments, commits and docs are English.
- **Origin:** Steps began on 2026-11-04 as a copy of Batna (`MohamedRef/Batna-app`), my internal-medicine app. It is now a separate app with its own identity, keys and cloud (§2).

## 2. Rules that never bend

These are my own decisions. They win over any default tool or habit.

**Money and privacy**
- $0 a month. Ask me before adding anything that could cost money.
- No secrets in the repo, ever: not in code, docs, workflows or commit messages. Only the names of variables and secrets are written down.
- No real patient data to any outside service. Tests never call live APIs or go online.

**Content**
- The textbook never goes in git: no PDF, no extracted text. Cropped book figures for lessons are the one exception. Items paraphrase the book, cite `src.page`, and quote what proves the fact. CI enforces this (`scripts/check-tracked-files.sh`).
- **No maximum length on First Aid quotes.** Never shorten a quote to fit a limit.
- **Content is never shortened or deduplicated.** First Aid is already condensed, so nothing leaves a lecture, a lesson or a course glossary because it also appears elsewhere. Each course glossary lists every term its lectures use, even when another course lists it too (validate refuses a term only when it appears twice in the same file).
- Every item ships `status: unverified`. Only I approve, through my in-app export (`content/approved.txt`).
- The book's answer is the exam answer. A newer guideline or another source's view goes in `update:` with its name and year, never as a silent change.
- No generated images. Lesson figures are inline SVG, crops of the book's figures (`book: true`, cited by page), or open-licence images with credit, licence and source.

**Code and tests**
- **Tests never depend on today's date.** Build dates from now (`now - 5 days`) or pass a fixed clock.
- **Tests never depend on the size of the content.** Use a small paper or one topic; keep each test well under a minute.
- Keep CI fast: a slow run is a bug to fix. Don't add per-PR jobs without asking me.
- One concern per file. A feature gets its own folder (`crates/app/src/<feature>/`), its own UI text file (`i18n/<feature>.rs`), its own CSS and its own `changes/<topic>.yaml`, and adds one declaration line to a shared list. Unit tests longer than a screen live in a sibling file (`plan.rs` → `plan/tests.rs`). This keeps parallel PRs off the same lines.
- UI uses logical CSS properties only (RTL). Layout follows window width (compact under 600dp, medium under 840dp, expanded from 840dp), never the device name. Touch targets are at least 48dp (`--target`).
- Review events are append-only; statuses and settings are last-writer-wins; synced tables carry `account_id`, `device_id`, an HLC timestamp and a tombstone.

**Merging**
- Branch from `origin/main`; PRs go to `main`.
- **Update a branch by merging `origin/main` into it. Never rebase, never force-push.**
- **A PR with every check green and no conflicts is squash-merged at once.** A green PR never waits.
- A red `main` comes first. A fix that would take long is split into smaller pieces worked in parallel.

**Separate from Batna**
- Never push to or open PRs on Batna for Steps work, and never depend on Batna crates.
- Never share a key, an update feed, a Google client, a Worker or a secret with Batna or any other app.
- Porting a Batna improvement is fine: adapt it to Steps' look and credit it in a comment ("adapted from batna-app main: <path>"). Batna's content and calculators are never ported ([BATNA-SYNC.md](BATNA-SYNC.md)).

**GitHub**
- Everything on GitHub reads as written by me. Commits, branch names, PR titles and bodies, issues, reviews, docs and code never name the tools the work was written with: no generated-by footers, co-author trailers or session links. CI fails on tracked files that do and warns on PR text and merge messages.
- No GitHub comments as a workflow. Issues close through "Fixes #N" in a PR, or a close reason.

**Tone**
- Warm, specific, encouraging. No guilt, no fear, no fake urgency, no dark patterns.

## 3. Repo map and architecture

Rust stable (edition 2024, at least 1.90) and Dioxus 0.7 with HTML/CSS. A Cargo workspace:

| Path | What it is |
|---|---|
| `crates/core` | Pure logic, no UI or IO, builds for wasm32: content schema and validation (`content.rs`, `validate.rs`, `lesson.rs` + `lesson/`, `glossary.rs`, `quote.rs`), FSRS and sessions (`srs.rs`, `review/`, `session.rs`), planner (`plan.rs`), timed exam (`exam.rs`), self-assessment (`selfassess.rs`), pages read (`pages.rs`), highlights (`highlight.rs`), UWorld misses (`uwmiss.rs`), question-bank tests (`qbank.rs`), packs (`pack.rs`), update feed (`update.rs`), themes (`theme.rs`). |
| `crates/app` | The app: one folder per feature (`today`, `study`, `bank`, `case`, `learn`, `lecture`, `plan`, `stats`, `oral`, `notes`, `own`, `qbank`, `labs`, `shortcuts`, `settings`, `update`, `cloud`, …). `store/` is SQLite plus the sync outbox; `platform/` is device glue (Android JNI, Mac, browser); `i18n/` holds all Arabic UI text, one file per feature; `prefs.rs` and `look.rs` hold appearance; `render_tests/` are the screen tests; `assets/` holds its CSS and small scripts; `tests/` the Node and browser checks. |
| `crates/protocol` | Types shared by app and server: sync ops and `TABLES` (`sync.rs`), the cloud API (`cloud.rs`), merge rules, the client state machine. |
| `crates/server` | Steps Cloud, the Cloudflare Worker (Durable Objects, no D1 or R2), with `wrangler.toml`. |
| `crates/tools` | The `steps-tools` CLI: `validate`, `refs`, `packs`, `ids`, `content` (next-id, new-item, diff, preflight), `anki`, `qbank sync`, `update` (changes, guard, publish), `verdicts`. |
| `crates/anki` | Builds the `.apkg`. |
| `content/` | All study content as YAML (§8). |
| `changes/` | One YAML per user-visible change; becomes the in-app "What's new". |
| `assets/` | Fonts (`fonts/readex-pro`), sounds, icon sources (`src/icon/*.svg`) and exported icons (`export/`). |
| `android/` | Kotlin glue called from Rust through JNI: oral mode with the screen off, reminders, Back key and bars, sharing, backups, installing updates, the Google sign-in return. |
| `web/` | Web shell: `index.html`, service worker `sw.js`, manifest, `wrangler.toml` for the static site. |
| `vendor/` | Patched Dioxus 0.7.10 `dioxus-desktop` and `dioxus-interpreter-js` (`[patch.crates-io]` in the root `Cargo.toml`), so the Android screen keeps updating after the phone is locked. Read `vendor/README.md` before any Dioxus upgrade; drop the folder once upstream ships the fix (DioxusLabs/dioxus#5812). |
| `scripts/` | Build, signing, release, check and sync scripts. |
| `fixtures/` | Sample content: `fixtures/content/sample.yaml` shows every item kind. |
| `.github/workflows/` | CI, platform runs, deploys and release (§6). |
| `justfile` | Every common command; `just` alone lists them. |

How the pieces talk: content YAML → `steps-tools packs build` → one signed content pack. Each app carries a built-in pack and downloads newer ones from the update feed, so content-only changes need no new APK. Each device keeps reviews, checks and settings in SQLite (IndexedDB in the browser) and pushes an outbox of ops to Steps Cloud over a WebSocket; other devices receive them within seconds. Everything works offline.

## 4. Build and run

Full detail: [RUN.md](RUN.md).

| Platform | One-time setup | Command |
|---|---|---|
| Mac desktop | Rust (`rustup`), `brew install just` | `just run` (`STEPS_WINDOW_SIZE=412x900` previews the phone; `STEPS_OPEN_SCREEN`, `STEPS_OPEN_LECTURE`, `STEPS_OPEN_SETTINGS` open a screen; `STEPS_DATA_DIR` uses another profile) |
| Android debug, on a cable | `cargo install dioxus-cli --version 0.7.10 --locked`, Java 17, Android SDK + NDK r27c, `rustup target add aarch64-linux-android` | `just android` (app id `app.steps`) |
| Android release APK | the keys (§10) | `just apk`, then `just install-android <serial>` |
| Mac release app | the keys (§10) | `just bundle-mac` → `dist/mac/Steps.app` |
| Web | `cargo install wasm-bindgen-cli@<version in Cargo.lock> --locked` | `just web` → http://localhost:8080 ([WEB.md](WEB.md)) |
| Steps Cloud locally | `just cloud-tools` | `just cloud-dev` → http://127.0.0.1:8787 |
| Anki file | — | `just anki` → `dist/anki/steps.apkg` ([ANKI.md](ANKI.md)) |

Sync in a local build needs `.env` (copy `.env.example`; git-ignored) with `STEPS_CLOUD_URL`, `STEPS_GOOGLE_CLIENT_ID` and `STEPS_GOOGLE_CLIENT_SECRET`. Without it the app runs offline and says sync isn't set up.

On Linux the desktop app needs `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libsoup-3.0-dev`. Without them, check with `--no-default-features`, or test `-p steps-core -p steps-tools`.

## 5. Tests and checks

- **`just check`** runs everything CI's per-PR job runs, against `origin/main`: the tracked-files check, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the Node and Python tests, `steps-tools validate`, `packs build --check`, `ids check` (ids my records point at must survive) and the What's new gate (commit `changes/<topic>.yaml` first). It needs `node` and `python3`. Without `just`, run the recipe's lines from the `justfile` by hand.
- **Screen tests** (`crates/app/src/render_tests/`) open the whole app with the real content, tap through it and read each screen. Each starts from empty data and never touches a real profile or the network. A second `App` over the same data tests a restart.
- **Browser checks** (Chromium): `crates/app/tests/screens-browser.cjs` (lessons and study, with the keyboard-focus and highlight checks, per PR) and `settings-browser.cjs` (daily). See [SCREENS-BROWSER-QA.md](SCREENS-BROWSER-QA.md) and [SETTINGS-BROWSER-QA.md](SETTINGS-BROWSER-QA.md).
- **Coverage:** at least 90% of lines, checked daily: `cargo llvm-cov --workspace --summary-only --ignore-filename-regex '(browser\.rs|browser_fixture\.rs|crates/app/src/platform/)'` (`cargo install cargo-llvm-cov` once).
- **Later dates:** `scripts/test-later-dates.sh` runs every test binary under `faketime` at +37 hours and +400 days.
- CI warns about any test that runs over a minute, and its run summary shows what a Study step costs at 100, 1,000 and 3,000 answers (informational, no timing gate).

## 6. CI, platform runs and deploys

| Workflow | When | What |
|---|---|---|
| `ci.yml` (CI) | every PR and push to `main` | "fmt, clippy, tests, content", about 5 to 13 minutes; the app is checked with `--no-default-features`, so no WebKitGTK install |
| `screens-browser.yml` | every PR | lesson and study browser check |
| `nightly.yml` (Nightly checks) | daily (see below) + by hand | Android arm64, web wasm32 and Worker compiles; the 90% coverage gate; tests on later dates |
| `mac.yml` (Mac) | daily + by hand | Mac clippy, tests and `dx bundle`; never on PRs, Mac runners are slow |
| `steps-android-preview.yml` | daily when `main` moved + by hand | signed preview APK on the `steps-preview` prerelease; PRs touching `android/` build it as a check |
| `settings-browser.yml`, `android-webview-regression.yml` | daily + by hand | Settings browser check; Android WebView renderer-loss tests |
| `daily.yml` | after every CI run | starts each platform workflow above that hasn't run on `main` for 20 hours (QBank sync: a week), and opens "Nightly checks have not run for 36 hours" when that happens |
| `alerts.yml` | after platform and deploy runs | opens "<workflow> failed on main" and closes it when the next run passes |
| `web-deploy.yml` | after green CI on `main`, when app or content changed | deploys the web app to `https://steps-web.hamorefaat85.workers.dev` |
| `cloud-deploy.yml` | after green CI on `main`, when `crates/server` or `crates/protocol` changed; or by hand | deploys Steps Cloud and checks sign-in answers |
| `qbank-sync.yml` | weekly, through `daily.yml` | refreshes B&B videos and UWorld ids; opens a PR when they change ([QBANK.md](QBANK.md)) |
| `release.yml` (Release) | by hand only | §10 |

- A PR waits only on CI and the screens check.
- A newer push to a branch cancels its older run, on `main` too.
- GitHub has never fired this repo's `schedule:` triggers, so `daily.yml` starts the platform runs after CI instead. The `schedule:` lines stay in case GitHub ever fires them.
- A run that still says "in progress" after its jobs finished, and answers "run not running" to Cancel, is GitHub recording the end late (5 to 30 minutes). Don't re-run or push for it.
- No alert issue doesn't prove a platform run happened. Before a release, look at the runs themselves.

## 7. How a change gets merged

1. `git fetch origin && git checkout -b <short-topic> origin/main`, named after the topic.
2. Make the change. Content stays inside its own course's folders (§8). Changes to `crates/`, the schema or shared files go in their own PRs.
3. Add `changes/<topic>.yaml` (or append to one): one line per change I can see, with `kind` (`new`, `improved`, `fixed`), `ar_eg` (at most 160 characters) and optional `en`. A housekeeping PR adds a file holding `[]`. CI fails without one ([RELEASE.md](RELEASE.md) §5).
4. Update this handbook if the change touches anything it describes (§12 for state, roadmap or known issues), and [RUN.md](RUN.md) if a command or setup step changes.
5. `git merge origin/main` (no rebase), `just check`, push, open the PR against `main`.
6. When CI and the screens check are green with no conflicts, **squash-merge at once**, pinned to the PR's head commit, with the title "<PR title> (#N)" and a short message written by hand (what changed, `Fixes #N`). Never use the plain "merge" method, and never accept a default message: some merge tools append a generated-by line or the branch's co-author trailers, and `main`'s history is never rewritten. CI warns on `main` if a merge message names a tool.

## 8. Content

How to write it: [CONTENT.md](CONTENT.md) (items), [LESSONS.md](LESSONS.md) (lessons), [SOURCES.md](SOURCES.md) (other sources), [AUTHORING.md](AUTHORING.md) (commands), [CONTENT-IDS.md](CONTENT-IDS.md) (ids), [WORKSTREAMS.md](WORKSTREAMS.md) (who owns which folders).

**Files**
- `content/program.yaml`: the 16 courses + `rr` (group `general` = First Aid Section II, `systems` = Section III).
- `content/index.yaml`: every lecture with its printed page range and its workstream (`chunk`).
- `content/<course>/LNN.yaml`: a lecture's items. `content/lessons/<course>/LNN.yaml`: its lessons (`steps.lesson/v1`).
- `content/book-refs/<course>/LNN.tsv`: locked page references.
- `content/glossary/<course>.yaml`: the course's terms. `content/glossary.yaml`: shared terms.
- `content/sources.yaml`: sources besides First Aid (`fa2026`). An item from another source carries `src.book: <id>` and goes under the same First Aid lecture and topic, never a parallel tree.
- `content/qbank/`: `uworld.yaml` (generated) and `bb-map.yaml` (B&B section → lecture). `content/labs.yaml`: lab values.
- `content/approved.txt`: items I approved. `content/ids-retired.txt`: ids never reused.

**Items:** each has `id` (`cvs.L04.001`, never reused), `kind` (`card`, `cloze`, `mcq`, `enumerate`, `essay`, `case`, `table`, `order`, `calc`), `topic` (a slug of First Aid's own heading, in book order), `src.page`, `src.quote` and `status: unverified`. One fact per card or cloze; lists become `enumerate`, optionally with a `hook` (a تحشيشة); comparisons become `table`; each topic gets at least one Step 1 vignette MCQ (A–E, one best answer, a `why` per option, one step past the diagnosis). The book's emphasis is tagged `high-yield`. A lesson step that teaches a list again lists its تحشيشة under `remind:`.

**Pipeline**
1. The book text per lecture lives outside git (§13).
2. Write items and lessons; `just content-next-id <lecture>` and `just content-new-item <lecture> <kind>` help.
3. `cargo run -p steps-tools -- refs --lecture <id> --write` checks and locks page references against the book text. A reference it can't confirm is read by eye and recorded in the verdicts file outside git.
4. `just content-preflight` runs validate, the pack check and the id check in one table; `just content-diff` shows the change per lecture; `just content-impact` says whether it ships without a new APK.
5. Merge. The web app redeploys by itself; installed apps get the new pack with the next release or content-only update. A change to `pack::SCHEMA` (now 3) ships only with a new app build.

**Approvals:** I mark ✓ / ✎ / ✗ in the app and export from Settings (`steps-verdicts-YYYY-MM-DD.yaml`); `steps-tools verdicts FILE` turns it into a fix list per lecture, and `--approve` adds my ✓ to `content/approved.txt`. Nothing else ever approves an item.

## 9. Steps Cloud and Google sign-in

Full detail: [CLOUD.md](CLOUD.md) and [WEB.md](WEB.md) § Sync.

- **What:** a Rust Cloudflare Worker (`crates/server`) on the free plan, named `steps-cloud`, at `https://steps-cloud.hamorefaat85.workers.dev`. One Durable Object per account is the sync hub; one per username handles password sign-in. Sign-in is Google (only my Gmail) or a username and password (stretched with Argon2id on the device; the server stores only a salted hash). Steps issues its own 15-minute access tokens and 60-day rotating refresh tokens.
- **Synced tables:** `review_events`, `item_status`, `settings`, `own_cards`, `self_assessments` (`steps_protocol::sync::TABLES`; a new table must be added there). Newer per-feature data (highlights, UWorld misses, question-bank tests) are synced settings. When `steps_protocol::sync` changes, deploy the hub before installing the new app.
- **Google Cloud:** project "Steps" (`steps-513803`), consent screen in Testing mode with my Gmail as test user. Two OAuth clients: "Steps desktop" (Desktop app type, used by the Mac and Android with PKCE and a loopback redirect) and a Web client for the web app. On Android the sign-in page returns to the app through `app.steps://signed-in`, and the code is exchanged only while Steps is in front.
- **Cloudflare:** my free account, `workers.dev` subdomain `hamorefaat85`.

GitHub Actions settings (Settings › Secrets and variables › Actions). Names only; values live in GitHub, Cloudflare and Google Cloud, never in the repo:

| Kind | Name | Used for |
|---|---|---|
| Secret | `CLOUDFLARE_API_TOKEN` | deploying Steps Cloud and the web app ("Edit Cloudflare Workers" token) |
| Secret | `STEPS_ALLOWED_EMAILS` | the Gmail allowlist |
| Variable | `STEPS_GOOGLE_CLIENT_IDS` | every client id the Worker accepts (desktop, then web, comma-separated); the deploy fails without it |
| Variable | `STEPS_GOOGLE_CLIENT_ID` | the Desktop client id built into the APK and Mac app |
| Secret | `STEPS_GOOGLE_CLIENT_SECRET` | the Desktop client's secret (an installed-app secret, public by Google's rules, still kept out of git) |
| Variable | `STEPS_WEB_GOOGLE_CLIENT_ID` | the Web client id |
| Variable | `STEPS_CLOUD_URL` | the Worker URL built into the apps |

The Worker's own secrets (`TOKEN_SIGNING_KEY`, `ALLOWED_EMAILS`) live in Cloudflare; the first deploy made the signing key. `crates/server/wrangler.toml` keeps `GOOGLE_CLIENT_IDS` empty on purpose, because the deploy passes it from the variable and would overwrite a dashboard edit.

If sign-in fails: check `STEPS_GOOGLE_CLIENT_IDS`, run Actions › Deploy Steps Cloud, then `curl https://steps-cloud.hamorefaat85.workers.dev/v1/health`. Android sign-in waiting and retries are in `crates/app/src/cloud/oauth.rs` and `engine.rs`; the return activity is `android/src/main/kotlin/app/steps/SignInReturn.kt`.

## 10. Releases, signing and updates

Full detail: [RELEASE.md](RELEASE.md) and [COMPATIBILITY.md](COMPATIBILITY.md).

**Versions** (since 2026-11-09, docs/RELEASE.md §1): a release is `YEAR.MONTH.N` (`2026.11.1`), worked out by `scripts/version.sh release` from this month's `v*` tags; nobody bumps `Cargo.toml` (it stays `2026.11.0`; CI writes the real version in with `scripts/version.sh apply`). The channel (alpha, beta, stable) is picked when the Release workflow runs. Build number = commit count on `main`, one counter for previews and releases; Android `versionCode` = `20000000 + build` and may never go down. Settings › About shows `2026.11.1 · بناء 812`. `steps-tools update guard` refuses versions that aren't newer. Before that: `1.0.0-alpha.1` to `1.0.0-alpha.3`.

**Permanent keys. Never make them again:** a new keystore forces uninstalling Steps on every device, and a new update key strands every installed build.

| Key | Where it lives | GitHub secret / variable |
|---|---|---|
| Android release keystore (RSA 4096, alias `steps`) | my Mac: `~/.steps/steps-release.jks`, password in the Keychain item `steps-android` | `STEPS_ANDROID_KEYSTORE`, `STEPS_ANDROID_KEYSTORE_PASSWORD`; public fingerprint in the variable `STEPS_ANDROID_CERT_SHA256` (workflows refuse an APK signed with anything else) |
| Update signing key (Ed25519) | secret half in the Keychain item `steps-update`; public half committed as `crates/app/update-key.pub` | `STEPS_UPDATE_SECRET` |
| Mac code-signing identity "Steps Code Signing" | my Mac's Keychain, also `~/.steps/steps-codesign.p12` (password: Keychain `steps-codesign`) | `STEPS_MAC_CERT_P12`, `STEPS_MAC_CERT_PASSWORD` |

`just release-keys` made them once; run again, it reuses every key and only refreshes the GitHub secrets. **Backup:** an encrypted `Steps-release-keys.dmg` from `just release-keys-backup` (keys, Keychain passwords, `RESTORE.txt`), kept in two places, passphrase in my password manager. On a new Mac: restore from it, then `just release-keys`.

**Release day:**
1. A PR sets the new `version` in `Cargo.toml` with its `changes/` line; merge it.
2. On that commit of `main`: Nightly checks, Mac and Android preview are green (start them by hand if needed), and Actions › Release with "Dry run" ticked is green.
3. Actions › Release with "Dry run" unticked. It builds the content pack, the signed APK and Steps.app, and publishes the `steps-v<version>` release (`steps-<version>.apk`, `Steps-<version>-mac.zip`) and the signed channel feed (`steps-alpha`, …).
4. Web and Cloud deploy by themselves after green CI on `main`.
5. Update the README's version and links, and §12 here.

**Updates:** installed apps update themselves from their channel's feed; Settings › التحديثات switches between preview, alpha, beta and stable without reinstalling. The manifest is Ed25519-signed and every file is checked against its SHA-256 and size. Every request (updates, Steps Cloud, Google sign-in) fails after 20 s without progress, and a failed check or download is retried as soon as Steps is back in front on Android. The Mac app needs `xattr -dr com.apple.quarantine /Applications/Steps.app` once after the first install (it is self-signed).

## 11. Look: icon, font, themes and styles

Steps has its own look, chosen by me on 2026-11-05. Full design system: [DESIGN.md](DESIGN.md).

- **Icon:** a bold petrol **S** that turns like a stair, with a gold summit dot. Sources `assets/src/icon/*.svg` and `web/icon.svg`; `just icons` re-exports the PNGs (`pip install cairosvg pillow`). The same mark is the CSS `--mark` and appears in the update sheet.
- **Font:** **Readex Pro**, bundled in `assets/fonts/readex-pro` and loaded by `crates/app/src/fonts.rs`.
- **Shape:** rising-step chips, a hero with a staircase and a gold summit, cards on a solid "ledge", a docked bottom bar with a top marker, the mark before the "Steps" wordmark.
- **Themes (10, plus three that follow the device):** صبح, بترولي, أسود AMOLED, سهر (amber, no blue), سكرَب, سيبيا, نيلي, تباين عالي, مذاكرة and مذاكرة خافت; plus زي الجهاز, زي الجهاز وبالليل أسود and مذاكرة زي الجهاز. Code in `crates/core/src/theme.rs`, where every colour pair is tested for WCAG AA. Theme ids never change. Each theme has a key-term pill colour (`--key`, `--key-ink`) for `==key==` in lessons.
- **Styles (10):** درجات (the default), خط رفيع, من غير حدود, فواصل, خط جانبي, رمادي فاتح, أبيض وأسود, خطوط, ملوّن, زجاج. Code in `crates/app/src/prefs.rs` (`Appearance`) and `crates/app/assets/appearance.css`. A device preference, never synced.
- **Lesson reading:** text size, line spacing and line width (tablet and Mac), from «Aa» in the lesson bar or Settings › Look; device preferences (`Reading` in `prefs.rs`, UI in `learn/reading.rs`).
- **Wide windows:** from 840dp the whole window is the canvas; from 1100dp info sheets open as a side panel and lessons can take two columns.
- Labels live in `crates/app/src/i18n/` (`THEME_*`, `APPEARANCE_*`). Feature stylesheets use tokens only (`--bg`, `--surface`, `--accent`, …), never their own colours or copies of shared parts. I prefer minimal looks, and alternatives offered as options rather than a single pick.

## 12. State, roadmap and known issues

*Last updated: 2026-11-09.*

### State
- **Latest release:** `1.0.0-alpha.3` (2026-11-06): APK, Mac app, web and Steps Cloud. <https://github.com/MohamedRef/Steps-app/releases/tag/steps-v1.0.0-alpha.3>. Earlier: alpha.1 and alpha.2 (both 2026-11-05).
- **Content:** all 84 lectures written (16 First Aid courses + Rapid Review), about 9,400 items (5,100 cards, 1,700 MCQs, 1,050 clozes, 1,500 tables and lists) and 360 lessons, with First Aid coverage gaps filled after alpha.1. All `unverified` until I approve them.
- **Quality:** two full review rounds (code, platform, UX, content by course) on 2026-11-04 and 2026-11-05; every finding of every severity was fixed before alpha.1.
- **Sync:** Steps Cloud is live with Google and password sign-in.
- **Sources:** only First Aid 2026 so far. B&B videos and UWorld ids come from a public mapping document ([QBANK.md](QBANK.md)).
- **On `main` since alpha.3, waiting for alpha.4:** the Mac and landscape-tablet layout and safer review keys; resizable study panes, focus mode and quick search (Ctrl/⌘+K); keyboard focus that follows the task; lesson text weighted by importance with a key-term pill per theme; the calm Study themes; my own highlights in lessons; lesson reading settings; mnemonic reminders on repeated lists (`pack::SCHEMA` 3, so it needs the new app build); Study sessions that survive leaving the app and restarts, with a constant-time check of the review log.

### Roadmap
- **Next release, `2026.11.1`** (calendar numbering from now on, §10): release what is on `main` (above), plus whatever daily use turns up.
- **More sources:** Pathoma, Sketchy, UWorld notes and my own materials, merged into the First Aid lectures and topics as they arrive ([SOURCES.md](SOURCES.md)). With the first one, `steps-tools refs` must learn to read its text.
- **Approvals:** I check items against the book in the app and export my verdicts; fixes follow per lecture.
- **Smaller files:** keep splitting the busiest shared files, one area per PR, with no change in behaviour. Done so far: UI text, unit tests, the lesson module.
- **Beta, then stable:** after daily use of the alphas; I decide when.
- Features that copy a commercial question bank stay low priority: I already have the banks.
- **Exam:** Step 1 around September 2027 (date not fixed). Since May 2026 it is 14 blocks of 30 minutes, up to 20 questions each.
- **Calendar versions (2026-11-09, from Batna #768, Mohamed's pick):** the next release is `2026.11.1`, tagged `v2026.11.1`, with `Steps-<version>-android.apk`, `-mac.zip` and `SHA256SUMS.txt`; installed alpha.3 builds take it as an ordinary update (higher version and versionCode, same keys and feeds).

### Known issues
- **#646 Nightly checks failed on main** (2026-11-08, on 71e5538). Open; it closes itself when Nightly passes again.
- Not yet confirmed on a real phone: the lock/unlock screen-freeze fix (`vendor/`), and restoring a Study session after Android ends the app.
- GitHub never fires this repo's schedules; `daily.yml` covers for them (§6).

## 13. Where things live outside the repo

The project files folder is a shared working folder that sits beside the repo (on the server it is `/mnt/project-files/`). It holds what must never be in git.

| What | Where |
|---|---|
| First Aid 2026 PDF | my Mac: `~/Shared Folder/USMLE/Books`; also the project files folder |
| Book text per lecture (reading order with `=== book page N` markers, and a column-preserving `.layout.txt` copy) | project files: `steps/courses/<course>/text/LNN.txt`; whole pages by PDF number in `steps/book/pNNN.txt`. If lost, regenerate from the PDF with `pdftotext` (printed page = PDF page − 22) |
| References read by eye | project files: `steps/courses/<course>/verdicts/LNN.tsv` |
| Research reports (apps, question banks, study methods) | project files: `steps/research/` |
| Review round briefs and results | project files: `critique/round-2026-11-04/`, `critique/round-2026-11-05/` |
| Batna | `MohamedRef/Batna-app`; on my Mac in `~/Documents/projects apps` |
| Release keys | my Mac (`~/.steps/`, Keychain) and the encrypted backup (§10) |
| Consoles | Cloudflare dashboard (Workers & Pages); Google Cloud Console, project "Steps" |

## 14. All docs

| Doc | About |
|---|---|
| [RUN.md](RUN.md) | running the app on each device, data files, env variables |
| [CONTENT.md](CONTENT.md), [LESSONS.md](LESSONS.md), [AUTHORING.md](AUTHORING.md) | writing items and lessons, and the commands around them |
| [SOURCES.md](SOURCES.md), [CONTENT-IDS.md](CONTENT-IDS.md), [WORKSTREAMS.md](WORKSTREAMS.md) | other sources, ids, folder ownership |
| [QBANK.md](QBANK.md), [ANKI.md](ANKI.md) | UWorld and B&B data; the Anki export |
| [DESIGN.md](DESIGN.md), [UI-MAINTENANCE.md](UI-MAINTENANCE.md) | design system; UI architecture and rules |
| [SCREENS-BROWSER-QA.md](SCREENS-BROWSER-QA.md), [SETTINGS-BROWSER-QA.md](SETTINGS-BROWSER-QA.md) | browser checks |
| [CLOUD.md](CLOUD.md), [WEB.md](WEB.md) | Steps Cloud; the web app |
| [RELEASE.md](RELEASE.md), [COMPATIBILITY.md](COMPATIBILITY.md), [CONTENT-DELIVERY.md](CONTENT-DELIVERY.md) | releases, keys and updates; version compatibility; content packs |
| [BATNA-SYNC.md](BATNA-SYNC.md) | porting improvements from Batna |

— Mohamed Refaat
