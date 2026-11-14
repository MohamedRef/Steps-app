# Writing Steps content

Steps teaches USMLE Step 1 from **First Aid for the USMLE Step 1 2026** (the primary source), with other sources merged in later (`docs/SOURCES.md`).

## Where things are

- Courses: `content/program.yaml` lists First Aid's 16 disciplines and organ systems (group `general` = Section II, `systems` = Section III). A course owns every lecture whose id starts with its id: `cvs.L04` is Cardiovascular.
- A lecture is one First Aid subsection, listed in `content/index.yaml` with its **printed** page range (printed page = PDF page − 22) and the workstream (`chunk`) that writes it (`docs/WORKSTREAMS.md`).
- One file per lecture: `content/<course>/LNN.yaml` (`content/cvs/L04.yaml`). Its `book: { chapter, pages }` repeats the index range; `chapter` is the course's position in `program.yaml` (bio = 1 … resp = 16).
- Lessons (the explanations) live beside it in `content/lessons/<course>/LNN.yaml` (`docs/LESSONS.md`).
- The book text for each lecture, outside git: `/mnt/project-files/steps/courses/<course>/text/LNN.txt` (reading order, split at `=== book page N`) and `LNN.layout.txt` (columns kept; read tables here). Never commit it.

## Items

- Every item has `id` (`cvs.L04.001`, never reused), `kind`, `topic`, `src.page`, `src.quote`, and `status: unverified`.
- The kinds are `card`, `cloze`, `mcq`, `enumerate`, `essay`, `case`, `table`, `order` and `calc`. `fixtures/content/sample.yaml` shows one of each.
- **Topics follow First Aid's own headings** (the bold left-column entries: "Heart morphogenesis", "Fetal circulation"). Use a slug of the heading as the topic id (`fetal-circulation`), in book order. Later sources add items to these same topics.
- **What to write for each First Aid entry:**
  - Every fact the book states gets a `card` or `cloze` (one fact each; clozes for lists and numbers).
  - Lists become `enumerate` (with a `hook`, a تحشيشة, where it helps). Comparisons (drug classes, syndromes side by side) become `table`.
  - Each topic gets at least one **Step 1 style `mcq`**: a clinical vignette (age, sex, setting, presentation, findings), one question that tests one step past the diagnosis (mechanism, associated finding, adverse effect), five options (A–E), one best answer, and a `why` line for every option saying why it is right or wrong.
  - **No answer cues.** The key must not stand out by form: keep it about as long as the other options (trim the key; never pad distractors with filler), and never end a distractor in a padding "only" or "alone". `steps-tools validate` prints, per lecture, how often the key is the single longest option, refuses a bank-wide share over 25%, and refuses a distractor ending in "only"/"alone" (`crates/core/src/validate/cues.rs`). Options are shuffled on every screen and in the Anki export (`exam::option_order`); options that refer to each other ("Both", "Neither", "A and C") are shown in written order, so spread their keys over A–E by hand.
  - **`high-yield` follows the book, never judgment** (Mohamed, 2026-11-14, issue #667). Tag an item `high-yield` only when First Aid itself emphasises the fact it tests, in its `src.quote` on its `src.page`:
    1. the quote holds one of the book's **red bold mnemonics** (First Aid 2026's only emphasis in body text), and a word carrying the red bold also appears in the item (it tests the mnemonic's fact); or
    2. the quote says one of the book's emphasis words, **most common, most frequent, #1, classic(ally), pathognomonic, hallmark, characteristic(ally), most important, leading cause, drug/treatment of choice, most specific, most sensitive, gold standard**, and the item says it too.

    Section titles ("High-Yield Organ Systems") call everything high-yield, so they don't count. **Rapid Review is the exception:** the book presents all of it as high-yield information (p. 707), so every `rr` item is tagged `high-yield` (Mohamed, 2026-11-14). Only where it is certain, well-established knowledge that a fact is high-yield may you tag it without the book's emphasis; prefer the book. `python3 scripts/high_yield_tags.py --pdf <First Aid PDF>` applies the rule (add `--write` to retag, name courses to limit it); about one item in seven qualifies outside Rapid Review. `steps-tools validate` prints the share per course and refuses a course over 30% (`rr` exempt).
- An `enumerate` item may carry a `hook`: a تحشيشة in Egyptian Arabic (medical terms in English) that makes the whole list stick: initials, a grouping with a count ("3 + 4 = 7"), a story or a pun. It must name exactly the item's points and add no new facts.
- **A topic that comes back brings its تحشيشة back** (Mohamed, 2026-11-08). When a lesson step teaches a list that already has a تحشيشة, its `remind:` list names it, and the app shows it under the step: an enumerate item id (`cvs.L04.002`, any course) or a mnemonic step as `<lesson id>:<WORD>` (`cvs.L04.lesson1:TTTTT`). If it was taught in another lesson, the box says «من درس …» and opens that lesson. Give a repeated list the same تحشيشة everywhere rather than a new one; when the points only partly overlap, remind of the old one anyway. `remind` goes on explain, figure, aside, mnemonic or open-table steps only (never under a question: it would give the answer). `validate` checks each entry points at a real hook. Details in `docs/LESSONS.md` rule 13.
- Write from the book only. First Aid's own mnemonics are welcome (credit them as the book's). Put newer guidance or another source's different view in `update: { text, ref }`.
- `src.page` is the page that holds the fact the item tests, and `src.quote` (required: every page chip shows it) is that page's own words containing it.
- An item from a source other than First Aid adds `src.book: <id>` (an id in `content/sources.yaml`); its `src.page` is that source's page and is not range-checked.

## Checks

- Page references are locked per course in `content/book-refs/<course>/LNN.tsv`. After changing a page or quote, run `cargo run -p steps-tools -- refs --lecture cvs.L04 --write` (it reads `/mnt/project-files/steps/courses/cvs/text/L04.txt`). A reference it can't confirm from the text gets read by eye and listed in `/mnt/project-files/steps/courses/<course>/verdicts/LNN.tsv` (`ref<TAB>page<TAB>ok<TAB>reason`; a quote the extracted text scrambles but the page image shows is listed as `ref<TAB>page<TAB>quote-ok<TAB>quote`).
- Run `just validate` before every PR. It checks the schema, unknown keys (typos), ids, page ranges, quotes and their locks (a quote that ends on a function word such as "of" or "as" was cut mid-sentence and is refused), MCQ shape, cloze syntax, table grids, sources, and that nothing is `approved` unless Mohamed approved it. The workflow around it: `docs/AUTHORING.md`.
