# Content ids and Mohamed's records

Steps stores his history against ids written in the content:

| Record | Points at |
|---|---|
| Review events and item states (`review_events`, `item_status`) | item id (`cvs.L01.001`) |
| Lesson progress | lesson id (`cvs.L01.lesson1`) |
| Planner, Today, course switcher | lecture id, topic id, course id |
| Verdicts and approvals (`content/approved.txt`) | item id |

An id that disappears, or an item id that starts naming a different kind of
question, silently cuts that history loose. Content changes are therefore
checked against the previous version of the content.

## The check

```sh
cargo run -p steps-tools -- ids check --base origin/main     # against a commit
cargo run -p steps-tools -- ids check --base-pack FILE       # against a published pack
cargo run -p steps-tools -- ids list                         # every id and its parent
```

CI runs `ids check --base HEAD^1` on every pull request (its base) and on
every push to main (the previous main). The rules (`crates/core/src/ids.rs`):

| Change | Result |
|---|---|
| New item, lesson, lecture, topic, course | allowed (`added`) |
| Stem, options, answer, explanation, `update:` corrected | allowed (no id changes) |
| Lesson under another lecture, lecture under another course | allowed, reported (`moved`) |
| An id removed or renumbered | **fails** (`removed`) |
| An item id kept but its `kind` changed | **fails** (`repurposed`) |
| Duplicate or misplaced ids | fail earlier, in `Pack::problems` and `steps-tools validate` |

The baseline is the content itself at the base commit (or in a published
pack), so there is no separate snapshot file to maintain.

## When an id really has to go

Never renumber or reuse ids, and never migrate his records silently. If an
id must be retired, decide first (in its own PR, with Mohamed's approval)
what happens to the records that point at it, then list the id in
`content/ids-retired.txt` with the reason. A retired id is never reused.
