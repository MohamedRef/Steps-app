use super::*;
use crate::content::{BookRef, IndexEntry, Topic};

fn lecture(id: &str, topics: &[(&str, &str)]) -> Lecture {
    Lecture {
        schema: crate::content::SCHEMA.into(),
        lecture: id.into(),
        title: id.into(),
        author: None,
        book: BookRef { chapter: 1, pages: [1, 2] },
        topics: topics
            .iter()
            .map(|(id, title)| Topic { id: id.to_string(), title: title.to_string() })
            .collect(),
        items: vec![],
    }
}

fn book() -> (Index, BTreeMap<String, Lecture>) {
    let entry = |id: &str, pages: [u32; 2]| IndexEntry { id: id.into(), title: id.into(), pages, chunk: 1 };
    let index = Index {
        schema: "steps.index/v1".into(),
        lectures: vec![
            entry("cvs.L03", [288, 299]),
            entry("heme.L04", [420, 439]),
            entry("zz.L01", [900, 910]),
        ],
    };
    let lectures = BTreeMap::from([
        ("cvs.L03".to_string(), lecture("cvs.L03", &[("heart-murmurs", "Heart murmurs")])),
        ("heme.L04".to_string(), lecture("heme.L04", &[("anemias", "Anemias")])),
    ]);
    (index, lectures)
}

fn row(line: usize, fields: &[(&str, &str)]) -> Row {
    Row { line, fields: fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
}

fn good(line: usize, id: &str) -> Row {
    row(
        line,
        &[
            ("id", id),
            ("category", "Pathoma"),
            ("page", "295"),
            ("topic", "heart murmurs"),
            ("stem", "A man has an apical holosystolic murmur. Most likely cause?"),
            ("A", "Aortic stenosis"),
            ("B", "Mitral regurgitation"),
            ("C", "Mitral stenosis"),
            ("answer", "b"),
            ("explanation", "Apical, holosystolic, to the axilla."),
            ("video", "https://youtu.be/Ab3_xYz-012?t=40"),
            ("video_title", "Murmurs"),
            ("tags", "murmurs, cardio"),
        ],
    )
}

fn image(sha: &str) -> FileImage {
    FileImage { image: Image { sha256: sha.into(), kind: "image/png".into(), bytes: 10, chunks: 1 } }
}

#[test]
fn a_row_becomes_a_question_in_the_lecture_its_page_names() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let (qs, problems) = read(&[good(2, "P-0001")], &places, &BTreeMap::new());
    assert!(problems.is_empty(), "{problems:?}");
    let q = &qs[0];
    assert_eq!(
        (q.lecture.as_str(), q.topic.as_deref(), q.page),
        ("cvs.L03", Some("heart-murmurs"), Some(295))
    );
    assert_eq!(q.options.len(), 3);
    assert_eq!(q.answer, 1);
    assert!(q.why.is_empty());
    assert_eq!(
        q.video.as_ref().map(|v| (v.youtube.as_str(), v.title.as_str())),
        Some(("Ab3_xYz-012", "Murmurs"))
    );
    assert_eq!(q.tags, ["murmurs", "cardio"]);
    assert_eq!(q.key(), "Pathoma/P-0001");
    // Its item: an approved MCQ in that lecture, in its own set.
    let item = q.item();
    assert!(item.id.starts_with("cvs.L03.mine-") && item.id.ends_with("-p-0001"), "{}", item.id);
    assert_eq!(crate::program::lecture_of(&item.id), "cvs.L03");
    assert_eq!(item.status, Status::Approved);
    assert_eq!(category_of(&item.tags), Some("Pathoma"));
    assert!(is_mine(&item.tags));
    let Body::Mcq { why, .. } = &item.body else { panic!() };
    assert_eq!(why.len(), 3, "an empty why line per option");
    // The same row on another device: the same ids.
    let (again, _) = read(&[good(9, "P-0001")], &places, &BTreeMap::new());
    assert_eq!(again[0].item().id, item.id);
}

#[test]
fn a_lecture_id_wins_over_a_page_and_topics_match_by_title_or_id() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let mut r = good(2, "P-1");
    r.fields.insert("lecture".into(), "heme.L04".into());
    r.fields.insert("topic".into(), "ANEMIAS".into());
    let (qs, problems) = read(&[r], &places, &BTreeMap::new());
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!((qs[0].lecture.as_str(), qs[0].topic.as_deref()), ("heme.L04", Some("anemias")));
}

#[test]
fn every_problem_names_its_row_and_good_rows_still_stand() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let bad = |line, change: &[(&str, &str)], drop: &[&str]| {
        let mut r = good(line, &format!("X-{line}"));
        for k in drop {
            r.fields.remove(*k);
        }
        for (k, v) in change {
            r.fields.insert(k.to_string(), v.to_string());
        }
        r
    };
    let rows = vec![
        good(2, "P-1"),
        bad(3, &[], &["id"]),
        bad(4, &[("id", "has space")], &[]),
        bad(5, &[], &["category"]),
        bad(6, &[("D", ""), ("E", "Tricuspid")], &[]),
        bad(7, &[("answer", "F")], &[]),
        bad(8, &[("why_A", "Only one")], &[]),
        bad(9, &[], &["page"]),
        bad(10, &[("page", "1500")], &[]),
        bad(11, &[("page", "page two")], &[]),
        bad(12, &[("lecture", "xx.L09")], &[]),
        bad(13, &[("topic", "Shock")], &[]),
        bad(14, &[("video", "https://vimeo.com/123")], &[]),
        bad(15, &[("image", "missing.png")], &[]),
        bad(16, &[], &["B", "C"]),
        bad(17, &[("page", "905")], &[]),
        good(18, "p-1"),
        bad(19, &[("colour", "red")], &[]),
    ];
    let (qs, problems) = read(&rows, &places, &BTreeMap::new());
    assert_eq!(qs.len(), 2, "rows 2 and 19 stand");
    let at = |line: usize| -> Vec<&Problem> {
        problems.iter().filter(|p| p.line == line).map(|p| &p.problem).collect()
    };
    assert_eq!(at(3), [&Problem::Missing("id")]);
    assert_eq!(at(4), [&Problem::BadId("has space".into())]);
    assert_eq!(at(5), [&Problem::Missing("category")]);
    assert_eq!(at(6), [&Problem::Gap('D')]);
    assert_eq!(at(7), [&Problem::BadAnswer("F".into())]);
    assert_eq!(at(8), [&Problem::WhyCount { options: 3, why: 1 }]);
    assert_eq!(at(9), [&Problem::NoPlace]);
    assert_eq!(at(10), [&Problem::PageNotInBook(1500)]);
    assert_eq!(at(11), [&Problem::BadPage("page two".into())]);
    assert_eq!(at(12), [&Problem::UnknownLecture("xx.L09".into())]);
    assert_eq!(at(13), [&Problem::UnknownTopic("Shock".into())]);
    assert_eq!(at(14), [&Problem::BadVideo("https://vimeo.com/123".into())]);
    assert_eq!(at(15), [&Problem::NoImage("missing.png".into())]);
    assert!(at(16).contains(&&Problem::FewOptions), "{:?}", at(16));
    assert_eq!(at(17), [&Problem::PageNotInBook(905)], "a lecture not written yet takes no questions");
    assert_eq!(at(18), [&Problem::Twice("p-1 (2)".into())], "ids are one per category, in any case");
    assert_eq!(at(0), [&Problem::UnknownColumn("colour".into())]);
    let (_, none) = read(&[], &places, &BTreeMap::new());
    assert_eq!(none, [RowProblem { line: 0, problem: Problem::NoRows }]);
}

#[test]
fn why_lines_come_for_every_option_or_none() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let mut r = good(2, "P-1");
    for (c, w) in [("A", "No."), ("B", "Right."), ("C", "No.")] {
        r.fields.insert(format!("why_{c}"), w.into());
    }
    let (qs, problems) = read(&[r], &places, &BTreeMap::new());
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(qs[0].why, ["No.", "Right.", "No."]);
    let Body::Mcq { why, .. } = &qs[0].item().body else { panic!() };
    assert_eq!(why, &qs[0].why);
}

#[test]
fn a_figure_is_found_by_its_name_and_stored_in_chunks() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let images = BTreeMap::from([("figures/Murmur.PNG".to_string(), image("ab"))]);
    let mut r = good(2, "P-1");
    r.fields.insert("image".into(), "murmur.png".into());
    let (qs, problems) = read(&[r], &places, &images);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(qs[0].image.as_ref().map(|i| i.sha256.as_str()), Some("ab"));

    let bytes: Vec<u8> = (0..(CHUNK_BYTES * 2 + 7)).map(|i| (i % 251) as u8).collect();
    let parts = chunks("cafe", &bytes);
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].0, "cafe.0");
    assert!(parts.iter().all(|(_, b64)| b64.len() <= 62_000), "each fits a sync op");
    let img = Image { sha256: "cafe".into(), kind: "image/png".into(), bytes: bytes.len() as u32, chunks: 3 };
    let map: BTreeMap<String, String> = parts.into_iter().collect();
    assert_eq!(join(&img, |k| map.get(k).cloned()), Some(bytes));
    assert_eq!(join(&img, |k| (k != "cafe.1").then(|| map[k].clone())), None, "a chunk still syncing");
    assert_eq!(image_kind(b"\x89PNG\r\n"), Some("image/png"));
    assert_eq!(image_kind(b"\xff\xd8\xff\xe0"), Some("image/jpeg"));
    assert_eq!(image_kind(b"RIFF\x0c\x00\x00\x00WEBPVP8L"), Some("image/webp"));
    assert_eq!(image_kind(b"%PDF-1.7"), None);
}

#[test]
fn youtube_links_of_every_kind_give_their_id() {
    for link in [
        "Ab3_xYz-012",
        "https://www.youtube.com/watch?v=Ab3_xYz-012",
        "https://youtube.com/watch?feature=share&v=Ab3_xYz-012&t=3",
        "https://m.youtube.com/watch?v=Ab3_xYz-012",
        "youtu.be/Ab3_xYz-012",
        "https://www.youtube.com/shorts/Ab3_xYz-012",
        "https://www.youtube.com/embed/Ab3_xYz-012?start=5",
    ] {
        assert_eq!(youtube_id(link).as_deref(), Some("Ab3_xYz-012"), "{link}");
    }
    for bad in ["https://vimeo.com/1", "https://youtube.com/watch?v=short", "javascript:alert(1)", ""] {
        assert_eq!(youtube_id(bad), None, "{bad}");
    }
}

#[test]
fn an_import_adds_new_ids_and_changes_edited_ones() {
    let (index, lectures) = book();
    let places = Places { index: &index, lectures: &lectures };
    let (old, _) = read(&[good(2, "P-1"), good(3, "P-2")], &places, &BTreeMap::new());
    let mut edited = good(3, "P-2");
    edited.fields.insert("stem".into(), "Edited stem".into());
    let (new, _) = read(&[good(2, "P-1"), edited, good(4, "P-3")], &places, &BTreeMap::new());
    let p = plan(&old, &new);
    assert_eq!(p.new, ["Pathoma/P-3"]);
    assert_eq!(p.changed, ["Pathoma/P-2"]);
    assert_eq!(p.same, ["Pathoma/P-1"]);
    assert_eq!(categories(&new), [("Pathoma".to_string(), 3)]);
    // Stored as JSON and back.
    let text = serde_json::to_string(&new[0]).unwrap();
    assert_eq!(serde_json::from_str::<OwnQuestion>(&text).unwrap(), new[0]);
}
