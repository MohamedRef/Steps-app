# Steps: working conventions

Steps is a personal USMLE Step 1 study app in Rust, for one learner: Mohamed. It is personal and is never published to a store. It lives in its own repo, `MohamedRef/Steps-app`, on `main`.

Steps was built on 2026-11-04 from a copy of Batna (`MohamedRef/Batna-app`, Mohamed's internal-medicine MSc app): same engine (FSRS review, study sessions, question bank, cases, lessons, sync, signed updates, themes, Egyptian Arabic UI), with the content and course model changed for First Aid. **Batna is a separate app: never push or open PRs there for Steps work, and never depend on Batna crates.** Copy a Batna module when it helps, adapt it, and credit the source path in a comment ("adapted from batna-app main: <path>").

The workstreams (which thread writes which course) are in `docs/WORKSTREAMS.md`.

## Sources
- **First Aid for the USMLE Step 1 2026 is the primary source.** Courses and lectures follow its Sections II and III: one course per discipline or organ system, one lecture per First Aid subsection (`cvs.L04` = Cardiovascular › Pathology). Ids and page ranges are in `content/index.yaml` and `content/program.yaml`.
- Pages are the **printed** page numbers (printed page = PDF page − 22), because that is what Mohamed sees in the book and its index.
- More sources will come (Pathoma, Sketchy, UWorld notes, Mohamed's own materials). They are merged into the same First Aid lecture and topic, never into a parallel tree: an item from another source carries `src.book: <id>` from `content/sources.yaml` (`docs/SOURCES.md`). First Aid stays the backbone and the exam answer; where a source differs, say so in the item.
- Book text, per lecture, outside git: `/mnt/project-files/steps/courses/<course>/text/LNN.txt` (reading order, `=== book page N` markers) and `LNN.layout.txt` (columns kept, easier for tables). Whole pages by PDF number: `/mnt/project-files/steps/book/pNNN.txt`. The PDF: `/mnt/project-files/First Aid for the USMLE Step 1 2026 [Medicalstudyzone.com].pdf`.

## Non-negotiables
- $0 a month. Ask before adding anything that could cost money.
- No secrets in the repo, ever.
- **The textbook never goes in git**: no PDF and no extracted text (cropped book figures for lessons are the one exception). Items paraphrase it, cite `src.page`, and quote only what proves the fact. CI enforces this.
- Every content item ships `status: unverified`. Only Mohamed approves (`content/approved.txt`, from his in-app export).
- The book's answer is the exam answer. Newer guidelines or a different source's view go in `update:` with the name and year, never as silent changes.
- No real patient data to any AI engine. Tests never call live APIs.
- No generated images. Lesson figures are inline SVG drawings, crops of the book's own figures (`book: true`, cited by page), or open-licence images with credit, licence and source (`docs/LESSONS.md` § Images).

## USMLE style
- Step 1 tests mechanisms in clinical vignettes. MCQs are **single best answer vignettes**: age, sex, setting, presentation, findings, then one question ("Which of the following is the most likely cause / mechanism / next finding?"), five options (A–E), one best answer, and a `why` line for every option. Test one step past the diagnosis (mechanism, associated finding, drug's adverse effect), as Step 1 does.
- Cards and clozes hold the First Aid facts, one fact each. Tag the book's own high-yield emphasis `high-yield`.
- Lessons explain the why in Egyptian Arabic so the facts stick (`docs/LESSONS.md`).

## Language
- Reply to Mohamed in Egyptian Arabic, and keep it short. Code, comments, commit messages and docs are in English.
- UI text is Egyptian Arabic (`crates/app/src/i18n.rs`, one module per feature). Medical content is in English, wrapped in `<bdi>` inside Arabic UI.

## Workflow
- Branch from `origin/main` and open PRs with base `main`. Before merging, merge the latest `origin/main`, run `just check`, wait for green CI, then squash-merge it yourself.
- `just check` = check-tracked-files, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `steps-tools validate`, `steps-tools packs build --check`.
- A content thread touches only its own course folders (`content/<course>/`, `content/lessons/<course>/`, `content/book-refs/<course>/`) plus appended glossary entries, so threads never conflict. Changes to `crates/`, the schema or shared files go through the core thread.
- Keep `docs/RUN.md` current.
- The Mac workflow runs nightly and by hand only, never per PR or per merge.
- Every PR adds `changes/<topic>.yaml` or appends to one (one line per change Mohamed can see: `kind` new/improved/fixed, `ar_eg`, optional `en`); the app's "What's new" is built from these (`docs/RELEASE.md` §5). Housekeeping PRs add a file holding `[]`. CI fails a PR without one.

## Code
- Rust stable, edition 2024. Workspace: `crates/core` (pure logic, no UI, no IO, builds for wasm32), `crates/app` (Dioxus 0.7 + HTML/CSS), `crates/tools`, `crates/anki`, `crates/protocol`, `crates/server` (the Cloudflare Worker).
- UI: logical CSS properties only (RTL). Layout by window width (compact under 600dp, medium under 840dp, expanded 840dp and up), never by device name.
- Review events are append-only. Statuses and settings are last-writer-wins. Synced tables carry `account_id`, `device_id`, an HLC timestamp and a tombstone.

## Tone
Warm, specific and encouraging. No guilt, no fear, no fake urgency, no dark patterns.
