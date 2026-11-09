# UI maintenance and review

How the UI is put together, where a shared change belongs, and the rules the
screen and browser tests hold. [DESIGN.md](DESIGN.md) has the visual
vocabulary.

## Architecture

The app uses Rust structs/enums with `impl` methods and Dioxus component
composition. It does not need an inheritance hierarchy to share card behavior.
`crates/core` owns pure logic, `protocol` owns boundary types, and `app` owns
rendering, persistence and platform adapters.

`Ui` in `state.rs` supplies reactive device preferences. Study data already has
a central `store::Review` context: `store/backend.rs` bumps its revision after
writes and after sync reloads. Use that write path and subscribe to its
revision rather than keeping another editable copy in each screen. A UI Card
and a study content card are distinct: the former shares presentation; the
latter must be updated through its storage operation.

## Where a shared change belongs

| Change | Owner | Effect |
|---|---|---|
| Card markup | `crates/app/src/ui.rs::Card`, `DivCard`, `CardButton` | All base-card surfaces; explicit native tags and attributes are retained |
| Navigation row markup and semantics | `ui.rs::ActionRow` | All Settings section links |
| Card radius, padding, border, shadow | `crates/app/assets/main.css`: `--radius-card`, `--card-pad`, `--card-border`, `--card-shadow` | Normal cards and collection surfaces; Stepped sets `--card-shadow` to its ledge |
| Row layout, icon, title and detail | `crates/app/assets/parts.css` | Every `.activity-row` |
| Default list presentation | `parts.rs::Layout::DEFAULT` | New collection choices; saved per-screen choices keep precedence |
| Settings sections and search terms | `settings/mod.rs`: `Section`, `GROUPS`, `SectionView` | Index, search and detail pane |
| Settings layout and controls | `crates/app/assets/settings.css` | Phone and wide layouts, also available to look updates |
| Settings keyboard boundary | `crates/app/assets/settings-dialog.js` | Focus containment, Escape/Back selection and focus restoration |

Keep feature CSS about layout. Reuse the tokens and components; add a modifier
for a real variant instead of copying a card skin. Interactive cards still
need real buttons; `Card` itself is a content surface.

## Settings behavior

The index groups Study/Reminders, Look/Device, and Account/Data. Search finds
sections by titles, summaries and field-related Arabic/English keywords. It
tolerates Arabic marks, alef variants and repeated spaces. It filters the
index, not the controls inside a section. Empty results offer Clear search.
Reopening clears the query; returning to the index keeps it. The Updates row
retains its live status and About shows the build version.

Phones show the index or one section. Wide windows show both with independent
scrolling and a section description. Invalid platform-only preview sections
fall back to Study. While Settings is open, background controls are inert;
Tab stays inside the active dialog and Escape uses the visible Back/Close
control. Closing restores focus while the study screen remains mounted.

## Preference recovery

`state.rs` owns app-wide state; root re-exports preserve existing imports.
`persistence/queue.rs` stores only the newest failed action per stable key.
A successful unrelated write does not hide another failure. Retrying snapshots
checks each generation before executing, performs IO without a live signal
borrow, and removes only successful current actions. Pending choices live in
memory; closing the app can lose them, as the notice explicitly states.

The notice appears in Settings while open and on the main surface otherwise.
Its live region announces failure and the retry action stays keyboard accessible.
Capture owned desired values and durable storage handles; do not capture
screen-local signals that could be dropped when a section closes.

`Ui::update_prefs` is the shared device update path. `prefs/storage.rs` writes
a uniquely owned temporary file, flushes it, then atomically replaces the
existing file; a failed publication keeps the previous preference file intact.
Device snapshots use one stable key so the newest complete choice wins.

Synced settings and collection/reminder choices use stable separate keys.
Review retention and daily-goal retries apply one command to current backend
settings, keeping unrelated updates and the existing fitted-parameter merge.
Recovery clears only the corresponding legacy write diagnostic. A successful
retry reloads synced preference globals so screens observe the recovered data.

Tests cover a real blocked directory, retained prior files where applicable,
latest-choice replacement, unrelated failures, repeated failed retry and a real
Dioxus reactive observer. Native WebView layout still requires device QA.

## Workflow module ownership

| Responsibility | Module |
|---|---|
| Lesson dispatch, shared gate and response context | `learn/steps.rs` |
| Figures, reveals and instructional blocks | `learn/steps/presentation.rs` |
| Check, order, match, sort, mini-case and table interactions | `learn/steps/{check,order,matching,sorting,mini_case,table}.rs` |
| Study setup and completion | `study/{setup,outcome}.rs` |
| Clinical case stages and recap | `case/player.rs` |
| Review backend's StudyStore implementation | `store/backend/study.rs` |
| Update state and execution | `update/{model,service}.rs` |

Keep callers on the existing feature facade. Change the owning stage rather
than copying its state into another screen. Public names are re-exported with
their existing visibility; internal helpers use parent-only visibility. Shared
state stays with the parent, and persistence operations keep their transactions.
Existing tests live beside the owning facade in a child `tests.rs` module.

## UI rules the tests hold

- **Touch targets:** every control in study, lessons, the bank and Settings is
  at least the shared 48dp `--target`; the compact `--tap` (40px) is only for
  dense chips outside those screens. The browser checks fail on a regression.
- **Words, not only colour:** a right or wrong answer, a suggested grade
  («مقترح») and a flagged question say so in words as well as colour.
- **Announcements and focus:** a result screen (study summary, bank result,
  finished lesson) moves focus to its heading so screen readers announce it;
  a tutor check reads its verdict through a polite live region; after a key
  reveals, grades, undoes or turns a step, focus moves to the new card or step.
- **Clean pages:** helper explanations for disabled actions are for screen
  readers only (`.ls-sr-only`); the visible page stays free of helper notes.
- **Back:** Android Back and Escape use the visible Back or Close control of
  the top-most layer (`data-back`), so Back never skips past where a screen
  was opened from.
- **Save failures retry in place:** a failed write says so where it happened,
  with «حاول تاني», through `persistence::WriteFailed`, and keeps the user's
  choice on screen. Review events are append-only and never queued.
- **Keys:** in a timed paper ← is next and → is previous (the app reads right
  to left), F flags; A–E never change an answer under an open dialog.
- **Narrow phones:** grids use `min(<size>, 100%)` column minimums, so nothing
  is wider than a 320px screen at the larger text size.

Device-only behaviour (real gestures, TalkBack and VoiceOver, native
WebViews) still needs a check on the phone, tablet and Mac.
