//! A lecture's page and the pages that stack with it: the references list,
//! the course map and the abbreviations dictionary; the book page sheet a
//! "ص N" chip opens; the abbreviations sheet of a lesson step.

use dioxus::prelude::*;
use steps_core::review::{Grade, Origin, ReviewEvent, StudyStore};

use super::App;
use crate::content::Library;
use crate::i18n::lecture as tx;
use crate::{Screen, Ui};

/// The first lecture in book order that has items.
fn ready() -> String {
    let lib = Library::get();
    lib.index
        .lectures
        .iter()
        .find(|e| lib.lectures.contains_key(&e.id))
        .expect("a lecture is written")
        .id
        .clone()
}

/// A lecture of the index that has no items yet.
fn unwritten() -> String {
    let lib = Library::get();
    lib.index
        .lectures
        .iter()
        .find(|e| !lib.lectures.contains_key(&e.id))
        .expect("a lecture to write")
        .id
        .clone()
}

fn open_ready() {
    crate::lecture::open(consume_context::<Ui>(), &ready());
}

/// The screen `main` shows.
pub(super) fn screen(app: &App) -> String {
    let html = app.html();
    html.split("<main")
        .nth(1)
        .and_then(|s| s.split("data-screen=\"").nth(1))
        .and_then(|s| s.split('"').next())
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_ready_lecture_shows_its_progress_and_every_way_in() {
    let id = ready();
    let lib = Library::get();
    let mut app = App::with(Screen::Learn, open_ready);
    assert!(app.run(crate::lecture::is_open));
    app.assert_has(lib.title(&id));
    app.assert_has(&format!("{} {}", tx::KICKER, crate::content::lecture_no(&id)));
    app.assert_has(tx::STUDY);
    app.assert_has(tx::CHECKED);
    app.assert_lacks(tx::REVIEW_DUE);
    if lib.lessons.contains_key(&id) {
        app.assert_has(tx::LESSONS);
    }
    // Studying it starts a session on the lecture, and the page closes.
    app.tap(tx::STUDY);
    assert_eq!(screen(&app), "Study");
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn reviews_due_get_the_primary_button() {
    let mut app = App::with(Screen::Today, || {
        let id = ready();
        let item = Library::get().lectures[&id]
            .items
            .iter()
            .find(|i| crate::study::PLAYABLE.contains(&i.body.kind()))
            .unwrap()
            .id
            .clone();
        let review = consume_context::<crate::store::Review>();
        review.backend.borrow_mut().record(ReviewEvent {
            item_id: item,
            grade: Grade::Good,
            origin: Origin::Study,
            at_ms: crate::store::now_ms() - 60 * 86_400_000,
            took_ms: 4000,
        });
        crate::lecture::open(consume_context::<Ui>(), &id);
    });
    app.assert_has(tx::DUE);
    app.assert_has(&format!("{} (1)", tx::REVIEW_DUE));
    app.tap(tx::REVIEW_DUE);
    assert_eq!(screen(&app), "Study");
}

#[test]
fn a_lecture_not_written_yet_says_so() {
    let mut app = App::with(Screen::Learn, || crate::lecture::open(consume_context::<Ui>(), &unwritten()));
    app.assert_has(tx::WRITING);
    app.assert_has(Library::get().title(&unwritten()));
    app.assert_lacks(tx::STUDY);
    // Back closes it.
    app.tap(tx::BACK);
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn its_lessons_questions_and_topics_open_from_the_page() {
    let id = ready();
    let lib = Library::get();
    // A lesson card opens the lesson.
    if let Some(file) = lib.lessons.get(&id) {
        let mut app = App::with(Screen::Today, open_ready);
        app.tap("ls-card-btn");
        let opened = app.run(|| crate::learn::OPEN.peek().clone()).expect("a lesson opened");
        assert!(file.lessons.iter().any(|l| l.id == opened), "{opened} is not one of {id}'s lessons");
        assert_eq!(screen(&app), "Learn");
    }
    // The bank's questions on it.
    let mcqs = lib.lectures[&id].items.iter().filter(|i| i.body.kind() == "mcq").count();
    if mcqs > 0 {
        let mut app = App::with(Screen::Today, open_ready);
        app.assert_has(tx::MCQS);
        app.tap(tx::MCQS);
        assert_eq!(screen(&app), "Bank");
    }
    // A topic starts a session on it.
    if !lib.lectures[&id].topics.is_empty() {
        let mut app = App::with(Screen::Today, open_ready);
        app.assert_has(tx::TOPICS);
        app.tap("tap-row");
        assert!(["Study", "Bank"].contains(&screen(&app).as_str()), "{}", screen(&app));
    }
}

#[test]
fn the_course_crumb_opens_the_course_map_and_back_returns() {
    let mut app = App::with(Screen::Learn, open_ready);
    app.tap(crate::i18n::courses::TITLE);
    app.assert_lacks(tx::STUDY);
    // Pages stack: the map closes back to the lecture.
    app.tap_nth(tx::BACK, 0);
    app.assert_has(tx::STUDY);
}

#[test]
fn a_page_closes_when_its_screen_is_left() {
    let mut app = App::with(Screen::Learn, open_ready);
    assert!(app.run(crate::lecture::is_open));
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Learn)), Some(ready()));
    app.tap(Screen::Today.label());
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn the_references_list_finds_lectures_and_opens_them_on_top() {
    let id = ready();
    let lib = Library::get();
    let mut app = App::with(Screen::Learn, || crate::lecture::open_references(consume_context::<Ui>()));
    app.assert_has(crate::i18n::refs::TITLE);
    app.assert_has(crate::i18n::refs::WRITING);
    app.type_into(crate::i18n::refs::SEARCH, "zzzz-nothing");
    app.assert_has(crate::i18n::refs::NO_MATCH);
    app.type_into(crate::i18n::refs::SEARCH, lib.title(&id));
    app.assert_lacks(crate::i18n::refs::NO_MATCH);
    // The first row with checks counted is the first ready lecture.
    app.tap(crate::i18n::refs::CHECKED);
    app.assert_has(tx::STUDY);
    // Back goes to the list under it, then to the screen.
    app.tap(tx::BACK);
    app.assert_has(crate::i18n::refs::LECTURES);
    app.tap(tx::BACK);
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn finished_lessons_are_ticked_on_their_lecture_page() {
    let id = ready();
    let Some(file) = Library::get().lessons.get(&id) else { return };
    let app = App::with(Screen::Learn, || {
        let id = ready();
        for (k, l) in Library::get().lessons[&id].lessons.iter().enumerate() {
            let m = if k == 0 {
                crate::learn::Mark { done_ms: 1, right: 3, asked: 4 }
            } else {
                crate::learn::Mark { done_ms: 1, right: 0, asked: 0 }
            };
            crate::store::shared().put_setting(&format!("lesson.{}", l.id), &m).unwrap();
        }
        crate::learn::reload();
        crate::lecture::open(consume_context::<Ui>(), &id);
    });
    app.assert_has("✓ 3/4");
    app.assert_has(&format!("{} {} {}", file.lessons.len(), tx::OF, file.lessons.len()));
}

#[test]
fn the_references_sort_and_add_a_card() {
    let mut app = App::with(Screen::Learn, || crate::lecture::open_references(consume_context::<Ui>()));
    // What's left to check first.
    app.tap("view-sort-btn");
    app.tap(crate::i18n::views::SORT_LEFT);
    app.assert_has(crate::i18n::refs::LECTURES);
    // ＋ opens the new card form.
    app.tap(crate::i18n::refs::ADD_CARD);
    assert!(app.run(crate::own::drafting));
}

#[test]
fn his_own_cards_start_a_session_from_the_references() {
    // The deck is shared by the whole test run (other threads may publish
    // theirs), so the card is saved again until the list shows it.
    let card = steps_core::own::OwnCard {
        id: "own.test1".into(),
        front: "What closes the foramen ovale after birth?".into(),
        back: "Higher left atrial pressure".into(),
        lecture: Some(ready()),
        page: Some(284),
        source: None,
        created_ms: 1,
    };
    let mut app = App::with(Screen::Learn, || {
        keep_review();
        crate::lecture::open_references(consume_context::<Ui>());
    });
    let study = crate::i18n::own::STUDY_THEM;
    for _ in 0..50 {
        REVIEW.with_borrow(|r| r.as_ref().unwrap().backend.borrow_mut().save_own(&card).unwrap());
        app.settle();
        if app.tap_if(study) {
            assert_eq!(screen(&app), "Study");
            assert!(!app.run(crate::lecture::is_open));
            return;
        }
    }
    panic!("his card never showed:\n{}", app.text());
}

thread_local! {
    static REVIEW: std::cell::RefCell<Option<crate::store::Review>> = const { std::cell::RefCell::new(None) };
}

fn keep_review() {
    REVIEW.with_borrow_mut(|r| *r = Some(consume_context::<crate::store::Review>()));
}
