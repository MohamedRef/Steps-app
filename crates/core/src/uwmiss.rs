//! UWorld questions Mohamed got wrong, logged by question id (QID) with the
//! reason he missed each one, and the patterns they make: which reason
//! comes up most and which First Aid lectures the misses fall in. Numbers,
//! reasons and his own short note only, never question text.
//!
//! Stored as one synced setting per QID (`uw.miss.<qid>`, last writer
//! wins, a tombstone on delete), so two devices logging two questions never
//! overwrite each other and logging a QID again replaces its reason.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::plan::Day;
use crate::review::ErrorReason;

/// Every entry's settings key starts with this, then the QID.
pub const PREFIX: &str = "uw.miss.";
pub const MAX_NOTE: usize = 300;
/// The top-patterns card speaks only once there are this many misses:
/// before that a "most common reason" is noise.
pub const PATTERN_MIN: usize = 3;
/// How many lectures the top-patterns card lists.
pub const TOP_LECTURES: usize = 5;

/// One question he missed, as stored (JSON in the setting's value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Miss {
    pub qid: u32,
    pub reason: ErrorReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The day he logged it.
    #[serde(with = "crate::selfassess::iso_day")]
    pub day: Day,
    pub created_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    NoteTooLong,
}

impl Miss {
    /// The synced setting this entry lives in.
    pub fn key(&self) -> String {
        key(self.qid)
    }

    /// Trims the note and drops an empty one.
    pub fn tidy(mut self) -> Self {
        self.note = self.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        self
    }

    /// What stops it being saved, if anything.
    pub fn problem(&self) -> Option<Problem> {
        self.note.as_ref().is_some_and(|n| n.chars().count() > MAX_NOTE).then_some(Problem::NoteTooLong)
    }
}

pub fn key(qid: u32) -> String {
    format!("{PREFIX}{qid}")
}

/// A QID from what he typed: its digits (`#1707`, ` 1707 `), never 0.
pub fn parse_qid(text: &str) -> Option<u32> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|q| *q > 0)
}

/// Newest first: by day, then by when it was logged, then by QID.
pub fn newest_first(misses: &[Miss]) -> Vec<Miss> {
    let mut v = misses.to_vec();
    v.sort_by(|a, b| b.day.cmp(&a.day).then(b.created_ms.cmp(&a.created_ms)).then(a.qid.cmp(&b.qid)));
    v
}

/// What the log says, read with `lecture_of` (a QID's First Aid lecture,
/// when the mapping files it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Patterns {
    pub total: usize,
    /// Every reason he used, most common first (ties in [`ErrorReason::ALL`] order).
    pub by_reason: Vec<(ErrorReason, usize)>,
    /// The lectures with the most misses, most first (ties by lecture id),
    /// at most [`TOP_LECTURES`], each with its QIDs in the order logged.
    pub by_lecture: Vec<(String, Vec<u32>)>,
    /// Misses the mapping files under no lecture.
    pub unfiled: usize,
}

impl Patterns {
    /// The reason behind most misses, once there are enough to say so and
    /// one reason clearly leads (no tie at the top).
    pub fn top_reason(&self) -> Option<(ErrorReason, usize)> {
        if self.total < PATTERN_MIN {
            return None;
        }
        match self.by_reason.as_slice() {
            [first, second, ..] if first.1 == second.1 => None,
            [first, ..] => Some(*first),
            [] => None,
        }
    }

    /// Percent of all misses, rounded.
    pub fn share(&self, n: usize) -> usize {
        (n * 100 + self.total / 2).checked_div(self.total).unwrap_or(0)
    }
}

pub fn patterns(misses: &[Miss], lecture_of: impl Fn(u32) -> Option<String>) -> Patterns {
    let mut reasons: BTreeMap<ErrorReason, usize> = BTreeMap::new();
    let mut lectures: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut unfiled = 0;
    let mut logged = misses.to_vec();
    logged.sort_by(|a, b| a.created_ms.cmp(&b.created_ms).then(a.qid.cmp(&b.qid)));
    for m in &logged {
        *reasons.entry(m.reason).or_default() += 1;
        match lecture_of(m.qid) {
            Some(l) => lectures.entry(l).or_default().push(m.qid),
            None => unfiled += 1,
        }
    }
    let mut by_reason: Vec<(ErrorReason, usize)> =
        ErrorReason::ALL.into_iter().filter_map(|r| Some((r, *reasons.get(&r)?))).collect();
    by_reason.sort_by_key(|r| std::cmp::Reverse(r.1));
    let mut by_lecture: Vec<(String, Vec<u32>)> = lectures.into_iter().collect();
    by_lecture.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    by_lecture.truncate(TOP_LECTURES);
    Patterns { total: misses.len(), by_reason, by_lecture, unfiled }
}

#[cfg(test)]
mod tests;
