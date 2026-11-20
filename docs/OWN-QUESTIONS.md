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
| `category` | required | the bank it is in, at most 30 characters; `MH` (any case) files it into the MH bank itself, beside the sealed VQ questions |
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

A lecture that isn't one is named with the lectures its course has (`imm.L01` to `imm.L04`), with the page as the other way. A row with a problem is left out and named («الصف 5: …», or «السؤال 5» in a JSON or YAML list); the other rows still go in. An unknown column is named and skipped.

## Adding and updating

On Android the file can also come from outside Steps (#836): **share** it to Steps, or **«Open with» Steps**, from Files, Drive or a chat app. Steps opens Settings › بياناتك with the file already read, as if picked in the card. A JSON file that is a Steps backup goes to the restore card instead. `app.steps.ShareActivity` copies the file into the app (at most 50 MB) and keeps its name in `PendingFile`; `crate::inbox` takes it when Steps comes to the front and hands it to the card. The manifest offers Steps for the same types as the card's picker (`platform::documents::QUESTIONS`, which a test checks against the manifest).

Picking a file shows what it would do, question by question, like Anki's import log (`ownq/report.rs`, 2026-11-20):
- a sentence that names the bank: «هيتضاف 12 سؤال ويتعدّل 3 أسئلة في «أسئلة Pathoma»»;
- a tile for each group: «هيتضاف», «هيتعدّل», «زي ما هو» and «هيتساب». The pressed tile lists its group, and before «ضيفهم» the rows left out come first.
- each listed question with:
  - its id, and a chip when it has a figure or a video;
  - the start of its stem;
  - its lecture and topic, its page, and its answer;
  - for a changed question, what changed («اتغيّر: الإجابة بقت C (كانت B)، الشرح», from `steps_core::ownq::changes`). A question that moved to another lecture also says that its answers start again.
- each row left out, with where it is and why.

«ضيفهم» stores the new and changed rows and turns the same view into what the import did:
- «اتضاف سؤالين في «أسئلة MH»», and the tiles «اتضاف», «اتعدّل», «زي ما هو» and «اتساب»;
- a tap on a listed question closes Settings and opens it in the bank;
- «افتح أسئلة MH في البنك» opens the bank on that set.

The report stays while Steps is open, until another file is read or «ضيف ملف تاني» is tapped, so it is still there after opening a question from it.

A question is keyed by its category and id: the same file again changes nothing, and a row edited in it replaces its question. A question that moves to another lecture gets a new item id (`<lecture>.mine-<category hash>-<id>`), so its answer history starts again. The card lists each category with its count. A tap on a category opens it in the bank, and «امسح» deletes the category's questions on every device; their answers stay in the log.

## In the app

- His questions are `approved` (he wrote them) and tagged `mine` and `mine:<category>`.
- A paper drawn from a category is that category's: its past papers show with it (`Outcome::set`).
- The explanation shows the why lines (when there are any), «الشرح», the objective, the video, and a `Pathoma · P-0001` chip with the book page when he gave one.
- Quick search finds a question by its id.
- A figure another device is still sending says so in place of the figure; the library opens again with questions synced from another device at the next calm moment (`update::view::use_content_switch`).

## Storage and sync

`own_questions` holds one `OwnQuestion` JSON row per question. `own_images` holds each figure under its SHA-256 in 45 KB chunks (`<sha256>.<n>`, `{kind, data}` with base64 data), so each chunk fits one sync op (64 KB). A figure already stored is never written again. Both tables are last writer wins, in `steps_protocol::sync::TABLES`; the hub must be deployed before the app that writes them (Steps Cloud deploys from `main` after CI).
