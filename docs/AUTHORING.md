# Authoring content

How to add or correct items and lessons, check them, and see what the
devices will get. Everything here is a `steps-tools` command (also a `just`
recipe). The rules for what to write are in `docs/CONTENT.md` (items) and
`docs/LESSONS.md` (lessons); this page is the workflow around them.

## Adding items to a lecture

| Step | Command | What it does |
|---|---|---|
| 1. Next id | `just content-next-id cvs.L01` | Prints the next free item id (`cvs.L01.204`) and lesson id. It skips ids listed in `content/ids-retired.txt`: ids are never reused. |
| 2. Start from an example | `just content-new-item cvs.L01 mcq` | Prints a ready-to-paste item of that kind (`card`, `cloze`, `mcq`, `enumerate`, `essay`, …) with that id, built from the worked example in `fixtures/content/sample.yaml`. Every text starts with `TODO(author)`. |
| 3. Write it | edit `content/cvs/L01.yaml` | Paste under `items:`, replace every `TODO(author)`, set `topic`, `src.page` and `src.quote`. `validate` refuses a file that still has the marker. |
| 4. Page references | `cargo run -p steps-tools -- refs --lecture cvs.L01 --write` | Where the book text is (`docs/CONTENT.md`). |
| 5. Check everything | `just content-preflight` | Runs every check below and prints one table. |
| 6. See the effect | `just content-diff` | Items and lessons added, changed and removed per lecture. |
| 7. Pull request | — | Add `changes/<topic>.yaml`. CI runs the same checks; `just content-impact` says whether the change ships without a new APK. |

Correcting an item is steps 3, 5 and 7: fix the text, never the id. A
lesson follows `docs/LESSONS.md`; step 1 gives its next id too.

## What preflight checks

| Check | Catches |
|---|---|
| content checks (`steps-tools validate`) | schema, unknown keys, ids, page ranges, quotes and their locks, MCQ shape, cloze syntax, tables, approvals, the glossary, lessons, unfinished `TODO(author)` |
| pack devices take (`packs build --check`) | `Pack::problems` (what a device checks before it takes a pack: unique and nested ids, lessons pointing at items and topics that ship, the program) and that the pack round-trips |
| content ids (`steps-tools ids check`) | an id his records point at removed or repurposed (`docs/CONTENT-IDS.md`) |

Each failing line names the file, the id and the reason. `--base` (default
`origin/main`) is what ids and lectures are compared with; `content diff
--base-pack FILE` and `ids check --base-pack FILE` compare with a published
pack instead.
