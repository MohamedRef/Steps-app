# UWorld question IDs and B&B videos per lecture

Each First Aid lecture in Steps lists its Boards and Beyond videos and, under each, the UWorld Step 1 question IDs (QIDs) on that topic, ready to paste into UWorld › Create Test › Custom Test. Steps keeps numbers and video names only, never question text.

## Where he sees it
- **More › أسئلة UWorld** (`qbank::IndexPage`): every course › lecture › B&B video in book order, with its count, a bar for the share of videos he marked done, a ✓ that puts it in the basket, and a copy button. A node that fits one test copies at once; a bigger one opens its tests under the row. The search field filters by course, lecture or video name; a bare number looks the QID up and opens its lecture.
- **The basket** (bottom of the index, opens on a tap): every ticked course, lecture and video in book order plus any QIDs he pastes (any separator), cut by `steps_core::qbank::tests` into custom tests of at most 40 (`MAX_TEST`). Settings, synced as `qbank.tests`: questions per test (5–40), split evenly (42 → 21 + 21) or full tests plus the rest (42 → 40 + 2), shuffle (the seed only changes on «لخبط تاني», so the tests he is copying one by one stay put), and leave out videos marked done. Repeats are dropped. Ticks and pasted ids last for the session.
- **Top of each lesson** (`qbank::LessonStrip`, first step): the lecture's B&B video count and UWorld QID count, a copy button and the video list.
- **Lecture page** (`qbank::LectureSection`): each video with its QIDs and a done mark, and a copy button for the whole lecture.
- It is modeled on the public "Exam Maker Tool" (uworld-tool.streamlit.app, same source doc): a B&B checkbox tree plus a test splitter. Steps puts both on one screen and adds search, counts, done videos, pasted ids with any separator, and a stable shuffle.

## Where it comes from
- The public Google Doc "Boards and Beyond Videos and UWorld Question IDs; no duplicates" (Eli Tanenbaum and volunteers; `bb-uworld` in `content/sources.yaml`). Each QID there is listed once, under the B&B video whose AnKing cards it tests. It covers about 86% of UWorld's Step 1 bank, and its tail lists QIDs UWorld retired.
- `content/qbank/bb-map.yaml` (hand-kept, `steps.bbmap/v1`) files each B&B section under a First Aid lecture, with single videos moved where First Aid teaches them (Antiarrhythmic Drugs → `cvs.L05`).
- `content/qbank/uworld.yaml` (generated, `steps.qbank/v1`): per lecture, in book order, each video and its QIDs; retired QIDs and repeats are left out.

## Keeping it current
- `scripts/qbank-sync.sh` downloads the doc's text export and rewrites `uworld.yaml`; `--check` only reports. It needs `curl` and `cargo`.
- `.github/workflows/qbank-sync.yml` runs it every Monday (and by hand from Actions) and opens a pull request when the numbers changed. Close and reopen that PR to start CI, then merge it like any other.
  It needs one repo setting, once: Settings › Actions › General › Workflow permissions › **Allow GitHub Actions to create and approve pull requests**. Without it the run pushes the branch `qbank/sync-<date>` and fails, saying so; open the PR from that branch by hand.
- When the doc adds a video the map doesn't cover, the run stops and names it (`no lecture for B&B Cardiology › … › …`). Add a line to `bb-map.yaml` and run again.
- `steps-tools validate` checks that every lecture in the map exists, that no QID appears twice or retired, and that `uworld.yaml` was not edited by hand.
