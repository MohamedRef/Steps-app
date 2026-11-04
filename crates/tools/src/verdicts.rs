//! Mohamed's ✓/✎/✗ export from the app (`steps.verdicts/v1`, chunk 3).
//!
//! `steps-tools verdicts FILE [--approve]` prints a fix list for each lecture
//! (✗ wrong and ✎ edited items, with his note and the fields he changed),
//! and with `--approve` adds his ✓ ids to `content/approved.txt`.
//!
//! The Anki exporter reads the same file (`anki --verdicts FILE`): ✗ items are
//! left out and ✎ items use his edited text.
//!
//! The types are `steps_core::review::verify`'s, the same ones the app writes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use steps_core::content::Item;
pub use steps_core::review::verify::{
    FieldValue, VERDICTS_SCHEMA as SCHEMA, Verdict, VerdictEntry, Verdicts,
};

pub fn read(path: &Path) -> Result<Verdicts, String> {
    let v: Verdicts = crate::read_yaml(path)?;
    if v.schema != SCHEMA {
        return Err(format!("{}: schema is {:?}, expected {SCHEMA:?}", path.display(), v.schema));
    }
    Ok(v)
}

/// The latest verdict for each item (the export may hold several).
pub fn latest(v: &Verdicts) -> BTreeMap<&str, &VerdictEntry> {
    let mut out: BTreeMap<&str, &VerdictEntry> = BTreeMap::new();
    for e in &v.verdicts {
        match out.get(e.id.as_str()) {
            Some(old) if old.at > e.at => {}
            _ => {
                out.insert(&e.id, e);
            }
        }
    }
    out
}

/// Whether he judged `item` as it is written now (`verify::fingerprint`).
/// `None`: the export is too old to say.
pub fn current(v: &VerdictEntry, item: &Item) -> Option<bool> {
    v.fingerprint.as_ref().map(|f| *f == steps_core::review::verify::fingerprint(item))
}

/// `item` with his edited fields applied. Fields that don't fit the item's
/// kind are reported, and the item is kept as written.
pub fn apply_edit(item: &Item, edit: &BTreeMap<String, FieldValue>) -> Result<Item, String> {
    let mut value = serde_json::to_value(item).map_err(|e| e.to_string())?;
    let map = value.as_object_mut().expect("an item serializes to a map");
    for (key, v) in edit {
        if matches!(key.as_str(), "id" | "kind" | "status") {
            return Err(format!("{}: the edit changes `{key}`, which only the YAML may change", item.id));
        }
        let kind = item.body.kind();
        if !steps_core::content::allowed_keys(kind).is_some_and(|k| k.contains(&key.as_str()))
            && key != "topic"
        {
            return Err(format!("{}: a {kind} has no field `{key}`", item.id));
        }
        map.insert(key.clone(), serde_json::to_value(v).map_err(|e| e.to_string())?);
    }
    serde_json::from_value(value)
        .map_err(|e| format!("{}: the edit doesn't fit a {}: {e}", item.id, item.body.kind()))
}

pub fn run(root: &Path, args: &[String]) -> Result<String, Vec<String>> {
    let Some(file) = args.first().filter(|a| !a.starts_with("--")) else {
        return Err(vec!["usage: steps-tools verdicts FILE [--approve]".into()]);
    };
    let verdicts = read(Path::new(file)).map_err(|e| vec![e])?;
    let (_, lectures) = crate::packs::load(root)?;
    let items: BTreeMap<&str, (&str, &Item)> = lectures
        .iter()
        .flat_map(|l| l.items.iter().map(move |i| (i.id.as_str(), (l.lecture.as_str(), i))))
        .collect();

    let latest = latest(&verdicts);
    let mut fixes: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut approved = Vec::new();
    let mut unknown = Vec::new();
    // Changed since he judged them: never approved, never applied (#18).
    let mut changed = Vec::new();
    // ✓ from an export without fingerprints: can't tell, so not approved.
    let mut unchecked = Vec::new();
    for (id, v) in &latest {
        let Some((lecture, item)) = items.get(id) else {
            unknown.push(id.to_string());
            continue;
        };
        if current(v, item) == Some(false) {
            changed.push(id.to_string());
            continue;
        }
        let mut line = match v.verdict {
            Verdict::Approved if current(v, item).is_none() => {
                unchecked.push(id.to_string());
                continue;
            }
            Verdict::Approved => {
                approved.push(id.to_string());
                continue;
            }
            Verdict::Wrong => format!("✗ {id} ({}, p. {}) marked wrong", item.body.kind(), item.src.page),
            Verdict::Edited => format!("✎ {id} ({}, p. {}) edited", item.body.kind(), item.src.page),
        };
        if let Some(note) = v.note.as_deref().filter(|n| !n.trim().is_empty()) {
            write!(line, "\n    note: {}", note.trim()).unwrap();
        }
        for (key, value) in &v.edit {
            let shown = match value {
                FieldValue::Text(t) => t.clone(),
                FieldValue::Index(i) => i.to_string(),
                FieldValue::List(l) => l.join(" | "),
            };
            write!(line, "\n    {key} → {shown}").unwrap();
        }
        if v.verdict == Verdict::Edited
            && let Err(e) = apply_edit(item, &v.edit)
        {
            write!(line, "\n    (can't apply: {e})").unwrap();
        }
        fixes.entry(lecture).or_default().push(line);
    }

    let mut out = String::new();
    if !verdicts.exported.is_empty() {
        writeln!(out, "From the app export of {}\n", verdicts.exported).unwrap();
    }
    if fixes.is_empty() {
        out.push_str("No ✎ or ✗ items: nothing to fix.\n");
    }
    for (lecture, lines) in &fixes {
        writeln!(out, "## {lecture}: {} to fix", lines.len()).unwrap();
        for l in lines {
            writeln!(out, "{l}").unwrap();
        }
        out.push('\n');
    }
    if !changed.is_empty() {
        writeln!(
            out,
            "Changed since he judged them, re-check in the app (not approved, not applied): {}",
            changed.join(", ")
        )
        .unwrap();
    }
    if !unchecked.is_empty() {
        writeln!(
            out,
            "✓ from an older export without fingerprints, not approved (export again from the app): {}",
            unchecked.join(", ")
        )
        .unwrap();
    }
    if !unknown.is_empty() {
        writeln!(out, "Not in the content any more (renumbered or not merged yet): {}", unknown.join(", "))
            .unwrap();
    }
    if args.iter().any(|a| a == "--approve") {
        let added = add_approved(&root.join("content/approved.txt"), &approved).map_err(|e| vec![e])?;
        write!(out, "content/approved.txt: {added} new approval(s) of {} ✓", approved.len()).unwrap();
    } else {
        write!(out, "{} ✓ approved (add them to content/approved.txt with --approve)", approved.len())
            .unwrap();
    }
    Ok(out)
}

/// Appends the ids not already in the file; returns how many were new.
fn add_approved(path: &Path, ids: &[String]) -> Result<usize, String> {
    let have: BTreeSet<String> = crate::read_approved(path);
    let new: Vec<&String> = ids.iter().filter(|i| !have.contains(*i)).collect();
    if new.is_empty() {
        return Ok(0);
    }
    let mut text = std::fs::read_to_string(path).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    for id in &new {
        writeln!(text, "{id}").unwrap();
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(new.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = r#"
schema: steps.verdicts/v1
exported: "2026-11-02T10:00:00Z"
verdicts:
  - { id: cvs.L04.001, verdict: approved, at: "2026-11-02T09:00:00Z", fingerprint: FP001 }
  - { id: cvs.L04.003, verdict: approved, at: "2026-11-01T09:00:00Z" }
  - { id: cvs.L04.003, verdict: wrong, at: "2026-11-02T09:05:00Z", note: "The answer is diastolic RV collapse, recheck B" }
  - { id: cvs.L04.004, verdict: edited, at: "2026-11-02T09:06:00Z", edit: { points: [Migratory polyarthritis, Pancarditis], prompt: "List two." } }
  - { id: cvs.L04.099, verdict: wrong, at: "2026-11-02T09:07:00Z" }
  - { id: cvs.L04.003, verdict: edited, at: "2026-11-01T08:00:00Z", edit: { answer: 2 } }
"#;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let cvs = dir.path().join("content/cvs");
        std::fs::create_dir_all(&cvs).unwrap();
        std::fs::write(
            dir.path().join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n",
        )
        .unwrap();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::copy(root.join("fixtures/content/sample.yaml"), cvs.join("L04.yaml")).unwrap();
        std::fs::write(dir.path().join("content/approved.txt"), "# header\ncvs.L04.009").unwrap();
        std::fs::write(dir.path().join("export.yaml"), EXPORT.replace("FP001", &fp("cvs.L04.001"))).unwrap();
        dir
    }

    /// The fingerprint of a fixture item as the app computes it.
    fn fp(id: &str) -> String {
        let lec: steps_core::content::Lecture =
            serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap();
        steps_core::review::verify::fingerprint(lec.items.iter().find(|i| i.id == id).unwrap())
    }

    #[test]
    fn changed_items_and_old_exports_are_never_approved() {
        let dir = tree();
        let export = format!(
            "schema: steps.verdicts/v1\nexported: x\nverdicts:\n  - {{ id: cvs.L04.001, verdict: approved, at: \"2026-11-02T09:00:00Z\", fingerprint: \"0000000000000000\" }}\n  - {{ id: cvs.L04.002, verdict: approved, at: \"2026-11-02T09:00:00Z\" }}\n  - {{ id: cvs.L04.003, verdict: wrong, at: \"2026-11-02T09:00:00Z\", fingerprint: \"{}\" }}\n",
            fp("cvs.L04.003")
        );
        let file = dir.path().join("stale.yaml");
        std::fs::write(&file, export).unwrap();
        let out = run(dir.path(), &[file.display().to_string(), "--approve".into()]).unwrap();
        assert!(
            out.contains(
                "Changed since he judged them, re-check in the app (not approved, not applied): cvs.L04.001"
            ),
            "{out}"
        );
        assert!(
            out.contains("without fingerprints, not approved (export again from the app): cvs.L04.002"),
            "{out}"
        );
        assert!(out.contains("✗ cvs.L04.003"), "{out}");
        assert!(out.ends_with("0 new approval(s) of 0 ✓"), "{out}");
        let text = std::fs::read_to_string(dir.path().join("content/approved.txt")).unwrap();
        assert_eq!(text, "# header\ncvs.L04.009");
    }

    #[test]
    fn fix_list_groups_by_lecture_and_keeps_the_latest_verdict() {
        let dir = tree();
        let file = dir.path().join("export.yaml").display().to_string();
        let out = run(dir.path(), &[file]).unwrap();
        assert!(out.contains("## cvs.L04: 2 to fix"), "{out}");
        assert!(
            out.contains("✗ cvs.L04.003 (mcq, p. 317) marked wrong\n    note: The answer is diastolic RV collapse, recheck B"),
            "{out}"
        );
        assert!(out.contains("points → Migratory polyarthritis | Pancarditis"), "{out}");
        assert!(
            out.contains("Not in the content any more (renumbered or not merged yet): cvs.L04.099"),
            "{out}"
        );
        assert!(out.ends_with("1 ✓ approved (add them to content/approved.txt with --approve)"), "{out}");
    }

    #[test]
    fn approve_appends_only_new_ids() {
        let dir = tree();
        let file = dir.path().join("export.yaml").display().to_string();
        run(dir.path(), &[file.clone(), "--approve".into()]).unwrap();
        let out = run(dir.path(), &[file, "--approve".into()]).unwrap();
        assert!(out.ends_with("0 new approval(s) of 1 ✓"), "{out}");
        let text = std::fs::read_to_string(dir.path().join("content/approved.txt")).unwrap();
        assert_eq!(text, "# header\ncvs.L04.009\ncvs.L04.001\n");
    }

    #[test]
    fn edits_apply_to_the_item_or_say_why_not() {
        let lec: steps_core::content::Lecture =
            serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap();
        let card = &lec.items[0];
        let edit = BTreeMap::from([(
            "back".to_string(),
            FieldValue::Text("The mitral valve, then the aortic.".into()),
        )]);
        let fixed = apply_edit(card, &edit).unwrap();
        assert!(
            matches!(&fixed.body, steps_core::content::Body::Card { back, .. } if back == "The mitral valve, then the aortic.")
        );
        let bad = BTreeMap::from([("answer".to_string(), FieldValue::Text("x".into()))]);
        assert!(apply_edit(&lec.items[2], &bad).unwrap_err().contains("doesn't fit a mcq"));
    }

    #[test]
    fn a_clean_export_has_nothing_to_fix_and_bad_files_are_named() {
        let dir = tree();
        let root = dir.path();
        let file = root.join("clean.yaml");
        std::fs::write(&file, "schema: steps.verdicts/v1\nexported: ''\nverdicts: []\n").unwrap();
        let out = run(root, &[file.display().to_string()]).unwrap();
        assert_eq!(
            out,
            "No ✎ or ✗ items: nothing to fix.\n0 ✓ approved (add them to content/approved.txt with --approve)"
        );
        std::fs::write(&file, "schema: steps.verdicts/v0\nexported: x\nverdicts: []\n").unwrap();
        let err = run(root, &[file.display().to_string()]).unwrap_err();
        assert!(err[0].ends_with("expected \"steps.verdicts/v1\""), "{err:?}");
        assert_eq!(
            run(root, &["--approve".to_string()]).unwrap_err(),
            ["usage: steps-tools verdicts FILE [--approve]"]
        );
    }

    #[test]
    fn an_edit_may_not_touch_ids_or_status() {
        let lec: steps_core::content::Lecture =
            serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap();
        let card = &lec.items[0];
        for key in ["id", "kind", "status"] {
            let edit = BTreeMap::from([(key.to_string(), FieldValue::Text("x".into()))]);
            let e = apply_edit(card, &edit).unwrap_err();
            assert_eq!(e, format!("cvs.L04.001: the edit changes `{key}`, which only the YAML may change"));
        }
        // A field of the right name but the wrong shape doesn't fit.
        let edit = BTreeMap::from([("front".to_string(), FieldValue::List(vec!["a".into()]))]);
        assert!(apply_edit(card, &edit).unwrap_err().contains("the edit doesn't fit a card"));
        // The topic may move.
        let edit = BTreeMap::from([("topic".to_string(), FieldValue::Text("rheumatic".into()))]);
        assert_eq!(apply_edit(card, &edit).unwrap().topic.as_deref(), Some("rheumatic"));
    }

    #[test]
    fn approving_twice_adds_nothing() {
        let dir = tree();
        let root = dir.path();
        let file = root.join("export.yaml").display().to_string();
        let first = run(root, &[file.clone(), "--approve".into()]).unwrap();
        assert!(first.ends_with("content/approved.txt: 1 new approval(s) of 1 ✓"), "{first}");
        let again = run(root, &[file, "--approve".into()]).unwrap();
        assert!(again.ends_with("content/approved.txt: 0 new approval(s) of 1 ✓"), "{again}");
        let text = std::fs::read_to_string(root.join("content/approved.txt")).unwrap();
        assert_eq!(text, "# header\ncvs.L04.009\ncvs.L04.001\n");
    }
}
