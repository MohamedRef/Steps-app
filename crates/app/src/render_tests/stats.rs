//! Stats (تقدّمي): its tabs, the numbers after some study, the weakest
//! topics, the mistakes log and the ways each one starts a session.

use dioxus::prelude::*;
use steps_core::review::{Grade, Origin, ReviewEvent, StudyStore};

use super::App;
use super::lecture::screen;
use crate::Screen;
use crate::content::Library;
use crate::i18n::stats as tx;

const DAY_MS: i64 = 86_400_000;

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

/// A few weeks of answers on the ready lecture: most remembered, some
/// missed lately (the mistakes), an MCQ missed too.
pub(super) fn some_study() {
    let lecture = &Library::get().lectures[&ready()];
    let review = consume_context::<crate::store::Review>();
    let now = crate::store::now_ms();
    let record = |item: &str, grade: Grade, days_ago: i64| {
        review.backend.borrow_mut().record(ReviewEvent {
            item_id: item.to_string(),
            grade,
            origin: Origin::Study,
            at_ms: now - days_ago * DAY_MS - 3_600_000,
            took_ms: 8_000,
        });
    };
    for (n, item) in lecture.items.iter().enumerate() {
        match n % 4 {
            0 | 1 => {
                record(&item.id, Grade::Good, 20);
                record(&item.id, Grade::Good, 6);
            }
            2 => {
                record(&item.id, Grade::Good, 12);
                record(&item.id, Grade::Again, 2);
            }
            _ => {}
        }
    }
}

fn tab(app: &mut App, label: &str) {
    super::sheets::tap_row(app, "tab", label);
}

fn stats_with_study() -> App {
    App::with(Screen::Stats, some_study)
}

#[test]
fn an_empty_start_says_nothing_yet_on_every_tab() {
    let mut app = App::open(Screen::Stats);
    app.assert_has(tx::HEAD);
    app.assert_has(tx::NOTHING_YET);
    app.assert_has(tx::WEAK_EMPTY);
    tab(&mut app, tx::TAB_MISTAKES);
    app.assert_has(tx::MISTAKES_EMPTY);
    tab(&mut app, tx::TAB_PLAN);
    app.assert_has(crate::i18n::plan::SETUP_TITLE);
    tab(&mut app, tx::TAB_PROGRESS);
    app.assert_has(tx::SUMMARY);
}

#[test]
fn progress_counts_what_was_studied_and_its_rows_start_sessions() {
    let id = ready();
    let mut app = stats_with_study();
    app.assert_lacks(tx::NOTHING_YET);
    app.assert_has(tx::SEEN);
    app.assert_has(tx::SEEN_TWICE);
    // The lecture's card: its bars and its topics.
    app.assert_has(Library::get().title(&id));
    app.assert_has(tx::COL_RETENTION);
    app.assert_has(tx::TOPICS);
    // The weakest topics start a session on just them.
    app.assert_lacks(tx::WEAK_EMPTY);
    app.tap(tx::WEAK_START);
    assert_eq!(screen(&app), "Study");
}

#[test]
fn a_topic_and_the_lecture_open_from_the_progress_tab() {
    let mut app = stats_with_study();
    app.tap("topic-link");
    assert!(["Study", "Bank"].contains(&screen(&app).as_str()));

    let mut app = stats_with_study();
    app.tap(tx::OPEN_LECTURE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Stats)), Some(ready()));

    let mut app = stats_with_study();
    app.tap(tx::STUDY_HINT);
    assert_eq!(screen(&app), "Study");

    let mut app = stats_with_study();
    app.tap(tx::APPROVED_HINT);
    app.assert_has(crate::i18n::settings::DATA_SUB);
}

#[test]
fn the_period_head_switches_week_and_month_and_opens_the_other_tabs() {
    let mut app = stats_with_study();
    app.assert_has(tx::WEEK);
    app.tap(tx::MONTH);
    app.assert_has(tx::MONTH_START);
    app.tap(tx::WEEK);
    // The recall ring opens the mistakes; the activity opens the plan.
    app.tap("st-hero");
    app.assert_has(tx::MISTAKES_TITLE);
    tab(&mut app, tx::TAB_PROGRESS);
    app.tap("st-activity");
    app.assert_has(crate::i18n::plan::SETUP_TITLE);
}

#[test]
fn the_mistakes_tab_lists_recent_misses_by_topic() {
    let mut app = stats_with_study();
    tab(&mut app, tx::TAB_MISTAKES);
    app.assert_lacks(tx::MISTAKES_EMPTY);
    app.assert_has(tx::MISSED);
    app.tap(tx::MISTAKES_START);
    assert_eq!(screen(&app), "Study");

    let mut app = stats_with_study();
    tab(&mut app, tx::TAB_MISTAKES);
    app.tap("mistake-topic");
    assert_eq!(screen(&app), "Study");

    let mut app = stats_with_study();
    tab(&mut app, tx::TAB_MISTAKES);
    app.tap(tx::MISTAKE_HINT);
    assert_eq!(screen(&app), "Study");
}

#[test]
fn another_screen_can_open_a_tab() {
    let mut app = App::with(Screen::Today, || {
        crate::stats::open(consume_context::<crate::Ui>(), crate::stats::Tab::Mistakes)
    });
    assert_eq!(screen(&app), "Stats");
    app.assert_has(tx::MISTAKES_TITLE);
    // Asked again while already there.
    app.run(|| *crate::plan::SETTINGS.write() = Default::default());
    tab(&mut app, tx::TAB_PROGRESS);
    app.assert_has(tx::SUMMARY);
}
