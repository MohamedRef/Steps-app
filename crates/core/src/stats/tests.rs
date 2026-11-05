use super::*;
use crate::review::{Confidence, Origin};

const LECTURE: &str = r#"
schema: steps.content/v1
lecture: cvs.L04
title: Pathology
book: { chapter: 1, pages: [300, 320] }
topics:
  - { id: valves, title: Valvular heart disease }
  - { id: shock, title: Shock and tamponade }
items:
  - id: cvs.L04.001
    kind: card
    topic: valves
    front: Which valve does rheumatic heart disease affect most often?
    back: The mitral valve.
    src: { page: 319 }
    status: unverified
  - id: cvs.L04.002
    kind: mcq
    topic: shock
    stem: Which is not part of Beck triad?
    options: [Hypotension, Distended neck veins, Distant heart sounds, Bradycardia]
    answer: 3
    why: [In it, In it, In it, "Tamponade raises the heart rate"]
    src: { page: 317 }
    status: unverified
  - id: cvs.L04.003
    kind: cloze
    topic: shock
    text: "Pulsus paradoxus is a fall in systolic BP of more than {{c1::10}} mm Hg on inspiration."
    src: { page: 317 }
    status: unverified
  - id: cvs.L04.004
    kind: card
    front: Rejected one
    back: x
    src: { page: 318 }
    status: unverified
"#;

fn lectures() -> Vec<Lecture> {
    vec![serde_yaml_ng::from_str(LECTURE).unwrap()]
}

const NOW: i64 = 20_000 * DAY_MS + 12 * 3_600_000;

fn ev(id: &str, grade: Grade, days_ago: i64) -> ReviewEvent {
    ReviewEvent {
        item_id: id.into(),
        grade,
        origin: Origin::Review,
        at_ms: NOW - days_ago * DAY_MS,
        took_ms: 5000,
        confidence: None,
        reason: None,
    }
}

fn clock() -> Clock {
    Clock { now_ms: NOW, zone: crate::zone::Zone::fixed(0), rollover_hour: 4 }
}

fn marks() -> BTreeMap<String, Status> {
    BTreeMap::from([
        ("cvs.L04.004".to_string(), Status::Rejected),
        ("cvs.L04.001".to_string(), Status::Approved),
    ])
}

#[test]
fn a_period_counts_its_own_days_only() {
    let events = [
        ev("cvs.L04.001", Grade::Good, 10),
        ev("cvs.L04.001", Grade::Again, 3),
        ev("cvs.L04.001", Grade::Good, 1),
        ev("cvs.L04.002", Grade::Good, 1),
        ev("cvs.L04.004", Grade::Good, 1),
    ];
    let week = period(&events, &marks(), clock(), 7, 0);
    assert_eq!(week.days.len(), 7);
    assert_eq!(week.days.last().unwrap().day, clock().today());
    assert_eq!(week.answers, 3, "the rejected item and the old answer are out");
    assert_eq!(week.study_days, 2);
    assert_eq!(week.minutes, 2, "5 s answers round up per day");
    // 001 was first seen 10 days ago; 002's first answer is a first sighting.
    assert_eq!(week.retention, Ratio { right: 1, total: 2 });
    let before = period(&events, &marks(), clock(), 7, 1);
    assert_eq!(before.answers, 1);
    assert_eq!(before.retention, Ratio::default(), "a first sighting isn't retention");
}

#[test]
fn counts_per_lecture_and_topic() {
    let events = [
        ev("cvs.L04.001", Grade::Good, 5),
        ev("cvs.L04.001", Grade::Again, 2),
        ev("cvs.L04.002", Grade::Again, 3),
        ev("cvs.L04.002", Grade::Good, 3),
        ev("cvs.L04.004", Grade::Good, 1),
    ];
    let s = lecture_stats(&lectures(), &events, &marks(), clock());
    let l = &s[0];
    assert_eq!(l.counts.total, 3, "the rejected item doesn't count");
    assert_eq!(l.counts.seen, 2);
    assert_eq!(l.counts.seen_twice, 1, "two answers on one day are one sighting");
    assert_eq!(l.counts.approved, 1);
    assert_eq!(l.counts.unverified, 2);
    // Only 001's later-day answer counts for retention.
    assert_eq!(l.counts.retention, Ratio { right: 0, total: 1 });
    assert_eq!(l.counts.mcq, Ratio { right: 1, total: 2 });
    assert_eq!(l.topics.len(), 2, "no untagged group once the rejected item is out");
    assert_eq!(l.topics[1].counts.total, 2);
    let p = progress(&s);
    assert_eq!((p[0].total, p[0].seen, p[0].seen_twice), (3, 2, 1));
}

#[test]
fn weak_topics_need_enough_answers() {
    let mut events = vec![ev("cvs.L04.001", Grade::Good, 20)];
    for d in 1..=4 {
        events.push(ev("cvs.L04.001", Grade::Again, d));
    }
    let s = lecture_stats(&lectures(), &events, &marks(), clock());
    assert!(weakest_topics(&s, 3).is_empty(), "4 counted answers are too few");
    events.push(ev("cvs.L04.001", Grade::Good, 0));
    for d in 1..=6 {
        events.push(ev("cvs.L04.002", Grade::Good, d));
    }
    let s = lecture_stats(&lectures(), &events, &marks(), clock());
    let weak = weakest_topics(&s, 3);
    assert_eq!(weak.len(), 1, "a topic with no misses isn't weak");
    assert_eq!(weak[0].topic.as_deref(), Some("valves"));
    assert!((weak[0].score - 0.8).abs() < 1e-6);
    let ids = topics_session(&lectures(), &weak, &events, &marks());
    assert_eq!(ids, ["cvs.L04.001"]);
}

#[test]
fn mistakes_log_window_and_recovery() {
    let events = [
        ev("cvs.L04.001", Grade::Again, 20), // too old
        ev("cvs.L04.002", Grade::Again, 3),
        ev("cvs.L04.002", Grade::Again, 2),
        ev("cvs.L04.002", Grade::Good, 1), // one right answer isn't recovery yet
        ev("cvs.L04.003", Grade::Again, 4),
        ev("cvs.L04.003", Grade::Good, 3),
        ev("cvs.L04.003", Grade::Good, 2),  // recovered
        ev("cvs.L04.004", Grade::Again, 1), // rejected
    ];
    let log = mistakes(&lectures(), &events, &marks(), NOW);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].topic.as_deref(), Some("shock"));
    assert_eq!(log[0].title, "Shock and tamponade");
    assert_eq!(
        log[0].items,
        [Mistake {
            item_id: "cvs.L04.002".into(),
            misses: 2,
            last_miss_ms: NOW - 2 * DAY_MS,
            reason: None,
            sure: false,
        }]
    );
    assert_eq!(mistakes_session(&log), ["cvs.L04.002"]);
}

fn tagged(
    id: &str,
    days_ago: i64,
    confidence: Option<Confidence>,
    reason: Option<ErrorReason>,
) -> ReviewEvent {
    ReviewEvent { confidence, reason, ..ev(id, Grade::Again, days_ago) }
}

#[test]
fn misses_he_was_sure_of_come_first() {
    let events = [
        tagged("cvs.L04.003", 5, Some(Confidence::Sure), Some(ErrorReason::Misread)),
        ev("cvs.L04.003", Grade::Good, 4),
        tagged("cvs.L04.002", 1, Some(Confidence::Guess), Some(ErrorReason::Knowledge)),
        ev("cvs.L04.001", Grade::Good, 9),
        tagged("cvs.L04.001", 2, None, None),
    ];
    let log = mistakes(&lectures(), &events, &marks(), NOW);
    let ids: Vec<&str> = log.iter().flat_map(|g| g.items.iter().map(|m| m.item_id.as_str())).collect();
    // Within the topic, the older sure miss leads the newer guess.
    assert_eq!(ids, ["cvs.L04.001", "cvs.L04.003", "cvs.L04.002"]);
    let m3 = &log[1].items[0];
    assert!(m3.sure);
    assert_eq!(m3.reason, Some(ErrorReason::Misread));
    assert_eq!(mistakes_session(&log), ["cvs.L04.003", "cvs.L04.002", "cvs.L04.001"]);
}

#[test]
fn sure_misses_come_back_for_a_retest_after_two_days() {
    let sure = |id: &str, days_ago| tagged(id, days_ago, Some(Confidence::Sure), None);
    let events = [
        sure("cvs.L04.001", 3), // due: not answered since
        sure("cvs.L04.002", 1), // too soon
        sure("cvs.L04.003", 5), // answered on a later day: done
        ev("cvs.L04.003", Grade::Good, 2),
        sure("cvs.L04.004", 3),                                  // rejected
        tagged("cvs.L04.002", 9, Some(Confidence::Guess), None), // a guess is no surprise
    ];
    assert_eq!(retest_due(&lectures(), &events, &marks(), clock()), ["cvs.L04.001"]);
    // A week and a day later it has had its chance.
    let late = [sure("cvs.L04.001", 8)];
    assert!(retest_due(&lectures(), &late, &marks(), clock()).is_empty());
}

#[test]
fn mistakes_count_and_filter_by_reason() {
    let events = [
        tagged("cvs.L04.001", 3, None, Some(ErrorReason::Knowledge)),
        // The latest miss's reason is the one that counts.
        tagged("cvs.L04.002", 3, None, Some(ErrorReason::Timing)),
        tagged("cvs.L04.002", 2, None, Some(ErrorReason::Knowledge)),
        tagged("cvs.L04.003", 1, Some(Confidence::Unsure), None),
    ];
    let log = mistakes(&lectures(), &events, &marks(), NOW);
    let (counts, none) = mistake_reasons(&log);
    assert_eq!(counts, [(ErrorReason::Knowledge, 2)]);
    assert_eq!(none, 1);
    let knowledge = mistakes_with_reason(&log, ErrorReason::Knowledge);
    assert_eq!(knowledge.len(), 2);
    assert!(knowledge.iter().all(|g| g.items.iter().all(|m| m.reason == Some(ErrorReason::Knowledge))));
    assert!(mistakes_with_reason(&log, ErrorReason::Misread).is_empty());
}

#[test]
fn rough_forecast_follows_the_ladder() {
    let events = [
        ev("cvs.L04.001", Grade::Good, 2), // first right answer: back after 1 day, overdue
        ev("cvs.L04.002", Grade::Good, 3),
        ev("cvs.L04.002", Grade::Good, 1), // second: back 3 days after
        ev("cvs.L04.003", Grade::Good, 4),
        ev("cvs.L04.003", Grade::Again, 0), // starts over: back tomorrow
        ev("cvs.L04.004", Grade::Good, 1),  // rejected
    ];
    let f = rough_forecast(&lectures(), &events, &marks(), clock(), 5);
    assert_eq!(f, [1, 1, 1, 0, 0]);
}

#[test]
fn first_sightings_per_day() {
    let events = [
        ev("cvs.L04.001", Grade::Good, 3),
        ev("cvs.L04.001", Grade::Good, 1),
        ev("cvs.L04.002", Grade::Again, 1),
        ev("cvs.L04.004", Grade::Good, 1), // rejected
    ];
    let today = clock().today();
    let got = first_seen_by_day(&lectures(), &events, &marks(), clock());
    assert_eq!(got, BTreeMap::from([(today.plus(-3), 1), (today.plus(-1), 1)]));
}

#[test]
fn day_start_progress_and_today_so_far() {
    let events = [
        ev("cvs.L04.001", Grade::Good, 2),
        ev("cvs.L04.001", Grade::Good, 0),
        ev("cvs.L04.001", Grade::Again, 0),
        ev("cvs.L04.002", Grade::Good, 0),
        ev("cvs.L04.004", Grade::Good, 0), // rejected
    ];
    let p = progress_at_day_start(&lectures(), &events, &marks(), clock());
    assert_eq!((p[0].total, p[0].seen, p[0].seen_twice), (3, 1, 0), "today's answers don't count yet");
    let done = done_today(&lectures(), &events, &marks(), clock());
    assert_eq!(done, DayDone { new: 1, reviews: 2 });
}

#[test]
fn ratio_percent() {
    assert_eq!(Ratio { right: 2, total: 3 }.percent(), Some(67));
    assert_eq!(Ratio::default().percent(), None);
}
