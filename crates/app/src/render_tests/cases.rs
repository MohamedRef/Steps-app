//! Cases: no case is written in the bundled content yet, so these tests add
//! a First Aid cardiovascular case for their thread (`case::tests::add_case`)
//! and walk it from the list through every step to the recap.

use steps_core::content::Item;
use steps_core::review::{Grade, Origin};

use super::App;
use super::kit::reviews;
use crate::Screen;
use crate::i18n::case as tx;

const LECTURE: &str = "Embryology";

/// Two steps: one with options, one he grades himself; and a note from a
/// newer source.
fn paradoxical_embolism() -> Item {
    serde_yaml_ng::from_str(
        r#"
id: cvs.L01.901
kind: case
topic: heart-morphogenesis
stem: A 34-year-old woman has sudden right arm weakness two weeks after a 14-hour flight; her left calf is swollen.
steps:
  - reveal: "Bubble study: bubbles reach the left atrium within three beats after Valsalva."
    question: Failure of which embryologic event explains her stroke?
    options: [Fusion of septum primum and septum secundum, Fusion of the endocardial cushions, Spiraling of the aorticopulmonary septum, Closure of the ductus arteriosus]
    answer: 0
    why: Unfused septa leave a patent foramen ovale, so a venous clot crosses right to left.
  - question: What is this kind of embolus called?
    why: A venous thromboembolus entering the systemic arteries is a paradoxical embolus.
src: { page: 284, quote: "Can lead to paradoxical emboli" }
status: unverified
update: { text: "Closure is considered for selected young patients with cryptogenic stroke.", ref: "AHA/ASA stroke guideline, 2021" }
"#,
    )
    .expect("the case parses")
}

#[test]
fn with_no_case_written_the_list_says_so() {
    let app = App::open(Screen::Cases);
    app.assert_has(tx::EMPTY);
}

#[test]
fn the_list_shows_the_case_with_its_steps_and_lecture() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let app = App::open(Screen::Cases);
    app.assert_has("A 34-year-old woman");
    app.assert_has(&tx::steps(2));
    app.assert_has(LECTURE);
    app.assert_lacks(tx::EMPTY);
}

#[test]
fn a_case_is_stepped_through_checked_and_saved_as_one_review() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap("A 34-year-old woman");
    // The player: back, lecture, unverified, the stem and the first reveal.
    app.assert_has(tx::BACK);
    app.assert_has(tx::UNVERIFIED);
    app.assert_has(tx::KNOWN);
    app.assert_has(tx::NEW_DATA);
    app.assert_has("Bubble study");
    app.assert_has("1 / 2");
    app.assert_has(tx::PICK_FIRST);
    // The stepper's current step scrolls to the question.
    app.tap(tx::NOW);

    // A wrong pick, changed once before checking.
    app.tap("Spiraling of the aorticopulmonary septum");
    app.tap("Fusion of the endocardial cushions");
    app.assert_lacks(tx::PICK_FIRST);
    app.tap(tx::CHECK);
    app.assert_has(tx::WRONG);
    app.assert_has(tx::MARK_RIGHT);
    app.assert_has(tx::MARK_YOURS);
    app.assert_has("patent foramen ovale");
    assert!(reviews().is_empty(), "a case is saved once, at the end");
    app.tap(tx::NEXT);

    // Step 2 has no options: show the answer, then grade himself.
    app.assert_has("2 / 2");
    app.assert_has(tx::CORRECT_WAS);
    app.tap(tx::SHOW_ANSWER);
    app.assert_has("paradoxical embolus");
    app.tap(tx::GOT_IT);
    app.tap(tx::FINISH);

    // The recap: one of two right is Hard.
    app.assert_has("1 / 2");
    app.assert_has(tx::SCORE);
    app.assert_has(tx::SOME_RIGHT);
    app.assert_has(tx::RECAP);
    app.assert_has(tx::YOURS);
    app.assert_has("AHA/ASA stroke guideline, 2021");
    let saved = reviews();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].event.item_id, "cvs.L01.901");
    assert_eq!(saved[0].event.origin, Origin::Case);
    assert_eq!(saved[0].event.grade, Grade::Hard);

    // Again starts it over from step 1.
    app.tap(tx::AGAIN);
    app.assert_has("1 / 2");
    app.assert_has(tx::PICK_FIRST);
}

#[test]
fn every_step_right_is_good_and_back_returns_to_the_list() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap("A 34-year-old woman");
    app.tap("Fusion of septum primum and septum secundum");
    app.tap(tx::CHECK);
    app.assert_has(tx::RIGHT);
    app.tap(tx::NEXT);
    app.tap(tx::SHOW_ANSWER);
    app.tap(tx::GOT_IT);
    app.tap(tx::FINISH);
    app.assert_has("2 / 2");
    app.assert_has(tx::ALL_RIGHT);
    assert_eq!(reviews()[0].event.grade, Grade::Good);
    app.tap(tx::BACK);
    app.assert_has(&tx::steps(2));
}

#[test]
fn every_step_missed_is_again() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap("A 34-year-old woman");
    app.tap("Closure of the ductus arteriosus");
    app.tap(tx::CHECK);
    app.tap(tx::NEXT);
    app.tap(tx::SHOW_ANSWER);
    app.tap(tx::MISSED);
    app.tap(tx::FINISH);
    app.assert_has(tx::FEW_RIGHT);
    app.assert_has("0 / 2");
    assert_eq!(reviews()[0].event.grade, Grade::Again);
}

#[test]
fn a_case_left_half_way_opens_where_it_was() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap("A 34-year-old woman");
    app.tap("Fusion of septum primum and septum secundum");
    app.tap(tx::CHECK);
    app.tap(tx::NEXT);
    // Away to Today and back: still on step 2, step 1 in the data as done.
    app.tap(Screen::Today.label());
    app.tap(Screen::Cases.label());
    app.assert_has("2 / 2");
    app.assert_has("Failure of which embryologic event");
    // The back button keeps it too.
    app.tap(tx::BACK);
    app.tap("A 34-year-old woman");
    app.assert_has("2 / 2");
    // The done step in the stepper scrolls to its data.
    app.tap("✓");
}

#[test]
fn the_case_list_can_be_grouped_by_lecture_and_sorted_by_book() {
    use crate::i18n::views as vx;
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap(vx::SORT_TITLE).tap(vx::GROUPED);
    app.assert_has("A 34-year-old woman");
    assert!(app.html().contains("class=\"coll-group\""), "a heading per lecture");
    app.tap(vx::SORT_TITLE).tap(vx::GROUPED);
    assert!(!app.html().contains("class=\"coll-group\""));
}

fn due_button() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let ui = use_context::<crate::Ui>();
    rsx! { button { class: "test-due", onclick: move |_| crate::case::open_due(ui), "open due cases" } }
}

#[test]
fn open_due_lands_on_the_list_from_a_case_left_open() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::with_part(Screen::Cases, due_button);
    app.tap("A 34-year-old woman");
    app.assert_has(tx::KNOWN);
    app.tap(Screen::Today.label());
    app.tap("open due cases");
    assert!(app.html().contains("data-screen=\"Cases\""));
    app.assert_lacks(tx::KNOWN);
    app.assert_has(&tx::steps(2));
}

thread_local! {
    static PLAYING: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn player_part() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let id = PLAYING.with(|p| p.borrow().clone());
    rsx! { div { class: "test-player", crate::case::CasePlayer { item_id: id, on_back: |_| {} } } }
}

#[test]
fn the_player_draws_nothing_for_an_id_that_is_not_a_case() {
    for id in ["cvs.L01.999", "cvs.L01.001"] {
        PLAYING.with(|p| *p.borrow_mut() = id.to_string());
        let app = App::with_part(Screen::Today, player_part);
        assert!(!app.html().contains("case-stepper"), "{id}");
    }
}

#[test]
fn the_feedback_scrolls_into_view_when_it_opens() {
    crate::case::tests::add_case(LECTURE, paradoxical_embolism());
    let mut app = App::open(Screen::Cases);
    app.tap("A 34-year-old woman");
    app.tap("Fusion of septum primum and septum secundum");
    app.tap(tx::CHECK);
    assert!(app.mount_events() > 0);
    app.assert_has(tx::RIGHT);
}
