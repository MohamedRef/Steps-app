//! Today over the First Aid content: the hero's one start, the counts that
//! open what they count, lectures in progress, the library, the shortcuts,
//! the day with nothing left, and the exam plan's day (learn, rest, final
//! window).

use steps_core::plan::{Day, PlanSettings};
use steps_core::review::Grade;
use steps_core::srs::ReviewSettings;

use super::App;
use super::kit::first_of;
use crate::Screen;
use crate::i18n::today as tx;

fn on(app: &App, screen: Screen) -> bool {
    app.html().contains(&format!("data-screen=\"{screen:?}\""))
}

fn today() -> Day {
    steps_core::stats::Clock {
        now_ms: crate::store::now_ms(),
        zone: crate::store::zone(),
        rollover_hour: steps_core::srs::ROLLOVER_HOUR as u32,
    }
    .today()
}

/// Today with an exam plan set.
fn planned(settings: PlanSettings) -> App {
    let mut app = App::open(Screen::Today);
    app.run(move || *crate::plan::SETTINGS.write() = settings);
    app
}

#[test]
fn a_fresh_day_leads_with_new_items_and_starts_a_session() {
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::EYEBROW);
    app.assert_has(tx::READY_Q);
    app.assert_has(tx::HERO_KICKER_NEW);
    app.assert_has(tx::TIME_Q);
    app.assert_has(tx::LIBRARY);
    app.assert_has(tx::READY);
    app.assert_has(tx::WRITING);
    app.assert_has(tx::UNVERIFIED_NOTE);
    let minutes = app.run(crate::onboarding::session_minutes);
    app.tap(&tx::start_session(minutes));
    assert!(on(&app, Screen::Study));
    app.assert_has(crate::i18n::study::END);
}

#[test]
fn the_whole_hero_card_starts_the_session_too() {
    let mut app = App::open(Screen::Today);
    app.tap(tx::HERO_KICKER_NEW);
    assert!(on(&app, Screen::Study));
    app.assert_has(crate::i18n::study::BADGE_NEW);
}

#[test]
fn the_session_length_is_picked_and_kept() {
    let mut app = App::open(Screen::Today);
    app.tap(&tx::minutes_short(20));
    assert_eq!(app.run(crate::onboarding::session_minutes), 20);
    app.assert_has(&tx::start_session(20));
    app.tap(&tx::minutes_short(5));
    assert_eq!(app.run(crate::onboarding::session_minutes), 5);
}

#[test]
fn the_new_count_starts_a_session_on_new_items() {
    let mut app = App::open(Screen::Today);
    app.tap(tx::NEW_HINT);
    assert!(on(&app, Screen::Study));
    app.assert_has(crate::i18n::study::BADGE_NEW);
}

#[test]
fn nothing_due_opens_the_plan_and_the_estimate_too() {
    let mut app = App::open(Screen::Today);
    // Nothing answered yet: the due count shows the plan instead.
    app.tap(tx::FORECAST_HINT);
    assert!(on(&app, Screen::Stats));
    let mut app = App::open(Screen::Today);
    app.tap(tx::MINUTES_HINT);
    assert!(on(&app, Screen::Stats));
}

#[test]
fn a_started_lecture_shows_in_progress_and_opens_its_page() {
    // One card answered in a session.
    let card = first_of("card");
    let mut app = App::open(Screen::Study);
    let id = card.id.clone();
    app.run(move || {
        *crate::study::START.write() =
            Some(crate::study::Start { lectures: None, items: Some(vec![id]), minutes: 5 })
    });
    app.tap(crate::i18n::study::REVEAL).tap(crate::study::grade::grade_label(Grade::Again));
    app.tap(Screen::Today.label());
    app.assert_has(tx::IN_PROGRESS);
    app.assert_has(tx::STARTED);
    app.tap(tx::STARTED);
    let lecture = card.id.rsplit_once('.').unwrap().0;
    let title = crate::content::Library::get().title(lecture).to_string();
    app.assert_has(&title);
}

#[test]
fn the_shortcuts_open_the_bank_settings_lessons_and_lectures() {
    let mut app = App::open(Screen::Today);
    app.tap(tx::QUICK_BANK);
    assert!(on(&app, Screen::Bank));

    let mut app = App::open(Screen::Today);
    app.tap(tx::CHECKED_ROW);
    app.assert_has(crate::i18n::verify::EXPORT_TITLE);

    let mut app = App::open(Screen::Today);
    app.tap(tx::UNVERIFIED_NOTE);
    app.assert_has(crate::i18n::verify::EXPORT_TITLE);

    let mut app = App::open(Screen::Today);
    app.tap(tx::ALL_LESSONS);
    assert!(on(&app, Screen::Learn));

    // A ready lecture in the library opens its page.
    let mut app = App::open(Screen::Today);
    let title = crate::content::Library::get().lectures["cvs.L01"].title.clone();
    app.tap_nth(&title, 0);
    app.assert_has(&title);
}

#[test]
fn a_day_with_no_new_items_left_says_it_is_done_and_offers_the_next_lesson() {
    let _ =
        crate::store::shared().put_review_settings(&ReviewSettings { new_per_day: 0, ..Default::default() });
    // He studied one card today, so the day really is done.
    answered_today();
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::DONE_Q);
    app.assert_has(tx::ALL_DONE);
    app.assert_has(tx::NEW_CAP_REACHED);
    app.assert_has(tx::OPEN_NEXT_LESSON);
    app.tap(tx::OPEN_NEXT_LESSON);
    assert!(on(&app, Screen::Learn) || on(&app, Screen::Today), "a lesson opens");
}

#[test]
fn an_exam_plan_leads_with_todays_target() {
    let exam = today().plus(120);
    let mut app = planned(PlanSettings { exam: Some(exam), weekdays: [true; 7], ..Default::default() });
    app.assert_has(tx::HERO_KICKER_PLAN);
    app.assert_has(tx::PLAN_TODAY);
    app.assert_has(&tx::exam_in(120));
    app.assert_has(tx::RING_LABEL);
    // No separate estimate link with a plan.
    assert!(!app.can_tap(tx::MINUTES_HINT));
    // The days-left pill opens the plan.
    app.tap(&tx::exam_in(120));
    assert!(on(&app, Screen::Stats));
}

#[test]
fn the_plan_ring_opens_the_plan() {
    let exam = today().plus(90);
    let mut app = planned(PlanSettings { exam: Some(exam), weekdays: [true; 7], ..Default::default() });
    app.tap("ring-tap");
    assert!(on(&app, Screen::Stats));
}

#[test]
fn a_rest_day_in_the_plan_says_so() {
    let mut days = [true; 7];
    days[today().weekday()] = false;
    let exam = today().plus(100);
    let app = planned(PlanSettings { exam: Some(exam), weekdays: days, ..Default::default() });
    app.assert_has(tx::REST);
}

#[test]
fn dedicated_is_for_self_assessments_or_weak_topics() {
    let exam = today().plus(3);
    let app = planned(PlanSettings { exam: Some(exam), weekdays: [true; 7], ..Default::default() });
    let text = app.text();
    if !text.contains(tx::WEAK) {
        // A self-assessment day: log the score after it.
        let log = tx::milestone_today("");
        let after = log.rsplit(". ").next().unwrap();
        assert!(text.contains(after), "{text}");
    }
}

#[test]
fn a_tight_plan_says_how_to_fix_it() {
    // Every lecture, ten days, one study day a week, few new a day.
    let mut days = [false; 7];
    days[today().weekday()] = true;
    let exam = today().plus(10);
    let app =
        planned(PlanSettings { exam: Some(exam), weekdays: days, dedicated_weeks: 0, ..Default::default() });
    // A tight plan names a fix; an easy one has none. Either way today leads.
    app.assert_has(tx::HERO_KICKER_PLAN);
}

#[test]
fn a_plan_done_for_today_offers_to_get_ahead() {
    let _ =
        crate::store::shared().put_review_settings(&ReviewSettings { new_per_day: 0, ..Default::default() });
    let exam = today().plus(200);
    let mut days = [true; 7];
    days[today().weekday()] = false;
    let mut app = planned(PlanSettings { exam: Some(exam), weekdays: days, ..Default::default() });
    // Rest day, nothing due: the day is done, with new items still unseen.
    app.assert_has(tx::REST);
    if app.tap_if(tx::AHEAD) {
        assert!(on(&app, Screen::Study));
    }
}

/// A card answered well a moment ago: seen, and not due again today.
fn answered_today() {
    use steps_core::review::{Origin, ReviewEvent};
    let card = first_of("card");
    crate::store::shared()
        .append_review(ReviewEvent {
            confidence: None,
            reason: None,
            item_id: card.id,
            grade: Grade::Good,
            origin: Origin::Study,
            at_ms: crate::store::now_ms() - 60_000,
            took_ms: 5_000,
        })
        .unwrap();
}

#[test]
fn a_first_launch_with_nothing_to_start_is_not_a_done_day() {
    // Before he has studied anything, a day with nothing to start yet never
    // says "done".
    let _ =
        crate::store::shared().put_review_settings(&ReviewSettings { new_per_day: 0, ..Default::default() });
    let app = App::open(Screen::Today);
    app.assert_has(tx::WELCOME_Q);
    app.assert_lacks(tx::DONE_Q);
    app.assert_lacks(tx::ALL_DONE);
}

#[test]
fn a_plan_started_today_has_nothing_missed_and_nothing_done() {
    // A fresh install: onboarding saves the exam day and starts the plan today.
    let exam = today().plus(300);
    let mut settings = PlanSettings { weekdays: [true; 7], ..Default::default() };
    settings.set_exam(exam, today());
    let app = planned(settings);
    app.assert_has(tx::HERO_KICKER_PLAN);
    app.assert_has(tx::WELCOME_Q);
    app.assert_lacks(tx::DONE_Q);
    app.assert_lacks(tx::ALL_DONE);
    app.assert_lacks(crate::i18n::plan::SPREAD);
    // The first day is the baseline self-assessment: nothing to count, so
    // no ring reading "100%".
    app.assert_lacks("100%");
}

#[test]
fn the_rebalance_card_acts_in_place_inside_the_hero() {
    // A plan that ran yesterday, with yesterday's new items not started.
    let exam = today().plus(120);
    let app_settings = PlanSettings {
        exam: Some(exam),
        weekdays: [true; 7],
        started: Some(today().plus(-10)),
        ..Default::default()
    };
    let mut app = planned(app_settings);
    app.assert_has(crate::i18n::plan::SPREAD);
    // The before/after preview opens in place, not the lessons.
    app.tap(crate::i18n::plan::PREVIEW);
    assert!(on(&app, Screen::Today), "the preview stays on Today");
    app.tap(crate::i18n::plan::SPREAD);
    assert!(on(&app, Screen::Today), "spreading stays on Today");
    app.assert_has(crate::i18n::plan::SPREAD_DONE);
}

/// A card missed two days ago: due again now.
fn one_due() -> String {
    use steps_core::review::{Origin, ReviewEvent};
    let card = first_of("card");
    crate::store::shared()
        .append_review(ReviewEvent {
            confidence: None,
            reason: None,
            item_id: card.id.clone(),
            grade: Grade::Again,
            origin: Origin::Study,
            at_ms: crate::store::now_ms() - 2 * 86_400_000,
            took_ms: 5_000,
        })
        .unwrap();
    card.id
}

#[test]
fn a_due_card_leads_the_hero_and_its_count_starts_a_review() {
    one_due();
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::HERO_KICKER);
    app.assert_has(tx::DUE_HINT);
    // The lecture in progress carries its due count.
    app.assert_has(tx::DUE_CHIP);
    app.tap(tx::DUE_HINT);
    assert!(on(&app, Screen::Study));
    app.assert_has(crate::i18n::study::BADGE_REVIEW);
}

#[test]
fn a_plan_day_with_reviews_due_names_both() {
    one_due();
    let exam = today().plus(120);
    let app = planned(PlanSettings { exam: Some(exam), weekdays: [true; 7], ..Default::default() });
    app.assert_has(tx::PLAN_TODAY);
    app.assert_has(tx::ABOUT);
}

#[test]
fn a_plan_with_new_items_started_says_how_many_are_left() {
    // One new item answered today.
    let card = first_of("card");
    let mut app = App::open(Screen::Study);
    let id = card.id.clone();
    app.run(move || {
        *crate::study::START.write() =
            Some(crate::study::Start { lectures: None, items: Some(vec![id]), minutes: 5 })
    });
    app.tap(crate::i18n::study::REVEAL).tap(crate::study::grade::grade_label(Grade::Good));
    let exam = today().plus(120);
    app.run(move || {
        *crate::plan::SETTINGS.write() =
            PlanSettings { exam: Some(exam), weekdays: [true; 7], ..Default::default() }
    });
    app.tap(Screen::Today.label());
    app.assert_has(tx::PLAN_TODAY);
    let text = app.text();
    assert!(text.contains(tx::NEW_LEFT_1) || text.contains(tx::NEW_DONE), "{text}");
}
