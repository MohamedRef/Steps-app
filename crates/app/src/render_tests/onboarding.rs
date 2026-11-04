//! First launch: the three quick screens, every optional detail, the
//! checks on each answer, and what is (and isn't) saved.

use dioxus::prelude::*;

use super::App;
use crate::Screen;
use crate::i18n::onboard as tx;
use crate::onboarding::{Answers, OPEN};
use crate::store::shared;

fn minutes(n: u32) -> String {
    crate::i18n::count::count(n, &crate::i18n::count::MINUTE)
}

fn plan(app: &mut App) -> steps_core::plan::PlanSettings {
    app.run(|| crate::plan::SETTINGS.peek().clone())
}

fn today() -> steps_core::plan::Day {
    crate::store::zone().study_day(crate::store::now_ms())
}

#[test]
fn the_quick_path_saves_the_time_and_exam_day_and_opens_a_lesson() {
    let mut app = App::first_launch(Screen::Today);
    app.assert_has(tx::WELCOME_TITLE);
    app.assert_has(&tx::progress(1, 3));
    app.tap(tx::CONTINUE);

    app.assert_has(tx::TIME_TITLE);
    app.assert_has(&tx::progress(2, 3));
    app.tap(&minutes(15));
    // A day already gone keeps him here, with a gentle note.
    app.form("input", "", "2020-01-01");
    app.tap(tx::CONTINUE);
    app.assert_has(tx::PICK_FUTURE);
    // Typing again clears it.
    app.form("input", "", "2099-01-01");
    app.assert_lacks(tx::PICK_FUTURE);
    app.tap(tx::CONTINUE);
    app.assert_has(tx::PICK_FAR);
    let exam = today().plus(120);
    app.form("input", "", &exam.iso());
    app.tap(tx::CONTINUE);

    app.assert_has(tx::READY_TITLE);
    app.assert_has(&tx::progress(3, 3));
    app.assert_lacks(tx::LATER);
    assert_eq!(plan(&mut app).exam, Some(exam));
    assert_eq!(Answers::load().minutes, Some(15));
    assert_eq!(app.run(crate::onboarding::session_minutes), 15);
    // Back goes one screen back, and on again.
    app.tap(tx::BACK);
    app.assert_has(tx::TIME_TITLE);
    app.tap(tx::CONTINUE);
    app.tap(tx::START_LESSON);
    assert!(!app.run(|| *OPEN.peek()), "closed");
    assert_eq!(shared().local_setting::<bool>("onboarded"), Some(true));
    app.assert_lacks(tx::READY_TITLE);
}

#[test]
fn clicking_through_unchanged_saves_nothing() {
    let mut app = App::first_launch(Screen::Today);
    let before = plan(&mut app);
    app.tap(tx::CONTINUE);
    app.tap(tx::CONTINUE);
    app.assert_has(tx::READY_TITLE);
    // The summary says what stands: every day, nothing studied, 19:00.
    app.assert_has(&tx::days_sub(before.weekdays.iter().filter(|d| **d).count()));
    app.assert_has(&tx::studied_sub(0));
    app.assert_has("19:00");
    app.tap(tx::START_TODAY);
    assert_eq!(plan(&mut app), before);
    assert_eq!(shared().setting::<Answers>("onboard"), None, "a new device can't overwrite synced answers");
    assert_eq!(shared().setting::<serde_json::Value>("remind"), None);
    app.assert_has("data-screen=\"Today\"");
}

#[test]
fn later_closes_it_and_settings_opens_it_again() {
    let mut app = App::first_launch(Screen::Today);
    app.tap(tx::LATER);
    app.assert_lacks(tx::WELCOME_TITLE);
    assert_eq!(shared().local_setting::<bool>("onboarded"), Some(true));
    app.tap(Screen::Settings.label());
    app.tap(crate::i18n::study_settings::SECTION);
    app.assert_has(tx::REOPEN_TITLE);
    app.tap(tx::REOPEN);
    app.assert_has(tx::WELCOME_TITLE);
}

fn to_ready(app: &mut App) {
    app.tap(tx::CONTINUE);
    app.tap(tx::CONTINUE);
    app.assert_has(tx::READY_TITLE);
}

#[test]
fn study_days_need_at_least_one() {
    let mut app = App::first_launch(Screen::Today);
    to_ready(&mut app);
    app.tap(tx::DAYS_ROW);
    app.assert_has(tx::DAYS_TITLE);
    let days = plan(&mut app).weekdays;
    for (w, name) in crate::i18n::plan::WEEKDAYS.iter().enumerate() {
        if days[w] {
            app.tap(name);
        }
    }
    app.tap(tx::DETAIL_DONE);
    app.assert_has(tx::PICK_A_DAY);
    // Picking a day clears the note; Done saves and goes back.
    let first = crate::i18n::plan::WEEKDAYS[0];
    app.tap(first);
    app.assert_lacks(tx::PICK_A_DAY);
    app.tap(tx::DETAIL_DONE);
    app.assert_has(tx::READY_TITLE);
    app.assert_has(&tx::days_sub(1));
    let saved = plan(&mut app).weekdays;
    assert_eq!(saved.iter().filter(|d| **d).count(), 1);
    assert!(saved[0]);
    // Back from a detail leaves it unsaved.
    app.tap(tx::DAYS_ROW);
    app.tap(crate::i18n::plan::WEEKDAYS[1]);
    app.tap(tx::BACK);
    app.assert_has(tx::READY_TITLE);
    assert_eq!(plan(&mut app).weekdays, saved);
}

#[test]
fn lectures_studied_before_are_picked_by_course() {
    let mut app = App::first_launch(Screen::Today);
    to_ready(&mut app);
    app.tap(tx::STUDIED_ROW);
    app.assert_has(tx::STUDIED_TITLE);
    let lectures = crate::content::Library::get().index.lectures.len();
    assert!(lectures > 0);
    // The first cardiovascular lecture is listed under its course.
    app.assert_has("Embryology");
    app.tap(tx::ALL);
    app.tap(tx::DETAIL_DONE);
    app.assert_has(&tx::studied_sub(lectures));
    assert_eq!(Answers::load().studied.len(), lectures);

    app.tap(tx::STUDIED_ROW);
    app.tap(tx::NONE);
    // One lecture, then up to the third of its course.
    app.tap("onboard-lecture");
    assert!(app.html().contains(r#"class="onboard-lecture" aria-pressed="true""#));
    app.tap("onboard-lecture");
    app.tap_nth(tx::UP_TO_HERE, 2);
    app.tap(tx::DETAIL_DONE);
    let studied = Answers::load().studied;
    assert_eq!(studied.len(), 3, "{studied:?}");
    let course = steps_core::program::course_of(&studied[0]).to_owned();
    assert!(studied.iter().all(|id| steps_core::program::course_of(id) == course), "{studied:?}");
    app.assert_has(&tx::studied_sub(3));
}

#[test]
fn reminder_times_are_added_changed_and_removed() {
    let mut app = App::first_launch(Screen::Today);
    to_ready(&mut app);
    app.tap(tx::REMIND_ROW);
    app.assert_has(tx::REMIND_TITLE);
    app.tap(tx::ADD_TIME);
    app.tap(tx::ADD_TIME);
    assert!(!app.can_tap(tx::ADD_TIME), "three at most");
    app.form("change", tx::REMIND_TITLE, "07:30");
    app.form("change", tx::REMIND_TITLE, "not a time");
    app.tap(tx::DETAIL_DONE);
    app.assert_has("07:30 · 21:00 · 23:00");
    let saved: crate::reminders::RemindPrefs = shared().setting("remind").unwrap();
    assert_eq!(saved.times, [7 * 60 + 30, 21 * 60, 23 * 60]);
    assert!(saved.on);

    // None left: reminders off.
    app.tap(tx::REMIND_ROW);
    for _ in 0..3 {
        app.tap(tx::REMOVE);
    }
    app.assert_has(tx::NO_REMINDERS);
    app.tap(tx::DETAIL_DONE);
    app.assert_has(tx::REMIND_OFF);
    let saved: crate::reminders::RemindPrefs = shared().setting("remind").unwrap();
    assert!(!saved.on);
}

#[test]
fn a_theme_is_picked_and_kept_on_this_device() {
    let mut app = App::first_launch(Screen::Today);
    to_ready(&mut app);
    app.tap(tx::THEME_ROW);
    app.assert_has(tx::THEME_TITLE);
    for name in [
        crate::i18n::settings::THEME_SYSTEM,
        crate::i18n::settings::THEME_SYSTEM_BLACK,
        crate::i18n::settings::THEME_PAPER,
        crate::i18n::settings::THEME_BLACK,
        crate::i18n::settings::THEME_NIGHT,
    ] {
        app.assert_has(name);
    }
    assert!(app.html().contains("--sw-a:"), "each theme has its swatch");
    app.tap(crate::i18n::settings::THEME_SHIFT);
    assert!(app.html().contains(r#"aria-checked="true""#));
    app.tap(tx::DETAIL_DONE);
    app.assert_has(crate::i18n::settings::THEME_SHIFT);
    assert_eq!(crate::prefs::DevicePrefs::load().theme, steps_core::theme::Theme::NightShift);
}

#[test]
fn todays_session_length_chips_save_a_new_default() {
    let mut app = App::open(Screen::Today);
    assert_eq!(app.run(crate::onboarding::session_minutes), 10);
    app.run(|| crate::onboarding::set_session_minutes(20));
    assert_eq!(app.run(crate::onboarding::session_minutes), 20);
    // The same length again writes nothing new.
    app.run(|| crate::onboarding::set_session_minutes(20));
    assert_eq!(Answers::load().minutes, Some(20));
}
