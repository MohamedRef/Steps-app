# UI follow-up status

Updated: 2026-11-01. Baseline: `ee5e304d7b32a3f59769421276547c45ab257f3f`. Current write-feedback checkpoint:
[558](https://github.com/MohamedRef/Steps-app/pull/558). This file is the status index; older test counts in PR descriptions
belong to those commits, not to the current application.

## Completed checkpoints

1. **Settings and shared preferences.** Searchable Settings, compact/wide
   presentation, shared root state and observable preference recovery are in
   main. Preference retries retain the latest value per key; unrelated writes
   do not erase failures. Recovery checkpoint: [521](https://github.com/MohamedRef/Steps-app/pull/521).
2. **One card contract per category.** Static, dynamic and interactive base
   cards use the shared primitives with native tags, attributes and handlers.
   The source guard prevents copied base-card markup. Checkpoint: [526](https://github.com/MohamedRef/Steps-app/pull/526).
3. **Workflow ownership.** Lesson, review/study, persistence and update stages
   have focused modules with stable feature facades and their original tests.
   Checkpoint: [531](https://github.com/MohamedRef/Steps-app/pull/531); see [UI-MAINTENANCE.md](UI-MAINTENANCE.md).
4. **Computed browser layout and keyboard.** The Settings browser job now
   passes 240 production-rendered cases, including Liquid Glass, normal and
   largest text, responsive widths and System themes in both OS schemes.
   Checkpoint: [532](https://github.com/MohamedRef/Steps-app/pull/532); [latest baseline result](https://github.com/MohamedRef/Steps-app/actions/runs/41302253081).
5. **Command-specific write feedback.** Case completion now uses the fallible store API and retains the final answer when saving fails. Calculation grading only completes after persistence succeeds, and repeated Finish cannot append a second case review. Checkpoint: [558](https://github.com/MohamedRef/Steps-app/pull/558).
6. **Progress documentation.** This index, the completion checklist and the
   browser matrix describe current evidence and separate it from device QA.

Steps also has [lesson and study browser coverage](https://github.com/MohamedRef/Steps-app/actions/runs/41302253008):
1,800 production-rendered layout cases on this baseline. The runner reports
8 of 360 study cases with a control under 48px, using the existing `--tap`
density tokens; that advisory is not a physical touch-target certification.
See [SCREENS-BROWSER-QA.md](SCREENS-BROWSER-QA.md).

## Verification receipts

- Baseline [CI](https://github.com/MohamedRef/Steps-app/actions/runs/41302252948) and
  [renderer regression](https://github.com/MohamedRef/Steps-app/actions/runs/41302253040) passed.
- Current command changes have their own formatting, strict Clippy, workspace
  tests, content checks and CI receipts on [558](https://github.com/MohamedRef/Steps-app/pull/558). Baseline green does not
  certify a later commit. The PR head is the source of truth for its checks.
- Browser fixtures use production components, CSS, fonts and preference types.
  Rust callbacks are not hydrated; browser checks do not prove application
  click/persistence integration or native Back behavior on a physical device.

## Physical device matrix still needed

| Device | Automated evidence | Manual visual and interaction QA |
|---|---|---|
| Mac | Nightly/manual build workflow; no per-PR Mac gate | Not performed in this task |
| S24 Ultra | Android arm64 build and renderer-controller regression | Not performed in this task |
| Tab S11 | Android arm64 build and renderer-controller regression | Not performed in this task |

Check compact/medium/expanded windows, maximum interface size, all appearances
and themes, Arabic/Latin text, focus/Tab/Escape, nested Settings/Updates sheets,
Android Back, screen readers and pen interactions on the actual devices.

## Remaining priorities

- Complete the physical matrix above and broader live-screen integration.
- Continue the separate audit of lesson position/completion and other background
  or feature writes. The command fixes above are not a blanket claim that every
  persistence path has recovery UI. Exports and content-edit forms keep their
  existing operation-specific results; never put append-only study events into
  the idempotent preference retry queue.
- Keep implementation details in the owning modules and shared presentation in
  the shared primitives; both applications retain independent sources/releases.
