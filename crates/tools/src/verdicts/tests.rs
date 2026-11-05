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
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
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
    assert!(out.contains("Not in the content any more (renumbered or not merged yet): cvs.L04.099"), "{out}");
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
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
    let card = &lec.items[0];
    let edit =
        BTreeMap::from([("back".to_string(), FieldValue::Text("The mitral valve, then the aortic.".into()))]);
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
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
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
