# UWorld question IDs and B&B videos per lecture

Each First Aid lecture in Steps lists its Boards and Beyond videos and, under each, the UWorld Step 1 question IDs (QIDs) on that topic, ready to paste into UWorld › Create Test › Custom Test. Steps keeps numbers and video names only, never question text.

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
