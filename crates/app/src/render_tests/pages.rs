//! First Aid pages read (A2): the page sheet's "قريت الصفحة دي", the
//! lecture page's count and marks, the switch in Settings › Review, and
//! Today's line about new items waiting for their pages.

use dioxus::prelude::*;

use super::App;
use crate::content::Library;
use crate::i18n::{book, lecture as ltx, study_settings as stx};
use crate::store::pages::{GATE_KEY, KEY};
use crate::store::shared;
use crate::{Screen, Ui};

/// The first written lecture (book order) and a playable item's page in it.
fn ready() -> (String, u32, u32, u32) {
    let lib = Library::get();
    let e = lib.index.lectures.iter().find(|e| lib.lectures.contains_key(&e.id)).expect("a written lecture");
    let page = lib.lectures[&e.id]
        .items
        .iter()
        .find(|i| crate::study::PLAYABLE.contains(&i.body.kind()) && i.src.book.is_none())
        .expect("a First Aid item")
        .src
        .page;
    (e.id.clone(), page, e.pages[0], e.pages[1])
}

/// The pages saved for the ready lecture's course.
fn saved() -> steps_core::pages::PagesRead {
    let course = steps_core::program::course_of(&ready().0).to_string();
    shared().setting(&format!("{KEY}{course}")).unwrap_or_default()
}

thread_local! {
    /// The app's review state, kept by the setup for the test to drive.
    static REVIEW: std::cell::RefCell<Option<crate::store::Review>> = const { std::cell::RefCell::new(None) };
}

fn open_sheet() {
    let (id, page, ..) = ready();
    crate::pagesheet::open(page, &id, None, false, None);
}

fn open_lecture() {
    crate::lecture::open(consume_context::<Ui>(), &ready().0);
}

#[test]
fn the_page_sheet_marks_its_page_read_and_back() {
    let (_, page, ..) = ready();
    let mut app = App::with(Screen::Today, open_sheet);
    app.assert_has(&format!("{} {page}", book::TITLE));
    app.tap(book::MARK_READ);
    assert!(saved().contains(page), "the page is saved for its course");
    app.assert_has(book::IS_READ);
    assert!(app.html().contains(r#"aria-pressed="true""#));
    app.tap(book::IS_READ);
    assert!(!saved().contains(page));
    app.assert_has(book::MARK_READ);
}

#[test]
fn the_lecture_page_counts_its_pages_read_and_marks_them() {
    let (_, _, first, last) = ready();
    let all = last - first + 1;
    let mut app = App::with(Screen::Learn, open_lecture);
    app.assert_has(ltx::READ_NONE);
    app.tap(ltx::READ_ALL);
    app.assert_has(&ltx::read_count(all, all));
    assert_eq!(saved().count_in(first, last), all);
    app.tap(ltx::UNREAD_ALL);
    app.assert_has(ltx::READ_NONE);
    assert_eq!(saved().count_in(first, last), 0);
    // "قريت لحد صفحة": every page from the first up to the one typed. A
    // number past the lecture stops at its last page.
    if all > 1 {
        app.form("input", ltx::READ_UPTO, &(first + 1).to_string());
        app.tap(ltx::READ_UPTO_GO);
        app.assert_has(&ltx::read_count(2, all));
        app.assert_has(&format!("{first}–{}", first + 1));
    }
    app.form("input", ltx::READ_UPTO, &(last + 50).to_string());
    app.tap(ltx::READ_UPTO_GO);
    app.assert_has(&ltx::read_count(all, all));
}

#[test]
fn the_settings_switch_saves_the_gate() {
    let mut app = App::with(Screen::Today, || {
        consume_context::<crate::settings::SettingsNav>().show(Some(crate::settings::Section::Study))
    });
    app.assert_has(stx::PAGE_GATE);
    app.assert_has(stx::PAGE_GATE_NOTE);
    app.form("change", stx::PAGE_GATE, "true");
    assert_eq!(shared().setting::<bool>(GATE_KEY), Some(true));
    app.form("change", stx::PAGE_GATE, "false");
    assert_eq!(shared().setting::<bool>(GATE_KEY), Some(false));
}

#[test]
fn today_says_how_many_new_items_wait_and_a_page_read_opens_some() {
    let lead = crate::i18n::today::locked(1);
    let lead = lead.split(':').next().unwrap().to_string();
    // No page read and no switch: nothing waits.
    let app = App::open(Screen::Today);
    app.assert_lacks(&lead);
    drop(app);
    // The gate on with nothing read: every new item waits.
    let mut app = App::with(Screen::Today, || {
        let review = consume_context::<crate::store::Review>();
        review.backend.borrow_mut().set_page_gate(true);
        REVIEW.with_borrow_mut(|r| *r = Some(review));
    });
    app.assert_has(&lead);
    let before = app.text();
    // Reading a page opens its items: the waiting count drops.
    let (_, page, ..) = ready();
    app.run(|| {
        REVIEW.with_borrow(|r| {
            r.as_ref().expect("the app's review state").backend.borrow_mut().set_pages_read(
                Library::get(),
                page,
                page,
                true,
            )
        })
    });
    assert_ne!(app.text(), before, "the counts follow the page read");
}
