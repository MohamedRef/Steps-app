//! The sheets and pages that open over any screen: a book page's sheet,
//! a lesson step's abbreviations and the dictionary, the course picker and
//! the course map.

use dioxus::prelude::*;

use super::App;
use super::lecture::screen;
use crate::content::Library;
use crate::{Screen, Ui};

/// The first lecture in book order that has items, and its first page.
fn ready() -> (String, u32) {
    let lib = Library::get();
    let e =
        lib.index.lectures.iter().find(|e| lib.lectures.contains_key(&e.id)).expect("a lecture is written");
    (e.id.clone(), e.pages[0])
}

/// A page of the ready lecture that some playable item cites.
fn cited_page() -> (String, u32) {
    let (id, _) = ready();
    let item = Library::get().lectures[&id]
        .items
        .iter()
        .find(|i| crate::study::PLAYABLE.contains(&i.body.kind()) && i.src.page > 0)
        .expect("a playable item with a page");
    (item.id.clone(), item.src.page)
}

const GROUPED: &str = "Table 1: Causes of aortic dissection\nHypertension\nConnective tissue disorders:\n  Marfan syndrome (fibrillin)\n  Ehlers-Danlos syndrome\nBicuspid aortic valve\n| Type | CVP | CO |\n|---|---|---|\n| Cardiogenic | ↑ | ↓ |";

fn open_page() {
    let (item, page) = cited_page();
    let hook = crate::pagesheet::Hook {
        text: "Marfan's **tall** aorta tears".into(),
        parts: vec![("M".into(), "Marfan syndrome".into()), ("E".into(), "Ehlers-Danlos syndrome".into())],
    };
    crate::pagesheet::open(page, &item, Some(GROUPED.into()), false, Some(hook));
}

/// Taps the clickable of class `class` whose words hold `words`.
#[track_caller]
pub(super) fn tap_row(app: &mut App, class: &str, words: &str) {
    let n = app
        .targets("click", words)
        .into_iter()
        .find(|&n| app.page.attr(n, "class").is_some_and(|c| c.split(' ').any(|k| k == class)))
        .unwrap_or_else(|| panic!("no .{class} with {words:?}:\n{}", app.listing("click")));
    app.fire("click", dioxus_html::EventData::Mouse(Default::default()), n);
}

fn book() -> &'static str {
    crate::i18n::book::TITLE
}

#[test]
fn a_book_page_sheet_lays_out_its_quote_and_what_steps_made_from_the_page() {
    let (item, page) = cited_page();
    let lib = Library::get();
    let (lecture, _) = ready();
    let mut app = App::with(Screen::Today, open_page);
    app.assert_has(&format!("{} {page}", book()));
    app.assert_has(&lib.crumb(&lecture));
    // The quote as the page has it: a title, a grouped table, a grid.
    app.assert_has("Table 1: Causes of aortic dissection");
    let html = app.html();
    assert!(html.contains("bk-q-table grouped"), "a grouped list is a table");
    assert!(html.contains("rowspan=\"2\""), "the group spans its two rows");
    assert!(html.contains("<th>CVP</th>"), "a grid keeps its head");
    // The تحشيشة and its letters.
    app.assert_has(crate::i18n::book::HOOK);
    app.assert_has("Ehlers-Danlos syndrome");
    // What is made from the page, and a session on just it.
    app.assert_has(crate::i18n::book::MADE);
    let content = crate::pagesheet::page_content(lib, page, &crate::pagesheet::scope(lib, &item), |_| false);
    let n = content.playable.len();
    app.tap(&format!("{} ({n})", crate::i18n::book::STUDY));
    assert_eq!(screen(&app), "Study");
    app.assert_lacks(&format!("{} {page}", book()));
}

#[test]
fn the_page_sheet_opens_its_lecture() {
    let (lecture, _) = ready();
    let mut app = App::with(Screen::Learn, open_page);
    app.tap(crate::i18n::book::OPEN_LECTURE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Learn)), Some(lecture));
    app.assert_lacks(crate::i18n::book::MADE);
}

#[test]
fn a_held_quote_waits_and_a_page_outside_the_lectures_says_so() {
    let mut app = App::with(Screen::Today, || crate::pagesheet::open(5, "", None, true, None));
    app.assert_has(crate::i18n::book::QUOTE_HELD);
    app.assert_has(crate::i18n::book::NO_LECTURE);
    app.assert_has(crate::i18n::book::NOTHING_YET);
    assert!(!app.can_tap(crate::i18n::book::OPEN_LECTURE));
    app.tap("bsheet-close");
    app.assert_lacks(crate::i18n::book::QUOTE_HELD);
}

#[test]
fn a_quote_list_reads_as_bullets_or_a_one_column_table() {
    // Untitled: bullets. Titled without groups: a one-column table.
    let mut app = App::with(Screen::Today, || {
        crate::pagesheet::open(
            284,
            "cvs.L01",
            Some("Hypertension\nBicuspid aortic valve".into()),
            false,
            None,
        )
    });
    assert!(app.html().contains("bk-q-list"));
    app.run(|| {
        crate::pagesheet::open(
            284,
            "cvs.L01",
            Some("Table 3: Valve lesions\nAortic stenosis\nMitral regurgitation".into()),
            false,
            None,
        )
    });
    let html = app.html();
    assert!(html.contains("class=\"d0\""), "{html}");
    assert!(!html.contains("<ul class=\"bk-q-list\""));
}

#[test]
fn a_page_chip_in_a_lesson_opens_its_sheet_and_abbreviations_open_theirs() {
    let mut app = super::learn::play(super::learn::lesson());
    // A step with abbreviations: the tip on PFO.
    super::learn::play_up_to(&mut app, "aside");
    super::learn::tap_here(&mut app, crate::i18n::abbr::CHIP);
    app.assert_has(crate::i18n::abbr::TITLE);
    app.assert_has("Patent foramen ovale");
    app.assert_has(crate::i18n::abbr::UNVERIFIED);
    // A page sheet replaces it, and back.
    app.tap(crate::i18n::abbr::DICT_OPEN);
    app.assert_lacks(crate::i18n::abbr::TITLE);
    app.assert_has(crate::i18n::abbr::DICT_TITLE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Cases)).as_deref(), Some(crate::abbrdict::PAGE));
}

#[test]
fn the_dictionary_lists_a_course_a_to_z_and_searches_it() {
    let mut app = App::with(Screen::Learn, || crate::abbrdict::open(consume_context::<Ui>(), None));
    use crate::i18n::abbr as tx;
    app.assert_has(tx::DICT_TITLE);
    let idx = crate::abbrdict::index();
    let (course, terms) = idx.iter().next().expect("a course has abbreviations");
    app.assert_has(&tx::count(terms.len()));
    app.assert_has(&terms[0].entry.term);
    app.type_into(tx::DICT_SEARCH, "zzzz-nothing");
    app.assert_has(tx::DICT_NO_MATCH);
    let pfo =
        terms.iter().find(|t| t.entry.term == "PFO").map(|t| &t.entry.term).unwrap_or(&terms[0].entry.term);
    app.type_into(tx::DICT_SEARCH, pfo);
    app.assert_lacks(tx::DICT_NO_MATCH);
    // A course chip picks the course.
    let short = &Library::get().program.courses.iter().find(|c| &c.id == course).unwrap().short;
    app.tap("abd-course");
    assert_eq!(app.run(|| crate::abbrdict::COURSE.peek().clone()), Some(course.clone()));
    let _ = short;
    // A lesson that uses the term opens it, and the page closes.
    app.tap("abd-lesson");
    assert_eq!(screen(&app), "Learn");
    assert!(app.run(|| crate::learn::OPEN.peek().clone()).is_some());
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn the_dictionary_opens_from_the_lesson_list() {
    let mut app = App::open(Screen::Learn);
    app.tap("abd-open");
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Learn)).as_deref(), Some(crate::abbrdict::PAGE));
    app.tap(crate::i18n::lecture::BACK);
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn the_course_picker_narrows_the_screens_to_one_course_and_back() {
    use crate::i18n::courses as tx;
    let (lecture, _) = ready();
    let course = steps_core::program::course_of(&lecture).to_string();
    let short = Library::get().program.courses.iter().find(|c| c.id == course).unwrap().short.clone();
    let mut app = App::open(Screen::Learn);
    app.tap("cf-chip");
    app.assert_has(tx::PICK_TITLE);
    app.assert_has(tx::ALL);
    assert!(app.run(crate::courses::focus::is_open));
    let title = Library::get().program.courses.iter().find(|c| c.id == course).unwrap().title.clone();
    tap_row(&mut app, "cf-row", &title);
    assert_eq!(app.run(crate::courses::focus::current), Some(course.clone()));
    assert!(!app.run(crate::courses::focus::is_open));
    let saved: Option<String> = crate::store::shared().setting("course_focus");
    assert_eq!(saved.as_deref(), Some(course.as_str()));
    // The chip names the course; picking "all" shows everything again.
    app.assert_has(&tx::chip_label(&short));
    app.tap("cf-chip");
    app.tap(tx::ALL);
    assert_eq!(app.run(crate::courses::focus::current), None);
    // Following another course's lecture moves the focus there.
    app.run(|| crate::courses::focus::set(Some("renal".into())));
    assert_eq!(app.run(crate::courses::focus::current).as_deref(), Some("renal"));
    app.run(|| crate::courses::focus::follow(&lecture));
    assert_eq!(app.run(crate::courses::focus::current), Some(course.clone()));
    // A course not in the program is not kept.
    app.run(|| crate::courses::focus::set(Some("nope".into())));
    assert_eq!(app.run(crate::courses::focus::current), None);
    // The store's value comes back on reload.
    crate::store::save_setting("course_focus", &"renal".to_string());
    app.run(crate::courses::focus::reload);
    assert_eq!(app.run(crate::courses::focus::current), Some("renal".into()));
}

#[test]
fn the_picker_closes_on_another_screen_and_leads_to_the_map() {
    let mut app = App::open(Screen::Learn);
    app.tap("cf-chip");
    app.tap(Screen::Study.label());
    assert!(!app.run(crate::courses::focus::is_open));
    app.tap(Screen::Learn.label());
    app.tap("cf-chip");
    app.tap(crate::i18n::courses::MAP_LINK);
    assert!(!app.run(crate::courses::focus::is_open));
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Learn)).as_deref(), Some(crate::courses::MAP));
}

#[test]
fn the_course_map_opens_a_course_and_changes_its_status() {
    use crate::i18n::courses as tx;
    let (lecture, _) = ready();
    let course = steps_core::program::course_of(&lecture).to_string();
    let title = Library::get().program.courses.iter().find(|c| c.id == course).unwrap().title.clone();
    let mut app = App::with(Screen::Today, || crate::courses::open_map(consume_context::<Ui>()));
    app.assert_has(tx::G_SYSTEMS);
    app.tap(&title);
    app.assert_has(tx::STATUS);
    app.tap(tx::LATER);
    let lib = Library::get();
    let c = lib.program.courses.iter().find(|c| c.id == course).unwrap();
    assert_eq!(app.run(|| crate::courses::status(c)), steps_core::program::CourseStatus::Later);
    app.assert_has(tx::LATER_HINT);
    app.tap(tx::ACTIVE);
    // Its lectures open on the Lessons screen, narrowed to it.
    app.tap(tx::OPEN_LECTURES);
    assert_eq!(screen(&app), "Learn");
    assert_eq!(app.run(crate::courses::focus::current), Some(course));
    assert!(!app.run(crate::lecture::is_open));
}

#[test]
fn course_statuses_come_back_from_the_store_and_the_map_counts_study() {
    let lib = Library::get();
    let (lecture, _) = ready();
    let course = steps_core::program::course_of(&lecture).to_string();
    let c = lib.program.courses.iter().find(|c| c.id == course).unwrap().clone();
    let mut app = App::with(Screen::Today, super::stats::some_study);
    let mut map = App::with(Screen::Today, || {
        super::stats::some_study();
        crate::courses::open_map(consume_context::<Ui>());
    });
    // A status saved elsewhere (a sync) shows after a reload.
    crate::store::save_setting(&format!("course.{course}"), &"maintenance".to_string());
    app.run(crate::courses::reload);
    assert_eq!(app.run(|| crate::courses::status(&c)), steps_core::program::CourseStatus::Maintenance);
    // The map counts the cards he started.
    map.run(crate::courses::reload);
    map.assert_has(crate::i18n::courses::MAINTENANCE);
    assert!(!map.has(&crate::i18n::courses::started(0, lib.lectures[&lecture].items.len())));
}
