# Questions from a file

Mohamed adds his own questions in Steps from a file: **Settings › بياناتك › «ضيف أسئلة من ملف»**. Each **category** in the file becomes a bank of its own in the question bank («أسئلة Pathoma», `exam::Set::Own`), beside Steps' own questions and the VQ bank. Each question joins the First Aid lecture it names, so the bank's lecture and topic filters, blocks, marks and the mistakes log take it like any other. The questions and their figures sync to every device through Steps Cloud.

Code: `steps_core::ownq` (rows → questions, problems, the import plan, figure chunks), `crates/app/src/ownq/` (reading the file, the card, the library), `crates/app/src/store/ownq.rs` (the tables).

## The file

- An Excel workbook (`.xlsx`): the sheet named `Questions` (else the first), headings on its first row, one question per row. The template is `crates/app/src/ownq/steps-questions-template.xlsx`, with two examples and a sheet of instructions in Arabic («اقرأني»); the card offers it as «نزّل التمبلت».
- Or JSON or YAML with the same fields: `{"questions": [...]}` or a bare list, with `options` and `why` as lists (or the sheet's letter columns), `tags` as a list and `video` as a link or `{youtube, title}`.
- Either one alone, or in a `.zip` with the figures its rows name (PNG, JPEG, WebP or GIF, at most 1 MB each). In a zip, the first `.xlsx` wins, then JSON, then YAML; folders, `__MACOSX` and hidden files are ignored.

| Column | | What |
|---|---|---|
| `id` | required | letters, digits, `-`, `_`, at most 40; unique in its category in any case |
| `category` | required | the bank it is in, at most 30 characters |
| `page` or `lecture` | one required | a printed First Aid page (the lecture whose range holds it, if written) or a lecture id such as `cvs.L04`, which wins |
| `topic` | optional | a topic of that lecture, by id or title, any case |
| `stem` | required | |
| `A` … `J` | at least A and B | in order, with no empty one before a filled one |
| `answer` | required | the right option's letter |
| `explanation` | optional | shown after the why lines |
| `why_A` … `why_J` | optional | all of them or none |
| `objective` | optional | shown as the educational objective |
| `image` | optional | a figure's file name in the zip (any case; folders don't matter) |
| `video`, `video_title` | optional | a YouTube link of any form, or its id, and the title to show |
| `tags` | optional | comma-separated, at most 10 |

A row with a problem is left out and named («الصف 5: …», or «السؤال 5» in a JSON or YAML list); the other rows still go in. An unknown column is named and skipped.

## Adding and updating

Picking a file shows what it would do («هيتضاف 12 · هيتعدّل 3 · 200 زي ما هي») and every problem, then «ضيفهم» stores the new and changed rows. A question is keyed by its category and id: the same file again changes nothing, and a row edited in it replaces its question. A question that moves to another lecture gets a new item id (`<lecture>.mine-<category hash>-<id>`), so its answer history starts again. The card lists each category with its count; «امسح» deletes a category's questions on every device (their answers stay in the log).

## In the app

- His questions are `approved` (he wrote them) and tagged `mine` and `mine:<category>`.
- A paper drawn from a category is that category's: its past papers show with it (`Outcome::set`).
- The explanation shows the why lines (when there are any), «الشرح», the objective, the video, and a `Pathoma · P-0001` chip with the book page when he gave one.
- Quick search finds a question by its id.
- A figure another device is still sending says so in place of the figure; the library opens again with questions synced from another device at the next calm moment (`update::view::use_content_switch`).

## Storage and sync

`own_questions` holds one `OwnQuestion` JSON row per question. `own_images` holds each figure under its SHA-256 in 45 KB chunks (`<sha256>.<n>`, `{kind, data}` with base64 data), so each chunk fits one sync op (64 KB). A figure already stored is never written again. Both tables are last writer wins, in `steps_protocol::sync::TABLES`; the hub must be deployed before the app that writes them (Steps Cloud deploys from `main` after CI).
