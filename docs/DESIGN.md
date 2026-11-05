# Steps design system

Steps' own identity (2026-11-05): the S mark, Readex Pro, the stepped shape
language, eight colour themes and five surface styles. Every screen builds
from the tokens and shared classes below; feature stylesheets add layout,
never their own colours or their own copy of a button, chip or sheet.

## Ownership

The foundation (this file, `crates/core/src/theme.rs`, `crates/app/src/prefs.rs`,
`crates/app/src/look.rs`, `assets/main.css` root tokens, `assets/parts.css`,
`assets/appearance.css`, `assets/liquid-glass.css`, `assets/shape.css`,  `assets/nav.css`,
`assets/desk.css`, `more.rs`, the shell in `main.rs`, Settings › Look) is
changed by one owner at a time. A screen that needs a new token or shared
part asks for it, or adds a feature-local class in its own stylesheet.

## Colour tokens

Set on `:root` by `steps_core::theme::Palette::css`, per theme. Never write
a hex value in a feature stylesheet.

| Token | Role | Paper | Night |
|---|---|---|---|
| `--bg` | page | `#f3f6f9` | `#11202b` |
| `--surface` | cards | `#ffffff` | `#192d3b` |
| `--surface-2` | raised: grouped rows, the bar, chips | `#eaf0f5` | `#213c4d` |
| `--text` | main text | `#193348` | `#eef4f8` |
| `--muted` | helper text | `#5a6f7f` | `#adc3d2` |
| `--line` | hairlines (`--line-soft` is fainter) | `#dce5ec` | `#435e70` |
| `--accent` | brand: main action, links, current tab (design: primary) | `#245f7c` | `#9bcde3` |
| `--on-accent` | text on a solid accent fill | `#ffffff` | `#132d3b` |
| `--accent-soft` | accent container: selected, active pill, tonal (design: accent) | `#d8eaf2` | `#234656` |
| `--accent-ink` | text on accent-soft when accent is too loud | `#27576b` | `#c0e0ee` |
| `--soft` | quiet container: notes, flat cards | `#e7f0f5` | `#20394a` |
| `--good`, `--good-soft` | right, done | `#277654` / `#e8f4ec` | `#a0d7b3` / `#1c3c31` |
| `--bad`, `--bad-soft` | wrong, needs work | `#ad493c` / `#fbece7` | `#f1b2a3` / `#4a2e30` |
| `--gold`, `--gold-soft` | supporting notes (guideline update, mnemonic), never a grade | `#8a641d` / `#fbf1d9` | `#e3c581` / `#3d3624` |
| `--sheet` | the info sheet: its own raised tint, never a card colour | `#fcf4e0` | `#2c4f64` |

Aliases in `parts.css` for code ported from the design reference:
`--primary` = `--accent`, `--on-primary` = `--on-accent`, `--good-bg`,
`--bad-bg`, `--gold-bg` = the `-soft` tokens. Careful: the reference's
`--accent` is our `--accent-soft`.

Themes (Arabic names in brackets): Paper («صبح»), Night («بترولي»), True
black («أسود AMOLED»), Night shift («سهر»), Scrubs («سكرَب», clinic teal),
Sepia («سيبيا», warm book paper with a cool sheet), Indigo («نيلي», a
blue-violet dark) and High contrast («تباين عالي»). Paper and Night carry the
redesign's steps-light / steps-dark
(Paper's muted is one step darker than the reference's `#5b7080` to pass AA
on `--surface-2`). True black keeps `#000000` page and cards with Night's
brand blue. Night shift keeps its amber accent: it must have no colour where
blue outweighs red. Theme ids never change. `theme.rs` tests every pairing
above against WCAG AA (4.5:1).

## Appearance: Stepped, Calm, Outline, Tinted and Liquid Glass

A device preference (`prefs::Appearance`, `appearance=` in
`device-prefs.txt`, default Stepped, never synced), picked in
Settings › Look › "نمط الواجهة". It sets
`data-appearance="stepped|calm|outline|tinted|liquid-glass"` on `.app`; only
CSS reads it, so switching keeps every screen's state. The old `glossy`
value loads as Stepped.

- Stepped (default, `assets/appearance.css`): Steps' own look. Cards stand
  on a solid ledge (`--ledge`, the hairline warmed toward the brand) through
  `--card-shadow`; brand-filled things (the hero, the primary button, the
  picked session length) stand on `--ledge-deep`. Pressing moves a button
  down onto its ledge instead of shrinking it. No blur.
- Calm: flat surfaces and hairlines, main.css as it is.
- Outline: cards are transparent with a firmer 1.5px `--outline`, like a
  study notebook; the bars take the page colour.
- Tinted: cards take `--soft`, tiles `--surface-2`, the bar `--soft`;
  answers stay on `--surface` so a question reads the same in every style.
- More contrast and forced colours drop ledges and tints for plain lines.
- Liquid Glass (`assets/liquid-glass.css`): an **iOS-inspired CSS treatment**, not
  Apple's native refraction. It reads as glass because there is something
  behind it: a fixed colour field from the theme (accent, accent-soft, gold,
  good) behind the page and Settings (`.app::before`,
  `.settings-window::before`). On `.app`, `--surface`, `--surface-2`,
  `--raised` and `--bg` become translucent copies of the solid values kept
  on `:root` (`--glass-solid`, `--glass-solid-2`, `--glass-solid-raised`,
  `--glass-solid-bg`), so every card, row, input and button turns to glass
  with no per-screen CSS. Overlays (sheets, dialogs, menus, onboarding, the
  figure viewer) and heroes get the solid values back. A bright rim and top
  glint (`light-dark()`: subtle on dark themes), pill controls and a tinted
  lens for the selected tab and segment. On phones the bottom bar floats over
  the page (same grid area as `.content`, which pads for it) so lists scroll
  under the frosted bar; focus mode is unchanged. Blur only on the bar, the
  Settings header, sticky bars and sheets, never per card. True black: the
  theme sets `--glass-field-on: 0` and `--glass-alpha*: 100%`, so pixels
  stay off. Ink painted in the surface colour must read
  `var(--glass-solid, var(--surface))` (test
  `surface_ink_stays_solid_under_liquid_glass`). Reduced transparency, more
  contrast and forced colours fall back to solid surfaces.

A feature that needs a style variant of its own class writes
`.app[data-appearance="stepped"] .my-class { … }` in its own stylesheet,
from tokens only (`--ledge`, `--ledge-deep`, `--outline` are set per style).

## Type, space, shape

Fonts are bundled (`fonts.rs`): Readex Pro (variable, 160–700) draws both
the Arabic UI and English terms (`bdi`, `--font-latin`). Sizes and spacing are
the existing rem tokens (`--fs-sm/md/lg/xl`, `--sp-1..6`, `--tap`,
`--tap-lg`, `--row`), so Settings › Look › interface size scales them all.
Radii: `--radius` 10px, `--radius-control` 13px, `--radius-card` 16px,
`--radius-sheet` 24px, `--radius-pill`. Touch targets: `--target` 48px.

## Shared classes

| Part | Markup | Notes |
|---|---|---|
| Buttons | `button.btn.btn-primary` / `.btn-tonal` / `.btn-secondary` / `.btn-ghost` / `.btn-good` / `.btn-danger`, `.btn-lg`, `.btn-block`, `.btn-link` | One `.btn-primary` per screen. |
| Icon button | `button.icon-button` with `aria-label` | 48dp touch area even when drawn smaller. |
| Page heading | `header.page-head > .page-eyebrow? + h1 + p.page-sub?` | Title ~22px, weight 600. |
| Section heading | `div.section-head > h2 + button.btn-ghost?` | The button opens the full list. |
| Status chip | `span.chip[data-tone=accent\|good\|bad\|gold]` | The word carries the state. |
| Card | `.card`, `.card-flat` | A card with one job is `card card-action` with its own `onclick`. |
| Hero | `.card.hero.card-action` (with onclick) or `button.hero`; its button `.btn-on-hero` | The one recommended next step. Test: `heroes_open_something`. |
| Stat tiles | `div.stat-tiles > button.tap.stat-tile > span.stat-num + span.stat-label` | Each opens what it counts. |
| Activity rows | `ul.tap-list > li > button.tap.tap-row > span.row-icon? + .tap-row-main + span.chev` | |
| Note | `.note[data-tone=gold\|good\|bad]?` | |
| Empty state | `div.empty > span.empty-icon + h2 + p + button.btn` | Say why and give the way forward. |
| Answer choice | `button.answer[data-state=selected\|correct\|wrong\|missed] > span.answer-key + span.answer-text + span.answer-mark` | Disabled after checking; `.answer-mark` says ✓/✗ in words or icon. |
| Counter | `span.num-ltr` | Any "3 / 12", %, count in Arabic text. |
| Progress | `div.progress[role=progressbar] > div.progress-fill` | `data-tone="good"` for done. |
| Rating row | `div.rating-row > button.rating[data-grade=again\|hard\|good\|easy] > span + span.rating-when` | Real intervals only. Content checks never sit here. |
| Sheet | `div.sheet-backdrop` + `div.sheet[role=dialog] > div.sheet-head` | Bottom sheet on phones, ≤430px box from 600px; scrolls inside. For forms, confirms and lists. |
| Info sheet | `crate::sheet::Sheet { label_id, on_close, head, children }` | What a tap opens to tell more (the book page). No scrim; `--sheet` colour; he drags it to any height (stops at 32/56/90%), flings or drags down to close, Esc/Back close; the size is remembered (`assets/sheet.js`). |
| Field | `label.field > span + input + span.field-hint? + span.field-error[role=alert]?`, `[data-invalid=true]` | Keep the typed text on errors. |
| Search | `label.search > svg + input[type=search]` | |
| Segmented | `div.segmented[role=radiogroup] > button[role=radio]` | |
| Switch | `label > … + input[type=checkbox][role=switch]` | |
| Choice list | `div.choice-list[role=radiogroup] > button.choice[role=radio]` | Settings-style pickers. |
| View switcher | `parts::ViewBar { of: Collection }` → `div.view-bar > div.view-sort? + div.view-seg[role=radiogroup]` | Cards / list / compact plus the screen's orders (and "by lecture" groups where offered). Saved per screen on this device (local setting `view.<key>`). `STEPS_VIEW=cards\|list\|compact` forces a layout for screenshots. |
| Collection | `ul.coll[data-layout] > li > button.coll-btn > span.coll-lead(.coll-badge) + span.coll-main(.coll-title + .coll-sub + .coll-extra) + span.coll-end + span.chev`, `h3.coll-group` | One markup, three looks. Compact hides `.coll-sub`/`.coll-extra`/`.coll-when-roomy` and shows `.coll-when-compact`. Test: `every_view_of_a_list_opens_its_items`. |

No dead UI: anything that looks tappable opens what it shows.
`crates/app/src/dead_ends.rs` lists the tappable-looking classes; add a new
one there when you make it.

### List screens

Every screen that lists things (lessons, other lectures, a lecture's
lessons, References, cases and calc items) has a view switcher, like a
Files app: cards, a list, or a compact list, and an order (book order,
what's left first, by name, due first: each screen offers the ones that
mean something). A new list screen declares a `parts::Collection`, puts a
`ViewBar` above its `ul.coll`, and builds its items from the collection
markup so all three layouts work without per-layout code.

## Shell and navigation

- Phones (under 600dp): top bar, then the screen, then a floating bar with
  five tabs: Today, Lessons (`Learn`), Review (`Study`), Questions (`Bank`),
  More. More (`more.rs`) is a bounded sheet with two groups, "درّب طريقة
  تفكيرك" (Cases, Oral) and "مراجعك وأدواتك" (Progress, Settings). Every
  screen off the bar has a row there; `more.rs` tests enforce it. Add a row
  to `GROUPS` (a `Row` variant) rather than a new tab.
- Tablet and Mac (600dp and up): the rail, or the labelled sidebar from
  840dp, shows every screen; More hides.
- Focused study: `crate::parts::use_focus_mode(on)` in a running lesson,
  session or question set hides the phone's bar and top bar
  (`.app[data-focus="true"]`). The screen must show its own way out, a
  close or finish button with `"data-back": "true"` so Android Back takes
  it. The rail stays from 600dp.
- `main.content` is the one scroll container. Its children keep their
  natural height, and `scroll-padding` keeps a focused field clear of a
  sticky action bar. `.tab-grid` and `.tab-cols` rows are `max-content`
  and start-aligned, so a short card never stretches and a tall one never
  collapses.

## Maintenance and shared primitives

See [UI-MAINTENANCE.md](UI-MAINTENANCE.md) for the code review, shared Card
and ActionRow components, Settings search and the remaining improvements.
