# Steps in Anki

Steps's content is exported as **one Anki file, `steps.apkg`**. It holds a `Steps` deck, one level per course and one subdeck per lecture under it, named by its number in the course and its real title (`Steps::Cardiovascular::01 Embryology`, `Steps::Biochemistry::01 Molecular`, …), so you study it while the app is still being built, and you import only one file however many lectures exist.

## From inside the app
Settings › Your data › **صدّر لـ Anki**. The app builds `steps.apkg` from the lectures it carries and your ✓/✎/✗ (✗ items left out, ✎ items with your text) plus your own cards (`Steps::My own cards`), saves it (phone and tablet: `Download/Steps/`; Mac: `~/Downloads/`) and opens it in AnkiDroid or Anki, which shows its import screen. The app carries merged lectures only, so until every lecture is merged the file in the project folder has more cards; both use the same ids, so importing either one updates the same cards.

## Import it
- **Phone or tablet:** open `steps.apkg` from the project files (or Files) and choose AnkiDroid. Then study the `Steps` deck, or one lecture's subdeck.
- **Mac:** double-click `steps.apkg`, or use File › Import in Anki.

## When new content lands
Import the newer `steps.apkg` the same way. Every note has a fixed id that comes from the item id (`cvs.L01.003`), so Anki **updates the existing cards in place**, adds the new ones, and keeps your reviews and due dates. Nothing is duplicated.

### Decks moved under their course (2026-10-29)

## What the cards look like
| Item kind | In Anki |
|---|---|
| card | Basic note: front, back |
| cloze | Cloze note, one card per `{{cN::…}}` |
| mcq | Basic: the stem and options A–E on the front; the answer letter and one line on why each option is right or wrong on the back. Options are shuffled the way the question bank shuffles them (number ladders smallest first, "all of the above" and "Both" kept in the book's order), seeded by the note id, so the order never changes between re-imports |
| enumerate | Basic: the prompt with "N points; pass at P", then the points |
| essay | Basic: "Essay outline: …", then the outline headings and bullets |
| case | One Basic note per step; each step shows the data revealed so far, with its options shuffled like an mcq |
| table | One Basic note per blank cell: "Table: row → column?" |
| order | Basic: "Put in order: …", then the steps in order |
| calc | Basic: the vignette, then the expected score and what it means |

Every back shows the lecture and **book page**, and an amber **Update** box where a current guideline differs from the book (the book stays the answer for the exam).

Tags: `cvs::L01` (lecture), `topic::<slug>`, `unverified` or `approved`, `draft` (not merged yet), and the item's own tags (`high-yield`, `numbers`, `book-question`). To study only what you have checked, search `tag:approved`. To drill numbers, search `tag:numbers`.

## Build it (developers)
```
just anki                                              # dist/anki/steps.apkg, every written lecture
cargo run -q -p steps-tools -- anki --lecture cvs.L01   # dist/anki/steps-cvs-L01.apkg, one lecture
cargo run -q -p steps-tools -- anki --verdicts export.yaml
cargo run -q -p steps-tools -- anki --own steps-export-2026-10-28.json
```
- `--own FILE` reads a full backup from the app (Settings › نسخة احتياطية من كل حاجة) and adds your own cards as one more subdeck, `Steps::My own cards`. They are tagged `own` (plus their lecture, e.g. `cvs.L09`), never `unverified`, because you wrote them. Deleted cards are left out.
- `--verdicts FILE` reads the app's ✓/✎/✗ export (`steps.verdicts/v1`). It leaves out items marked ✗, uses the edited text for ✎ items, and tags ✓ items `approved`.
- A lecture that isn't merged yet comes from the project's shared folder (`content-wip/<course>/LNN.yaml` under `/mnt/project-files/steps`, or `--drafts DIR`), so the file never shrinks; those notes are tagged `draft`. `--merged-only` uses only `content/`.
- `content/approved.txt` also marks items `approved`. Items with `status: rejected` are always left out.
- The deck itself is built by the `steps-anki` crate, shared with the app's export button.
- The deck, note type and note ids match the old `make_apkg.py` (genanki), so decks imported from either one update each other. Both note types have a sixth field, `Notes`, with Mohamed's own note on the item (from the app, or `--own <export>` in `steps-tools anki`); the first import of a deck with it asks Anki to update the note type, and reviews are kept. The Rust exporter replaces that script; `crates/tools/src/anki.rs` has the test that pins the ids.

## Fix list from the app
```
cargo run -q -p steps-tools -- verdicts export.yaml            # ✎/✗ items grouped by lecture, with notes and edits
cargo run -q -p steps-tools -- verdicts export.yaml --approve  # also adds the ✓ ids to content/approved.txt
```
Each verdict carries the item's fingerprint as he judged it. A verdict on an item whose text changed since is never approved or applied (here or in `anki --verdicts`): the tool lists it as "changed since he judged them" and the app asks him again. ✓ from exports made before fingerprints existed are not approved either; export again from the app.
The fixes go into each lecture's YAML from this list; only Mohamed's own ✓ goes into `content/approved.txt`.
