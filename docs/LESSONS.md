# Writing Steps lessons (`steps.lesson/v1`)

Lessons are the teaching layer: Mohamed opens a lesson, understands the
*why*, and only then reviews the lecture's cards. Cards and questions stay in
`content/cvs/LNN.yaml`; lessons live beside them in
**`content/lessons/cvs/LNN.yaml`**, one file per lecture, and point at the
cards by id.

The model to copy is `content/lessons/cvs/L01.yaml`.

## The rules (the validator enforces most of them)

1. **The book is the answer.** Teach only from `/mnt/project-files/steps/courses/<course>/text/LNN.txt`.
   Every step that teaches a fact gets `page:`: the printed page where that
   fact is (`steps-tools refs` checks it, see docs/CONTENT.md). A `quote:`
   is required (every page chip shows it) and must be that page's own words
   holding the fact. No length cap (Mohamed, 2026-10-29): quote the full
   passage or table, as text with one row per line, never an image. Only a figure step showing the book's own
   figure may go without one. The book text never goes in git.
2. **Guideline differences** go in an `update` step with `ref:` naming the
   guideline and year. Never change the book's answer. Unsure → leave it out.
3. **`status: unverified`** on every lesson. Only Mohamed approves.
4. **Language.** Explanations are Egyptian Arabic (عامية مصرية), warm and
   direct, like a senior resident explaining at 2 am. Medical terms, drug
   names, numbers with units, and anything the exam writes in English stay
   in English: write them inline and the app isolates them left-to-right.
   Case stems, check questions and options may be in English (the exam is).
5. **Teach the why.** For every list or number, give the mechanism or the
   logic that makes it obvious. If the book gives no reason, an accepted
   physiology reason is fine, said plainly; never invent book facts.
6. **Make it stick.** Each lesson should have at least one of: a mnemonic,
   an everyday analogy, a light Egyptian joke (`tone: joke`, kind, never
   about patients' suffering, no politics or religion), an exam trap.
7. **Interactive every 2–3 screens.** A lesson needs at least one question
   step; aim for one every two or three steps. Feedback explains *why*
   each wrong option is wrong.
8. **Short.** 6–10 minutes, 8–16 steps. A lecture usually needs 3–7 lessons
   that follow the book's order and together cover every page.
9. **Link to the cards.** End with a `try` step (1–5 playable items: card,
   cloze, enumerate, essay, table, order; never an MCQ or a case, which the
   player drops) and list the lecture items the lesson teaches in
   `practice:`. Every item of the lecture is in some lesson's `practice:`.
   `steps-tools validate` checks all three.
10. **A picture in every lesson.** At least one figure per lesson (validate
    checks it), more where
    they teach. Draw diagrams, pathways, vicious cycles, algorithms, scales
    and anatomy sketches as inline SVG, with theme classes (below) or
    `var(--token)` colours; no scripts, links, embedded images, `<style>` or
    fixed colours. Labels in English. Under 24 KB. Use a real image
    (see [Images](#images)) where it teaches better than a drawing:
    histology, US/CT/MRI, endoscopy, clinical signs, or the book's own figure.
11. **Rich and easy to read.** Never cut information to make a step
    shorter; add the explanation that makes it click. Readability comes from
    shape: short paragraphs (about 3 lines), bullets for lists,
    `**term**: meaning` rows for definitions. Use `**bold**` for real terms
    only, and mark the 1–3 terms that matter most in the step with
    `==term==` (the marker highlight; the validator allows at most 3).
12. **Every abbreviation is explained.** A step's «اختصارات» chip (beside
    its «ص» chip) lists each abbreviation the step shows, text and figure
    labels alike, with its expansion from the glossary. So the
    validator fails on any word, in a step or a recap line, with two or more capitals (`HRS-AKI`, `NITs`,
    `TIF1γ`) that the glossary doesn't explain. Add an entry (`kind: abbr`,
    the expansion in English, likeliest Step 1 meaning first, then "also
    …") to your course's `content/glossary/<course>.yaml`, or, when the word
    isn't an abbreviation (emphasis like `NOT`, a Roman numeral, a
    mnemonic's letters, a trial name), add it to that file's `plain:` list.
    Shared terms live in `content/glossary.yaml`; a term already there or in
    another course's file is refused as a duplicate, so reuse it (ask the
    core thread to reword a shared meaning). The same entries fill the per-course «قاموس
    الاختصارات» (Lessons screen, More), with the lessons that use each one.

## File shape

```yaml
schema: steps.lesson/v1
lecture: cvs.L01
lessons:
  - id: cvs.L01.lesson1          # .lessonN, never reused
    title: "الكبد بيتليف إزاي؟"
    hook: "سطر واحد يخليه عايز يفتح الدرس"
    minutes: 8
    pages: [1, 4]               # inside the lecture's pages
    topics: [definition, causes] # the lecture's topic ids
    goals: ["...", "..."]        # optional, Arabic
    status: unverified
    steps: [...]
    recap: ["...", "..."]        # take-aways on the end screen
    practice: [cvs.L01.001, ...]  # items of this lecture
```

## Step kinds

Every step may have `page:` and `quote:`.

| kind | fields | what he sees |
|---|---|---|
| `explain` | `title?`, `text` | Prose. Blank line = new paragraph, `- ` = bullet, `**bold**` = a term (weight only), `==key==` = one of the step's 1–3 key terms (marker). |
| `figure` | `title?`, `svg` or `image`, `caption`, `alt` | A drawing or a picture; tap to zoom. |
| `reveal` | `prompt`, `answer` | "فكّر الأول": think, then tap to see the answer. Use for every "why?". |
| `mnemonic` | `word`, `parts: [{key, text}]`, `hook?` | Letter tiles that open one by one. The keys must spell the word. |
| `aside` | `tone` (tip, exam, trap, analogy, joke), `text` | A coloured box. |
| `update` | `text`, `ref` | Guideline note; says the book wins in the exam. |
| `check` | `question`, `options` (3–5), `answer` (0-based), `why` (one per option) | Single best answer; options are shuffled. |
| `order` | `prompt`, `steps` (3–8, correct order), `why?` | Tap the steps in order. |
| `match` | `prompt`, `pairs` (3–6 `[left, right]`), `why?` | Tap left then right. Right sides must differ. |
| `sort` | `prompt`, `bins` (2–4), `cards` (3–10 `{text, bin}`), `why?` | Drop cards into bins. |
| `case` | `stem`, `steps: [{reveal?, question, options?, answer?, why}]` | A mini case, one decision at a time. Options are shuffled in the app, like `check`. |
| `table` | `title`, `columns`, `rows`, `hide?` (`[row, col]`) | A table with cells to recall. |
| `try` | `items` (1–5 ids) | The lecture's own cards, graded into spaced review. |

## SVG classes

Shapes: `fg-box` (plain box), `fg-ink` (dark outline), `fg-thin` (dashed
grey), `fg-acc` / `fg-good` / `fg-bad` (tinted box with coloured border),
`fg-dot-acc` / `fg-dot-good` / `fg-dot-bad` (solid fill), `fg-arrow`,
`fg-arrow-acc`, `fg-arrow-bad` (lines; add `marker-end="url(#yourId)"`),
`fg-head`, `fg-head-acc`, `fg-head-bad` (arrowhead fills inside `<marker>`).
Text: default is body colour; `t-sm`, `t-lg`, `t-b`, `t-mut`, `t-acc`,
`t-good`, `t-bad`, `t-ar` (Arabic font, right to left). Give marker ids a
lecture prefix (`L03a`). A `stroke-width="N"` attribute overrides the class
width.

SVG `<text>` has no `<bdi>`, so keep Arabic and English in separate `<text>`
elements: brackets or numbers around an English term inside an Arabic
`t-ar` label come out flipped. Lesson text (explain, captions, options) is
isolated for you, so write it naturally there. In lesson text an arrow
between Arabic words is drawn `←` (along the reading direction) whether you
write `←` or `→`; inside an English run it stays as written. In SVG labels
nothing is changed: write `←` between Arabic words, `→` between English.

## Images

A figure takes `image:` instead of `svg:`. The file goes in
**`content/lessons/img/cvs/`** (name it `L04-mitral-stenosis-echo.webp`: lecture,
then what it shows). WebP, about 800 px wide, under 150 KB.

From the book (Mohamed allowed book figures in the repo, 2026-10-28; the
book's *text* still never goes in git). The step's `page:` cites it:

```yaml
- kind: figure
  page: 23
  title: "شكل الـ varices في المنظار"
  image: { file: cvs/L04-aortic-dissection-ct.webp, book: true }
  caption: "..."
  alt: "Endoscopy view of three large oesophageal varices with red wale signs."
```

Crop it from `/mnt/project-files/First Aid for the USMLE Step 1 2026 [Medicalstudyzone.com].pdf (printed page = PDF page - 22)`:
`pdftoppm -r 200 -f P -l P -png book.pdf /tmp/p` renders PDF page P (the
printed page differs; check the page number on the render), then crop and
save as WebP with Pillow (`img.crop(box).save(out, "WEBP", quality=80)`).

Open-licence images (public domain, CC0, CC BY, CC BY-SA, CC BY-NC; never
ND) need `credit`, `license` and `source` (the https page it came from), and
a line in `ATTRIBUTION.md`:

```yaml
  image:
    file: cvs/L04-mitral-stenosis-echo.webp
    credit: "James Heilman, MD"
    license: CC BY-SA 4.0
    source: https://commons.wikimedia.org/wiki/File:...
```

The app shows the credit under the caption. The pack build embeds the file,
so nothing is fetched at runtime.

## Check before the PR

`cargo run -p steps-tools -- validate` checks schema, ids, pages, quotes,
item links, SVG safety, image files, sizes and licences, key-term counts,
mnemonic spelling, option counts and abbreviations (rule 12). Then run the
app (`just run`, or `STEPS_OPEN_SCREEN=learn`) and play every lesson once.

## Saved responses

An unfinished lesson is the device-local setting `lessonpos.<id>`
(`steps_core::response::Attempt`, version 2). Besides the step he is on and
the first result of every step, it keeps each step's response (`Draft`): the
option picked, the order built, the pairs matched, the bins chosen, the case
parts answered, reveals, mnemonic letters and table cells opened. Responses
use authored indexes, never screen positions, and carry a fingerprint of the
step; editing a step drops its saved response. A step's state (open, draft,
checked, answered before, skipped) is derived from the restored attempt before
Check or Continue is decided. Records from before version 2 have results but
no responses: those steps show the answer key. Any answered question has a
↻ button that starts it over as practice; the first result stays the one
that counts. No explanatory notes on the page (Mohamed, 2026-10-29).
