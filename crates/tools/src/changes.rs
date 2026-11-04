//! "What's new" for each build (`docs/RELEASE.md` §5).
//!
//! A pull request with a change Mohamed can see adds `changes/<topic>.yaml`:
//!
//! ```yaml
//! - kind: new          # new | improved | fixed
//!   ar_eg: Steps بتقولك إيه اتغيّر مع كل تحديث
//!   en: Every update says what changed   # optional
//! ```
//!
//! When a feed is built, the files added since the build already on the
//! feed are gathered (one merged pull request at a time), and that build's
//! entry goes on top of the feed's history. Lines a pull request appends to
//! an existing file count too. A file holding `[]` says "nothing to tell".
//! CI fails a pull request that adds no lines (`.github/workflows/ci.yml`).
//!
//! A commit without a file (pushed straight to main, or merged before the
//! check) is looked up by title in [`RETRO`], which also rewrites the
//! English-only lines older feeds carry; only then does it fall back to its
//! title in English. CI, docs, refactor and test titles are skipped.
//!
//! Adapted from kabar-app main: `crates/tools/src/changes.rs` (anyhow and
//! clap swapped for this crate's plain errors; English optional).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use steps_core::update::{BuildChanges, Change, ChangeKind, HISTORY_BUILDS, UpdateManifest, UpdateNotes};

/// Where the files live, from the repository root.
pub const DIR: &str = "changes";
/// What pull requests merged without a file said, by title: Arabic lines,
/// or `[]` for nothing to tell.
pub const RETRO: &str = "changes/retro/titles.yaml";
/// A line longer than this doesn't fit the notification or the sheet.
const MAX_CHARS: usize = 160;
/// The feed stays well under the app's 256 KB limit.
const MAX_HISTORY_BYTES: usize = 160 * 1024;

type Res<T> = Result<T, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    kind: ChangeKind,
    ar_eg: String,
    #[serde(default)]
    en: String,
}

/// Parses and checks one `changes/*.yaml`.
pub fn parse(text: &str) -> Res<Vec<Change>> {
    let lines: Vec<Line> = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    convert(lines)
}

/// Parses and checks [`RETRO`] (missing: empty).
pub fn retro(root: &Path) -> Res<BTreeMap<String, Vec<Change>>> {
    let path = root.join(RETRO);
    let Ok(text) = std::fs::read_to_string(&path) else { return Ok(BTreeMap::new()) };
    parse_retro(&text).map_err(|e| format!("{RETRO}: {e}"))
}

fn parse_retro(text: &str) -> Res<BTreeMap<String, Vec<Change>>> {
    let map: BTreeMap<String, Vec<Line>> = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    map.into_iter()
        .map(|(title, lines)| Ok((title.clone(), convert(lines).map_err(|e| format!("{title:?}: {e}"))?)))
        .collect()
}

fn convert(lines: Vec<Line>) -> Res<Vec<Change>> {
    lines
        .into_iter()
        .map(|l| {
            for (lang, s) in [("ar_eg", &l.ar_eg), ("en", &l.en)] {
                let n = s.trim().chars().count();
                if lang == "ar_eg" && n == 0 {
                    return Err("ar_eg is empty".to_owned());
                }
                if n > MAX_CHARS {
                    return Err(format!("{lang} is {n} characters (at most {MAX_CHARS})"));
                }
                if s.contains('\n') {
                    return Err(format!("{lang} is more than one line"));
                }
            }
            if !l.ar_eg.chars().any(is_arabic) {
                return Err(format!("ar_eg has no Arabic: {:?}", l.ar_eg));
            }
            Ok(Change {
                kind: l.kind,
                text: UpdateNotes { en: l.en.trim().into(), ar_eg: l.ar_eg.trim().into() },
            })
        })
        .collect()
}

fn is_arabic(c: char) -> bool {
    ('\u{0600}'..='\u{06FF}').contains(&c)
}

/// A merged pull request without a changes file: its title, unless it's
/// housekeeping Mohamed never sees.
pub fn from_title(title: &str) -> Option<Change> {
    let t = clean_title(title);
    let t = t.as_str();
    let lower = t.to_lowercase();
    let first = lower.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
    const NOISE: [&str; 20] = [
        "ci", "build", "chore", "refactor", "docs", "doc", "test", "tests", "style", "deps", "bump",
        "release", "lint", "fmt", "clippy", "critic", "critique", "merge", "revert", "wip",
    ];
    if t.is_empty() || NOISE.contains(&first) || lower.contains("readme") || lower.contains("github action") {
        return None;
    }
    let kind = match first {
        "fix" | "fixes" | "fixed" => ChangeKind::Fixed,
        "add" | "adds" | "new" | "introduce" | "support" => ChangeKind::New,
        _ => ChangeKind::Improved,
    };
    let en: String = t.chars().take(MAX_CHARS).collect();
    Some(Change { kind, text: UpdateNotes { en, ..UpdateNotes::default() } })
}

/// A pull request's title as the feed shows it: no "(#123)", "(Fixes
/// #402)" or "(Part of #140)", no final full stop.
fn clean_title(title: &str) -> String {
    let mut t = title.trim().to_owned();
    while let Some(open) = t.rfind(" (")
        && t.ends_with(')')
        && t[open..].contains('#')
    {
        t.truncate(open);
    }
    t.trim().trim_end_matches('.').to_owned()
}

fn git(root: &Path, args: &[&str]) -> Res<String> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The title a merge commit or a squashed commit carries.
fn title_of(subject: &str, body: &str) -> String {
    if subject.starts_with("Merge pull request") {
        body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_owned()
    } else {
        subject.to_owned()
    }
}

/// Everything merged into `HEAD` after `since` (just `HEAD` itself when
/// `since` is unknown or not in this history), oldest first.
pub fn collect(root: &Path, since: Option<&str>) -> Res<Vec<Change>> {
    let retro = retro(root)?;
    let head = git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    let since = since.filter(|s| git(root, &["merge-base", "--is-ancestor", s, "HEAD"]).is_ok());
    let commits: Vec<String> = match since {
        Some(s) => git(root, &["rev-list", "--first-parent", "--reverse", &format!("{s}..HEAD")])?
            .lines()
            .map(str::to_owned)
            .collect(),
        None => vec![head],
    };
    let mut out = Vec::new();
    for c in &commits {
        let parent = git(root, &["rev-parse", "--verify", "--quiet", &format!("{c}^1")]).ok();
        let fragments = fragments(root, parent.as_deref().map(str::trim), c)?;
        if fragments.files == 0 {
            let log = git(root, &["log", "-1", "--format=%s%x00%b", c])?;
            let (subject, body) = log.split_once('\0').unwrap_or((&log, ""));
            let title = title_of(subject.trim(), body);
            match retro.get(clean_title(&title).as_str()) {
                Some(lines) => out.extend(lines.iter().cloned()),
                None => out.extend(from_title(&title)),
            }
            continue;
        }
        out.extend(fragments.lines);
    }
    Ok(out)
}

/// What one commit did to `changes/<topic>.yaml`.
struct Fragments {
    /// Fragment files added or changed.
    files: usize,
    /// The lines it added.
    lines: Vec<Change>,
    /// A new file holding `[]`: nothing to tell.
    nothing_to_tell: bool,
}

fn fragments(root: &Path, parent: Option<&str>, c: &str) -> Res<Fragments> {
    // `--no-renames`: a fragment moved or replaced by a similar one is a
    // delete plus an add (#320). Lines a deleted file held aren't new when
    // they show up again in another file.
    let touched = match parent {
        Some(p) => {
            git(root, &["diff", "--no-renames", "--name-status", "--diff-filter=AMD", p, c, "--", DIR])?
        }
        None => git(root, &["ls-tree", "-r", "--name-only", c, "--", DIR])?
            .lines()
            .map(|f| format!("A\t{f}\n"))
            .collect(),
    };
    // Fragments only: `changes/<topic>.yaml`, not the retro folder.
    let all: Vec<(&str, &str)> = touched
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, f)| f.ends_with(".yaml") && f.matches('/').count() == 1)
        .collect();
    let old = |f: &str| match parent {
        Some(p) => parse(&git(root, &["show", &format!("{p}:{f}")]).unwrap_or_default()).unwrap_or_default(),
        None => Vec::new(),
    };
    let mut before: Vec<Change> = all.iter().filter(|(s, _)| *s == "D").flat_map(|(_, f)| old(f)).collect();
    let files: Vec<(&str, &str)> = all.into_iter().filter(|(s, _)| *s != "D").collect();
    let at = |e: String, f: &str| format!("{f} at {}: {e}", &c[..7.min(c.len())]);
    let mut out = Fragments { files: files.len(), lines: Vec::new(), nothing_to_tell: false };
    for (status, f) in files {
        let now = parse(&git(root, &["show", &format!("{c}:{f}")])?).map_err(|e| at(e, f))?;
        out.nothing_to_tell |= status == "A" && now.is_empty();
        // A file that grew: only the lines this commit added.
        if status == "M" {
            before.extend(old(f));
        }
        out.lines.extend(now.into_iter().filter(|l| !before.contains(l)));
    }
    Ok(out)
}

/// The pull request check (`.github/workflows/ci.yml`): `HEAD` against
/// `base` (the merge commit's own first parent, so a base that moved since
/// can't lend its files) must add at least one line, or a new file holding
/// `[]`. An edit that adds nothing doesn't count. Returns the lines added.
pub fn gate(root: &Path, base: &str) -> Res<Vec<Change>> {
    let base = git(root, &["rev-parse", "--verify", base])?.trim().to_owned();
    let found = fragments(root, Some(&base), "HEAD")?;
    if found.lines.is_empty() && !found.nothing_to_tell {
        return Err(format!(
            "no new line in {DIR}/<topic>.yaml: add one line per change Mohamed can see (kind new/improved/fixed, ar_eg in Egyptian Arabic, optional en), or a new file holding [] when there is nothing to tell (docs/RELEASE.md §5)"
        ));
    }
    Ok(found.lines)
}

/// This build's entry on top of the previous feed's history (same channel),
/// without repeats, cut to [`HISTORY_BUILDS`] and the size limit.
///
/// English-only lines that [`RETRO`] names are rewritten on the way, so
/// older builds catch up with their Arabic; a build left with no lines
/// goes.
pub fn history(
    previous: Option<&UpdateManifest>,
    entry: Option<BuildChanges>,
    retro: &BTreeMap<String, Vec<Change>>,
) -> Vec<BuildChanges> {
    let mut all: Vec<BuildChanges> = entry.into_iter().collect();
    for old in previous.map(|m| m.history.as_slice()).unwrap_or_default() {
        if !all.iter().any(|e| e.key() == old.key()) {
            let mut old = old.clone();
            old.changes = old
                .changes
                .into_iter()
                .flat_map(|c| {
                    match retro.get(c.text.en.as_str()).filter(|_| c.text.ar_eg.trim().is_empty()) {
                        Some(lines) => lines.clone(),
                        None => vec![c],
                    }
                })
                .collect();
            // A build whose every line was housekeeping says nothing.
            if !old.changes.is_empty() {
                all.push(old);
            }
        }
    }
    all.truncate(HISTORY_BUILDS);
    while all.len() > 1 && serde_json::to_vec(&all).map_or(0, |b| b.len()) > MAX_HISTORY_BYTES {
        all.pop();
    }
    all
}

/// The one-paragraph notes older apps show: the first few lines.
pub fn notes_from(changes: &[Change]) -> UpdateNotes {
    let pick = |f: fn(&UpdateNotes) -> &str| {
        changes
            .iter()
            .take(3)
            .map(|c| {
                let s = f(&c.text);
                if s.is_empty() { c.text.en.as_str() } else { s }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    UpdateNotes { en: pick(|n| if n.en.is_empty() { &n.ar_eg } else { &n.en }), ar_eg: pick(|n| &n.ar_eg) }
}

/// Checks every file under `changes/`.
pub fn check_all(root: &Path) -> Res<usize> {
    let dir = root.join(DIR);
    let mut n = 0;
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().is_some_and(|x| x == "yaml") {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            n += 1;
        }
    }
    retro(root)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_changes_file_checks_out() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(check_all(&root).unwrap() > 0);
    }

    /// A throwaway repository.
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for args in [&["init", "-q"][..], &["config", "user.email", "t@t"], &["config", "user.name", "t"]] {
            git(dir.path(), args).unwrap();
        }
        dir
    }

    /// Writes (or with `None`, deletes) files and commits them.
    fn step(root: &Path, files: &[(&str, Option<&str>)]) -> String {
        for (f, text) in files {
            let path = root.join(f);
            match text {
                Some(t) => {
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(&path, t).unwrap();
                }
                None => std::fs::remove_file(&path).unwrap(),
            }
        }
        git(root, &["add", "-A"]).unwrap();
        git(root, &["commit", "-q", "--allow-empty", "-m", "step"]).unwrap();
        git(root, &["rev-parse", "HEAD"]).unwrap().trim().to_owned()
    }

    const ONE: &str = "- kind: new\n  ar_eg: درس جديد عن الصمامات\n  en: A new heart valves lesson\n";
    const TWO: &str = "- kind: new\n  ar_eg: درس جديد عن الصمامات\n  en: A new heart valves lesson\n- kind: fixed\n  ar_eg: الصورة بقت أوضح\n";

    #[test]
    fn the_gate_wants_a_new_line() {
        let dir = repo();
        let root = dir.path();
        let step = |files: &[(&str, Option<&str>)]| step(root, files);
        let base = step(&[("changes/cvs-valves.yaml", Some(ONE))]);
        // A comment or whitespace edit adds nothing.
        step(&[("changes/cvs-valves.yaml", Some(&format!("# note\n{ONE}")))]);
        assert!(gate(root, &base).is_err());
        // An append counts, even when the file is renamed on the way.
        step(&[("changes/cvs-valves.yaml", None), ("changes/cvs-valves-2.yaml", Some(TWO))]);
        let lines = gate(root, &base).unwrap();
        assert_eq!(lines.iter().map(|c| c.text.ar_eg.as_str()).collect::<Vec<_>>(), ["الصورة بقت أوضح"]);
        assert_eq!(collect(root, Some(&base)).unwrap().len(), 1);
        // A new file holding [] says "nothing to tell".
        let base = step(&[]);
        step(&[("changes/ci.yaml", Some("[]\n"))]);
        assert!(gate(root, &base).unwrap().is_empty());
        let base = step(&[]);
        step(&[("README", Some("x"))]);
        assert!(gate(root, &base).is_err());
    }

    #[test]
    fn a_file_needs_egyptian_arabic() {
        let c = parse("- kind: fixed\n  ar_eg: الصوت مبقاش بيقطع\n").unwrap();
        assert_eq!((c[0].kind, c[0].text.ar_eg.as_str()), (ChangeKind::Fixed, "الصوت مبقاش بيقطع"));
        assert!(parse("- kind: fixed\n  ar_eg: Audio\n").is_err());
        assert!(parse("- kind: new\n  en: Audio\n").is_err());
        assert!(parse("- kind: moved\n  ar_eg: صوت\n").is_err());
        assert!(parse("[]").unwrap().is_empty());
    }

    #[test]
    fn titles_fall_back_without_the_noise() {
        let k = |t: &str| from_title(t).map(|c| (c.kind, c.text.en));
        assert_eq!(
            k("Bank: open each view at its top (#151)"),
            Some((ChangeKind::Improved, "Bank: open each view at its top".into()))
        );
        assert_eq!(k("Fix the timer in mocks").unwrap().0, ChangeKind::Fixed);
        assert_eq!(k("Add L19 lessons").unwrap().0, ChangeKind::New);
        for noise in ["CI: cache dx", "Refactor the store", "docs: RUN.md", "Merge branch 'main'"] {
            assert_eq!(k(noise), None, "{noise}");
        }
        assert_eq!(title_of("Merge pull request #9 from a/b", "\nNew mocks\n"), "New mocks");
    }

    fn entry(build: u64) -> BuildChanges {
        BuildChanges {
            version: "0.1.0".into(),
            build,
            published_at: "2026-10-28T10:00:00Z".into(),
            changes: vec![from_title(&format!("Better thing {build}")).unwrap()],
            content_only: false,
        }
    }

    #[test]
    fn history_puts_the_new_build_first_and_stays_short() {
        let mut prev: UpdateManifest =
            serde_json::from_str(include_str!("../../../fixtures/update/update.json")).unwrap();
        prev.history = (1..=HISTORY_BUILDS as u64).rev().map(entry).collect();
        let none = BTreeMap::new();
        let h = history(Some(&prev), Some(entry(HISTORY_BUILDS as u64 + 1)), &none);
        assert_eq!(h.len(), HISTORY_BUILDS);
        assert_eq!(h[0].build, HISTORY_BUILDS as u64 + 1);
        let again = history(Some(&prev), Some(entry(HISTORY_BUILDS as u64)), &none);
        assert_eq!(again.iter().filter(|e| e.build == HISTORY_BUILDS as u64).count(), 1);
        assert!(history(None, None, &none).is_empty());
    }

    #[test]
    fn retro_gives_old_english_lines_their_arabic() {
        let retro = parse_retro(
            "\"Better thing 2\":\n  - kind: fixed\n    ar_eg: حاجة اتصلحت\n  - kind: new\n    ar_eg: حاجة جديدة\n\"Better thing 1\": []\n",
        )
        .unwrap();
        let mut prev: UpdateManifest =
            serde_json::from_str(include_str!("../../../fixtures/update/update.json")).unwrap();
        prev.history = vec![entry(2), entry(1)];
        let h = history(Some(&prev), Some(entry(3)), &retro);
        assert_eq!(h.len(), 2, "build 1 only had housekeeping");
        let lines: Vec<(u64, ChangeKind, &str)> =
            h.iter().flat_map(|b| b.changes.iter().map(move |c| (b.build, c.kind, line_of(c)))).collect();
        assert_eq!(
            lines,
            [
                (3, ChangeKind::Improved, "Better thing 3"),
                (2, ChangeKind::Fixed, "حاجة اتصلحت"),
                (2, ChangeKind::New, "حاجة جديدة"),
            ]
        );
        assert!(parse_retro("\"Title\":\n  - kind: new\n    ar_eg: English only\n").is_err());
        assert_eq!(clean_title("Lessons: tables (Fixes #9) (#287)"), "Lessons: tables");
    }

    fn line_of(c: &Change) -> &str {
        if c.text.ar_eg.is_empty() { &c.text.en } else { &c.text.ar_eg }
    }

    #[test]
    fn notes_for_older_apps_take_the_first_lines() {
        let c = parse(
            "- kind: new\n  ar_eg: حاجة جديدة\n  en: A new thing\n- kind: fixed\n  ar_eg: حاجة اتصلحت\n",
        )
        .unwrap();
        let n = notes_from(&c);
        assert_eq!(n.ar_eg, "حاجة جديدة\nحاجة اتصلحت");
        assert_eq!(n.en, "A new thing\nحاجة اتصلحت");
    }

    #[test]
    fn lines_must_be_short_and_single() {
        let long = "ب".repeat(MAX_CHARS + 1);
        let e = parse(&format!("- kind: new\n  ar_eg: {long}\n")).unwrap_err();
        assert_eq!(e, format!("ar_eg is {} characters (at most {MAX_CHARS})", MAX_CHARS + 1));
        let e = parse("- kind: new\n  ar_eg: \"سطر\\nتاني\"\n").unwrap_err();
        assert_eq!(e, "ar_eg is more than one line");
        let e = parse("- kind: new\n  ar_eg: \"\"\n").unwrap_err();
        assert_eq!(e, "ar_eg is empty");
        assert!(parse("- kind: new\n  ar_eg: صمامات\n  note: x\n").is_err(), "unknown keys are refused");
    }

    #[test]
    fn commits_without_a_file_fall_back_to_retro_or_their_title() {
        let dir = repo();
        let root = dir.path();
        // The first commit, with no parent: its files are all new.
        let first = step(root, &[("changes/cvs-valves.yaml", Some(ONE))]);
        assert_eq!(collect(root, None).unwrap().len(), 1);
        // A commit that names no file: its title, or retro's Arabic for it.
        std::fs::create_dir_all(root.join("changes/retro")).unwrap();
        std::fs::write(root.join(RETRO), "\"Faster bank\":\n  - kind: improved\n    ar_eg: البنك بقى أسرع\n")
            .unwrap();
        git(root, &["add", "-A"]).unwrap();
        git(root, &["commit", "-q", "-m", "docs: retro titles"]).unwrap();
        git(root, &["commit", "-q", "--allow-empty", "-m", "Faster bank (#12)"]).unwrap();
        git(root, &["commit", "-q", "--allow-empty", "-m", "Add aortic dissection cards"]).unwrap();
        git(root, &["commit", "-q", "--allow-empty", "-m", "CI: cache cargo"]).unwrap();
        let found = collect(root, Some(&first)).unwrap();
        let lines: Vec<(ChangeKind, &str)> = found.iter().map(|c| (c.kind, line_of(c))).collect();
        assert_eq!(
            lines,
            [(ChangeKind::Improved, "البنك بقى أسرع"), (ChangeKind::New, "Add aortic dissection cards")]
        );
        // A `since` that isn't in this history: just HEAD.
        assert!(collect(root, Some("0000000")).unwrap().is_empty(), "HEAD is CI noise");
        // A broken retro file, and a broken fragment, are named.
        step(root, &[(RETRO, Some("- not a map"))]);
        assert!(retro(root).unwrap_err().starts_with(RETRO));
        assert!(check_all(root).is_err());
        step(root, &[(RETRO, None), ("changes/bad.yaml", Some("- kind: moved\n  ar_eg: صوت\n"))]);
        let e = collect(root, None).unwrap_err();
        assert!(e.starts_with("changes/bad.yaml at "), "{e}");
        assert!(check_all(root).unwrap_err().contains("bad.yaml"));
        assert!(gate(root, "no-such-ref").unwrap_err().starts_with("git rev-parse --verify no-such-ref"));
        assert!(check_all(&root.join("nowhere")).is_err());
    }

    #[test]
    fn history_drops_old_builds_to_stay_small() {
        let big = |build: u64| BuildChanges {
            changes: (0..400)
                .map(|i| Change {
                    kind: ChangeKind::New,
                    text: UpdateNotes {
                        en: format!("Cardiology lesson {build}.{i:03} {}", "x".repeat(100)),
                        ar_eg: "درس".into(),
                    },
                })
                .collect(),
            ..entry(build)
        };
        let mut prev: UpdateManifest =
            serde_json::from_str(include_str!("../../../fixtures/update/update.json")).unwrap();
        prev.history = vec![big(2), big(1)];
        let h = history(Some(&prev), Some(big(3)), &BTreeMap::new());
        assert!(serde_json::to_vec(&h).unwrap().len() <= MAX_HISTORY_BYTES, "{}", h.len());
        assert_eq!(h[0].build, 3, "the new build always stays");
    }
}
