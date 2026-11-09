# Sources

First Aid for the USMLE Step 1 2026 (`fa2026`) is the backbone: its sections are the courses, its subsections the lectures, its headings the topics. Every other source is merged **into** that tree, never beside it, so one topic shows everything Mohamed has on it.

## Adding a source

1. Mohamed sends it (a PDF, notes, a video list). Its text goes outside git in `/mnt/project-files/steps/sources/<id>/`, never in the repo.
2. Add it to `content/sources.yaml`: `id` (short, lowercase: `pathoma`, `sketchy-micro`, `uw-notes`), `title`, `short` (the page chip: `Pathoma`), and `kind` (`book`, `video`, `qbank` or `notes`: what `src.page` counts).
3. For each fact the source adds, find the First Aid lecture and topic it belongs to and write the item there, with `src: { book: <id>, page: N, quote: "…" }`. A fact First Aid already has gets no second item: add the source's explanation to the lesson instead, or an `update` when the source disagrees.
4. Lessons cite the source on the step (`book: <id>` beside `page:`) where the app supports it; until then keep a source's teaching in items.
5. A source's page references are not yet checked by `steps-tools refs` (it reads First Aid only). When the first non-First-Aid source arrives, `refs` is extended to read `/mnt/project-files/steps/sources/<id>/`.

## When sources disagree

The First Aid answer is the exam answer in Steps. Another source's different number or claim goes in `update: { text, ref }` on the First Aid item, naming the source and year.
