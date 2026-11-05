use super::*;

#[test]
fn memory_sink_keeps_order() {
    let mut sink = MemorySink::default();
    for (i, grade) in [Grade::Again, Grade::Good].into_iter().enumerate() {
        sink.record(ReviewEvent {
            item_id: format!("cvs.L04.00{i}"),
            grade,
            origin: Origin::Study,
            at_ms: i as i64,
            took_ms: 1000,
            confidence: None,
            reason: None,
        });
    }
    assert_eq!(sink.events.len(), 2);
    assert_eq!(sink.events[1].grade, Grade::Good);
}

#[test]
fn hlc_orders_by_wall_then_counter_then_device() {
    let h = |w, c, d: &str| Hlc { wall_ms: w, counter: c, device: d.into() };
    assert!(h(1, 9, "z") < h(2, 0, "a"));
    assert!(h(2, 0, "z") < h(2, 1, "a"));
    assert!(h(2, 1, "a") < h(2, 1, "b"));
}

#[test]
fn hlc_clock_never_goes_back() {
    let mut c = HlcClock::new("s24");
    let a = c.tick(1000);
    let b = c.tick(1000);
    let back = c.tick(500); // the wall clock jumped back
    assert!(a < b && b < back);
    c.observe(&Hlc { wall_ms: 5000, counter: 3, device: "tab".into() });
    let after = c.tick(2000);
    assert_eq!((after.wall_ms, after.counter), (5000, 4));
}

#[test]
fn a_stamp_from_a_wrong_clock_does_not_pin_later_writes() {
    // #539: written with the date a day ahead, then the clock is fixed.
    const DAY: i64 = 86_400_000;
    let mut c = HlcClock::new("s24");
    c.observe(&Hlc { wall_ms: 1_000 + DAY, counter: 7, device: "s24".into() });
    let next = c.tick(1_000);
    assert_eq!((next.wall_ms, next.counter), (1_000, 0), "back to the real clock");
    assert!(c.tick(1_000) > next, "and still going up from there");
    // Within the hub's skew, a stamp ahead is kept as before.
    let mut c = HlcClock::new("s24");
    c.observe(&Hlc { wall_ms: 61_000, counter: 2, device: "tab".into() });
    assert_eq!(c.tick(1_000).wall_ms, 61_000);
}

#[test]
fn enumerate_grades_against_the_pass_mark() {
    assert_eq!(enumerate_grade(2, 6, 4), Grade::Again);
    assert_eq!(enumerate_grade(4, 6, 4), Grade::Hard);
    assert_eq!(enumerate_grade(5, 6, 4), Grade::Good);
    assert_eq!(enumerate_grade(6, 6, 4), Grade::Easy);
    // When every point is needed to pass, full recall is Good.
    assert_eq!(enumerate_grade(3, 3, 3), Grade::Good);
    assert_eq!(enumerate_grade(0, 3, 0), Grade::Again);
}

#[test]
fn names_round_trip() {
    for g in Grade::ALL {
        assert_eq!(Grade::parse(g.as_str()), Some(g));
    }
    for o in Origin::ALL {
        assert_eq!(Origin::parse(o.as_str()), Some(o));
    }
}

#[test]
fn confidence_and_reason_names_round_trip() {
    for c in Confidence::ALL {
        assert_eq!(Confidence::parse(c.as_str()), Some(c));
        assert_eq!(serde_json::to_string(&c).unwrap(), format!("\"{}\"", c.as_str()));
    }
    for r in ErrorReason::ALL {
        assert_eq!(ErrorReason::parse(r.as_str()), Some(r));
        assert_eq!(serde_json::to_string(&r).unwrap(), format!("\"{}\"", r.as_str()));
    }
    assert_eq!(ErrorReason::parse("bad_luck"), None);
}

#[test]
fn events_from_older_builds_still_read() {
    let old = r#"{"item_id":"hb.L01.001","grade":"again","origin":"bank","at_ms":1,"took_ms":900}"#;
    let e: ReviewEvent = serde_json::from_str(old).unwrap();
    assert_eq!(e, ReviewEvent::new("hb.L01.001", Grade::Again, Origin::Bank, 1, 900));
    // And an event without them writes nothing new, so older builds read it too.
    assert_eq!(serde_json::to_string(&e).unwrap(), old);
}

#[test]
fn a_tagged_event_round_trips() {
    let e = ReviewEvent {
        confidence: Some(Confidence::Sure),
        reason: Some(ErrorReason::SecondGuessed),
        ..ReviewEvent::new("hb.L01.001", Grade::Again, Origin::Study, 1, 900)
    };
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains("\"confidence\":\"sure\"") && json.contains("\"reason\":\"second_guessed\""));
    assert_eq!(serde_json::from_str::<ReviewEvent>(&json).unwrap(), e);
    assert!(e.sure_miss());
    assert!(!ReviewEvent { grade: Grade::Good, ..e }.sure_miss());
}

#[test]
fn logged_review_flattens_the_event() {
    let l = LoggedReview {
        id: "s24-1-0".into(),
        hlc: Hlc { wall_ms: 1, counter: 0, device: "s24".into() },
        event: ReviewEvent {
            item_id: "cvs.L04.001".into(),
            grade: Grade::Good,
            origin: Origin::Review,
            at_ms: 1,
            took_ms: 900,
            confidence: None,
            reason: None,
        },
    };
    let json = serde_json::to_string(&l).unwrap();
    assert!(json.contains("\"item_id\":\"cvs.L04.001\""));
    assert_eq!(serde_json::from_str::<LoggedReview>(&json).unwrap(), l);
}

#[test]
fn ratings_and_evidence() {
    assert_eq!(Grade::ALL.map(Grade::rating), [1, 2, 3, 4]);
    assert_eq!(Grade::parse("perfect"), None);
    assert!(Origin::Bank.is_evidence() && Origin::Mock.is_evidence());
    assert!(!Origin::Study.is_evidence() && !Origin::Oral.is_evidence());
    assert_eq!(HlcClock::new("s24").device(), "s24");
}

#[test]
fn only_recall_kinds_are_self_rated() {
    let lec: crate::content::Lecture =
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
    let rated: Vec<&str> = lec.items.iter().filter(|i| self_rated(&i.body)).map(|i| i.id.as_str()).collect();
    // The enumerate (J♥NES), the essay (aortic dissection) and the case.
    assert_eq!(rated, ["cvs.L04.004", "cvs.L04.005", "cvs.L04.006"]);
}

/// The least a store must write; everything else falls back to the defaults.
#[derive(Default)]
struct Bare {
    events: Vec<ReviewEvent>,
}

impl StudyStore for Bare {
    fn record(&mut self, event: ReviewEvent) {
        self.events.push(event);
    }
    fn due(&self, _now_ms: i64) -> Vec<String> {
        self.events.iter().map(|e| e.item_id.clone()).collect()
    }
    fn is_new(&self, item_id: &str) -> bool {
        !self.events.iter().any(|e| e.item_id == item_id)
    }
    fn is_hidden(&self, _item_id: &str) -> bool {
        false
    }
    fn next_due_ms(&self, _now_ms: i64) -> Option<i64> {
        None
    }
}

#[test]
fn store_defaults_record_and_stay_quiet() {
    let handle = StudyStoreHandle::new(Bare::default());
    let event = ReviewEvent {
        confidence: None,
        reason: None,
        item_id: "cvs.L04.002".into(),
        grade: Grade::Hard,
        origin: Origin::Study,
        at_ms: 10,
        took_ms: 4000,
    };
    let mut store = handle.0.borrow_mut();
    assert_eq!(store.record_logged(event), Ok(None), "recorded, but it can't be taken back");
    assert!(!store.is_new("cvs.L04.002") && store.is_new("cvs.L04.003"));
    assert_eq!(store.due(0), ["cvs.L04.002"]);
    assert!(!store.retract("s24-1-0"));
    assert_eq!(store.new_left_today(0), usize::MAX);
    assert_eq!(store.intervals("cvs.L04.002", 0), None);
    assert!(store.log().is_empty() && store.marks().is_empty() && store.forecast(7, 0).is_empty());
    let lec: crate::content::Lecture =
        serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
    assert_eq!(store.effective(&lec.items[0]), lec.items[0], "no edit: the item as written");
    assert!(!store.is_hidden("cvs.L04.001") && store.next_due_ms(0).is_none());
}

proptest::proptest! {
    /// Whatever the wall clock does and whatever the other device's
    /// stamps say, every new local stamp sorts after the last one and
    /// after everything observed, so a local edit always wins LWW.
    #[test]
    fn stamps_only_go_up(
        steps in proptest::collection::vec((proptest::prelude::any::<bool>(), 0i64..5_000, 0i64..5_000, 0u32..4), 1..60),
    ) {
        let mut c = HlcClock::new("s24u");
        let mut last: Option<Hlc> = None;
        let mut seen_max: Option<Hlc> = None;
        for (remote, now, remote_wall, remote_counter) in steps {
            if remote {
                let seen = Hlc { wall_ms: remote_wall, counter: remote_counter, device: "tab".into() };
                c.observe(&seen);
                seen_max = seen_max.max(Some(seen));
                continue;
            }
            let next = c.tick(now);
            proptest::prop_assert!(next.wall_ms >= now, "never behind the wall clock");
            proptest::prop_assert_eq!(next.device.as_str(), "s24u");
            if let Some(prev) = &last {
                proptest::prop_assert!(&next > prev);
            }
            if let Some(seen) = &seen_max {
                proptest::prop_assert!((next.wall_ms, next.counter) > (seen.wall_ms, seen.counter));
            }
            last = Some(next);
        }
    }
}
