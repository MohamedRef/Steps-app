//! Stats and the mistakes log: pure functions of the content, the review log
//! and Mohamed's item statuses, so every device shows the same numbers.
//! (Ideas adapted from kabar-app main: crates/core/src/stats/mod.rs.)
//!
//! Rules, stated once:
//! - **Seen:** the item has at least one answer. **Seen twice:** answers on
//!   two different study days (a same-session retry doesn't count).
//! - **Retention** is measured, not predicted: over the last
//!   [`RETENTION_WINDOW_DAYS`], the share of answers to items already seen on
//!   an earlier day that weren't "Again". First sightings don't count.
//! - **MCQ accuracy:** the share of answers to `mcq` items that weren't "Again".
//! - **Weak topic:** a topic with at least [`WEAK_MIN_ANSWERS`] counted
//!   answers, scored `0.6 × misses on retention + 0.4 × misses on MCQs`.
//!   A topic that hasn't been studied is never "weak": that's the planner's job.
//! - **Mistake:** an "Again" in the last [`MISTAKE_WINDOW_DAYS`], unless the
//!   item has since been answered right [`RECOVERED_AFTER`] times. Its reason
//!   is the one he gave for the latest miss; misses he was sure of come first.
//! - Rejected items (✗) count nowhere.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::content::{Lecture, Status};
use crate::plan::{Day, LectureProgress};
use crate::review::{ErrorReason, Grade, ReviewEvent};

pub const RETENTION_WINDOW_DAYS: i64 = 30;
pub const WEAK_MIN_ANSWERS: u32 = 5;
pub const MISTAKE_WINDOW_DAYS: i64 = 14;
pub const RECOVERED_AFTER: u32 = 2;
const DAY_MS: i64 = 86_400_000;

/// How the clock maps to study days (see [`Day::from_unix_ms`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub now_ms: i64,
    /// His time zone: each answer's day uses the offset in force then (#40).
    pub zone: crate::zone::Zone,
    pub rollover_hour: u32,
}

impl Clock {
    pub fn day_of(&self, ms: i64) -> Day {
        Day(self.zone.day_from(ms, self.rollover_hour as u8) as i32)
    }

    pub fn today(&self) -> Day {
        self.day_of(self.now_ms)
    }
}

/// A count of right answers out of all answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ratio {
    pub right: u32,
    pub total: u32,
}

impl Ratio {
    fn add(&mut self, right: bool) {
        self.total += 1;
        self.right += u32::from(right);
    }

    fn merge(&mut self, other: Ratio) {
        self.right += other.right;
        self.total += other.total;
    }

    /// `None` with no answers yet.
    pub fn share(self) -> Option<f32> {
        (self.total > 0).then(|| self.right as f32 / self.total as f32)
    }

    /// Whole percent, rounded.
    pub fn percent(self) -> Option<u32> {
        self.share().map(|s| (s * 100.0).round() as u32)
    }
}

/// The counts shown for a lecture and for each of its topics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    /// Items in the app, rejected ones left out.
    pub total: u32,
    pub seen: u32,
    pub seen_twice: u32,
    pub retention: Ratio,
    pub mcq: Ratio,
    pub approved: u32,
    pub unverified: u32,
}

impl Counts {
    fn merge(&mut self, o: &Counts) {
        self.total += o.total;
        self.seen += o.seen;
        self.seen_twice += o.seen_twice;
        self.retention.merge(o.retention);
        self.mcq.merge(o.mcq);
        self.approved += o.approved;
        self.unverified += o.unverified;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicStats {
    /// The topic slug; `None` collects items without one.
    pub id: Option<String>,
    pub title: String,
    pub counts: Counts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LectureStats {
    /// `hb.L01`.
    pub id: String,
    pub title: String,
    pub counts: Counts,
    /// In the lecture's own topic order; the untagged group last.
    pub topics: Vec<TopicStats>,
}

/// Every answer per item, in time order.
fn answers_by_item(events: &[ReviewEvent]) -> HashMap<&str, Vec<&ReviewEvent>> {
    let mut by: HashMap<&str, Vec<&ReviewEvent>> = HashMap::new();
    for e in events {
        by.entry(e.item_id.as_str()).or_default().push(e);
    }
    for list in by.values_mut() {
        list.sort_by_key(|e| e.at_ms);
    }
    by
}

/// The status in force: Mohamed's own mark if any, else the file's.
fn status_of(id: &str, file: Status, marks: &BTreeMap<String, Status>) -> Status {
    marks.get(id).copied().unwrap_or(file)
}

/// Per-lecture and per-topic stats, in the order of `lectures`.
pub fn lecture_stats(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
) -> Vec<LectureStats> {
    let by_item = answers_by_item(events);
    let since = clock.now_ms - RETENTION_WINDOW_DAYS * DAY_MS;
    lectures
        .iter()
        .map(|lecture| {
            let mut topics: Vec<TopicStats> = lecture
                .topics
                .iter()
                .map(|t| TopicStats {
                    id: Some(t.id.clone()),
                    title: t.title.clone(),
                    counts: Counts::default(),
                })
                .collect();
            let mut untagged = TopicStats { id: None, title: String::new(), counts: Counts::default() };
            for item in &lecture.items {
                let status = status_of(&item.id, item.status, marks);
                if status == Status::Rejected {
                    continue;
                }
                let slot = item
                    .topic
                    .as_deref()
                    .and_then(|t| topics.iter().position(|x| x.id.as_deref() == Some(t)));
                let c = match slot {
                    Some(i) => &mut topics[i].counts,
                    None => &mut untagged.counts,
                };
                c.total += 1;
                match status {
                    Status::Approved => c.approved += 1,
                    _ => c.unverified += 1,
                }
                let Some(answers) = by_item.get(item.id.as_str()) else { continue };
                c.seen += 1;
                let days: BTreeSet<Day> = answers.iter().map(|e| clock.day_of(e.at_ms)).collect();
                if days.len() >= 2 {
                    c.seen_twice += 1;
                }
                let first_day = clock.day_of(answers[0].at_ms);
                for e in answers {
                    let right = e.grade != Grade::Again;
                    if e.at_ms >= since && clock.day_of(e.at_ms) > first_day {
                        c.retention.add(right);
                    }
                    if item.body.kind() == "mcq" {
                        c.mcq.add(right);
                    }
                }
            }
            if untagged.counts.total > 0 {
                topics.push(untagged);
            }
            let mut counts = Counts::default();
            for t in &topics {
                counts.merge(&t.counts);
            }
            LectureStats { id: lecture.lecture.clone(), title: lecture.title.clone(), counts, topics }
        })
        .collect()
}

/// The planner's view of the stats.
pub fn progress(stats: &[LectureStats]) -> Vec<LectureProgress> {
    stats
        .iter()
        .map(|l| LectureProgress {
            id: l.id.clone(),
            total: l.counts.total,
            seen: l.counts.seen,
            seen_twice: l.counts.seen_twice,
        })
        .collect()
}

/// The planner's view as it stood when today's study day began. Today's
/// share of the plan is worked out from this, so it holds still while he
/// works through it instead of shrinking with every answer.
pub fn progress_at_day_start(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
) -> Vec<LectureProgress> {
    let today = clock.today();
    let before: Vec<ReviewEvent> = events.iter().filter(|e| clock.day_of(e.at_ms) < today).cloned().collect();
    progress(&lecture_stats(lectures, &before, marks, clock))
}

/// What today's study day has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayDone {
    /// Items answered for the first time today.
    pub new: u32,
    /// Answers today to items first seen on an earlier day.
    pub reviews: u32,
}

/// Today's [`DayDone`]. Rejected items left out.
pub fn done_today(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
) -> DayDone {
    let today = clock.today();
    let by_item = answers_by_item(events);
    let mut done = DayDone::default();
    for item in lectures.iter().flat_map(|l| &l.items) {
        if status_of(&item.id, item.status, marks) == Status::Rejected {
            continue;
        }
        let Some(answers) = by_item.get(item.id.as_str()) else { continue };
        if answers.first().is_some_and(|e| clock.day_of(e.at_ms) == today) {
            done.new += 1;
        } else {
            done.reviews += answers.iter().filter(|e| clock.day_of(e.at_ms) == today).count() as u32;
        }
    }
    done
}

/// Rough intervals (days) after the 1st, 2nd, 3rd… day an item was answered
/// right; "Again" starts it over. A stand-in for FSRS's own forecast until
/// the scheduler lands (chunk 3).
const ROUGH_LADDER: [i32; 6] = [1, 3, 7, 14, 30, 60];

/// Reviews due per day, today first, for `days` days. Overdue items count on
/// today. See [`ROUGH_LADDER`].
pub fn rough_forecast(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
    days: usize,
) -> Vec<u32> {
    let by_item = answers_by_item(events);
    let today = clock.today();
    let mut out = vec![0u32; days];
    for item in lectures.iter().flat_map(|l| &l.items) {
        if status_of(&item.id, item.status, marks) == Status::Rejected {
            continue;
        }
        let Some(answers) = by_item.get(item.id.as_str()) else { continue };
        let mut step = 0usize;
        let mut last_day = None;
        for e in answers {
            let d = clock.day_of(e.at_ms);
            if e.grade == Grade::Again {
                step = 0;
            } else if last_day != Some(d) {
                step = (step + 1).min(ROUGH_LADDER.len());
            }
            last_day = Some(d);
        }
        let Some(last) = last_day else { continue };
        let due = last.plus(ROUGH_LADDER[step.saturating_sub(1)]);
        let at = today.until(due).max(0) as usize;
        if let Some(slot) = out.get_mut(at) {
            *slot += 1;
        }
    }
    out
}

/// Items seen for the first time per study day, oldest first: the past half
/// of the pace chart. Rejected items left out.
pub fn first_seen_by_day(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
) -> BTreeMap<Day, u32> {
    let by_item = answers_by_item(events);
    let mut out = BTreeMap::new();
    for item in lectures.iter().flat_map(|l| &l.items) {
        if status_of(&item.id, item.status, marks) == Status::Rejected {
            continue;
        }
        if let Some(first) = by_item.get(item.id.as_str()).and_then(|a| a.first()) {
            *out.entry(clock.day_of(first.at_ms)).or_insert(0) += 1;
        }
    }
    out
}

/// One study day in a [`Period`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayActivity {
    pub day: Day,
    pub answers: u32,
    /// Time spent answering, rounded up to whole minutes.
    pub minutes: u32,
}

/// What a stretch of days held, for the progress screen's period switch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Period {
    /// Every study day of the period, oldest first, quiet days included.
    pub days: Vec<DayActivity>,
    pub answers: u32,
    pub minutes: u32,
    /// Days with at least one answer.
    pub study_days: u32,
    /// The retention rule over just these days: answers to items first seen
    /// on an earlier day, right unless "Again".
    pub retention: Ratio,
}

/// The `days` study days that end `ago` periods back (`ago` 0 ends today,
/// 1 is the stretch just before it). Rejected items count nowhere.
pub fn period(
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    clock: Clock,
    days: u32,
    ago: u32,
) -> Period {
    let days = days.max(1) as i32;
    let last = clock.today().0 - days * ago as i32;
    let first = last - days + 1;
    let mut per_day: Vec<(u32, u32)> = vec![(0, 0); days as usize];
    let mut first_day: HashMap<&str, Day> = HashMap::new();
    let mut sorted: Vec<&ReviewEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.at_ms);
    let mut retention = Ratio::default();
    for e in sorted {
        if marks.get(&e.item_id) == Some(&Status::Rejected) {
            continue;
        }
        let day = clock.day_of(e.at_ms);
        let seen = *first_day.entry(e.item_id.as_str()).or_insert(day);
        if !(first..=last).contains(&day.0) {
            continue;
        }
        let slot = &mut per_day[(day.0 - first) as usize];
        slot.0 += 1;
        slot.1 += e.took_ms;
        if day > seen {
            retention.add(e.grade != Grade::Again);
        }
    }
    let days: Vec<DayActivity> = per_day
        .into_iter()
        .enumerate()
        .map(|(i, (answers, ms))| DayActivity {
            day: Day(first + i as i32),
            answers,
            minutes: ms.div_ceil(60_000),
        })
        .collect();
    Period {
        answers: days.iter().map(|d| d.answers).sum(),
        minutes: days.iter().map(|d| d.minutes).sum(),
        study_days: days.iter().filter(|d| d.answers > 0).count() as u32,
        retention,
        days,
    }
}

/// A topic that needs work, for the "study these" card.
#[derive(Debug, Clone, PartialEq)]
pub struct WeakTopic {
    pub lecture: String,
    pub topic: Option<String>,
    pub title: String,
    /// 0 (all right) to 1 (all missed).
    pub score: f32,
    pub retention: Ratio,
    pub mcq: Ratio,
}

/// The `n` weakest topics, weakest first. See the module rules.
pub fn weakest_topics(stats: &[LectureStats], n: usize) -> Vec<WeakTopic> {
    let mut out: Vec<WeakTopic> = stats
        .iter()
        .flat_map(|l| l.topics.iter().map(move |t| (l, t)))
        .filter(|(_, t)| t.counts.retention.total + t.counts.mcq.total >= WEAK_MIN_ANSWERS)
        .filter_map(|(l, t)| {
            let miss = |r: Ratio| r.share().map(|s| 1.0 - s);
            let score = match (miss(t.counts.retention), miss(t.counts.mcq)) {
                (Some(r), Some(m)) => 0.6 * r + 0.4 * m,
                (Some(x), None) | (None, Some(x)) => x,
                (None, None) => return None,
            };
            (score > 0.0).then(|| WeakTopic {
                lecture: l.id.clone(),
                topic: t.id.clone(),
                title: if t.title.is_empty() { l.title.clone() } else { t.title.clone() },
                score,
                retention: t.counts.retention,
                mcq: t.counts.mcq,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.lecture.cmp(&b.lecture)));
    out.truncate(n);
    out
}

/// One item in the mistakes log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mistake {
    pub item_id: String,
    /// "Again" answers inside the window.
    pub misses: u32,
    pub last_miss_ms: i64,
    /// Why he missed it last time, when he said.
    pub reason: Option<ErrorReason>,
    /// He was sure of an answer that was wrong, inside the window: a
    /// high-confidence error, the one most worth re-testing.
    pub sure: bool,
}

/// Mistakes of one topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MistakeGroup {
    pub lecture: String,
    pub topic: Option<String>,
    pub title: String,
    /// Newest miss first.
    pub items: Vec<Mistake>,
}

/// The mistakes log, grouped by lecture then topic in book order.
pub fn mistakes(
    lectures: &[Lecture],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
    now_ms: i64,
) -> Vec<MistakeGroup> {
    let by_item = answers_by_item(events);
    let since = now_ms - MISTAKE_WINDOW_DAYS * DAY_MS;
    let mut out = Vec::new();
    for lecture in lectures {
        let mut groups: Vec<MistakeGroup> = Vec::new();
        for item in &lecture.items {
            if status_of(&item.id, item.status, marks) == Status::Rejected {
                continue;
            }
            let Some(answers) = by_item.get(item.id.as_str()) else { continue };
            let Some(last) = answers.iter().rposition(|e| e.grade == Grade::Again) else { continue };
            if answers[last].at_ms < since || answers.len() - 1 - last >= RECOVERED_AFTER as usize {
                continue;
            }
            let misses =
                answers.iter().filter(|e| e.grade == Grade::Again && e.at_ms >= since).count() as u32;
            let sure = answers.iter().any(|e| e.at_ms >= since && e.sure_miss());
            let mistake = Mistake {
                item_id: item.id.clone(),
                misses,
                last_miss_ms: answers[last].at_ms,
                reason: answers[last].reason,
                sure,
            };
            match groups.iter_mut().find(|g| g.topic == item.topic) {
                Some(g) => g.items.push(mistake),
                None => {
                    let title = item
                        .topic
                        .as_deref()
                        .and_then(|t| lecture.topics.iter().find(|x| x.id == t))
                        .map_or_else(|| lecture.title.clone(), |t| t.title.clone());
                    groups.push(MistakeGroup {
                        lecture: lecture.lecture.clone(),
                        topic: item.topic.clone(),
                        title,
                        items: vec![mistake],
                    });
                }
            }
        }
        let order = |g: &MistakeGroup| {
            g.topic
                .as_deref()
                .and_then(|t| lecture.topics.iter().position(|x| x.id == t))
                .unwrap_or(usize::MAX)
        };
        groups.sort_by_key(order);
        for g in &mut groups {
            g.items.sort_by_key(|m| (!m.sure, std::cmp::Reverse(m.last_miss_ms)));
        }
        out.extend(groups);
    }
    out
}

/// Item ids for a session from the mistakes log: misses he was sure of
/// first, then most-missed, then newest.
pub fn mistakes_session(groups: &[MistakeGroup]) -> Vec<String> {
    let mut all: Vec<&Mistake> = groups.iter().flat_map(|g| &g.items).collect();
    all.sort_by(|a, b| {
        b.sure.cmp(&a.sure).then(b.misses.cmp(&a.misses)).then(b.last_miss_ms.cmp(&a.last_miss_ms))
    });
    all.into_iter().map(|m| m.item_id.clone()).collect()
}

/// How many mistakes in the log carry each reason, in [`ErrorReason::ALL`]
/// order, reasons with none left out; then how many have no reason.
pub fn mistake_reasons(groups: &[MistakeGroup]) -> (Vec<(ErrorReason, u32)>, u32) {
    let mut counts = BTreeMap::<ErrorReason, u32>::new();
    let mut none = 0;
    for m in groups.iter().flat_map(|g| &g.items) {
        match m.reason {
            Some(r) => *counts.entry(r).or_default() += 1,
            None => none += 1,
        }
    }
    let listed = ErrorReason::ALL.into_iter().filter_map(|r| counts.get(&r).map(|&n| (r, n))).collect();
    (listed, none)
}

/// The log narrowed to mistakes with one reason; empty groups dropped.
pub fn mistakes_with_reason(groups: &[MistakeGroup], reason: ErrorReason) -> Vec<MistakeGroup> {
    groups
        .iter()
        .filter_map(|g| {
            let items: Vec<Mistake> = g.items.iter().filter(|m| m.reason == Some(reason)).cloned().collect();
            (!items.is_empty()).then(|| MistakeGroup { items, ..g.clone() })
        })
        .collect()
}

/// Item ids for a session on the given topics: items already seen first
/// (they are what's slipping), then the topic's unseen items, rejected left out.
pub fn topics_session(
    lectures: &[Lecture],
    topics: &[WeakTopic],
    events: &[ReviewEvent],
    marks: &BTreeMap<String, Status>,
) -> Vec<String> {
    let seen: BTreeSet<&str> = events.iter().map(|e| e.item_id.as_str()).collect();
    let mut first = Vec::new();
    let mut later = Vec::new();
    for t in topics {
        let Some(lecture) = lectures.iter().find(|l| l.lecture == t.lecture) else { continue };
        for item in lecture.items.iter().filter(|i| i.topic == t.topic) {
            if status_of(&item.id, item.status, marks) == Status::Rejected {
                continue;
            }
            if seen.contains(item.id.as_str()) {
                first.push(item.id.clone());
            } else {
                later.push(item.id.clone());
            }
        }
    }
    first.extend(later);
    first
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Confidence, Origin};

    const LECTURE: &str = r#"
schema: steps.content/v1
lecture: hb.L01
title: Liver Cirrhosis
book: { chapter: 1, pages: [1, 22] }
topics:
  - { id: definition, title: Definition and causes }
  - { id: child-pugh, title: Child-Pugh and MELD }
items:
  - id: hb.L01.001
    kind: card
    topic: definition
    front: Anatomical definition of cirrhosis?
    back: Diffuse fibrosis with regenerative nodules.
    src: { page: 1 }
    status: unverified
  - id: hb.L01.002
    kind: mcq
    topic: child-pugh
    stem: Which is not part of Child-Pugh?
    options: [Bilirubin, Albumin, INR, Creatinine]
    answer: 3
    why: [In it, In it, In it, That's MELD]
    src: { page: 5 }
    status: unverified
  - id: hb.L01.003
    kind: cloze
    topic: child-pugh
    text: "Child-Pugh class C starts at {{c1::10}} points."
    src: { page: 5 }
    status: unverified
  - id: hb.L01.004
    kind: card
    front: Rejected one
    back: x
    src: { page: 6 }
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
            ("hb.L01.004".to_string(), Status::Rejected),
            ("hb.L01.001".to_string(), Status::Approved),
        ])
    }

    #[test]
    fn a_period_counts_its_own_days_only() {
        let events = [
            ev("hb.L01.001", Grade::Good, 10),
            ev("hb.L01.001", Grade::Again, 3),
            ev("hb.L01.001", Grade::Good, 1),
            ev("hb.L01.002", Grade::Good, 1),
            ev("hb.L01.004", Grade::Good, 1),
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
            ev("hb.L01.001", Grade::Good, 5),
            ev("hb.L01.001", Grade::Again, 2),
            ev("hb.L01.002", Grade::Again, 3),
            ev("hb.L01.002", Grade::Good, 3),
            ev("hb.L01.004", Grade::Good, 1),
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
        let mut events = vec![ev("hb.L01.001", Grade::Good, 20)];
        for d in 1..=4 {
            events.push(ev("hb.L01.001", Grade::Again, d));
        }
        let s = lecture_stats(&lectures(), &events, &marks(), clock());
        assert!(weakest_topics(&s, 3).is_empty(), "4 counted answers are too few");
        events.push(ev("hb.L01.001", Grade::Good, 0));
        for d in 1..=6 {
            events.push(ev("hb.L01.002", Grade::Good, d));
        }
        let s = lecture_stats(&lectures(), &events, &marks(), clock());
        let weak = weakest_topics(&s, 3);
        assert_eq!(weak.len(), 1, "a topic with no misses isn't weak");
        assert_eq!(weak[0].topic.as_deref(), Some("definition"));
        assert!((weak[0].score - 0.8).abs() < 1e-6);
        let ids = topics_session(&lectures(), &weak, &events, &marks());
        assert_eq!(ids, ["hb.L01.001"]);
    }

    #[test]
    fn mistakes_log_window_and_recovery() {
        let events = [
            ev("hb.L01.001", Grade::Again, 20), // too old
            ev("hb.L01.002", Grade::Again, 3),
            ev("hb.L01.002", Grade::Again, 2),
            ev("hb.L01.002", Grade::Good, 1), // one right answer isn't recovery yet
            ev("hb.L01.003", Grade::Again, 4),
            ev("hb.L01.003", Grade::Good, 3),
            ev("hb.L01.003", Grade::Good, 2),  // recovered
            ev("hb.L01.004", Grade::Again, 1), // rejected
        ];
        let log = mistakes(&lectures(), &events, &marks(), NOW);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].topic.as_deref(), Some("child-pugh"));
        assert_eq!(log[0].title, "Child-Pugh and MELD");
        assert_eq!(
            log[0].items,
            [Mistake {
                item_id: "hb.L01.002".into(),
                misses: 2,
                last_miss_ms: NOW - 2 * DAY_MS,
                reason: None,
                sure: false,
            }]
        );
        assert_eq!(mistakes_session(&log), ["hb.L01.002"]);
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
            tagged("hb.L01.003", 5, Some(Confidence::Sure), Some(ErrorReason::Misread)),
            ev("hb.L01.003", Grade::Good, 4),
            tagged("hb.L01.002", 1, Some(Confidence::Guess), Some(ErrorReason::Knowledge)),
            ev("hb.L01.001", Grade::Good, 9),
            tagged("hb.L01.001", 2, None, None),
        ];
        let log = mistakes(&lectures(), &events, &marks(), NOW);
        let ids: Vec<&str> = log.iter().flat_map(|g| g.items.iter().map(|m| m.item_id.as_str())).collect();
        // Within the topic, the older sure miss leads the newer guess.
        assert_eq!(ids, ["hb.L01.001", "hb.L01.003", "hb.L01.002"]);
        let m3 = &log[1].items[0];
        assert!(m3.sure);
        assert_eq!(m3.reason, Some(ErrorReason::Misread));
        assert_eq!(mistakes_session(&log), ["hb.L01.003", "hb.L01.002", "hb.L01.001"]);
    }

    #[test]
    fn mistakes_count_and_filter_by_reason() {
        let events = [
            tagged("hb.L01.001", 3, None, Some(ErrorReason::Knowledge)),
            // The latest miss's reason is the one that counts.
            tagged("hb.L01.002", 3, None, Some(ErrorReason::Timing)),
            tagged("hb.L01.002", 2, None, Some(ErrorReason::Knowledge)),
            tagged("hb.L01.003", 1, Some(Confidence::Unsure), None),
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
            ev("hb.L01.001", Grade::Good, 2), // first right answer: back after 1 day, overdue
            ev("hb.L01.002", Grade::Good, 3),
            ev("hb.L01.002", Grade::Good, 1), // second: back 3 days after
            ev("hb.L01.003", Grade::Good, 4),
            ev("hb.L01.003", Grade::Again, 0), // starts over: back tomorrow
            ev("hb.L01.004", Grade::Good, 1),  // rejected
        ];
        let f = rough_forecast(&lectures(), &events, &marks(), clock(), 5);
        assert_eq!(f, [1, 1, 1, 0, 0]);
    }

    #[test]
    fn first_sightings_per_day() {
        let events = [
            ev("hb.L01.001", Grade::Good, 3),
            ev("hb.L01.001", Grade::Good, 1),
            ev("hb.L01.002", Grade::Again, 1),
            ev("hb.L01.004", Grade::Good, 1), // rejected
        ];
        let today = clock().today();
        let got = first_seen_by_day(&lectures(), &events, &marks(), clock());
        assert_eq!(got, BTreeMap::from([(today.plus(-3), 1), (today.plus(-1), 1)]));
    }

    #[test]
    fn day_start_progress_and_today_so_far() {
        let events = [
            ev("hb.L01.001", Grade::Good, 2),
            ev("hb.L01.001", Grade::Good, 0),
            ev("hb.L01.001", Grade::Again, 0),
            ev("hb.L01.002", Grade::Good, 0),
            ev("hb.L01.004", Grade::Good, 0), // rejected
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
}
