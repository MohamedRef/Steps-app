# Compatibility and versioning

Steps has several things that are versioned on their own: the app, the
signed update feed, the content pack the feed carries, the stylesheets in
that pack, the study database and the sync protocol. This page states, for
each one, what a newer or older counterpart does, so a change never strands
a device that has not updated yet. The code is the source of truth; each
rule names where it lives and the test that holds it.

## The numbers

| Thing | Where | What orders it | Who checks it |
|---|---|---|---|
| App version | workspace `version` (`2026.11.0`; a release `YEAR.MONTH.N`, docs/RELEASE.md §1) | `Version` in `crates/core/src/update.rs` | the updater; Android's `versionCode` is `20000000 + build` |
| App build | commits on `main` for every CI build, preview or release (`scripts/version.sh`); `0` for a local build | `(version, build)` | the updater, the content pack |
| Feed format | `UPDATE_FORMAT` in `crates/core/src/update.rs` | integer | `verify_manifest` |
| Pack container | `MAGIC` = `STEPSPK1` in `crates/core/src/pack.rs` | the magic bytes | `Pack::decode` |
| Pack schema | `pack::SCHEMA` (the content types a build reads) | integer | `Pack::decode` |
| File schemas | `steps.content/v1`, `steps.lesson/v1`, `steps.program/v1`, `steps.glossary/v1` | exact tag | `Pack::problems`, `steps-tools validate` |
| Stylesheet build | `UpdateManifest::ui_build` | exact match | `update::mod`, `update::recovery` |
| Database schema | `MIGRATIONS` in `crates/app/src/store/mod.rs` (`user_version`) | integer | the store |
| Sync protocol | `PROTOCOL_VERSION` in `crates/protocol` | integer | Steps Cloud's hub |
| Content ids | `docs/CONTENT-IDS.md` | never reused | `steps-tools ids check` in CI |

## The update feed

- **The signature comes first.** `update.json` is verified over its exact
  bytes before anything in it is read, so the rules below only apply to a
  feed Steps's own key signed.
- **A newer feed format is refused politely.** Only `format` is read first;
  above `UPDATE_FORMAT` the device says Steps itself needs a manual update.
  Bump it only for a change an old app would misread, never for an addition.
- **New fields are additions** with `#[serde(default)]`; old apps ignore
  unknown fields. `history` and `ui_build` arrived this way. Test:
  `a_manifest_without_history_still_parses`.
- **New platforms and change kinds are skipped, not fatal.** An asset whose
  `platform`, or a "what's new" line whose `kind`, this build doesn't know is
  dropped while reading; the rest of the feed still works. A damaged entry
  of a known platform still fails the feed. Tests:
  `a_feed_with_a_platform_or_change_kind_from_a_newer_build_still_parses`,
  `a_damaged_entry_of_a_known_platform_still_fails_the_feed`.
  Builds older than this rule refuse a feed with an unknown platform or
  kind, so do not publish one while a device that matters still runs such a
  build.
- **The channel must match.**

## The content pack

- **Container.** `STEPSPK1` names the layout; a different layout gets a new
  magic, and an old app fails to decode it and keeps what it has.
- **Schema.** A pack whose `schema` is above the build's `pack::SCHEMA` is
  refused (`a_pack_for_newer_content_types_is_refused`). The content types
  use `deny_unknown_fields`, so a new field or item kind needs a schema bump
  and an app build that reads it; older apps keep their content until they
  update.
- **Integrity.** `Pack::problems` (ids, references, listed files, schema
  tags) runs before a pack is taken, when it is opened, and in
  `steps-tools packs build` and `update verify`. A refused pack is
  remembered for the app build that refused it and not fetched again until
  the app changes; the device keeps its last good pack (`docs/RELEASE.md`).
- **Stylesheets** apply only on the exact build in `ui_build`; other builds
  use the content with their own look.
- **Ids.** Lecture, item, topic and lesson ids study records point at are
  never removed, renamed or repurposed without an entry in
  `content/ids-retired.txt` (`docs/CONTENT-IDS.md`).

## Study data

- **Database.** `MIGRATIONS` only grows; each entry runs once and bumps
  `user_version`. A database from a newer build is refused rather than
  opened and damaged.
- **Sync.** Client and Steps Cloud exchange `PROTOCOL_VERSION` in `Hello`;
  a mismatch is refused with a message. Additions use `#[serde(default)]`.
- Review events are append-only; statuses and settings are last-writer-wins.
  Neither is rewritten by an update.

## Checklist for a change

| You are changing | Do |
|---|---|
| A field in `update.json` | add it with `#[serde(default)]`; old apps ignore it |
| A new platform or change kind | add the enum variant; check no device still runs a build older than the skip rule |
| The meaning of a feed field | bump `UPDATE_FORMAT` |
| The pack's byte layout | new `MAGIC` |
| A content field or item kind | bump `pack::SCHEMA` and ship it with an app build (3 since 2026-11-08: lesson steps' `remind`) |
| Stylesheets | nothing: `ui_build` is set when the feed is built |
| The database | append to `MIGRATIONS`, never edit an old entry |
| A sync wire type | add with `#[serde(default)]`, or bump `PROTOCOL_VERSION` |
| A content id | follow `docs/CONTENT-IDS.md` |
