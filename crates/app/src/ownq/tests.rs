//! Reading files of his questions, and them in the library and on screen.

use std::io::Write as _;

use super::file::{self, FileProblem, Shape};
use super::import::{TEMPLATE, TEMPLATE_NAME, check};
use crate::content::Library;
use crate::render_tests::App;
use crate::{Screen, i18n};

/// A PNG header is all a figure needs here.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n0000";

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, bytes) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }
    out.into_inner()
}

const JSON: &str = r#"{"questions": [
  {"id": "U-1", "category": "UWorld notes", "lecture": "cvs.L03", "stem": "Which murmur is holosystolic at the apex?",
   "options": ["Aortic stenosis", "Mitral regurgitation"], "answer": "B",
   "why": ["Right upper sternal border.", "Right. Apex, to the axilla."],
   "video": {"youtube": "Ab3_xYz-012", "title": "Murmurs"}, "tags": ["murmurs"]},
  {"id": "U-2", "category": "UWorld notes", "page": 295, "stem": "Second?", "options": ["One", "Two"], "answer": "C"}
]}"#;

#[test]
fn the_template_with_its_figure_reads_with_no_problem() {
    let read = file::read(TEMPLATE_NAME, TEMPLATE);
    assert_eq!(read.shape, Some(Shape::Sheet));
    assert_eq!(read.rows.len(), 2);
    assert_eq!(read.rows[0].line, 2, "rows count as the sheet shows them");
    // Alone, the second example's figure is missing; in a zip with it, both stand.
    let alone = check(TEMPLATE_NAME, TEMPLATE);
    assert_eq!(alone.questions.len(), 1);
    assert_eq!(
        alone.problems,
        [format!(
            "الصف 3: {}",
            i18n::ownq::problem(&steps_core::ownq::Problem::NoImage("P-0002.png".into()))
        )]
    );
    let zipped =
        zip_of(&[("my questions/steps-questions-template.xlsx", TEMPLATE), ("my questions/P-0002.png", PNG)]);
    let both = check("questions.zip", &zipped);
    assert!(both.problems.is_empty(), "{:?}", both.problems);
    assert_eq!(both.questions.len(), 2);
    assert_eq!(both.plan.new.len(), 2);
    let q = &both.questions[0];
    assert_eq!((q.lecture.as_str(), q.topic.as_deref(), q.answer), ("cvs.L03", Some("heart-murmurs"), 1));
    assert_eq!(q.why.len(), 5);
    assert_eq!(both.questions[1].image.as_ref().map(|i| i.kind.as_str()), Some("image/png"));
    assert_eq!(both.questions[1].video.as_ref().map(|v| v.title.as_str()), Some("Iron studies"));
}

#[test]
fn json_and_yaml_take_lists_and_count_by_question() {
    let c = check("notes.json", JSON.as_bytes());
    assert_eq!(c.questions.len(), 1);
    assert_eq!(
        c.problems,
        [format!("السؤال 2: {}", i18n::ownq::problem(&steps_core::ownq::Problem::BadAnswer("C".into())))]
    );
    let q = &c.questions[0];
    assert_eq!(q.why, ["Right upper sternal border.", "Right. Apex, to the axilla."]);
    assert_eq!(q.video.as_ref().map(|v| v.youtube.as_str()), Some("Ab3_xYz-012"));
    assert_eq!(q.tags, ["murmurs"]);
    let yaml =
        "- id: Y-1\n  category: Mine\n  page: 295\n  stem: A stem?\n  options: [One, Two]\n  answer: a\n";
    let y = check("mine.yaml", yaml.as_bytes());
    assert!(y.problems.is_empty(), "{:?}", y.problems);
    assert_eq!(y.questions[0].answer, 0);
}

#[test]
fn questions_filed_under_mh_join_the_mh_bank_itself() {
    let json = r#"[{"id": "VQ-1315", "category": "mh", "lecture": "imm.L04", "topic": "Immunosuppressants",
        "stem": "Which drug binds cyclophilin?", "options": ["Cyclosporine", "Sirolimus"], "answer": "A"}]"#;
    let mut app = App::open(Screen::Today);
    let c = check("mh.json", json.as_bytes());
    assert!(c.problems.is_empty(), "{:?}", c.problems);
    let item = c.questions[0].item();
    assert_eq!(steps_core::exam::Set::of_tags(&item.tags), steps_core::exam::Set::Vq);
    crate::store::shared().put_own_questions(&c.questions, &[]).unwrap();
    Library::reload();
    app.run(|| *crate::content::REV.write() += 1);
    // The side menu's MH entry shows without the sealed bank's key, and the
    // bank has one MH set, not a second of the same name.
    assert!((crate::more::MH.shows)());
    app.tap_tab(Screen::Bank);
    app.tap(i18n::vq::SET);
    app.assert_has(i18n::vq::BANK_TITLE);
    let lectures: Vec<_> = Library::get().lectures.values().cloned().collect();
    let sets = steps_core::exam::Set::present(&steps_core::exam::mcqs(&lectures));
    assert_eq!(sets, [steps_core::exam::Set::Steps, steps_core::exam::Set::Vq], "no second MH set");
    app.assert_has(&i18n::bank::questions(1));
    // A lecture that isn't one says which the course has.
    let wrong = check("w.json", json.replace("imm.L04", "imm.L08").as_bytes());
    assert_eq!(wrong.problems.len(), 1);
    assert!(
        wrong.problems[0].contains("imm.L01") && wrong.problems[0].contains("imm.L04"),
        "{:?}",
        wrong.problems
    );
}

#[test]
fn a_file_that_isnt_one_says_what_is_wrong() {
    assert_eq!(file::read("notes.pdf", b"%PDF").problems, [FileProblem::Kind("notes.pdf".into())]);
    assert_eq!(file::read("q.zip", &zip_of(&[("a.txt", b"x")])).problems, [FileProblem::NoQuestions]);
    assert!(matches!(file::read("q.zip", b"not a zip").problems[..], [FileProblem::Unreadable(_)]));
    assert!(matches!(file::read("q.json", b"{nope").problems[..], [FileProblem::Unreadable(_)]));
    let big = vec![0x89u8; steps_core::ownq::MAX_IMAGE_BYTES + 1];
    let read =
        file::read("q.zip", &zip_of(&[("q.json", JSON.as_bytes()), ("big.png", &big), ("doc.gif", b"text")]));
    assert!(read.problems.contains(&FileProblem::ImageTooBig("big.png".into())), "{:?}", read.problems);
    assert!(read.problems.contains(&FileProblem::NotAnImage("doc.gif".into())), "{:?}", read.problems);
}

/// Adds the template's two questions, through the card's own `apply`.
fn add_template() {
    let review = dioxus::prelude::consume_context::<crate::store::Review>();
    let checked = check("t.zip", &zip_of(&[("t.xlsx", TEMPLATE), ("P-0002.png", PNG)]));
    assert_eq!(super::import::apply(&checked, &review), Ok((2, 0)));
}

#[test]
fn added_questions_join_their_lecture_as_a_bank_of_their_own() {
    let mut app = App::with(Screen::Today, add_template);
    let checked = check("t.zip", &zip_of(&[("t.xlsx", TEMPLATE), ("P-0002.png", PNG)]));
    assert!(
        checked.plan.new.is_empty() && checked.plan.changed.is_empty(),
        "the same file again changes nothing"
    );
    let lib = Library::get();
    let q = &checked.questions[0];
    let item = lib.lectures["cvs.L03"].items.iter().find(|i| i.id == q.item_id()).expect("in its lecture");
    assert_eq!(item.status, steps_core::content::Status::Approved);
    assert_eq!(super::name(&item.id).as_deref(), Some("Pathoma · P-0001"));
    let figure = super::image_url(&checked.questions[1].item_id()).unwrap();
    assert!(figure.starts_with("data:image/png;base64,"), "{figure}");

    // The bank: its own set, and Steps' questions without it.
    app.tap_tab(Screen::Bank);
    let set = i18n::ownq::set("Pathoma");
    app.tap(&set);
    app.assert_has(&i18n::ownq::bank_title("Pathoma"));
    app.assert_has(&i18n::bank::questions(2));

    // Quick search finds it by its id; answered, its explanation, objective,
    // video and chip show.
    app.run(|| crate::shortcuts::open_palette(true));
    app.type_into(i18n::keys::QS_TITLE, "p-0001");
    app.assert_has(i18n::ownq::KIND);
    app.tap("Pathoma · P-0001");
    app.assert_has(&q.stem);
    app.answer("Mitral regurgitation").tap(i18n::bank::CHECK);
    app.assert_has(i18n::ownq::EXPLANATION);
    app.assert_has("An apical holosystolic murmur radiating to the axilla is mitral regurgitation.");
    app.assert_has(q.objective.as_deref().unwrap());
    app.assert_has("Pathoma · P-0001");
    app.tap(i18n::bank::LEAVE);

    // Settings: the category with its count, and deleting it everywhere.
    app.tap(Screen::Settings.label());
    app.tap(i18n::settings::DATA);
    app.assert_has(i18n::ownq::TITLE);
    app.assert_has(&set);
    app.tap(i18n::ownq::DELETE);
    app.assert_has(i18n::ownq::DELETE_SURE);
    app.tap(i18n::ownq::DELETE_YES);
    app.assert_has(&i18n::ownq::deleted(2));
    assert!(Library::get().mine.get(&q.item_id()).is_none());
    assert!(crate::store::shared().own_questions().is_empty());
}
