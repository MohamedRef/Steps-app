//! Oral mode over the First Aid cardiovascular content: the setup and its
//! kept settings, a round asked, revealed, ticked and rated (each rating a
//! review in the store), the think timer, Again coming back, and the summary.

use steps_core::review::{Grade, Origin};

use super::App;
use super::kit::reviews;
use crate::Screen;
use crate::i18n::oral as tx;

/// The setup, voice off (no speech engine under test), on cvs.L01.
fn setup() -> App {
    let mut app = App::open(Screen::Oral);
    // The voice checkbox: the first field without text (the lecture select
    // has its options' text, so it sorts last).
    app.form_nth("change", "", 0, "false");
    app
}

fn start(app: &mut App) {
    app.tap(tx::START);
}

#[test]
fn the_setup_says_how_many_can_be_asked() {
    let app = App::open(Screen::Oral);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::HONEST);
    app.assert_has(tx::AVAILABLE);
    app.assert_has(tx::ALL_LECTURES);
    app.assert_has(tx::VOICE);
    app.assert_has(tx::RATE);
    app.assert_has(tx::AUTO_HINT);
    let title = crate::content::Library::get().title("cvs.L01").to_string();
    app.assert_has(&title);
}

#[test]
fn the_settings_change_and_are_kept_for_next_time() {
    let mut app = App::open(Screen::Oral);
    app.tap("20");
    app.form("input", "", "25");
    app.assert_has(&format!("{}: {}", tx::THINK, crate::i18n::count::count(25, &crate::i18n::count::SECOND)));
    // An out-of-range value is held to the range.
    app.form("input", "", "90");
    app.assert_has(&crate::i18n::count::count(30, &crate::i18n::count::SECOND));
    app.tap(tx::RATES[2]);
    // Voice off hides the rate.
    app.form_nth("change", "", 0, "false");
    app.assert_lacks(tx::RATE);
    // A lecture picked.
    app.form("change", tx::ALL_LECTURES, "cvs.L01");
    drop(app);
    let app = App::open(Screen::Oral);
    app.assert_has(&crate::i18n::count::count(30, &crate::i18n::count::SECOND));
    app.assert_lacks(tx::RATE);
    assert!(app.html().contains("aria-checked=\"true\">20<"), "{}", app.html());
}

#[test]
fn a_round_is_asked_revealed_ticked_and_rated() {
    let mut app = setup();
    start(&mut app);
    app.assert_has(tx::LISTEN);
    app.assert_has(tx::END);
    app.assert_has(tx::REPEAT);
    app.assert_has(tx::NOTES);
    let left = |app: &App| app.text().matches(tx::LEFT).count();
    assert!(left(&app) > 0);
    // His notes for this question only.
    app.type_into(tx::NOTES_HINT, "septum primum");
    app.tap(tx::SHOW);
    app.assert_has(tx::ANSWER);
    app.assert_has(tx::TICK_HINT);
    app.assert_has(tx::YOUR_NOTES);
    app.assert_has("septum primum");
    app.assert_has(&format!("{} 0 /", tx::SAID));
    // Tick the first point.
    app.tap("oral-point");
    app.assert_has(&format!("{} 1 /", tx::SAID));
    app.tap("oral-point");
    app.tap("oral-point");
    app.tap(tx::GOOD_HINT);
    let saved = reviews();
    assert_eq!(saved.len(), 1);
    assert_eq!((saved[0].event.origin, saved[0].event.grade), (Origin::Oral, Grade::Good));
    // The next question is asked, its notes empty.
    app.assert_has(tx::SHOW);
    app.assert_lacks(tx::YOUR_NOTES);
}

#[test]
fn again_comes_back_later_and_the_summary_lists_it() {
    let mut app = setup();
    start(&mut app);
    app.tap(tx::SHOW);
    app.tap(tx::AGAIN_HINT);
    assert_eq!(reviews()[0].event.grade, Grade::Again);
    app.tap(tx::END);
    app.assert_has(tx::DONE);
    app.assert_has(tx::DONE_AGAIN);
    app.assert_has(tx::MISSED);
    app.assert_lacks(tx::NONE_MISSED);
    // A missed one opens a study session on it (or its lecture page).
    app.tap(tx::MISSED_HINT);
    let html = app.html();
    assert!(html.contains("data-screen=\"Study\"") || html.contains("data-screen=\"Oral\""), "{html}");
}

#[test]
fn ending_at_once_misses_nothing_and_another_round_starts() {
    let mut app = setup();
    start(&mut app);
    app.tap(tx::END);
    app.assert_has(tx::DONE);
    app.assert_has(tx::NONE_MISSED);
    app.tap(tx::ANOTHER);
    app.assert_has(tx::SHOW);
    app.tap(tx::END);
    app.tap(tx::BACK);
    app.assert_has(tx::HONEST);
}

#[test]
fn the_think_timer_counts_down_and_can_be_paused_and_restarted() {
    let mut app = setup();
    start(&mut app);
    app.wait(300);
    app.assert_has(tx::THINKING);
    app.assert_has(tx::PAUSE_TIMER);
    app.tap(tx::PAUSE_TIMER);
    app.assert_has(tx::PAUSED);
    app.assert_has(tx::RESUME_TIMER);
    app.tap(tx::RESUME_TIMER);
    app.assert_has(tx::THINKING);
    app.tap(tx::RESTART_TIMER);
    app.wait(300);
    app.assert_has(tx::THINKING);
    // Repeat asks it again from the start.
    app.tap(tx::REPEAT);
    app.wait(300);
    app.assert_has(tx::PAUSE_TIMER);
}

#[test]
fn a_second_tap_on_a_rating_grades_once() {
    let mut app = setup();
    start(&mut app);
    app.tap(tx::SHOW);
    app.tap(tx::GOOD_HINT);
    // The next question shows unasked: no rating to tap twice.
    assert!(!app.can_tap(tx::GOOD_HINT));
    // Show straight after a rating waits a moment, so a double tap can't
    // flash the next answer.
    app.tap(tx::SHOW);
    app.assert_lacks(tx::TICK_HINT);
    app.wait(800);
    app.tap(tx::SHOW);
    app.assert_has(tx::TICK_HINT);
    assert_eq!(reviews().len(), 1);
}

/// A round whose first question is an enumeration (it has a pass mark):
/// rounds are shuffled, so end and start again until one is.
fn enumeration_first(app: &mut App) {
    start(app);
    for _ in 0..200 {
        if app.has(tx::PASS) {
            return;
        }
        app.tap(tx::END).tap(tx::ANOTHER);
    }
    panic!("no enumeration came first in 200 rounds");
}

#[test]
fn an_enumeration_says_its_pass_mark_and_when_enough_was_said() {
    let mut app = setup();
    enumeration_first(&mut app);
    app.tap(tx::SHOW);
    app.assert_has(tx::PASS);
    app.assert_lacks(tx::ENOUGH);
    let text = app.text();
    let en = super::kit::ids_of("enumerate")
        .into_iter()
        .map(|id| super::kit::item(&id))
        .find(|i| matches!(&i.body, steps_core::content::Body::Enumerate { prompt, .. } if text.contains(prompt.as_str())))
        .expect("the enumeration on screen");
    let steps_core::content::Body::Enumerate { points, .. } = &en.body else { unreachable!() };
    for p in points {
        app.tap(p);
    }
    app.assert_has(tx::ENOUGH);
}

#[test]
fn an_enumeration_asks_what_else_after_the_think_time() {
    let mut app = setup();
    // The shortest think time.
    app.form("input", "", "10");
    enumeration_first(&mut app);
    app.wait(10_400);
    app.assert_has(tx::WHAT_ELSE);
}

#[test]
fn auto_continue_moves_on_without_a_rating() {
    let mut app = setup();
    // The auto-continue checkbox: the second field without text.
    app.form_nth("change", "", 1, "true");
    start(&mut app);
    app.tap(tx::SHOW);
    app.assert_has(tx::TICK_HINT);
    app.wait(8_400);
    // Heard, not rated: nothing saved, the next question asked.
    app.assert_lacks(tx::TICK_HINT);
    assert!(reviews().is_empty());
    app.tap(tx::END);
    app.assert_has(tx::DONE_HEARD);
}

#[test]
fn kept_settings_out_of_range_are_brought_back_in() {
    let odd = serde_json::json!({ "lecture": "zzz.L09", "size": 7, "think": 99, "voice": false, "rate": 9, "auto_continue": false });
    crate::store::shared().put_local_setting("oral.settings", &odd).unwrap();
    let app = App::open(Screen::Oral);
    app.assert_has(&crate::i18n::count::count(30, &crate::i18n::count::SECOND));
    app.assert_lacks(tx::RATE);
    let kept: serde_json::Value = crate::store::shared().local_setting("oral.settings").unwrap();
    assert_eq!(kept["size"], 10);
    assert_eq!(kept["lecture"], serde_json::Value::Null);
    assert_eq!(kept["rate"], 2);
}

#[test]
fn with_the_voice_on_a_missing_engine_falls_back_to_text() {
    let mut app = App::open(Screen::Oral);
    start(&mut app);
    app.wait(200);
    // No speech engine under test: the question stays on screen as text.
    app.assert_has(tx::END);
    app.tap(tx::SHOW);
    app.assert_has(tx::ANSWER);
}
