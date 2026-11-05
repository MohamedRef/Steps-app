use super::*;
use crate::review::{Hlc, Origin};

const T0: i64 = 1_793_610_000_000; // 2026-11-02 09:00 UTC = 11:00 Cairo

fn ev(id: &str, item: &str, at_ms: i64, grade: Grade, origin: Origin) -> LoggedReview {
    LoggedReview {
        id: id.into(),
        hlc: Hlc { wall_ms: at_ms, counter: 0, device: "s24".into() },
        event: ReviewEvent::new(item, grade, origin, at_ms, 4000),
    }
}

fn review(id: &str, item: &str, at_ms: i64, grade: Grade) -> LoggedReview {
    ev(id, item, at_ms, grade, Origin::Review)
}

#[test]
fn replay_keeps_summer_answers_on_their_study_day_after_the_clocks_go_back() {
    // Egypt's summer time (UTC+3) ends 2026-10-29 21:00 UTC (#40).
    static CAIRO: [(i64, i32); 1] = [(1_793_307_600_000, 120)];
    let cairo = Zone { base: 180, changes: &CAIRO };
    // 04:30 summer time on 1 and 3 October: two study days apart.
    let first = 1_790_818_200_000; // 2026-10-01T01:30Z
    let log = [review("e1", "x", first, Grade::Good), review("e2", "x", first + 2 * DAY_MS, Grade::Good)];
    let summer = Scheduler::from_settings(&ReviewSettings::default(), 180).replay_all(&log);
    let zoned = Scheduler::from_settings(&ReviewSettings::default(), cairo).replay_all(&log);
    // With the offset in force when he answered, the log replays exactly as it did in October.
    assert_eq!(zoned["x"], summer["x"]);
    // Today's +2 alone would move the 04:30 answer back to the study day before.
    let (z, w) = (DayClock::zoned(cairo), DayClock::local(120));
    assert_eq!(w.day(first), z.day(first) - 1);
}

#[test]
fn good_answers_grow_the_interval() {
    let s = Scheduler::default();
    let a = s.apply(&CardState::new("x"), Grade::Good, T0);
    let b = s.apply(&a, Grade::Good, a.due_ms.unwrap());
    assert!(b.due_ms.unwrap() - b.last_review_ms.unwrap() > a.due_ms.unwrap() - a.last_review_ms.unwrap());
    assert_eq!(b.first_review_ms, Some(T0));
    assert_eq!(b.reps, 2);
}

#[test]
fn again_is_a_lapse_only_after_learning() {
    let s = Scheduler::default();
    let a = s.apply(&CardState::new("x"), Grade::Again, T0);
    assert_eq!(a.lapses, 0);
    let b = s.apply(&a, Grade::Again, T0 + DAY_MS);
    assert_eq!(b.lapses, 1);
    // A short interval is at least ten minutes.
    assert!(a.due_ms.unwrap() - T0 >= MIN_SHORT_INTERVAL_MS);
}

#[test]
fn replay_is_order_independent_and_deduplicated() {
    let s = Scheduler::default();
    let log = vec![review("1", "x", T0, Grade::Good), review("2", "x", T0 + 3 * DAY_MS, Grade::Hard)];
    let mut shuffled = vec![log[1].clone(), log[0].clone(), log[0].clone()];
    let a = s.replay_all(&log);
    let b = s.replay_all(&shuffled);
    assert_eq!(a, b);
    shuffled.truncate(1);
    assert_ne!(s.replay_all(&shuffled), a);
}

#[test]
fn day_clock_rolls_over_at_4am_cairo() {
    let c = DayClock::local(120);
    // 01:30 UTC = 03:30 Cairo: still yesterday's study day.
    let before = T0 - 7 * 3_600_000 - 30 * 60_000;
    let after = before + 60 * 60_000; // 04:30 Cairo
    assert_eq!(c.day(before) + 1, c.day(after));
    assert_eq!(c.day(c.day_start_ms(c.day(after))), c.day(after));
    assert_eq!(c.day_start_ms(c.day(T0)), T0 - 9 * 3_600_000 + 2 * 3_600_000);
}

#[test]
fn due_puts_most_forgotten_first() {
    let s = Scheduler::default();
    let weak = s.apply(&CardState::new("weak"), Grade::Again, T0);
    let strong = s.apply(&CardState::new("strong"), Grade::Easy, T0);
    let later = T0 + 400 * DAY_MS;
    let due = s.due([&strong, &weak], later);
    assert_eq!(due.iter().map(|c| c.item_id.as_str()).collect::<Vec<_>>(), ["weak", "strong"]);
    assert!(s.due([&strong], T0 + 60_000).is_empty());
}

#[test]
fn intervals_are_ordered_and_match_apply() {
    let s = Scheduler::default();
    let a = s.apply(&CardState::new("x"), Grade::Good, T0);
    let at = T0 + 5 * DAY_MS;
    let i = s.intervals(&a, at);
    assert!(i[0] <= i[1] && i[1] <= i[2] && i[2] <= i[3], "{i:?}");
    let good = s.apply(&a, Grade::Good, at);
    assert_eq!(s.day.day(good.due_ms.unwrap()) - s.day.day(at), i[2].round() as i64);
}

#[test]
fn higher_retention_means_shorter_intervals() {
    let at = |r: Retention| {
        let s = Scheduler::from_settings(&ReviewSettings { retention: r, ..Default::default() }, 120);
        let a = s.apply(&CardState::new("x"), Grade::Good, T0);
        s.intervals(&a, T0 + 3 * DAY_MS)[2]
    };
    assert!(at(Retention::R95) < at(Retention::R90));
    assert!(at(Retention::R90) < at(Retention::R85));
}

#[test]
fn bank_answers_follow_the_evidence_rules() {
    let s = Scheduler::default();
    let bank = |id, at, g| ev(id, "x", at, g, Origin::Bank);
    // A new item: a bank answer waits for proper study.
    assert!(s.replay_all(&[bank("1", T0, Grade::Good)]).get("x").unwrap().is_new());
    let studied = review("0", "x", T0, Grade::Good);
    let base = s.replay_all(std::slice::from_ref(&studied))["x"].clone();
    // Same day: ignored, even a miss.
    let same = s.replay_all(&[studied.clone(), bank("1", T0 + 3_600_000, Grade::Again)]);
    assert_eq!(same["x"], base);
    // Next day, not due yet: a right answer is ignored, a miss counts.
    let next = T0 + DAY_MS;
    assert!(base.due_ms.unwrap() > next);
    assert_eq!(s.replay_all(&[studied.clone(), bank("1", next, Grade::Good)])["x"], base);
    assert_eq!(s.replay_all(&[studied.clone(), bank("1", next, Grade::Again)])["x"].reps, 2);
    // Once due, a right answer counts.
    let due = base.due_ms.unwrap() + 1;
    assert_eq!(s.replay_all(&[studied, bank("1", due, Grade::Good)])["x"].reps, 2);
}

#[test]
fn started_today_and_the_daily_cap() {
    let s = Scheduler::default();
    let log = vec![
        review("1", "a", T0 - DAY_MS, Grade::Good),
        review("2", "b", T0, Grade::Good),
        review("3", "a", T0, Grade::Good),
    ];
    let states = s.replay_all(&log);
    assert_eq!(s.started_today(states.values(), T0 + 3_600_000), 1);
    let settings = ReviewSettings { new_per_day: 15, ..Default::default() };
    assert_eq!(new_left_today(&settings, 1), 14);
    assert_eq!(new_left_today(&settings, 20), 0);
}

#[test]
fn forecast_counts_by_day() {
    let s = Scheduler::default();
    let a = s.apply(&CardState::new("a"), Grade::Again, T0);
    let b = s.apply(&CardState::new("b"), Grade::Good, T0);
    let f = s.forecast([&a, &b], T0, 30);
    assert_eq!(f.iter().sum::<u32>(), 2);
    assert_eq!(f[0], 1);
}

#[test]
fn training_needs_400_reviews_and_refits_weekly() {
    let s = Scheduler::default();
    let mut log = Vec::new();
    for item in 0..150 {
        for (n, gap) in [0, 1, 4, 11].into_iter().enumerate() {
            let g = if (item + n) % 7 == 0 { Grade::Again } else { Grade::Good };
            log.push(review(&format!("{item}-{n}"), &format!("i{item}"), T0 + gap * DAY_MS + item as i64, g));
        }
    }
    assert_eq!(s.training_items(&log).len(), 450);
    assert!(s.optimize(&log[..200], T0).is_none());
    let p = s.optimize(&log, T0).expect("fits");
    assert_eq!(p.reviews, 450);
    assert!(Scheduler::new(&p.weights, 0.9, DayClock::default()).is_ok());

    let mut settings = ReviewSettings::default();
    assert!(settings.refit_due(T0));
    settings.fsrs = Some(p);
    assert!(!settings.refit_due(T0 + 6 * DAY_MS));
    assert!(settings.refit_due(T0 + 7 * DAY_MS));
}

#[test]
fn settings_round_trip_and_default_on_missing_fields() {
    let s: ReviewSettings = serde_json::from_str(r#"{"retention":"95"}"#).unwrap();
    assert_eq!(s.retention, Retention::R95);
    assert_eq!(s.new_per_day, 30);
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<ReviewSettings>(&json).unwrap(), s);
}
