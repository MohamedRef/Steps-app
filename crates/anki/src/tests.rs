use std::io::Read as _;

use super::*;

fn fixture() -> Lecture {
    serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap()
}

#[test]
fn ids_match_the_python_exporter() {
    // Known answers from genanki.guid_for and make_apkg.py's mid(): the
    // algorithm is fixed, so these strings must never change.
    assert_eq!(guid_for("hb.L01.001"), "kO@wJGlHO&");
    assert_eq!(guid_for("hb.L01.006#s2"), "HxUBC_F~bH");
    assert_eq!(guid_for("cvs.L04.001"), "Lz#&&A3Z1k");
    assert_eq!(Model::Basic.id(), 1_650_575_226);
    assert_eq!(Model::Cloze.id(), 3_011_687_230);
    assert_eq!(stable_id("steps-deck-cvs.L04"), 556_397_074);
    assert_eq!(deck_name(&fixture(), None), "Steps::L04 Pathology");
    let program: steps_core::program::Program =
        serde_yaml_ng::from_str(include_str!("../../../content/program.yaml")).unwrap();
    let mut cvs = fixture();
    cvs.lecture = "cvs.L01".into();
    cvs.title = "Embryology".into();
    assert_eq!(deck_name(&cvs, Some(&program)), "Steps::Cardiovascular::01 Embryology");
    let mut renal = fixture();
    renal.lecture = "renal.L03".into();
    renal.title = "Physiology".into();
    assert_eq!(deck_name(&renal, Some(&program)), "Steps::Renal::03 Physiology");
    let opts = Options { program: Some(&program), ..Default::default() };
    let deck = build(&[cvs, renal.clone(), renal], &opts);
    let names: Vec<_> = deck.decks.iter().map(|(_, n, _)| n.as_str()).collect();
    assert_eq!(names.iter().filter(|n| **n == "Steps::Renal").count(), 1, "{names:?}");
    assert_eq!(names.iter().filter(|n| **n == "Steps::Cardiovascular").count(), 1, "{names:?}");
    assert_eq!(esc("a<b & \"c\"\nd'"), "a&lt;b &amp; &quot;c&quot;<br>d&#x27;");
}

#[test]
fn options_shuffle_per_note_and_keep_the_book_order_when_they_must() {
    let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let four = s(&["Penicillin", "Aspirin", "Prednisone", "Pericardiocentesis"]);
    // Same id, same order: a re-import doesn't move the options.
    assert_eq!(option_order("cvs.L02.001", &four), option_order("cvs.L02.001", &four));
    // Across notes the key lands on every letter, not always A.
    let keys: BTreeSet<usize> = (1..=40)
        .map(|n| option_order(&format!("cvs.L02.{n:03}"), &four).iter().position(|&i| i == 0).unwrap())
        .collect();
    assert_eq!(keys.len(), 4, "{keys:?}");
    // "All of the above" and number ladders keep their sense.
    assert_eq!(option_order("cvs.L04.001", &s(&["a", "b", "c", "All of the above"])), [0, 1, 2, 3]);
    assert_eq!(option_order("cvs.L04.001", &s(&["HCM", "DCM", "Both", "Neither"])), [0, 1, 2, 3]);
    assert_eq!(option_order("cvs.L04.001", &s(&["Above 30", "Above 10", "Above 20"])), [1, 2, 0]);
}

#[test]
fn approved_status_in_the_file_tags_approved() {
    let mut lec = fixture();
    lec.items[1].status = Status::Approved;
    let deck = build(std::slice::from_ref(&lec), &Options::default());
    let id = lec.items[1].id.clone();
    let note = deck.notes.iter().find(|n| n.fields[4] == id).unwrap();
    assert!(note.tags.contains(&"approved".to_string()), "{:?}", note.tags);
    assert!(deck.notes[0].tags.contains(&"unverified".to_string()));
}

#[test]
fn every_kind_becomes_notes() {
    let lec = fixture();
    let approved = BTreeSet::from(["cvs.L04.001".to_string()]);
    let opts = Options { approved: Some(&approved), ..Options::default() };
    let deck = build(std::slice::from_ref(&lec), &opts);
    // 7 one-note items + 2 case steps + 3 table blanks.
    assert_eq!(deck.notes.len(), 11);
    assert_eq!(deck.card_count(), 13, "the cloze has three blanks");
    assert_eq!(deck.notes[0].tags, ["cvs::L04", "topic::valves", "approved", "high-yield"]);
    assert_eq!(deck.notes[1].tags, ["cvs::L04", "topic::shock", "unverified", "high-yield"]);
    // Options are shuffled per note id (#87); the answer and the why lines follow.
    let mcq = &deck.notes[2];
    assert!(mcq.fields[0].ends_with(
        "A. Vegetation on the tricuspid valve<br>B. Diastolic collapse of the right ventricle<br>\
             C. Asymmetric septal hypertrophy<br>D. Dilated left ventricle with a low ejection fraction<br>\
             E. Flail mitral leaflet"
    ));
    assert!(
        mcq.fields[1].starts_with(
            "<b>B</b><br><div class=\"why\"><b>A</b>: Tricuspid vegetations suggest endocarditis"
        )
    );
    assert!(mcq.fields[1].contains("<b>B</b>: Tamponade compresses the heart"));
    let step2 = deck.notes.iter().find(|n| n.fields[4] == "cvs.L04.006#s2").unwrap();
    assert!(step2.fields[0].contains("CXR: mediastinal widening.<br><br>CT angiography"));
    assert!(step2.fields[0].contains("<b>Step 2:</b> What is the treatment?"));
    assert_eq!(
        step2.fields[1],
        "<b>D</b> Stanford type A involves the ascending aorta and is treated with surgery."
    );
    let blank = deck.notes.iter().find(|n| n.fields[4] == "cvs.L04.007#r1c2").unwrap();
    assert_eq!(blank.fields[0], "Shock hemodynamics: <b>Cardiogenic</b> → <b>PCWP</b>?");
    assert_eq!(blank.fields[1], "↑");
    let order = deck.notes.last().unwrap();
    assert_eq!(order.fields[2], "Pathology · Book p. 317");
    assert_eq!(
        deck.decks.iter().map(|d| d.1.as_str()).collect::<Vec<_>>(),
        ["Steps", "Steps::L04 Pathology"]
    );
}

#[test]
fn wrong_items_drop_out_and_edits_replace_the_book_text() {
    let lec = fixture();
    let wrong = BTreeSet::from(["cvs.L04.003".to_string()]);
    let approved = BTreeSet::from(["cvs.L04.001".to_string(), "cvs.L04.008".to_string()]);
    let mut edited = lec.items[0].clone();
    if let Body::Card { back, .. } = &mut edited.body {
        *back = "Mitral.".into();
    }
    let edits = BTreeMap::from([("cvs.L04.001".to_string(), edited)]);
    let drafts = BTreeSet::from(["cvs.L04".to_string()]);
    let opts = Options {
        approved: Some(&approved),
        wrong: Some(&wrong),
        edits: Some(&edits),
        drafts: Some(&drafts),
        program: None,
        notes: None,
    };
    let deck = build(&[lec], &opts);
    assert_eq!(deck.skipped, 1);
    assert_eq!(deck.notes.len(), 10);
    assert_eq!(deck.notes[0].fields[1], "Mitral.");
    assert!(deck.notes[0].tags.contains(&"unverified".to_string()));
    let order = deck.notes.iter().find(|n| n.fields[4] == "cvs.L04.008").unwrap();
    assert_eq!(order.tags, ["cvs::L04", "topic::pericardium", "approved", "draft"]);
}

#[test]
fn his_notes_fill_the_notes_field_of_their_item_only() {
    let lec = fixture();
    let mine =
        BTreeMap::from([("cvs.L04.006".to_string(), "Beck triad <-> tamponade\nsee p. 16".to_string())]);
    let deck = build(std::slice::from_ref(&lec), &Options { notes: Some(&mine), ..Options::default() });
    for n in &deck.notes {
        let want = if n.fields[4].starts_with("cvs.L04.006") {
            "Beck triad &lt;-&gt; tamponade<br>see p. 16"
        } else {
            ""
        };
        assert_eq!(n.fields[5], want, "{}", n.fields[4]);
    }
    assert!(deck.notes.iter().any(|n| !n.fields[5].is_empty()));
    assert!(BACK_TAIL.contains("{{#Notes}}"));
}

#[test]
fn the_apkg_is_an_anki_collection() {
    let dir = tempfile::tempdir().unwrap();
    let deck = build(&[fixture()], &Options::default());
    let bytes = to_apkg(&deck, 1_800_000_000_000).unwrap();

    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut media = String::new();
    zip.by_name("media").unwrap().read_to_string(&mut media).unwrap();
    assert_eq!(media, "{}");
    let mut collection = Vec::new();
    zip.by_name("collection.anki2").unwrap().read_to_end(&mut collection).unwrap();
    let db_path = dir.path().join("collection.anki2");
    std::fs::write(&db_path, &collection).unwrap();

    // Read it back the way the brief asks: with Python's sqlite3.
    let script = r#"
import json, sqlite3, sys
db = sqlite3.connect(sys.argv[1])
assert db.execute("pragma integrity_check").fetchone()[0] == "ok"
notes = db.execute("select count(*) from notes").fetchone()[0]
cards = db.execute("select count(*) from cards").fetchone()[0]
orphans = db.execute("select count(*) from cards where nid not in (select id from notes)").fetchone()[0]
models, decks = map(json.loads, db.execute("select models, decks from col").fetchone())
fields = db.execute("select flds from notes where guid = 'Lz#&&A3Z1k'").fetchone()[0].split("\x1f")
names = sorted(d["name"] for d in decks.values())
print(notes, cards, orphans, sorted(m["name"] for m in models.values()), names, fields[4])
"#;
    let out = match std::process::Command::new("python3").args(["-I", "-c", script]).arg(&db_path).output() {
        Ok(out) => out,
        Err(e) => {
            eprintln!("python3 not found ({e}); checking with rusqlite only");
            let db = Connection::open(&db_path).unwrap();
            let n: i64 = db.query_row("select count(*) from notes", [], |r| r.get(0)).unwrap();
            assert_eq!(n, 11);
            return;
        }
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "11 13 0 ['Steps Basic', 'Steps Cloze'] ['Default', 'Steps', 'Steps::L04 Pathology'] cvs.L04.001"
    );
}
