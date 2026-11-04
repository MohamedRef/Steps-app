//! Review events: what every study screen reports after an answer, how they
//! are stamped for sync (HLC), which of them move the schedule, and the seam
//! between the study screens and the store ([`StudyStore`]).
//!
//! Scheduling itself is in [`crate::srs`]; Mohamed's ✓ / ✎ / ✗ verdicts on
//! the content are in [`verify`].

pub mod verify;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::content::{Body, Item, Status};

/// The four FSRS ratings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    pub const ALL: [Grade; 4] = [Self::Again, Self::Hard, Self::Good, Self::Easy];

    /// FSRS's rating number (1–4).
    pub fn rating(self) -> u32 {
        match self {
            Self::Again => 1,
            Self::Hard => 2,
            Self::Good => 3,
            Self::Easy => 4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Hard => "hard",
            Self::Good => "good",
            Self::Easy => "easy",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|g| g.as_str() == s)
    }
}

/// Which screen produced the answer, for stats and for weighting later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Study,
    Review,
    Bank,
    Mock,
    Case,
    Oral,
}

impl Origin {
    pub const ALL: [Origin; 6] = [Self::Study, Self::Review, Self::Bank, Self::Mock, Self::Case, Self::Oral];

    /// Answers from the question bank and mock exams are evidence about an
    /// item, not a review of it: they move its schedule only under the rules
    /// in [`crate::srs::Scheduler::admits`].
    pub fn is_evidence(self) -> bool {
        matches!(self, Self::Bank | Self::Mock)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Review => "review",
            Self::Bank => "bank",
            Self::Mock => "mock",
            Self::Case => "case",
            Self::Oral => "oral",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.as_str() == s)
    }
}

/// One answer to one item. Append-only: never edited, never deleted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewEvent {
    pub item_id: String,
    pub grade: Grade,
    pub origin: Origin,
    /// Unix time in milliseconds.
    pub at_ms: i64,
    /// Time spent on the item, in milliseconds.
    pub took_ms: u32,
}

/// Where answers go. The store implements it (`crate::store` in the app).
pub trait ReviewSink {
    fn record(&mut self, event: ReviewEvent);
}

/// Keeps events in memory: for tests and for screens that land before the store.
#[derive(Debug, Default)]
pub struct MemorySink {
    pub events: Vec<ReviewEvent>,
}

impl ReviewSink for MemorySink {
    fn record(&mut self, event: ReviewEvent) {
        self.events.push(event);
    }
}

// --- Sync stamps ----------------------------------------------------------

/// The HLC lives in `steps-protocol`, which Steps Cloud shares.
pub use steps_protocol::Hlc;

/// Hands out increasing [`Hlc`]s for one device, even when the wall clock
/// goes backwards.
#[derive(Debug, Clone)]
pub struct HlcClock {
    device: String,
    last_ms: i64,
    counter: u32,
}

impl HlcClock {
    pub fn new(device: impl Into<String>) -> Self {
        Self { device: device.into(), last_ms: i64::MIN, counter: 0 }
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    pub fn tick(&mut self, now_ms: i64) -> Hlc {
        if now_ms > self.last_ms {
            self.last_ms = now_ms;
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        Hlc { wall_ms: self.last_ms, counter: self.counter, device: self.device.clone() }
    }

    /// Moves past a stamp seen elsewhere (another device, or the store on
    /// start-up), so the next local stamp sorts after it.
    pub fn observe(&mut self, seen: &Hlc) {
        if (seen.wall_ms, seen.counter) > (self.last_ms, self.counter) {
            self.last_ms = seen.wall_ms;
            self.counter = seen.counter;
        }
    }
}

/// A review event as stored and synced: with its own id and HLC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedReview {
    /// Unique per event; replays ignore duplicates.
    pub id: String,
    pub hlc: Hlc,
    #[serde(flatten)]
    pub event: ReviewEvent,
}

// --- Self-rating ----------------------------------------------------------

/// Kinds with no single right answer to check: Mohamed rates his own recall
/// (Again / Hard / Good / Easy) after seeing the model answer.
pub fn self_rated(body: &Body) -> bool {
    matches!(body, Body::Enumerate { .. } | Body::Essay { .. } | Body::Case { .. })
}

/// A grade from an "Enumerate" item: how many of `total` points he recalled,
/// against the `pass` mark. Below the pass mark is Again; all of them (when
/// that is more than the pass mark) is Easy; a bare pass is Hard.
pub fn enumerate_grade(recalled: usize, total: usize, pass: usize) -> Grade {
    let pass = pass.clamp(1, total.max(1));
    if recalled < pass {
        Grade::Again
    } else if recalled >= total && total > pass {
        Grade::Easy
    } else if recalled == pass && pass < total {
        Grade::Hard
    } else {
        Grade::Good
    }
}

// --- The seam with the study player ---------------------------------------

/// What the study player (chunk 2) and the other study screens need from the
/// review store. The app provides one [`StudyStoreHandle`] through context.
pub trait StudyStore {
    fn record(&mut self, event: ReviewEvent);
    /// [`Self::record`], returning the id the event was logged under (so it
    /// can be taken back), or `None` when it can't be taken back. `Err`
    /// when the answer wasn't saved: the caller keeps the item open so it
    /// can be rated again, and says so.
    fn record_logged(&mut self, event: ReviewEvent) -> Result<Option<String>, String> {
        self.record(event);
        Ok(None)
    }
    /// Takes a logged answer back (Undo): the event gets its tombstone, the
    /// one correction the append-only log allows, and the schedule is
    /// replayed without it. True when it is gone afterwards; an event that
    /// is already gone writes nothing. False when it couldn't be saved.
    fn retract(&mut self, _event_id: &str) -> bool {
        false
    }
    /// Item ids due now, most overdue (most forgotten) first. No new items,
    /// nothing Mohamed marked ✗.
    fn due(&self, now_ms: i64) -> Vec<String>;
    /// Never answered yet.
    fn is_new(&self, item_id: &str) -> bool;
    /// Marked ✗ wrong: out of review until the content is fixed.
    fn is_hidden(&self, item_id: &str) -> bool;
    /// The item as Mohamed should see it: with his ✎ edit applied.
    fn effective(&self, item: &Item) -> Item {
        item.clone()
    }
    /// When the next item falls due, if any.
    fn next_due_ms(&self, now_ms: i64) -> Option<i64>;
    /// How many new items may still start today (the daily new-item cap).
    fn new_left_today(&self, _now_ms: i64) -> usize {
        usize::MAX
    }
    /// The interval in days each grade (Again, Hard, Good, Easy) would give
    /// the item now, for "1d · 3d · 9d" on the grade buttons.
    fn intervals(&self, _item_id: &str, _now_ms: i64) -> Option<[f32; 4]> {
        None
    }
    /// Every answer logged, oldest first (stats, mistakes, the planner).
    fn log(&self) -> Vec<ReviewEvent> {
        Vec::new()
    }
    /// Mohamed's verdicts that still match the content: ✓ and ✎ count as
    /// `Approved`, ✗ as `Rejected`. Items he hasn't checked are absent.
    fn marks(&self) -> BTreeMap<String, Status> {
        BTreeMap::new()
    }
    /// Reviews falling due on each of the next `days` study days (index 0 is
    /// today, overdue included), from FSRS. Empty when the store can't tell.
    fn forecast(&self, _days: usize, _now_ms: i64) -> Vec<u32> {
        Vec::new()
    }
}

/// The shared store, as the app hands it out through Dioxus context.
#[derive(Clone)]
pub struct StudyStoreHandle(pub Rc<RefCell<dyn StudyStore>>);

impl StudyStoreHandle {
    pub fn new(store: impl StudyStore + 'static) -> Self {
        Self(Rc::new(RefCell::new(store)))
    }
}

#[cfg(test)]
mod tests {
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
            serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap();
        let rated: Vec<&str> =
            lec.items.iter().filter(|i| self_rated(&i.body)).map(|i| i.id.as_str()).collect();
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
            serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap();
        assert_eq!(store.effective(&lec.items[0]), lec.items[0], "no edit: the item as written");
        assert!(!store.is_hidden("cvs.L04.001") && store.next_due_ms(0).is_none());
    }
}
