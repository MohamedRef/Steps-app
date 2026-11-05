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

/// How sure Mohamed was before the answer came out: one optional tap.
/// A miss he was sure of is the one most worth re-testing (Butler 2011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Sure,
    Unsure,
    Guess,
}

impl Confidence {
    pub const ALL: [Confidence; 3] = [Self::Sure, Self::Unsure, Self::Guess];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sure => "sure",
            Self::Unsure => "unsure",
            Self::Guess => "guess",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

/// Why a wrong answer was wrong, picked after it: the "why I missed it" log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorReason {
    /// He didn't know the fact.
    Knowledge,
    /// He misread the question or an option.
    Misread,
    /// He had it right, then changed his answer.
    SecondGuessed,
    /// He knew the facts but a step of the reasoning went wrong.
    Reasoning,
    /// He ran out of time or rushed.
    Timing,
}

impl ErrorReason {
    pub const ALL: [ErrorReason; 5] =
        [Self::Knowledge, Self::Misread, Self::SecondGuessed, Self::Reasoning, Self::Timing];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Knowledge => "knowledge",
            Self::Misread => "misread",
            Self::SecondGuessed => "second_guessed",
            Self::Reasoning => "reasoning",
            Self::Timing => "timing",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == s)
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
    /// How sure he was before the answer came out, when he said.
    /// Optional, so events from older builds and devices still read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// Why he missed it, when he said. Only on "Again" answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ErrorReason>,
}

impl ReviewEvent {
    /// An answer with no confidence or reason given.
    pub fn new(item_id: impl Into<String>, grade: Grade, origin: Origin, at_ms: i64, took_ms: u32) -> Self {
        Self { item_id: item_id.into(), grade, origin, at_ms, took_ms, confidence: None, reason: None }
    }

    /// A miss he was sure of: re-test it first.
    pub fn sure_miss(&self) -> bool {
        self.grade == Grade::Again && self.confidence == Some(Confidence::Sure)
    }
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

    /// True when the last stamp is further ahead of `now_ms` than Steps
    /// Cloud accepts: the next [`Self::tick`] starts again from now.
    pub fn is_ahead_of(&self, now_ms: i64) -> bool {
        self.last_ms > now_ms.saturating_add(steps_protocol::sync::MAX_CLOCK_SKEW_MS)
    }

    pub fn tick(&mut self, now_ms: i64) -> Hlc {
        // A stamp further ahead than Steps Cloud accepts came from a clock
        // that was wrong (or a backup stamped in the future). Keeping it
        // would pin every later write there, and the hub would refuse them
        // all until real time caught up (#539): start again from now.
        if self.is_ahead_of(now_ms) {
            self.last_ms = now_ms;
            self.counter = 0;
            return Hlc { wall_ms: now_ms, counter: 0, device: self.device.clone() };
        }
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
    /// The First Aid pages read, when the page gate is on: new items start
    /// only from these pages (`session::Inputs::pages`). `None` = no gate.
    fn pages_read(&self) -> Option<&crate::pages::PagesRead> {
        None
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
mod tests;
