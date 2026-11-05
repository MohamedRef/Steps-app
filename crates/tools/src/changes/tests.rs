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
        serde_json::from_str(include_str!("../../../../fixtures/update/update.json")).unwrap();
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
        serde_json::from_str(include_str!("../../../../fixtures/update/update.json")).unwrap();
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
    let c =
        parse("- kind: new\n  ar_eg: حاجة جديدة\n  en: A new thing\n- kind: fixed\n  ar_eg: حاجة اتصلحت\n")
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
        serde_json::from_str(include_str!("../../../../fixtures/update/update.json")).unwrap();
    prev.history = vec![big(2), big(1)];
    let h = history(Some(&prev), Some(big(3)), &BTreeMap::new());
    assert!(serde_json::to_vec(&h).unwrap().len() <= MAX_HISTORY_BYTES, "{}", h.len());
    assert_eq!(h[0].build, 3, "the new build always stays");
}
