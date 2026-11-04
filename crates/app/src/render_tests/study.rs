//! The study player over the First Aid cardiovascular content: setup,
//! each kind of item, grades saved to the review store, undo, retries,
//! the "study these" question and the summary.

use steps_core::content::Body;
use steps_core::review::{Grade, Origin};

use super::App;
use super::kit::{first_of, ids_of, item, reviews};
use crate::Screen;
use crate::i18n::study as tx;
use crate::study::{START, Start};

/// The Study screen running a session on exactly `ids`.
fn session(ids: &[&str]) -> App {
    let mut app = App::open(Screen::Study);
    let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    app.run(move || *START.write() = Some(Start { lectures: None, items: Some(ids), minutes: 5 }));
    app
}

fn grade_label(g: Grade) -> &'static str {
    crate::study::grade::grade_label(g)
}

fn front_back(id: &str) -> (String, String) {
    match item(id).body {
        Body::Card { front, back } => (front, back),
        _ => panic!("{id} is not a card"),
    }
}

#[test]
fn a_card_is_revealed_graded_and_saved_as_a_study_review() {
    let card = first_of("card");
    let (front, back) = front_back(&card.id);
    let mut app = session(&[&card.id]);
    app.assert_has(&front);
    app.assert_has(tx::THINK_FIRST);
    app.assert_has(tx::BADGE_NEW);
    app.assert_has(tx::UNVERIFIED);
    app.assert_lacks(&back);
    assert!(!app.can_tap(grade_label(Grade::Good)), "no grades before the answer");

    app.tap(tx::REVEAL);
    app.assert_has(&back);
    app.assert_has(tx::GRADE_LABEL);
    app.assert_has(tx::GRADE_HINT);
    // The book quote opens once the answer is out.
    app.assert_has(tx::BOOK_QUOTE);
    assert!(reviews().is_empty());

    app.tap(grade_label(Grade::Good));
    let saved = reviews();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].event.item_id, card.id);
    assert_eq!(saved[0].event.grade, Grade::Good);
    assert_eq!(saved[0].event.origin, Origin::Study);

    // One item: the session ends on its summary, which takes the focus.
    app.assert_has(tx::SUMMARY_TITLE);
    assert!(app.mount_events() > 0);
    app.assert_has(tx::SUM_ALL_SOLID);
    app.assert_has(tx::NEXT_SESSION);
    assert!(app.can_tap(tx::UNDO_LAST));
}

#[test]
fn again_brings_the_card_back_as_a_retry_and_onto_the_summary_list() {
    let card = first_of("card");
    let (front, _) = front_back(&card.id);
    let mut app = session(&[&card.id]);
    app.tap(tx::REVEAL).tap(grade_label(Grade::Again));
    // Same card again, marked as a retry.
    app.assert_has(tx::BADGE_RETRY);
    app.assert_has(&front);
    app.tap(tx::REVEAL).tap(grade_label(Grade::Good));
    let grades: Vec<Grade> = reviews().iter().map(|r| r.event.grade).collect();
    assert_eq!(grades, [Grade::Again, Grade::Good]);
    app.assert_has(tx::SUMMARY_TITLE);
    app.assert_has(tx::SUM_BACK_SOON);
    app.assert_lacks(tx::SUM_ALL_SOLID);
}

#[test]
fn undo_takes_the_grade_back_and_reopens_the_card_with_its_answer() {
    let cards = ids_of("card");
    let (a, b) = (&cards[0], &cards[1]);
    let (_, back_a) = front_back(a);
    let (front_b, _) = front_back(b);
    let mut app = session(&[a, b]);
    app.assert_has("1 / 2");
    app.tap(tx::REVEAL).tap(grade_label(Grade::Hard));
    assert_eq!(reviews().len(), 1);
    app.assert_has(&front_b);
    app.assert_has("2 / 2");

    app.tap(tx::UNDO_HINT);
    assert!(reviews().is_empty(), "the undone answer is no longer in the log");
    app.assert_has(tx::UNDONE);
    // Back on the first card, its answer already showing.
    app.assert_has(&back_a);
    app.assert_has("1 / 2");
    app.tap(grade_label(Grade::Easy));
    assert_eq!(reviews()[0].event.grade, Grade::Easy);
}

#[test]
fn undo_on_the_summary_reopens_the_last_item() {
    let card = first_of("card");
    let (_, back) = front_back(&card.id);
    let mut app = session(&[&card.id]);
    app.tap(tx::REVEAL).tap(grade_label(Grade::Good));
    app.assert_has(tx::SUMMARY_TITLE);
    app.tap(tx::UNDO_LAST);
    assert!(reviews().is_empty());
    app.assert_has(&back);
    app.assert_has(tx::UNDONE);
}

#[test]
fn ending_early_shows_the_summary_and_another_session_goes_back_to_setup() {
    let cards = ids_of("card");
    let mut app = session(&[&cards[0], &cards[1]]);
    app.tap(tx::END);
    app.assert_has(tx::SUMMARY_EMPTY);
    app.assert_has(tx::NEXT_TOMORROW);
    app.tap(tx::ANOTHER);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::LECTURES);
}

#[test]
fn ids_the_player_cannot_show_end_on_an_empty_summary() {
    let mcq = first_of("mcq");
    let app = session(&[&mcq.id, "cvs.L99.001"]);
    app.assert_has(tx::SUMMARY_EMPTY);
    assert!(!app.can_tap(tx::UNDO_LAST));
}

#[test]
fn the_setup_starts_a_session_from_the_ready_lectures() {
    let mut app = App::open(Screen::Study);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::TIME);
    // A ready lecture whose title no other lecture shares, so its chip is
    // the only one with that name.
    let lib = crate::content::Library::get();
    let lecture = lib
        .lectures
        .keys()
        .find(|id| {
            let t = lib.title(id);
            !lib.lectures[*id].topics.is_empty()
                && lib.lectures.keys().filter(|o| lib.title(o) == t).count() == 1
        })
        .expect("a ready lecture with a unique title")
        .clone();
    let title = lib.title(&lecture).to_string();
    app.assert_has(&title);
    app.assert_has(tx::CLEAR_ALL);

    // Clearing every lecture leaves nothing to study.
    app.tap(tx::CLEAR_ALL);
    app.assert_has(tx::PICK_ALL);
    app.assert_has(tx::NOTHING_NOW);
    assert!(!app.can_tap(tx::START));
    // Picking the lecture back shows its topics.
    app.tap(&title);
    app.assert_has(tx::TOPICS);
    let topic = lib.lectures[&lecture].topics[0].title.clone();
    app.tap(&topic);
    app.tap(&topic);
    // One lecture of several: the button picks them all, then clears.
    app.tap(tx::PICK_ALL);
    app.assert_has(tx::CLEAR_ALL);

    // A time choice changes the plan; the preview counts it.
    let twenty = crate::i18n::count::count(20, &crate::i18n::count::MINUTE);
    app.tap(&twenty);
    app.assert_has(tx::ABOUT);
    app.tap(tx::START);
    app.assert_has(tx::END);
    app.assert_has(tx::BADGE_NEW);
    assert!(app.html().contains("st-progress"));
}

#[test]
fn a_topic_narrows_the_session_to_its_items() {
    let mut app = App::open(Screen::Study);
    let lecture = &crate::content::Library::get().lectures["cvs.L01"];
    let last = lecture.topics.last().unwrap();
    // Only cvs.L01 picked, so its topics show.
    app.tap(tx::CLEAR_ALL);
    app.tap(crate::content::Library::get().title("cvs.L01"));
    app.tap(&last.title);
    app.tap(tx::START);
    let shown = app.text();
    let in_topic = lecture
        .items
        .iter()
        .filter(|i| i.topic.as_deref() == Some(last.id.as_str()))
        .filter(|i| crate::study::PLAYABLE.contains(&i.body.kind()))
        .any(|i| match &i.body {
            Body::Card { front, .. } => shown.contains(front.as_str()),
            Body::Enumerate { prompt, .. } | Body::Order { prompt, .. } => shown.contains(prompt.as_str()),
            Body::Table { title, .. } => shown.contains(title.as_str()),
            Body::Cloze { .. } => shown.contains(tx::KIND_CLOZE),
            _ => false,
        });
    assert!(in_topic, "the first item is from {}:\n{shown}", last.title);
}

#[test]
fn a_cloze_typed_right_is_marked_right_and_suggests_a_grade() {
    let cloze = first_of("cloze");
    let Body::Cloze { text, .. } = &cloze.body else { unreachable!() };
    // Each blank's answer, in order.
    let answers: Vec<String> = text
        .split("{{")
        .skip(1)
        .map(|s| s.split("}}").next().unwrap().split("::").nth(1).unwrap().to_string())
        .collect();
    let mut app = session(&[&cloze.id]);
    app.assert_has(tx::KIND_CLOZE);
    for (i, a) in answers.iter().enumerate() {
        app.type_into(&format!("{} {}", tx::BLANK, i + 1), a);
    }
    app.press(&format!("{} 1", tx::BLANK), "Enter");
    app.assert_has(tx::CLOZE_RIGHT);
    app.assert_has(tx::SUGGESTED);
    app.assert_has(tx::SUGGESTED_SHORT);
    app.tap(grade_label(Grade::Good));
    assert_eq!(reviews()[0].event.item_id, cloze.id);
}

#[test]
fn a_cloze_left_blank_and_revealed_is_marked_wrong() {
    let cloze = first_of("cloze");
    let mut app = session(&[&cloze.id]);
    app.type_into(&format!("{} 1", tx::BLANK), "nope");
    app.tap(tx::REVEAL_BLANKS);
    app.assert_has(tx::CLOZE_WRONG);
    // What he typed stays on screen, struck through.
    app.assert_has("nope");
    app.tap(grade_label(Grade::Again));
    assert_eq!(reviews()[0].event.grade, Grade::Again);
}

#[test]
fn a_cloze_with_a_number_has_the_numbers_tip_and_checks_by_button() {
    use crate::study::cloze::{Seg, segments};
    let numeric = ids_of("cloze").into_iter().find(|id| match item(id).body {
        Body::Cloze { text, .. } => segments(&text)
            .iter()
            .any(|s| matches!(s, Seg::Blank { answer, .. } if answer.chars().any(|c| c.is_ascii_digit()))),
        _ => false,
    });
    let Some(id) = numeric else { return };
    let mut app = session(&[&id]);
    app.assert_has(tx::CLOZE_TIP);
    app.tap(tx::CHECK);
    app.assert_has(tx::CLOZE_WRONG);
}

#[test]
fn an_enumeration_pre_ticks_what_he_typed_and_grades() {
    let en = first_of("enumerate");
    let Body::Enumerate { points, pass, hook, .. } = &en.body else { unreachable!() };
    let mut app = session(&[&en.id]);
    app.assert_has(&tx::enum_goal(points.len(), *pass));
    app.type_into(tx::ENUM_PLACEHOLDER, &points[0]);
    app.tap(tx::SHOW_POINTS);
    app.assert_has(tx::WHAT_I_WROTE);
    app.assert_has(tx::PRE_TICKED);
    for p in points {
        app.assert_has(p);
    }
    if let Some(h) = hook {
        app.assert_has(tx::ENUM_HOOK);
        app.assert_has(h);
    }
    // Tick every point: enough to pass.
    for i in 0..points.len() {
        app.form_nth("change", "", i, "true");
    }
    app.assert_has(tx::ENUM_PASS);
    // Untick one: short of the pass mark.
    app.form_nth("change", "", 0, "false");
    if points.len() - 1 < *pass {
        app.assert_has(tx::ENUM_SHORT_A);
    }
    app.tap(grade_label(Grade::Hard));
    assert_eq!(reviews()[0].event.grade, Grade::Hard);
}

#[test]
fn an_enumeration_opened_blank_shows_how_many_are_left() {
    let en = first_of("enumerate");
    let mut app = session(&[&en.id]);
    app.tap(tx::SHOW_POINTS);
    app.assert_lacks(tx::WHAT_I_WROTE);
    app.assert_has(tx::TICK_WHAT_YOU_GOT);
    app.assert_has(tx::ENUM_SHORT_A);
}

#[test]
fn a_table_is_revealed_cell_by_cell_marked_and_graded() {
    use crate::i18n::table as tb;
    let table = first_of("table");
    let Body::Table { title, rows, blanks, .. } = &table.body else { unreachable!() };
    let mut app = session(&[&table.id]);
    app.assert_has(title);
    app.assert_has(tb::HINT_TAP);
    let first = &rows[blanks[0][0]][blanks[0][1]];
    app.assert_lacks(first);
    app.tap(tb::REVEAL_CELL);
    app.assert_has(first);
    app.tap(tb::KNEW_IT);
    // Flip it to wrong and back.
    app.tap(tb::MARK_WRONG);
    app.tap(tb::MARK_RIGHT);
    // The rest at once, then each marked.
    app.tap(tb::REVEAL_ALL);
    while app.tap_if(tb::MISSED_IT) {}
    app.assert_has(&tb::recalled(1, blanks.len()));
    app.tap(grade_label(Grade::Hard));
    assert_eq!(reviews()[0].event.item_id, table.id);
}

#[test]
fn a_table_typed_in_is_checked_against_the_book() {
    use crate::i18n::table as tb;
    let table = first_of("table");
    let Body::Table { rows, blanks, .. } = &table.body else { unreachable!() };
    let mut app = session(&[&table.id]);
    app.tap(tb::TYPING_ON);
    app.assert_has(tb::HINT_TYPE);
    let answer = &rows[blanks[0][0]][blanks[0][1]];
    // Short answers get a box; long ones stay a tap to reveal.
    let typable = app.count("input", tb::TYPE_CELL);
    assert!(typable >= 1);
    app.form_nth("input", tb::TYPE_CELL, 0, answer);
    if typable > 1 {
        app.form_nth("input", tb::TYPE_CELL, 1, "zzz");
    }
    app.tap(tb::CHECK_TYPED);
    if typable > 1 {
        // The wrong guess shows struck through beside the answer.
        app.assert_has("zzz");
    }
    // Cells shown without typing wait for his own mark.
    while app.tap_if(tb::KNEW_IT) {}
    app.assert_has(&tb::recalled(blanks.len() - usize::from(typable > 1), blanks.len()));
    app.tap(grade_label(Grade::Good));
    assert_eq!(reviews().len(), 1);
    // Typing off again is offered before a check.
    let mut app = session(&[&table.id]);
    app.tap(tb::TYPING_ON).tap(tb::TYPING_OFF);
    app.assert_has(tb::HINT_TAP);
}

#[test]
fn an_order_is_placed_moved_checked_and_graded() {
    use crate::i18n::order as or;
    let order = first_of("order");
    let Body::Order { prompt, steps } = &order.body else { unreachable!() };
    let mut app = session(&[&order.id]);
    app.assert_has(prompt);
    app.assert_has(or::HINT);
    // Place one, send it back, place again, clear.
    app.tap(&steps[1]);
    app.tap(or::SEND_BACK);
    app.tap(&steps[0]);
    app.tap(or::CLEAR);
    // Place them in book order, then swap the first two and swap them back.
    for s in steps {
        app.tap(s);
    }
    app.tap_nth(or::MOVE_DOWN, 0);
    app.tap_nth(or::MOVE_UP, 0); // the first row's "up" is disabled
    app.tap(or::CHECK);
    app.assert_has(or::ALL_IN_PLACE);
    app.assert_has(or::CORRECT_ORDER);
    app.tap(grade_label(Grade::Easy));
    assert_eq!(reviews()[0].event.grade, Grade::Easy);
}

#[test]
fn an_order_out_of_place_says_which_steps_moved() {
    use crate::i18n::order as or;
    let order = first_of("order");
    let Body::Order { steps, .. } = &order.body else { unreachable!() };
    let mut app = session(&[&order.id]);
    for s in steps.iter().rev() {
        app.tap(s);
    }
    app.tap(or::CHECK);
    app.assert_has(or::YOUR_ORDER);
    app.assert_lacks(or::ALL_IN_PLACE);
    app.assert_has(&or::should_be(steps.len()));
}

#[test]
fn study_these_during_a_session_asks_first() {
    let cards = ids_of("card");
    let (front_c, _) = front_back(&cards[2]);
    let mut app = session(&[&cards[0], &cards[1]]);
    let more = vec![cards[2].clone()];
    let ask = move || *START.write() = Some(Start { lectures: None, items: Some(more.clone()), minutes: 5 });

    // Keep: the question goes, the session stays.
    app.run(ask.clone());
    app.assert_has(tx::PENDING_TITLE);
    app.tap(tx::PENDING_KEEP);
    app.assert_lacks(tx::PENDING_TITLE);
    app.assert_has("1 / 2");

    // Add: the item joins the end.
    app.run(ask.clone());
    app.tap(tx::PENDING_ADD);
    app.assert_has("1 / 3");

    // Replace: a new session on just that card.
    app.run(ask);
    app.tap(tx::PENDING_REPLACE);
    app.assert_has("1 / 1");
    app.assert_has(&front_c);
}

#[test]
fn a_start_for_lectures_plans_from_them() {
    let mut app = App::open(Screen::Study);
    app.run(|| {
        *START.write() = Some(Start { lectures: Some(vec!["cvs.L01".into()]), items: None, minutes: 10 })
    });
    app.assert_has(tx::END);
    app.assert_has(tx::BADGE_NEW);
    // A second request after it ends simply starts.
    app.tap(tx::END);
    app.run(|| {
        *START.write() = Some(Start { lectures: Some(vec!["cvs.L01".into()]), items: None, minutes: 5 })
    });
    app.assert_has(tx::END);
}

#[test]
fn a_reviewed_card_shows_as_a_review_with_intervals() {
    let card = first_of("card");
    let mut app = session(&[&card.id]);
    app.tap(tx::REVEAL).tap(grade_label(Grade::Good));
    drop(app);
    let mut app2 = session(&[&card.id]);
    app2.assert_has(tx::BADGE_REVIEW);
    app2.tap(tx::REVEAL);
    // Each grade says when it brings the card back.
    assert!(app2.html().contains("rating-when"));
}

#[test]
fn the_session_survives_leaving_the_screen() {
    let cards = ids_of("card");
    let (front, _) = front_back(&cards[0]);
    let mut app = session(&[&cards[0], &cards[1]]);
    app.tap(Screen::Today.label());
    app.tap(Screen::Study.label());
    app.assert_has(&front);
    app.assert_has(tx::END);
}

#[test]
fn verifying_from_the_grade_row_hides_the_check_next_time() {
    use crate::i18n::verify as vx;
    let card = first_of("card");
    let mut app = session(&[&card.id]);
    app.tap(tx::REVEAL);
    app.assert_has(vx::QUESTION);
    app.tap(vx::RIGHT);
    app.assert_has(vx::SAVED_APPROVED);
    app.tap(grade_label(Grade::Good));
    drop(app);
    let mut again = session(&[&card.id]);
    again.tap(tx::REVEAL);
    again.assert_lacks(vx::QUESTION);
}

// ---- Kinds no bundled lecture has yet, drawn through the item frame ----

thread_local! {
    static GRADED: std::cell::RefCell<Vec<Grade>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn fixture(id: &str) -> steps_core::content::Item {
    let lec: steps_core::content::Lecture =
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
    lec.items.into_iter().find(|i| i.id == id).unwrap()
}

fn framed(item: steps_core::content::Item) -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    rsx! {
        crate::study::ItemView {
            item,
            lecture_title: "Pathology".to_string(),
            topic_title: Some("Aortic dissection".to_string()),
            fresh: true,
            retry: false,
            on_grade: move |g: Grade| GRADED.with(|v| v.borrow_mut().push(g)),
        }
    }
}

fn essay_part() -> dioxus::prelude::Element {
    framed(fixture("cvs.L04.005"))
}

fn mcq_part() -> dioxus::prelude::Element {
    framed(first_of("mcq"))
}

fn updated_card_part() -> dioxus::prelude::Element {
    framed(
        serde_yaml_ng::from_str(
            r#"
id: cvs.L01.902
kind: card
front: What can a patent foramen ovale cause?
back: Paradoxical emboli.
src: { page: 284 }
status: unverified
update: { text: "PFO closure is advised for selected patients under 60 with cryptogenic stroke.", ref: "AAN practice advisory, 2020" }
"#,
        )
        .unwrap(),
    )
}

fn graded() -> Vec<Grade> {
    GRADED.with(|v| v.borrow().clone())
}

#[test]
fn an_essay_is_written_compared_with_the_outline_and_rated() {
    let mut app = App::with_part(Screen::Today, essay_part);
    app.assert_has("Describe aortic dissection.");
    app.assert_has(tx::ESSAY_HOW);
    app.assert_has(tx::KIND_ESSAY);
    app.assert_has("Pathology · Aortic dissection");
    app.type_into(tx::ESSAY_PLACEHOLDER, "Risk factors\nPresentation");
    app.tap(tx::COMPARE);
    app.assert_has(tx::MODEL_OUTLINE);
    app.assert_has(tx::WHAT_I_WROTE);
    app.assert_has("Stanford classification");
    app.assert_has("Marfan syndrome");
    app.assert_has(&format!("{} 2 {} 4", tx::HEADINGS_GOT, tx::OF));
    // Ticks change the count.
    app.form_nth("change", "", 3, "true");
    app.assert_has(&format!("{} 3 {} 4", tx::HEADINGS_GOT, tx::OF));
    app.tap(grade_label(Grade::Good));
    assert_eq!(graded(), [Grade::Good]);
}

#[test]
fn an_essay_compared_blank_shows_the_outline_alone() {
    let mut app = App::with_part(Screen::Today, essay_part);
    app.tap(tx::COMPARE);
    app.assert_lacks(tx::WHAT_I_WROTE);
    app.assert_has(&format!("{} 0 {} 4", tx::HEADINGS_GOT, tx::OF));
}

#[test]
fn a_kind_the_player_cannot_show_can_be_skipped() {
    let mut app = App::with_part(Screen::Today, mcq_part);
    app.assert_has(tx::NOT_HERE_YET);
    app.assert_has(tx::KIND_MCQ);
    app.tap(tx::SKIP);
    assert_eq!(graded(), [Grade::Good]);
}

#[test]
fn a_newer_source_shows_under_the_item_and_the_book_still_wins() {
    let app = App::with_part(Screen::Today, updated_card_part);
    app.assert_has(tx::UPDATE_TITLE);
    app.assert_has(tx::UPDATE_AFTER);
    app.assert_has(tx::UPDATE_BOOK_WINS);
    app.assert_has("AAN practice advisory, 2020");
}

#[test]
fn an_order_step_is_dragged_into_place() {
    use crate::i18n::order as or;
    let order = first_of("order");
    let Body::Order { steps, .. } = &order.body else { unreachable!() };
    let mut app = session(&[&order.id]);
    for s in steps.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev() {
        app.tap(s);
    }
    // Placed: [n-2, n-1]. Drag the second onto the first: [n-1, n-2].
    app.drag("dragstart", "or-step", 1);
    app.drag("dragover", "or-step", 0);
    app.drag("drop", "or-step", 0);
    app.drag("dragend", "or-step", 0);
    let text = app.text();
    let a = text.find(steps[steps.len() - 1].as_str()).unwrap();
    let b = text.find(steps[steps.len() - 2].as_str()).unwrap();
    assert!(a < b, "the last step now comes first");
    app.assert_has(or::HINT);
}

#[test]
fn a_card_due_in_another_course_is_counted_and_all_courses_bring_it_in() {
    use steps_core::review::ReviewEvent;
    let lib = crate::content::Library::get();
    let card = lib.lectures["cvs.L01"].items.iter().find(|i| i.body.kind() == "card").unwrap().clone();
    // Another written course is in focus, so the cvs card is due elsewhere.
    let Some(other) = lib.lectures.keys().map(|id| id.split('.').next().unwrap()).find(|c| *c != "cvs")
    else {
        return;
    };
    let other = other.to_string();
    let day = 86_400_000;
    // Missed two days ago: due again now.
    crate::store::shared()
        .append_review(ReviewEvent {
            confidence: None,
            reason: None,
            item_id: card.id.clone(),
            grade: Grade::Again,
            origin: Origin::Study,
            at_ms: crate::store::now_ms() - 2 * day,
            took_ms: 5_000,
        })
        .unwrap();
    // One of his own cards, so the setup has a deck in every course.
    crate::store::shared()
        .put_own_card(&steps_core::own::OwnCard {
            id: "own.1".into(),
            front: "Where do the mitral and tricuspid valves come from?".into(),
            back: "The fused endocardial cushions of the AV canal.".into(),
            lecture: None,
            page: None,
            source: None,
            created_ms: 1,
        })
        .unwrap();
    crate::store::shared().put_setting("course_focus", &other).unwrap();
    let mut app = App::open(Screen::Study);
    app.assert_has(&tx::due_elsewhere(1));
    app.tap(tx::ELSEWHERE_ALL);
    app.assert_lacks(&tx::due_elsewhere(1));
    let title = crate::content::Library::get().title("cvs.L01").to_string();
    app.assert_has(&title);
}

#[test]
fn a_course_with_nothing_ready_says_so() {
    let Some(empty) = super::kit::empty_course() else { return };
    crate::store::shared().put_setting("course_focus", &empty).unwrap();
    let app = App::open(Screen::Study);
    app.assert_has(tx::EMPTY_LIBRARY);
    app.assert_has(crate::i18n::own::TITLE);
}

#[test]
fn coming_back_to_a_summary_keeps_it() {
    let card = first_of("card");
    let mut app = session(&[&card.id]);
    app.tap(tx::REVEAL).tap(grade_label(Grade::Good));
    app.tap(Screen::Today.label());
    app.tap(Screen::Study.label());
    app.assert_has(tx::SUMMARY_TITLE);
}
