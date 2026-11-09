# Lesson and study browser regression

The `Lesson and study browser regression` workflow renders the production
lesson player and study screen through the test-only SSR renderer
(`crates/app/src/learn/browser.rs`, `crates/app/src/study/browser.rs`) and
checks them in Chromium with the app's own stylesheets, fonts, theme and
text-size CSS (`crates/app/src/browser_fixture.rs`). It sits beside the
Settings suite ([SETTINGS-BROWSER-QA.md](SETTINGS-BROWSER-QA.md)) and uses the
same pinned Playwright 1.62.1. There is no copied HTML or CSS, no real user
data, no live API and no app launch.

## What is rendered

| Screen | States |
|---|---|
| Lesson player | The first step of each of the 13 step kinds in book order, opened at that step from a saved attempt (the way the app reopens a lesson), in Steps and in Pages: 26 files; plus two mixed Arabic and English steps (Liver Cirrhosis, steps 6 and 9: "label: value" bullets, arrows, English full stops), whose screenshots show the reading order: 28 files |
| Lesson figures | Every drawn SVG figure of every lesson on one page (`export_screen_fixtures_figures`), checked once at 384px |
| Study | Setup; a running session on the first item of each kind the player shows (card, cloze, enumerate, essay, table, order); an enumerate list with its points open and its تحشيشة under them; a card reopened by Undo; the summary: 10 files |

The store is in memory and the review log is empty, so items are fresh.

## Matrix

| Dimension | Cases |
|---|---|
| Width | 320, 384, 600, 840, 1280 CSS px (height 900) |
| Theme | System, in the light and dark OS schemes |
| Appearance | Stepped and Calm |
| Text size | Normal and Larger |
| Direction | Arabic RTL |

Each file is loaded once and switched the way the app switches: the prefs
stylesheet and `data-appearance` on `.app`. 38 files × 40 = 1,520 cases, about
20 seconds.

## Assertions

| Check | Rule |
|---|---|
| Layout | RTL; the page and `main.content` are no wider than the screen; no control outside it (sideways table scrollers excepted) |
| Touch targets | Lesson player: every control at least the shared 48dp `--target`. Study: at least `--tap` capped at 48px, the token its buttons use; how many study cases still have a control under 48px is printed |
| Names | Every visible control has an accessible name (text, label, `aria-label` or `title`) |
| Heading | A visible `h1` |
| Back | A visible `data-back` target for Escape and Android Back while a lesson or a session runs |
| Lesson | Exactly the saved step is shown, with its kind; the step meter's `progressbar` value; a visible way forward; an unanswered question offers Skip and keeps its gate note screen-reader only |
| Study | The running item's kind and the session `progressbar` |
| Figures | Every `<text>` label of every lesson figure stays inside its picture's `viewBox`: one past the edge is cut off on the phone |
| Own highlights | `crates/app/tests/highlight-browser.cjs` on one lesson step at 1280px: select a word → the bar shows → yellow draws a shape and sends one `add:` with that word; tap it → green sends `put:`, Remove sends `del:` and the shape goes; a saved highlight draws again on a fresh page |
| Study foreground | The production `study/foreground.js` on `study-item-card.html` at 384px, scrolled inside the card: a `visibilitychange` and a `pageshow` (Android resuming the WebView, a page back from the cache) send `visible` twice and leave the same card, once, at the same scroll place |
| Keyboard focus | `crates/app/tests/keyboard-focus-browser.cjs` runs `shortcuts/focus.js` on small study and lesson pages: a keyboard reveal focuses Good, a keyboard grade focuses the next card's control (or the card itself), a lesson step focuses the new step; a mouse click, an open dialog and typing in an answer keep their own focus |
| Errors | Any browser page error fails the job |

Calm, Larger, 384px screenshots in both schemes and `report.json` are
uploaded as `screens-browser-results`; a failure attaches `failure.png`.

## Known gaps

`KNOWN` at the top of `crates/app/tests/screens-browser.cjs` lists what was
already off on `main` when the suite was added (2026-10-30). Those are left
out of the failures but stay listed in the log and `report.json`; anything new
fails. An entry nothing matches any more prints a warning: remove it with its
fix.

All gaps found on 2026-10-30 were fixed in #540, #541 and #542; the list is
empty, so every check above applies everywhere.

## Run

```sh
APP_BROWSER_FIXTURES="$PWD/target/screen-fixtures" cargo test -p steps-app --no-default-features --locked export_screen_fixtures -- --ignored
# Install the pinned Playwright package and Chromium in a separate tool folder.
NODE_PATH=/path/to/browser-tools/node_modules node crates/app/tests/screens-browser.cjs target/screen-fixtures target/screen-results
```

## Scope

Rust callbacks are not hydrated, so taps, answers, saving, grading, swiping
and navigation are not exercised here; the Rust tests and `system-back.test.cjs`
keep those. A passing Chromium run is not device QA: Android WebView, the S24
Ultra and Tab S11, the Mac app, TalkBack, VoiceOver and S Pen stay in the
device matrix.
