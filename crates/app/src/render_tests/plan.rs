//! The exam planner (Stats › خطة الامتحان): setting the exam day, the
//! study days and the review cap, the plan it draws, and editing it.

use dioxus::prelude::*;
use steps_core::plan::{Day, PlanSettings};

use super::App;
use super::lecture::screen;
use crate::Screen;
use crate::i18n::plan as tx;

/// Today's study day, as the app counts it.
fn today() -> Day {
    steps_core::stats::Clock {
        now_ms: crate::store::now_ms(),
        zone: crate::store::zone(),
        rollover_hour: steps_core::srs::ROLLOVER_HOUR as u32,
    }
    .today()
}

fn planner() -> App {
    let mut app = App::open(Screen::Stats);
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app
}

/// Sends `value` to the input whose `type` is `kind` (date, number).
fn enter(app: &mut App, kind: &str, value: &str) {
    let n = app
        .page
        .walk()
        .into_iter()
        .filter(|&n| app.page.nodes[n].listeners.contains(&"input") && app.page.attr(n, "type") == Some(kind))
        // The exam day is the first date; the review cap is the last number.
        .reduce(|first, n| if kind == "number" { n } else { first })
        .unwrap_or_else(|| panic!("no {kind} field:\n{}", app.listing("input")));
    let data = dioxus_html::SerializedFormData::new(value.to_string(), Vec::new());
    app.fire("input", dioxus_html::EventData::Form(data), n);
}

/// Whether the weekday chip `name` is on.
fn pressed(app: &App, name: &str) -> bool {
    app.page.walk().into_iter().any(|n| {
        app.page.attr(n, "class") == Some("weekday")
            && app.page.text(n) == name
            && app.page.attr(n, "aria-pressed") == Some("true")
    })
}

fn saved() -> PlanSettings {
    crate::store::shared().setting("plan").unwrap_or_default()
}

#[test]
fn setting_an_exam_date_draws_the_plan() {
    let mut app = planner();
    app.assert_has(tx::SETUP_TITLE);
    app.assert_lacks(tx::CANCEL);
    let exam = today().plus(60);
    enter(&mut app, "date", &exam.iso());
    enter(&mut app, "number", "200");
    app.tap(tx::SAVE);
    let s = saved();
    assert_eq!(s.exam, Some(exam));
    assert_eq!(s.review_cap, 200);
    // The plan: when the exam is, today's share, the load, the pace, the
    // passes and the days ahead.
    app.assert_has(tx::EXAM_ON);
    app.assert_has(&exam.ymd().0.to_string());
    app.assert_has(tx::LOAD_TITLE);
    app.assert_has(&format!("{} (200)", tx::LOAD_KEY_CAP));
    app.assert_has(tx::PROGRESS);
    app.assert_has(tx::FIRST_PASS);
    app.assert_has(tx::WEEK);
    app.assert_has(tx::ALL_DAYS);
    assert!(app.can_tap(tx::EDIT));
}

#[test]
fn a_bad_date_or_no_study_day_is_refused() {
    let mut app = planner();
    // No day picked.
    app.tap(tx::SAVE);
    app.assert_has(tx::PICK_DAY);
    // Today itself is too soon.
    enter(&mut app, "date", &today().iso());
    app.assert_lacks(tx::PICK_DAY);
    app.tap(tx::SAVE);
    app.assert_has(tx::PICK_DAY);
    // More than three years away.
    enter(&mut app, "date", &today().plus(4 * 366).iso());
    app.tap(tx::SAVE);
    app.assert_has(tx::PICK_DAY_FAR);
    // Every weekday off.
    enter(&mut app, "date", &today().plus(30).iso());
    for d in tx::WEEKDAYS {
        if pressed(&app, d) {
            app.tap(d);
        }
    }
    assert!(tx::WEEKDAYS.iter().all(|d| !pressed(&app, d)));
    app.tap(tx::SAVE);
    app.assert_has(tx::PICK_WEEKDAY);
    assert_eq!(saved().exam, None, "nothing was saved");
}

#[test]
fn the_plan_can_be_edited_or_left_as_it_was() {
    let exam = today().plus(45);
    let mut app = planner();
    enter(&mut app, "date", &exam.iso());
    app.tap(tx::SAVE);
    app.tap(tx::EDIT);
    app.assert_has(tx::SETUP_TITLE);
    // Cancel keeps the plan.
    enter(&mut app, "date", &today().plus(90).iso());
    app.tap(tx::CANCEL);
    assert_eq!(saved().exam, Some(exam));
    app.assert_has(tx::EXAM_ON);
    // A new day replaces it.
    app.tap(tx::EDIT);
    enter(&mut app, "date", &today().plus(90).iso());
    app.tap(tx::SAVE);
    assert_eq!(saved().exam, Some(today().plus(90)));
}

#[test]
fn today_in_the_plan_starts_a_session_and_its_progress_opens_the_numbers() {
    let mut app = planner();
    enter(&mut app, "date", &today().plus(60).iso());
    // Every day a study day, so today has its share.
    for d in tx::WEEKDAYS {
        if !pressed(&app, d) {
            app.tap(d);
        }
    }
    app.tap(tx::SAVE);
    app.assert_has(&format!("{}:", tx::TODAY));
    // The progress card opens the progress tab.
    app.tap(tx::PROGRESS_HINT);
    app.assert_has(crate::i18n::stats::SUMMARY);
    // Today's numbers start a session.
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app.tap(tx::START_HINT);
    assert_eq!(screen(&app), "Study");
}

#[test]
fn a_day_of_the_plan_opens_its_lecture() {
    let mut app = planner();
    enter(&mut app, "date", &today().plus(60).iso());
    app.tap(tx::SAVE);
    if app.can_tap(tx::DAY_HINT) {
        app.tap(tx::DAY_HINT);
        assert!(app.run(crate::lecture::is_open));
    }
}

#[test]
fn a_rest_day_says_so_and_points_to_the_next_study_day() {
    // Only tomorrow's weekday is a study day: today rests.
    let mut app = App::with(Screen::Stats, || {
        let today = today();
        let mut weekdays = [false; 7];
        weekdays[today.plus(1).weekday()] = true;
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(today.plus(40)), weekdays, ..PlanSettings::default() };
    });
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app.assert_has(tx::REST_TODAY);
    app.assert_has(tx::FOCUS_REST);
}

#[test]
fn a_plan_too_tight_for_the_days_left_says_so() {
    // Three days left and nothing seen: the last days are dedicated.
    let mut app = App::with(Screen::Stats, || {
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(today().plus(3)), weekdays: [true; 7], ..PlanSettings::default() };
    });
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app.assert_has(tx::TIGHT);
    app.assert_has(tx::TIGHT_DEDICATED);
    assert!(app.has(tx::FOCUS_WEAK) || app.has(tx::FOCUS_MOCK), "{}", app.text());
    app.assert_has(tx::PACE_NOT_DONE);
}

#[test]
fn a_roomy_plan_is_on_track() {
    let mut app = App::with(Screen::Stats, || {
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(today().plus(400)), weekdays: [true; 7], ..PlanSettings::default() };
    });
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app.assert_has(tx::ON_TRACK);
    app.assert_has(tx::PACE_DONE_ON);
    app.assert_has(tx::DEDICATED_FROM);
}

/// Every item seen once, yesterday.
fn all_seen() {
    use steps_core::review::StudyStore;
    let review = consume_context::<crate::store::Review>();
    let now = crate::store::now_ms();
    for l in crate::content::Library::get().lectures.values() {
        for i in &l.items {
            review.backend.borrow_mut().record(steps_core::review::ReviewEvent {
                item_id: i.id.clone(),
                grade: steps_core::review::Grade::Good,
                origin: steps_core::review::Origin::Study,
                at_ms: now - 86_400_000,
                took_ms: 5000,
            });
        }
    }
    *crate::plan::SETTINGS.write() =
        PlanSettings { exam: Some(today().plus(3)), weekdays: [true; 7], ..PlanSettings::default() };
}

#[test]
fn the_last_days_and_a_passed_exam_read_differently() {
    // Everything seen and inside dedicated: no new items.
    let mut app = App::with(Screen::Stats, all_seen);
    super::sheets::tap_row(&mut app, "tab", crate::i18n::stats::TAB_PLAN);
    app.assert_has(tx::DEDICATED);
    // The exam day itself: done.
    app.run(|| {
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(today()), weekdays: [true; 7], ..PlanSettings::default() }
    });
    app.assert_has(tx::EXAM_DONE);
}

thread_local! {
    static REVIEW: std::cell::RefCell<Option<crate::store::Review>> = const { std::cell::RefCell::new(None) };
}

fn keep_review() {
    REVIEW.with_borrow_mut(|r| *r = Some(consume_context::<crate::store::Review>()));
}

fn target(app: &mut App) -> Option<crate::plan::DailyTarget> {
    let review = REVIEW.with_borrow(|r| r.clone()).expect("kept by keep_review");
    app.run(|| crate::plan::target_now(&*review.backend.borrow()))
}

#[test]
fn the_daily_target_follows_the_plan() {
    let mut app = App::with(Screen::Today, keep_review);
    // No plan: the review settings' cap rules.
    assert!(target(&mut app).is_none());
    app.run(|| {
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(today().plus(60)), weekdays: [true; 7], ..PlanSettings::default() };
    });
    let t = target(&mut app).expect("a target");
    assert_eq!(t.days_left, 60);
    assert!(t.today().is_some());
    assert!(t.new_goal() >= t.new_left());
    let review = REVIEW.with_borrow(|r| r.clone()).unwrap();
    let left = app.run(|| crate::plan::new_left_today(&*review.backend.borrow(), crate::store::now_ms()));
    assert_eq!(left, t.new_left() as usize);
    // Saving and reloading keep it.
    let s = app.run(|| crate::plan::SETTINGS.peek().clone());
    crate::plan::save(&s);
    app.run(|| *crate::plan::SETTINGS.write() = PlanSettings::default());
    app.run(crate::plan::reload);
    assert_eq!(app.run(|| crate::plan::SETTINGS.peek().exam), Some(today().plus(60)));
}
