# Content delivery: what we measured and what we decided

The roadmap asked two questions about the signed content pack: should it be
split into modules, and should devices download only what changed? Both were
measured on real history before deciding. This page keeps the numbers and the
decisions, and says when to look again.

## How it was measured

For each of the last 60 first-parent commits on `main` (29 Oct to
1 Nov 2026, about 17 merges a day):

1. Build the pack with today's `steps-tools packs build`, from
   `git archive` of that commit.
2. Decode each pack: `STEPSPK1`, then raw DEFLATE of the `Pack` JSON.
3. Compare each pack with the one before, part by part (each lecture,
   each lesson file, each stylesheet, the index and the program).

## What the pack holds

The pack is 6.58 MB compressed and 20.9 MB of JSON. In the JSON:

| Part | Size (uncompressed) |
|---|---:|
| Lesson files | 12.85 MB |
| Lectures (items) | 7.83 MB |
| Stylesheets (`look`) | 0.22 MB |
| Index and program | 0.02 MB |

## How it changes

Each of the 59 merges, compared with the one before:

| What changed | Merges | Share of the JSON that differs |
|---|---:|---|
| Nothing (byte-identical pack) | 22 | 0 |
| Stylesheets only | 24 | at most 0.4 % |
| A lecture and its lesson file (new chapter work) | 13 | 0.3 to 2.4 % |

Any change at all costs a full 6.6 MB download, because the pack is
compressed as a single stream.

## Decisions

### Identical content: never downloaded again (done)

These need no format or protocol change:

- **The pack in use.** `content_update` already skips a feed naming the
  same pack as the one in use.
- **The last good pack.** A feed naming it takes it from disk (#568).
- **A pack this build can't take.** It is remembered until the app changes
  (#568).
- **The pack built into the app.** The preview APK carries the digest of
  its own content (`STEPS_BUILTIN_CONTENT`), so a phone that has just
  installed it doesn't fetch that pack from the feed (#569).

### Modular packs: not now

- **Nothing is optional.** Mohamed studies all 20 courses offline, and
  lessons point at items and topics across lectures. Every device would
  fetch every module anyway.
- **Validation is whole-pack.** `Pack::problems` checks ids and references
  across the index, lectures and lessons together. The last-known-good and
  crash-loop rules (`recovery.rs`) act on one digest. Modules would need a
  consistent set of versions, a validator across modules, and a good set to
  fall back to.

### Part-level differential download: not now, with a trigger

The 24 stylesheet-only and 13 lecture merges change at most 2.4 % of the
content, but each costs 6.6 MB. A delta would need a new pack layout in
which each part is compressed separately (a new `MAGIC`, see
`docs/COMPATIBILITY.md`), plus a signed part index and range fetches.
That changes the container format older apps read. The full download, by
contrast, runs on Wi-Fi only, and release channels publish far less often
than `steps-preview`.

Look again when any of these becomes true:

- automatic downloads are allowed on mobile data;
- the pack grows past about 25 MB (for example, with many book figures);
- a release channel starts publishing content more than weekly.
