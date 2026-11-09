//! Learn: the lesson list, opening a lesson, playing every kind of step
//! (reveal, mnemonic, figure, table, check, order, match, sort, mini case,
//! the lecture's own cards) and reaching the summary.
//!
//! The bundled lessons don't hold every step kind, so most step tests play
//! a lesson written here from First Aid's cardiovascular embryology
//! (`lesson()`), through the real player inside the whole app.

use std::cell::RefCell;

use dioxus::prelude::*;
use steps_core::lesson::Lesson;

use super::App;
use super::dom::Dom;
use crate::content::Library;
use crate::i18n::learn as tx;
use crate::{AppAt, Screen};

thread_local! {
    /// The lesson the player under test shows, for this test thread.
    static LESSON: RefCell<Option<Lesson>> = const { RefCell::new(None) };
    /// The app's review state, kept by the test child for the test to read.
    static REVIEW: RefCell<Option<crate::store::Review>> = const { RefCell::new(None) };
}

#[allow(non_snake_case)]
fn TestPlayer() -> Element {
    use_hook(|| REVIEW.with_borrow_mut(|r| *r = Some(consume_context::<crate::store::Review>())));
    match LESSON.with_borrow(Clone::clone) {
        Some(l) => crate::learn::test_player(l),
        None => rsx! {},
    }
}

fn player_root() -> Element {
    // Cases has little of its own on an empty store, so the player's
    // buttons are the ones a test finds.
    rsx! { AppAt { start: Screen::Cases, TestPlayer {} } }
}

/// The whole app with the player on `lesson` drawn over it.
pub(super) fn play(lesson: Lesson) -> App {
    LESSON.with_borrow_mut(|l| *l = Some(lesson));
    mount(player_root)
}

/// The app from `root` (an `AppAt` with children), onboarding done.
pub(super) fn mount(root: fn() -> Element) -> App {
    dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let dir = crate::prefs::test_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    crate::onboarding::mark_done_for_tests();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut vdom = VirtualDom::new(root);
    let mut page = Dom::default();
    {
        let _rt = rt.enter();
        vdom.rebuild(&mut page);
    }
    let mut app = App { vdom, page, rt };
    app.settle();
    app
}

/// Two recall cards of a ready lecture, for the lesson's try step.
fn two_cards() -> Vec<String> {
    let lib = Library::get();
    lib.index
        .lectures
        .iter()
        .filter_map(|e| lib.lectures.get(&e.id))
        .flat_map(|l| l.items.iter())
        .filter(|i| i.body.kind() == "card")
        .take(2)
        .map(|i| i.id.clone())
        .collect()
}

pub(super) const ID: &str = "cvs.L01.test";

/// A lesson with one step of every kind, on First Aid's heart embryology.
pub(super) fn lesson() -> Lesson {
    let cards = two_cards();
    assert_eq!(cards.len(), 2, "the bundle has recall cards");
    let yaml = format!(
        r##"
id: {ID}
title: "Heart embryology, every step"
hook: "Who builds what in the embryonic heart"
minutes: 7
pages: [284, 285]
topics: [heart-morphogenesis]
status: unverified
steps:
  - kind: explain
    title: "The first organ to work"
    text: "The heart beats by **week 4** of development."
    page: 284
    quote: "First functional organ in vertebrate embryos; beats spontaneously by 4-5 weeks of development."
  - kind: reveal
    prompt: "What happens when left-right dynein fails?"
    answer: "Dextrocardia, as in **Kartagener syndrome**."
  - kind: mnemonic
    word: "TTT"
    parts:
      - {{ key: "T1", text: "Transposition of great arteries" }}
      - {{ key: "T2", text: "Tetralogy of Fallot" }}
      - {{ key: "T3", text: "Truncus arteriosus" }}
    hook: "Neural crest that never migrates leaves three Ts"
    page: 284
    quote: "Conotruncal abnormalities associated with failure of neural crest cells to migrate."
  - kind: figure
    title: "Closing the foramen ovale"
    svg: "<svg viewBox='0 0 10 10'><circle cx='5' cy='5' r='4'/></svg>"
    caption: "Septum primum closes against septum secundum."
    alt: "Two septa closing the foramen ovale"
  - kind: aside
    tone: tip
    text: "A PFO is in a quarter of people."
  - kind: aside
    tone: exam
    text: "Down syndrome with a heart defect: think AVSD."
  - kind: aside
    tone: trap
    text: "A VSD is not an ASD."
  - kind: aside
    tone: joke
    text: "Three Ts walk into a bar."
  - kind: update
    text: "Newer guidance on PFO closure after cryptogenic stroke."
    ref: "AHA/ASA 2021"
  - kind: table
    title: "AVSD, complete and partial"
    columns: ["Type", "What is open"]
    rows:
      - ["Complete", "Common AV valve + ASD + VSD"]
      - ["Partial", "Primum ASD + cleft mitral valve"]
    hide: [[0, 1], [1, 1]]
  - kind: check
    question: "A 35-year-old woman has a stroke after a long flight. Bubble study shows right-to-left flow. Failure of what?"
    options:
      - "Fusion of septum primum with septum secundum"
      - "Neural crest cell migration"
      - "Fusion of the endocardial cushions"
      - "Cardiac looping"
    answer: 0
    why:
      - "Right: a PFO lets a paradoxical embolus through."
      - "Neural crest failure gives conotruncal defects."
      - "Cushion failure gives an AVSD."
      - "Looping failure gives dextrocardia."
  - kind: order
    prompt: "Put the heart's development in order"
    steps: ["Heart tube forms", "Cardiac looping", "Septation"]
    why: "Tube, then loop, then septa."
  - kind: match
    prompt: "Match each embryonic structure to what it becomes"
    pairs:
      - ["Left horn of sinus venosus", "Coronary sinus"]
      - ["Truncus arteriosus", "Ascending aorta and pulmonary trunk"]
    why: "Smooth parts come from vessels."
  - kind: sort
    prompt: "Sort each defect by its cause"
    bins: ["Neural crest", "Endocardial cushions"]
    cards:
      - {{ text: "Tetralogy of Fallot", bin: 0 }}
      - {{ text: "Complete AVSD", bin: 1 }}
    why: "Conotruncal defects come from the neural crest."
  - kind: case
    stem: "A newborn is cyanotic in the first hours of life."
    steps:
      - question: "Which defect is most likely?"
        options: ["Transposition of great arteries", "Patent ductus arteriosus"]
        answer: 0
        why: "Early cyanosis: TGA."
      - reveal: "Echo shows parallel great arteries."
        question: "What keeps him alive?"
        why: "A shunt that mixes the circulations."
  - kind: try
    items: [{a}, {b}]
recap:
  - "The heart beats by week 4."
  - "PFO: paradoxical embolus."
practice: [{a}, {b}]
"##,
        a = cards[0],
        b = cards[1],
    );
    serde_yaml_ng::from_str(&yaml).expect("the test lesson parses")
}

/// One step of `lesson()` on its own, with a title of its kind.
pub(super) fn one_step(kind: &str) -> Lesson {
    let mut l = lesson();
    l.steps.retain(|s| s.body.kind() == kind);
    l.steps.truncate(1);
    l
}

/// The kind of the step on screen (`data-kind` of the stage not hidden).
fn step_kind(app: &App) -> String {
    let html = app.html();
    html.split("<article")
        .skip(1)
        .map(|s| s.split('>').next().unwrap_or(""))
        .find(|tag| tag.contains("ls-stage") && !tag.contains("hidden"))
        .and_then(|tag| tag.split("data-kind=\"").nth(1))
        .map(|s| s.split('"').next().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// The review log the app's study store holds.
pub(super) fn review_log(app: &mut App) -> Vec<steps_core::review::ReviewEvent> {
    use steps_core::review::StudyStore;
    app.settle();
    REVIEW.with_borrow(|r| r.as_ref().expect("the app's review state").backend.borrow().log())
}

/// Presses a key on the node listening for keydown that carries `label`.
/// (`App::key` leaves out `which`, which the event data now needs.)
pub(super) fn press(app: &mut App, label: &str, key: &str) {
    let n = app.target("keydown", label, 0);
    let json = serde_json::json!({
        "element": 0, "name": "keydown", "bubbles": true,
        "data": {
            "char_code": 0, "key": key, "code": "Unidentified", "key_code": 0, "location": 0, "which": 0,
            "alt_key": false, "ctrl_key": false, "meta_key": false, "shift_key": false,
            "repeat": false, "is_composing": false,
        }
    });
    let ev: dioxus_html::HtmlEvent = serde_json::from_value(json).expect("a keydown event");
    app.fire("keydown", ev.data, n);
}

/// Taps the first clickable whose `attr` is exactly `value`.
#[track_caller]
pub(super) fn tap_where(app: &mut App, attr: &str, value: &str) {
    let n = app
        .page
        .walk()
        .into_iter()
        .find(|&n| app.page.nodes[n].listeners.contains(&"click") && app.page.attr(n, attr) == Some(value))
        .unwrap_or_else(|| panic!("nothing to tap with {attr}={value:?}:\n{}", app.listing("click")));
    app.fire("click", dioxus_html::EventData::Mouse(Default::default()), n);
}

/// The step card on screen (the stage that is not hidden).
fn stage(app: &App) -> usize {
    app.page
        .walk()
        .into_iter()
        .find(|&n| {
            app.page.attr(n, "class").is_some_and(|c| c.contains("ls-stage"))
                && app.page.attr(n, "hidden") != Some("true")
        })
        .unwrap_or_else(|| panic!("no step on screen:\n{}", app.text()))
}

/// Taps the innermost button carrying `label` inside the step on screen
/// (steps behind it stay mounted, hidden, and may carry the same words).
#[track_caller]
pub(super) fn tap_here(app: &mut App, label: &str) {
    let at = stage(app);
    let inside = |mut n: usize| loop {
        if n == at {
            return true;
        }
        match app.page.nodes[n].parent {
            Some(p) => n = p,
            None => return false,
        }
    };
    let n = app
        .targets("click", label)
        .into_iter()
        .find(|&n| inside(n))
        .unwrap_or_else(|| panic!("nothing in this step carries {label:?}:\n{}", app.text()));
    app.fire("click", dioxus_html::EventData::Mouse(Default::default()), n);
}

/// Continue (or Finish) at the player's foot.
fn next(app: &mut App) {
    app.tap("ls-next");
}

fn close_sheet(app: &mut App) {
    app.tap("bsheet-close");
}

#[test]
fn a_lesson_plays_every_kind_of_step_and_ends_on_its_summary() {
    let mut app = play(lesson());
    let n = lesson().steps.len();
    app.assert_has(&format!("1/{n}"));

    // Explain, with its page chip: the book page sheet shows the step's quote.
    assert_eq!(step_kind(&app), "explain");
    app.assert_has("The first organ to work");
    tap_here(&mut app, "ص 284");
    app.assert_has(crate::i18n::book::TITLE);
    app.assert_has("beats spontaneously by 4-5 weeks");
    close_sheet(&mut app);
    app.assert_lacks("beats spontaneously by 4-5 weeks");
    next(&mut app);

    // Reveal: Continue waits; Skip is offered until he reveals.
    assert_eq!(step_kind(&app), "reveal");
    assert!(app.can_tap(tx::SKIP));
    app.assert_lacks("Kartagener syndrome");
    tap_here(&mut app, tx::REVEAL);
    app.assert_has("Kartagener syndrome");
    assert!(!app.can_tap(tx::SKIP));
    next(&mut app);

    // Mnemonic: one letter, then all; its page chip carries the تحشيشة.
    assert_eq!(step_kind(&app), "mnemonic");
    app.assert_lacks("Tetralogy of Fallot");
    tap_here(&mut app, "T1");
    app.assert_has("Transposition of great arteries");
    app.assert_lacks("Tetralogy of Fallot");
    tap_here(&mut app, tx::SHOW_ALL);
    app.assert_has("Tetralogy of Fallot");
    app.assert_has("Truncus arteriosus");
    tap_here(&mut app, "ص 284");
    app.assert_has(crate::i18n::book::HOOK);
    app.assert_has("Neural crest that never migrates");
    close_sheet(&mut app);
    next(&mut app);

    // Figure: a tap opens it full screen; Escape or × closes it.
    assert_eq!(step_kind(&app), "figure");
    app.assert_has("Septum primum closes against septum secundum.");
    tap_here(&mut app, "Two septa closing the foramen ovale");
    app.assert_has(tx::ZOOM_HINT);
    press(&mut app, "ls-viewer", "Escape");
    app.assert_lacks(tx::ZOOM_HINT);
    tap_here(&mut app, "Two septa closing the foramen ovale");
    app.tap(tx::ZOOM_CLOSE);
    app.assert_lacks(tx::ZOOM_HINT);
    next(&mut app);

    // Asides, each with its own label, and the guideline update.
    for (label, words) in [
        (tx::TIP, "quarter of people"),
        (tx::EXAM, "think AVSD"),
        (tx::TRAP, "not an ASD"),
        (tx::JOKE, "walk into a bar"),
    ] {
        assert_eq!(step_kind(&app), "aside");
        app.assert_has(label);
        app.assert_has(words);
        next(&mut app);
    }
    assert_eq!(step_kind(&app), "update");
    app.assert_has(tx::UPDATE);
    app.assert_has("AHA/ASA 2021");
    app.assert_has(tx::BOOK_WINS);
    next(&mut app);

    // Table: hidden cells open one by one, or all at once.
    assert_eq!(step_kind(&app), "table");
    app.assert_has(tx::TABLE_HINT);
    app.assert_lacks("Common AV valve");
    tap_here(&mut app, "ls-cell-hidden");
    app.assert_has("Common AV valve + ASD + VSD");
    app.assert_lacks("cleft mitral valve");
    tap_here(&mut app, tx::SHOW_ALL);
    app.assert_has("Primum ASD + cleft mitral valve");
    app.assert_lacks(tx::TABLE_HINT);
    next(&mut app);

    // Check: a pick can change before صحّح; then each option explains itself.
    assert_eq!(step_kind(&app), "check");
    assert!(!app.can_tap(tx::CHECK_IT));
    tap_here(&mut app, "Neural crest cell migration");
    tap_here(&mut app, "Fusion of septum primum with septum secundum");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::RIGHT);
    app.assert_has(tx::MARK_MINE_RIGHT);
    app.assert_has("a PFO lets a paradoxical embolus through");
    // حاول تاني: practice only; the first result stays the one that counts.
    tap_here(&mut app, tx::RETRY);
    tap_here(&mut app, "Cardiac looping");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::NOT_QUITE);
    app.assert_has(tx::MARK_MINE_WRONG);
    app.assert_has(tx::MARK_RIGHT);
    next(&mut app);

    // Order: tiles fill the slots; a slot or Undo takes one back out.
    assert_eq!(step_kind(&app), "order");
    app.assert_has(&tx::placed_of(0, 3));
    tap_here(&mut app, "Septation");
    app.assert_has(&tx::placed_of(1, 3));
    tap_here(&mut app, tx::UNDO);
    app.assert_has(&tx::placed_of(0, 3));
    tap_here(&mut app, "Heart tube forms");
    tap_here(&mut app, "Septation");
    tap_here(&mut app, &format!("{}: Septation", tx::REMOVE));
    tap_here(&mut app, "Cardiac looping");
    tap_here(&mut app, "Septation");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::RIGHT);
    app.assert_has("Tube, then loop, then septa.");
    next(&mut app);

    // Match: a wrong pair counts as a miss.
    assert_eq!(step_kind(&app), "match");
    app.assert_has(tx::MATCH_HINT);
    tap_here(&mut app, "Left horn of sinus venosus");
    tap_here(&mut app, "Ascending aorta and pulmonary trunk");
    tap_here(&mut app, "Coronary sinus");
    tap_here(&mut app, "Truncus arteriosus");
    tap_here(&mut app, "Ascending aorta and pulmonary trunk");
    app.assert_has(&tx::misses(1));
    app.assert_has("Smooth parts come from vessels.");
    next(&mut app);

    // Sort: tap a card, then its bin; a placed card goes back on a tap.
    assert_eq!(step_kind(&app), "sort");
    app.assert_has(&tx::sort_left(2, 2));
    tap_here(&mut app, "Tetralogy of Fallot");
    app.assert_has(tx::SORT_PICKED);
    tap_here(&mut app, "ls-bin-head");
    app.assert_has(&tx::sort_left(1, 2));
    tap_here(&mut app, "Complete AVSD");
    app.tap_nth("ls-bin-head", 0);
    tap_here(&mut app, &format!("{}: Complete AVSD", tx::REMOVE));
    app.assert_has(&tx::sort_left(1, 2));
    tap_here(&mut app, "Complete AVSD");
    tap_here(&mut app, "Endocardial cushions");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::RIGHT);
    app.assert_has("Conotruncal defects come from the neural crest.");
    next(&mut app);

    // Mini case: pick and check each part, then go on; a part without
    // options is shown with a tap.
    assert_eq!(step_kind(&app), "case");
    app.assert_has("A newborn is cyanotic");
    app.assert_lacks("What keeps him alive?");
    tap_here(&mut app, "Patent ductus arteriosus");
    tap_here(&mut app, "Transposition of great arteries");
    app.tap(tx::CHECK_IT);
    app.assert_has("Early cyanosis: TGA.");
    tap_here(&mut app, tx::CASE_NEXT);
    app.assert_has("Echo shows parallel great arteries.");
    app.assert_has("What keeps him alive?");
    tap_here(&mut app, tx::CASE_SHOW);
    app.assert_has("A shunt that mixes the circulations.");
    next(&mut app);

    // Try: the lecture's own cards, graded as real reviews.
    assert_eq!(step_kind(&app), "try");
    app.assert_has(&format!("{} 1 {} 2", tx::TRY_OF, tx::OF));
    tap_here(&mut app, crate::i18n::study::REVEAL);
    tap_where(&mut app, "data-grade", "again");
    app.assert_has(&format!("{} 2 {} 2", tx::TRY_OF, tx::OF));
    tap_here(&mut app, crate::i18n::study::REVEAL);
    tap_where(&mut app, "data-grade", "good");
    app.assert_has(tx::TRY_DONE);
    assert_eq!(review_log(&mut app).len(), 2, "both cards went to the review log");
    app.assert_has(tx::FINISH);
    next(&mut app);

    // The summary: first tries, the recap, and the practice session.
    app.assert_has(tx::DONE_TITLE);
    app.assert_has("The heart beats by week 4.");
    app.assert_has("PFO: paradoxical embolus.");
    let m = app.run(|| crate::learn::mark(ID)).expect("the finished lesson is marked");
    // The five question steps: check, order, sort and the case right the
    // first time (the case's open part only shows), the match with a miss.
    // The try step's cards are reviews, not part of the score.
    assert_eq!(m.asked, 5, "{m:?}");
    assert_eq!(m.right, 4, "{m:?}");
    app.assert_has(&format!("{}/{}", m.right, m.asked));
    app.assert_has(tx::score_line(m.right, m.asked));
    app.tap(&format!("{} (2)", tx::PRACTICE));
    assert_eq!(super::lecture::screen(&app), "Study");
}

/// Answers the step on screen of `lesson()` right, the shortest way.
fn answer_right(app: &mut App) {
    match step_kind(app).as_str() {
        "reveal" => {
            tap_here(app, tx::REVEAL);
        }
        "table" => tap_here(app, tx::SHOW_ALL),
        "mnemonic" => tap_here(app, tx::SHOW_ALL),
        "check" => {
            tap_here(app, "Fusion of septum primum with septum secundum");
            app.tap(tx::CHECK_IT);
        }
        "order" => {
            for s in ["Heart tube forms", "Cardiac looping", "Septation"] {
                tap_here(app, s);
            }
            app.tap(tx::CHECK_IT);
        }
        "match" => {
            for s in [
                "Left horn of sinus venosus",
                "Coronary sinus",
                "Truncus arteriosus",
                "Ascending aorta and pulmonary trunk",
            ] {
                tap_here(app, s);
            }
        }
        "sort" => {
            tap_here(app, "Tetralogy of Fallot");
            tap_here(app, "Neural crest");
            tap_here(app, "Complete AVSD");
            tap_here(app, "Endocardial cushions");
            app.tap(tx::CHECK_IT);
        }
        "case" => {
            tap_here(app, "Transposition of great arteries");
            app.tap(tx::CHECK_IT);
            tap_here(app, tx::CASE_NEXT);
            tap_here(app, tx::CASE_SHOW);
        }
        "try" => {
            for _ in 0..2 {
                tap_here(app, crate::i18n::study::REVEAL);
                tap_where(app, "data-grade", "good");
            }
        }
        _ => {}
    }
}

/// Plays `lesson()` right up to the step of `kind`, which is left open.
pub(super) fn play_up_to(app: &mut App, kind: &str) {
    while step_kind(app) != kind {
        answer_right(app);
        next(app);
    }
}

#[test]
fn all_right_first_time_scores_every_question() {
    let mut app = play(lesson());
    play_up_to(&mut app, "");
    app.assert_has(tx::DONE_TITLE);
    let m = app.run(|| crate::learn::mark(ID)).unwrap();
    assert_eq!((m.right, m.asked), (5, 5));
    app.assert_has(tx::score_line(5, 5));
    // Practice first, the next lesson of the lecture second, the list last.
    let next = app.run(|| crate::learn::next_lesson(&["cvs.L01".to_string()]).map(|l| l.id.clone()));
    if let Some(next) = next {
        app.assert_has(tx::NEXT_LESSON);
        app.tap(tx::NEXT_LESSON);
        assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), Some(next));
        assert_eq!(super::lecture::screen(&app), "Learn");
    }
}

#[test]
fn skipped_questions_are_not_scored_and_back_returns_a_step() {
    let mut app = play(lesson());
    let n = lesson().steps.len();
    // Back is off on the first step.
    assert!(app.html().contains("disabled"));
    next(&mut app);
    app.assert_has(&format!("2/{n}"));
    app.tap(tx::BACK);
    app.assert_has(&format!("1/{n}"));
    // Skip every step that asks for something.
    let mut guard = 0;
    while !app.has(tx::DONE_TITLE) {
        guard += 1;
        assert!(guard < 40, "the lesson ends");
        if app.can_tap(tx::SKIP) {
            app.tap(tx::SKIP);
        } else {
            next(&mut app);
        }
    }
    let m = app.run(|| crate::learn::mark(ID)).unwrap();
    assert_eq!((m.right, m.asked), (0, 0), "skipped questions are left out of the score");
    // No score line without a question answered.
    app.assert_lacks(tx::FIRST_TRY);
    app.tap(tx::BACK_TO_LIST);
}

#[test]
fn the_pages_view_turns_with_arrows_and_switches_back() {
    let mut app = play(lesson());
    let n = lesson().steps.len();
    app.tap(tx::TO_PAGES);
    assert!(app.html().contains("data-view=\"pages\""));
    app.assert_has(&format!("{} 1 {} {n}", tx::PAGE_OF, tx::OF_N));
    app.tap("ls-arrow-next");
    app.assert_has(&format!("2/{n}"));
    // A reveal: the next arrow waits, Skip shows instead of the page count.
    assert!(app.can_tap(tx::SKIP));
    tap_here(&mut app, tx::REVEAL);
    app.tap("ls-arrow-next");
    app.assert_has(&format!("3/{n}"));
    app.tap(tx::TO_STEPS);
    assert!(app.html().contains("data-view=\"steps\""));
    assert_eq!(app.run(|| *crate::learn::VIEW.peek()), crate::learn::View::Steps);
}

#[test]
fn closing_a_lesson_keeps_its_place_and_reopening_restores_every_response() {
    let mut app = play(lesson());
    play_up_to(&mut app, "try");
    // Leave it part-way; the list then reads "continue".
    app.tap(tx::CLOSE);
    assert_eq!(app.run(|| crate::learn::resume_step(ID)), Some(lesson().steps.len() - 1));
    drop(app);

    // Opening it again lands on the same step, and each step behind it
    // shows what he did, read-only.
    let mut app = play(lesson());
    assert_eq!(step_kind(&app), "try");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "case");
    app.assert_has("Early cyanosis: TGA.");
    app.assert_has("A shunt that mixes the circulations.");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "sort");
    app.assert_has(tx::RIGHT);
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "match");
    app.assert_has("Smooth parts come from vessels.");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "order");
    app.assert_has("Tube, then loop, then septa.");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "check");
    app.assert_has(tx::MARK_MINE_RIGHT);
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "table");
    app.assert_lacks(tx::TABLE_HINT);
    for _ in 0..6 {
        app.tap(tx::BACK);
    }
    assert_eq!(step_kind(&app), "figure");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "mnemonic");
    // Every letter he opened stays open.
    app.assert_has("Truncus arteriosus");
    app.tap(tx::BACK);
    assert_eq!(step_kind(&app), "reveal");
    app.assert_has("Kartagener syndrome");
    assert!(!app.can_tap(tx::SKIP));
}

#[test]
fn answers_from_an_older_attempt_show_the_key_and_can_be_redone() {
    use steps_core::response::Attempt;
    let l = lesson();
    let n = l.steps.len();
    // An attempt saved before responses were kept: results, no drafts.
    let mut a = Attempt::new(n, 1);
    a.at = n - 1;
    for (k, s) in l.steps.iter().enumerate() {
        if s.body.is_question() || s.body.kind() == "try" {
            a.answers[k] = Some(k % 2 == 0);
        }
    }
    a.drafts.clear();
    crate::store::shared().put_local_setting(&format!("lessonpos.{ID}"), &a).unwrap();
    let mut app = play(l);
    // The try step was graded then: a done note, not its cards again.
    assert_eq!(step_kind(&app), "try");
    app.assert_has(tx::TRY_DONE);
    app.assert_lacks(&format!("{} 1", tx::TRY_OF));
    for (kind, key) in [
        ("case", "Transposition of great arteries"),
        ("sort", "Complete AVSD"),
        ("match", "Coronary sinus"),
        ("order", "Heart tube forms"),
        ("check", "Fusion of septum primum with septum secundum"),
    ] {
        app.tap(tx::BACK);
        assert_eq!(step_kind(&app), kind);
        app.assert_has(tx::KEY);
        app.assert_has(key);
    }
    // حاول تاني: the check is a fresh task again.
    tap_here(&mut app, tx::RETRY);
    tap_here(&mut app, "Cardiac looping");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::NOT_QUITE);
}

#[test]
fn wrong_answers_show_what_should_have_been() {
    let mut app = play(lesson());
    play_up_to(&mut app, "order");
    for s in ["Septation", "Cardiac looping", "Heart tube forms"] {
        tap_here(&mut app, s);
    }
    app.tap(tx::CHECK_IT);
    app.assert_has(&tx::in_place(1, 3));
    app.assert_has(&tx::should_be(3));
    next(&mut app);
    answer_right(&mut app);
    next(&mut app);
    // Sort: one card in the wrong bin.
    tap_here(&mut app, "Tetralogy of Fallot");
    tap_here(&mut app, "Endocardial cushions");
    tap_here(&mut app, "Complete AVSD");
    tap_here(&mut app, "Endocardial cushions");
    app.tap(tx::CHECK_IT);
    app.assert_has(&tx::in_place(1, 2));
    app.assert_has("→ Neural crest");
    next(&mut app);
    // The case: a wrong pick, then the part done again.
    tap_here(&mut app, "Patent ductus arteriosus");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::MARK_MINE_WRONG);
    tap_here(&mut app, tx::RETRY);
    app.assert_lacks("Early cyanosis: TGA.");
    tap_here(&mut app, "Transposition of great arteries");
    app.tap(tx::CHECK_IT);
    app.assert_has(tx::MARK_MINE_RIGHT);
    tap_here(&mut app, tx::CASE_NEXT);
    tap_here(&mut app, tx::CASE_SHOW);
    next(&mut app);
    play_up_to(&mut app, "");
    let m = app.run(|| crate::learn::mark(ID)).unwrap();
    // Order, sort and case wrong on the first try; check and match right.
    assert_eq!((m.right, m.asked), (2, 5));
}

#[test]
fn a_photo_figure_credits_its_author_and_licence() {
    let mut l = one_step("figure");
    let yaml = r#"
kind: figure
title: "A heart"
image:
  file: heart.png
  credit: "Jane Doe"
  license: "CC BY 4.0"
  source: "https://example.org/heart"
  data: "data:image/png;base64,AAAA"
caption: "The four chambers."
alt: "A drawn heart with four chambers"
"#;
    l.steps = vec![serde_yaml_ng::from_str(yaml).expect("a figure step")];
    let mut app = play(l);
    app.assert_has("Jane Doe · CC BY 4.0");
    assert!(app.html().contains("ls-photo"));
    tap_here(&mut app, "A drawn heart with four chambers");
    app.assert_has(tx::ZOOM_HINT);
    // The last step's button reads "finish".
    app.assert_has(tx::FINISH);
}

// ---- The lesson list, over the bundled lessons ----

/// The first bundled lesson and its lecture.
fn first_lesson() -> (String, Lesson) {
    let (lec, l) = Library::get().all_lessons().next().expect("a lesson is written");
    (lec.to_string(), l.clone())
}

/// A course that has lectures in the index but no lesson yet.
fn course_without_lessons() -> Option<String> {
    let lib = Library::get();
    lib.program.courses.iter().map(|c| c.id.clone()).find(|c| {
        lib.index.lectures.iter().any(|e| steps_core::program::course_of(&e.id) == c)
            && !lib.lessons.keys().any(|l| steps_core::program::course_of(l) == c)
    })
}

/// Words as the page shows them: lesson text glues an Arabic article to its
/// word with a no-break space and pads English with a hair space.
fn loose(s: &str) -> String {
    s.replace('\u{a0}', " ").replace('\u{200A}', "")
}

#[track_caller]
fn assert_shows(app: &App, words: &str) {
    assert!(loose(&app.text()).contains(&loose(words)), "the page should show {words:?}:\n{}", app.text());
}

/// Taps the row of a lesson by its hook.
fn tap_lesson(app: &mut App, l: &Lesson) {
    let n = app
        .page
        .walk()
        .into_iter()
        .filter(|&n| app.page.nodes[n].listeners.contains(&"click"))
        .filter(|&n| loose(&app.page.text(n)).contains(&loose(&l.hook)))
        .min_by_key(|&n| app.page.text(n).len())
        .unwrap_or_else(|| panic!("no row for {}:\n{}", l.id, app.listing("click")));
    app.fire("click", dioxus_html::EventData::Mouse(Default::default()), n);
}

fn learn() -> App {
    App::open(Screen::Learn)
}

fn finish(id: &str, right: u32, asked: u32) {
    let m = crate::learn::Mark { done_ms: 1, right, asked };
    crate::store::shared().put_setting(&format!("lesson.{id}"), &m).unwrap();
}

#[test]
fn the_list_opens_the_next_lesson_and_keeps_its_place() {
    let (lec, l) = first_lesson();
    let total = Library::get().all_lessons().count();
    let mut app = learn();
    app.assert_has(tx::HEAD);
    app.assert_has(&tx::done_of(0, total));
    app.assert_has(Library::get().title(&lec));
    app.assert_has(tx::START_FIRST);
    app.assert_has(tx::STATE_NEXT);
    // The lecture card's button opens its first lesson in the player.
    app.tap("ls-lec-go");
    assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), Some(l.id.clone()));
    app.assert_has(&format!("1/{}", l.steps.len()));
    // One step on, then closed: the list says where it stands.
    if !app.can_tap(tx::SKIP) {
        next(&mut app);
    } else {
        app.tap(tx::SKIP);
    }
    app.tap(tx::CLOSE);
    assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), None);
    app.assert_has(tx::STATE_CURRENT);
    app.assert_has(&tx::step_of(2, l.steps.len()));
    app.assert_has(&tx::resume_at(2, l.steps.len()));
    // The row opens it again at that step.
    tap_lesson(&mut app, &l);
    app.assert_has(&format!("2/{}", l.steps.len()));
}

#[test]
fn finished_lessons_show_their_score_and_the_list_says_all_done() {
    let lib = Library::get();
    let ids: Vec<String> = lib.all_lessons().map(|(_, l)| l.id.clone()).collect();
    let mut app = App::with(Screen::Learn, || {
        for (k, (_, l)) in Library::get().all_lessons().enumerate() {
            // One with questions answered, the rest all skipped.
            if k == 0 { finish(&l.id, 3, 4) } else { finish(&l.id, 0, 0) }
        }
        crate::learn::reload();
    });
    app.assert_has(&tx::done_of(ids.len(), ids.len()));
    app.assert_has(tx::STATE_DONE);
    app.assert_has("3/4");
    app.assert_has(tx::ALL_LESSONS_DONE);
    assert_eq!(app.run(|| crate::learn::next_lesson(&[]).map(|l| l.id.clone())), None);
}

#[test]
fn search_finds_lessons_and_says_when_nothing_matches() {
    let (lec, l) = first_lesson();
    let mut app = learn();
    let word = l.topics.first().cloned().unwrap_or_else(|| Library::get().lecture_tag(&lec));
    app.type_into(tx::SEARCH_LABEL, &word);
    assert_shows(&app, &l.hook);
    assert!(app.can_tap(tx::CLEAR_SEARCH));
    // The lecture's heading opens its page.
    app.tap("ls-group-head");
    assert!(app.run(crate::lecture::is_open));
    app.tap(crate::i18n::lecture::BACK);
    // The search box kept its words.
    assert_shows(&app, &l.hook);
    app.type_into(tx::SEARCH_LABEL, "zzzz-no-such-lesson");
    app.assert_has(&tx::no_match("zzzz-no-such-lesson"));
    app.tap(tx::CLEAR_SEARCH);
    app.assert_lacks(&tx::no_match("zzzz-no-such-lesson"));
    app.assert_has(tx::OTHER_LECTURES);
}

#[test]
fn a_course_without_lessons_still_lists_its_lectures() {
    let Some(course) = course_without_lessons() else { return };
    let mut app = learn();
    app.run(|| crate::courses::focus::set(Some(course.clone())));
    app.assert_has(tx::COURSE_NO_LESSONS);
    app.assert_has(tx::WRITING);
    // A lecture not written yet opens its page (book pages, cards).
    app.tap("ls-soon");
    assert!(app.run(crate::lecture::is_open));
}

#[test]
fn searching_one_course_offers_every_course() {
    let (lec, l) = first_lesson();
    let mut app = learn();
    app.run(|| crate::courses::focus::set(Some(steps_core::program::course_of(&lec).to_string())));
    app.type_into(tx::SEARCH_LABEL, "zzzz-no-such-lesson");
    app.assert_has(&tx::no_match("zzzz-no-such-lesson"));
    app.tap(tx::SEARCH_ALL_COURSES);
    assert_eq!(app.run(crate::courses::focus::current), None);
    let _ = l;
}

#[test]
fn another_lecture_and_the_lecture_page_open_from_the_list() {
    let (lec, _) = first_lesson();
    let mut app = learn();
    // The picked lecture's page.
    app.tap(tx::LECTURE_PAGE);
    assert!(app.run(crate::lecture::is_open));
    app.assert_has(Library::get().title(&lec));
    app.tap(crate::i18n::lecture::BACK);
    // "Other lectures" rows: one with lessons moves up; one without opens its page.
    app.tap("ls-other");
    assert_eq!(super::lecture::screen(&app), "Learn");
}

#[test]
fn today_offers_the_next_lesson_and_then_the_one_part_way() {
    let (_, l) = first_lesson();
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::NEXT_KICKER);
    app.tap("ls-now");
    assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), Some(l.id.clone()));
    assert_eq!(super::lecture::screen(&app), "Learn");
    if app.can_tap(tx::SKIP) {
        app.tap(tx::SKIP);
    } else {
        next(&mut app);
    }
    // Closing goes back where it was opened from.
    app.tap(tx::CLOSE);
    assert_eq!(super::lecture::screen(&app), "Today");
    app.assert_has(tx::NOW_KICKER);
    app.assert_has(&tx::step_of(2, l.steps.len()));
}

#[test]
fn a_study_item_links_to_the_lesson_that_explains_it() {
    let l = lesson();
    let mut app = play(l);
    play_up_to(&mut app, "try");
    // The try step's card is a study item: "explain it to me" opens its lesson.
    let card = two_cards().remove(0);
    let teaching = app.run(|| crate::learn::lesson_for_item(&card).map(|l| l.id.clone()));
    if let Some(teaching) = teaching {
        tap_here(&mut app, tx::EXPLAIN);
        assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), Some(teaching));
    }
}

#[test]
fn the_lesson_view_is_a_setting() {
    // Settings › Review holds the switch.
    let mut app = App::with(Screen::Today, || {
        consume_context::<crate::settings::SettingsNav>().show(Some(crate::settings::Section::Study))
    });
    app.assert_has(tx::VIEW_TITLE);
    app.tap(tx::VIEW_PAGES_HINT);
    assert_eq!(app.run(|| *crate::learn::VIEW.peek()), crate::learn::View::Pages);
    let saved: Option<crate::learn::View> = crate::store::shared().setting("lesson_view");
    assert_eq!(saved, Some(crate::learn::View::Pages));
    app.tap(tx::VIEW_STEPS_HINT);
    assert_eq!(app.run(|| *crate::learn::VIEW.peek()), crate::learn::View::Steps);
}

#[test]
fn a_view_saved_elsewhere_comes_back_on_reload() {
    let mut app = learn();
    crate::store::save_setting("lesson_view", &crate::learn::View::Pages);
    app.run(crate::learn::reload);
    assert_eq!(app.run(|| *crate::learn::VIEW.peek()), crate::learn::View::Pages);
}

#[test]
fn other_lectures_sort_by_what_is_left() {
    let mut app = learn();
    // The second view bar is the other lectures'.
    app.tap_nth("view-sort-btn", 1);
    app.tap(crate::i18n::views::SORT_LEFT);
    app.assert_has(tx::OTHER_LECTURES);
    app.assert_has(tx::WRITING);
}

#[test]
fn today_has_no_lesson_card_once_every_lesson_is_done() {
    let app = App::with(Screen::Today, || {
        for (_, l) in Library::get().all_lessons() {
            finish(&l.id, 1, 1);
        }
        crate::learn::reload();
    });
    assert!(!app.can_tap("ls-now"));
}

#[test]
fn today_still_offers_a_lesson_part_way_in_another_course() {
    let Some(course) = course_without_lessons() else { return };
    let (_, l) = first_lesson();
    let mut app = App::open(Screen::Today);
    app.tap("ls-now");
    next(&mut app);
    app.tap(tx::CLOSE);
    app.run(|| crate::courses::focus::set(Some(course.clone())));
    app.assert_has(tx::NOW_KICKER);
    app.tap("ls-now");
    assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), Some(l.id));
}

thread_local! {
    static EXPLAIN: RefCell<String> = const { RefCell::new(String::new()) };
}

#[allow(non_snake_case)]
fn Explain() -> Element {
    let item_id = EXPLAIN.with_borrow(Clone::clone);
    rsx! { div { class: "test-explain", crate::learn::ExplainLink { item_id } } }
}

fn explain_root() -> Element {
    rsx! { AppAt { start: Screen::Cases, Explain {} } }
}

#[test]
fn explain_links_only_show_where_a_lesson_teaches_the_item() {
    // An item of a lecture with no lessons: nothing to link.
    let lib = crate::content::Library::get();
    let bare =
        lib.lectures.iter().find(|(id, _)| !lib.lessons.contains_key(*id)).and_then(|(_, l)| l.items.first());
    if let Some(bare) = bare {
        EXPLAIN.with_borrow_mut(|e| *e = bare.id.clone());
        let app = mount(explain_root);
        assert!(!app.can_tap(tx::EXPLAIN), "{}", bare.id);
    }
    // An item a lesson practises: the link opens that lesson.
    let (_, l) = first_lesson();
    let Some(item) = l.practice.first().cloned() else { return };
    EXPLAIN.with_borrow_mut(|e| *e = item.clone());
    let mut app = mount(explain_root);
    app.tap(tx::EXPLAIN);
    let want = app.run(|| crate::learn::lesson_for_item(&item).map(|l| l.id.clone()));
    assert_eq!(app.run(|| crate::learn::OPEN.peek().clone()), want);
}

#[test]
fn reading_choices_restyle_the_lesson_and_stay_on_this_device() {
    use crate::prefs::{ReadSize, ReadSpacing, ReadWidth, Reading};
    let mut app = play(lesson());
    // The book's defaults, on the player the lesson CSS keys on.
    let html = app.html();
    for attr in ["data-read-size=\"m\"", "data-read-spacing=\"normal\"", "data-read-width=\"book\""] {
        assert!(html.contains(attr), "{attr}");
    }
    // Closed: × is the Escape target. Open: «Aa» is, so Escape and Back
    // close the popover before they close the lesson.
    assert!(!app.has(tx::READ_SPACING));
    app.tap(tx::READING_OPEN);
    app.assert_has(tx::READ_SPACING);
    app.assert_has(tx::READ_SAMPLE[1]);
    let html = app.html();
    assert!(html.contains("aria-expanded=\"true\""), "the button says the popover is open");
    let escape = html.matches("aria-keyshortcuts=\"Escape\"").count();
    assert_eq!(escape, 1, "only the popover answers Escape while it is open");

    app.tap(tx::READ_SIZE_LARGER);
    app.tap(tx::READ_SPACING_AIRY);
    app.tap(tx::READ_WIDTH_NARROW);
    let html = app.html();
    for attr in ["data-read-size=\"xl\"", "data-read-spacing=\"airy\"", "data-read-width=\"narrow\""] {
        assert!(html.contains(attr), "{attr}");
    }
    let want = Reading { size: ReadSize::Larger, spacing: ReadSpacing::Airy, width: ReadWidth::Narrow };
    assert_eq!(app.run(|| crate::prefs::DevicePrefs::load().reading), want, "kept in the device file");

    // Done closes it; the lesson is where it was.
    app.tap(tx::READ_DONE);
    assert!(!app.has(tx::READ_SPACING));
    app.assert_has(&format!("1/{}", lesson().steps.len()));
}
