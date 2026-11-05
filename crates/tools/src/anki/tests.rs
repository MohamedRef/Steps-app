use super::*;

#[test]
fn unmerged_lectures_come_from_the_shared_folder() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::create_dir_all(d.join("content/cvs")).unwrap();
    std::fs::create_dir_all(d.join("steps/content-wip/cvs")).unwrap();
    std::fs::write(
            d.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n  - { id: cvs.L05, title: Pharmacology, pages: [321, 328], chunk: 6 }\n  - { id: cvs.L03, title: Physiology, pages: [288, 299], chunk: 6 }\n",
        )
        .unwrap();
    let sample = include_str!("../../../../fixtures/content/sample.yaml");
    std::fs::write(d.join("content/cvs/L04.yaml"), sample).unwrap();
    // Merged wins over wip; a broken wip file is left out.
    std::fs::write(d.join("steps/content-wip/cvs/L04.yaml"), "not: read").unwrap();
    std::fs::write(d.join("steps/content-wip/cvs/L05.yaml"), sample.replace("cvs.L04", "cvs.L05")).unwrap();
    std::fs::write(d.join("steps/content-wip/cvs/L03.yaml"), "lecture: [").unwrap();
    let args = |extra: &[&str]| {
        let mut a: Vec<String> =
            ["--drafts", "steps", "--out", "out"].iter().map(|s| s.to_string()).collect();
        a[1] = d.join("steps").display().to_string();
        a[3] = d.join("out").display().to_string();
        a.extend(extra.iter().map(|s| s.to_string()));
        a
    };
    let report = run(d, &args(&[])).unwrap();
    assert!(report.contains("steps.apkg: 22 notes"), "{report}");
    assert!(report.contains("cvs.L05: 11 notes (draft, not merged yet)"), "{report}");
    assert!(report.contains("content-wip/cvs/L03.yaml") && report.contains("(left out)"), "{report}");
    let merged = run(d, &args(&["--merged-only"])).unwrap();
    assert!(merged.contains("steps.apkg: 11 notes"), "{merged}");
}

#[test]
fn verdicts_drop_wrong_items_and_use_edits() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::create_dir_all(d.join("content/cvs")).unwrap();
    std::fs::write(
            d.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n",
        )
        .unwrap();
    std::fs::write(d.join("content/cvs/L04.yaml"), include_str!("../../../../fixtures/content/sample.yaml"))
        .unwrap();
    std::fs::write(
            d.join("export.yaml"),
            "schema: steps.verdicts/v1\nexported: x\nverdicts:\n  - { id: cvs.L04.003, verdict: wrong, at: a }\n  - { id: cvs.L04.001, verdict: edited, at: a, edit: { back: \"The mitral valve, then the aortic.\" } }\n  - { id: cvs.L04.002, verdict: edited, at: a, edit: { nope: x } }\n  - { id: cvs.L04.004, verdict: wrong, at: b, fingerprint: \"0000000000000000\" }\n",
        )
        .unwrap();
    let args: Vec<String> =
        ["--merged-only", "--verdicts", "e", "--out", "o"].iter().map(|s| s.to_string()).collect();
    let mut args = args;
    args[2] = d.join("export.yaml").display().to_string();
    args[4] = d.join("out").display().to_string();
    let report = run(d, &args).unwrap();
    assert!(report.contains("steps.apkg: 10 notes"), "{report}");
    assert!(report.contains("left out: 1 item(s) marked ✗ or rejected"), "{report}");
    assert!(report.contains("cvs.L04.002: a cloze has no field `nope` (kept the book version)"), "{report}");
}

#[test]
fn own_cards_from_an_export_get_their_own_subdeck() {
    let card = |id: &str, deleted: i64| {
        let value = format!(
            r#"{{"id":"{id}","front":"BNP in heart failure?","back":"≥ 100 pg/mL","lecture":"cvs.L09","created_ms":1}}"#
        );
        serde_json::json!({"id": id, "value": value, "deleted": deleted})
    };
    let doc = serde_json::json!({"format": "steps-export", "tables": {"own_cards": {"rows": [card("own.a", 0), card("own.b", 1)]}}});
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    std::fs::write(&path, doc.to_string()).unwrap();
    let cards = own_cards(&path).unwrap();
    assert_eq!(cards.len(), 1, "deleted cards stay out");

    let lec = steps_core::own::lecture(&cards);
    let deck = build(std::slice::from_ref(&lec), &Options::default());
    assert_eq!(deck.decks[1].1, "Steps::My own cards");
    let note = &deck.notes[0];
    assert_eq!(note.tags, ["own::cards", "own", "cvs.L09"]);
    assert_eq!(note.fields[2], "My own cards");
}

use crate::testkit::{args, content_tree, write};

#[test]
fn one_lecture_and_own_cards_from_an_export() {
    let dir = content_tree();
    let root = dir.path();
    let out = root.join("out");
    let o = out.to_str().unwrap();
    let report = run(root, &args(&["--merged-only", "--lecture", "cvs.L04", "--out", o])).unwrap();
    assert!(report.contains("steps-cvs-L04.apkg: 11 notes"), "{report}");
    assert!(out.join("steps-cvs-L04.apkg").exists());
    let err = run(root, &args(&["--merged-only", "--lecture", "cvs.L09", "--out", o])).unwrap_err();
    assert_eq!(err, ["cvs.L09: no such lecture written yet"]);

    // An export with no live own cards is a warning; a file that isn't an export is an error.
    let export = root.join("export.json");
    std::fs::write(&export, r#"{"format":"steps-export","tables":{}}"#).unwrap();
    let e = export.to_str().unwrap();
    let report = run(root, &args(&["--merged-only", "--own", e, "--out", o])).unwrap();
    assert!(report.contains("warning: ") && report.contains("no own cards in this export"), "{report}");
    std::fs::write(&export, r#"{"format":"anki-export"}"#).unwrap();
    let err = run(root, &args(&["--merged-only", "--own", e, "--out", o])).unwrap_err();
    assert!(err[0].contains("not a Steps export"), "{err:?}");
    std::fs::write(&export, "{").unwrap();
    assert!(own_cards(&export).is_err());
    assert!(own_cards(&root.join("none.json")).is_err());
    // With one lecture only, own cards stay out.
    let card = r#"{\"id\":\"own.a\",\"front\":\"BNP in heart failure?\",\"back\":\"≥ 100 pg/mL\",\"lecture\":\"cvs.L04\",\"created_ms\":1}"#;
    std::fs::write(
            &export,
            format!(r#"{{"format":"steps-export","tables":{{"own_cards":{{"rows":[{{"id":"own.a","value":"{card}","deleted":0}}]}}}}}}"#),
        )
        .unwrap();
    let report =
        run(root, &args(&["--merged-only", "--own", e, "--lecture", "cvs.L04", "--out", o])).unwrap();
    assert!(report.contains("11 notes"), "{report}");
    let report = run(root, &args(&["--merged-only", "--own", e, "--out", o])).unwrap();
    assert!(report.contains("steps.apkg: 12 notes"), "{report}");
}

#[test]
fn approvals_count_only_on_the_text_he_saw() {
    let dir = content_tree();
    let root = dir.path();
    let lec: Lecture = serde_yaml_ng::from_str(crate::testkit::SAMPLE).unwrap();
    let fp = steps_core::review::verify::fingerprint(&lec.items[0]);
    write(
        root,
        "export.yaml",
        &format!(
            "schema: steps.verdicts/v1\nexported: x\nverdicts:\n  - {{ id: cvs.L04.001, verdict: approved, at: a, fingerprint: \"{fp}\" }}\n  - {{ id: cvs.L04.002, verdict: approved, at: a }}\n  - {{ id: cvs.L04.003, verdict: edited, at: a, fingerprint: \"0000000000000000\", edit: {{ answer: 1 }} }}\n  - {{ id: cvs.L04.099, verdict: edited, at: a, edit: {{ back: x }} }}\n"
        ),
    );
    let out = root.join("out");
    let a = args(&[
        "--merged-only",
        "--verdicts",
        root.join("export.yaml").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    let report = run(root, &a).unwrap();
    assert!(report.contains("11 notes") && !report.contains("left out"), "{report}");
    let bad = args(&["--verdicts", root.join("none.yaml").to_str().unwrap()]);
    assert!(run(root, &bad).is_err());
    // A drafts folder whose wip lecture says it is another lecture.
    write(
        root,
        "content/index.yaml",
        &format!(
            "{}  - {{ id: cvs.L05, title: Pharmacology, pages: [321, 328], chunk: 6 }}\n",
            crate::testkit::INDEX
        ),
    );
    write(root, "wip/content-wip/cvs/L05.yaml", crate::testkit::SAMPLE);
    let report =
        run(root, &args(&["--drafts", root.join("wip").to_str().unwrap(), "--out", out.to_str().unwrap()]))
            .unwrap();
    assert!(report.contains("says it is cvs.L04 (left out)"), "{report}");
}

#[test]
fn notes_on_items_come_from_an_export() {
    let row = |id: &str, value: serde_json::Value, deleted: i64| serde_json::json!({"id": id, "value": value.to_string(), "deleted": deleted});
    let doc = serde_json::json!({"format": "steps-export", "tables": {"notes": {"rows": [
        row("item:cvs.L01.003", serde_json::json!({"anchor": {"on": "item", "id": "cvs.L01.003"}, "text": "mine", "created_ms": 1, "edited_ms": 1}), 0),
        row("item:cvs.L01.004", serde_json::json!({"anchor": {"on": "item", "id": "cvs.L01.004"}, "text": "gone", "created_ms": 1, "edited_ms": 1}), 1),
        row("page:284", serde_json::json!({"anchor": {"on": "page", "id": 284}, "text": "page", "created_ms": 1, "edited_ms": 1}), 0),
    ]}}});
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    std::fs::write(&path, doc.to_string()).unwrap();
    let notes = item_notes(&path).unwrap();
    assert_eq!(notes, BTreeMap::from([("cvs.L01.003".to_string(), "mine".to_string())]));
    assert!(item_notes(&dir.path().join("missing.json")).is_err());
}
