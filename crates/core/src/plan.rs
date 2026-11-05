//! The exam planner: spreads the items Mohamed hasn't seen yet over the study
//! days left before his exam, so that every lecture is seen at least twice
//! and the last days stay free for mock exams and weak topics.
//!
//! The plan is a pure function of today's progress, so it is recomputed every
//! day: a missed day never piles up as "debt", it only nudges the next days'
//! targets up a little.
//!
//! Rules, stated once:
//! - **Phases:** *systems* (the first pass through First Aid), *integration*
//!   (the second pass, with more mixed qbank questions) and *dedicated* (the
//!   last [`PlanSettings::dedicated_weeks`] weeks: qbank blocks, weak topics
//!   and self-assessments, no new items). Dedicated is never shrunk to fit:
//!   a plan that doesn't fit suggests a later exam day instead.
//! - **Learn window:** the study days before dedicated. New items end at the
//!   *first-pass deadline*, which leaves the last part of the window
//!   ([`SECOND_PASS_SHARE`], at least [`MIN_SECOND_PASS_DAYS`] study days when
//!   the window allows) for a second pass, where every lecture gets a revisit
//!   session.
//! - **Self-assessments:** a baseline when the plan starts, a checkpoint about
//!   every [`CHECKPOINT_EVERY`] days, one a week in dedicated, and a Free 120
//!   [`FREE_120_BEFORE`] days before the exam (see [`milestones`]). A
//!   self-assessment day brings no new items and no qbank target.
//! - **Day weights:** a light day takes half a normal day's share of new
//!   items and qbank questions, a heavy day one and a half. The weekly
//!   catch-up day, days off and vacations take no new items.
//! - **Missed work** is spread over the next days by default. Mohamed can
//!   move it to the next catch-up day instead, and undo that (see
//!   [`Rebalance`]).
//! - **Review cap:** each new item is expected back on the study days at
//!   [`REVIEW_LADDER`] after it is learned (a typical FSRS ladder at 90%
//!   retention). A day takes no more new items than keep every later day's
//!   expected reviews under [`PlanSettings::review_cap`]. Items that don't fit
//!   spill into the second pass, then the plan says it is tight and how many
//!   extra study days a week would make it fit.

use serde::{Deserialize, Serialize};

/// A calendar day, counted from 1970-01-01. Local dates only: the app turns a
/// clock reading into a `Day` with the study-day rollover (4 am).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Day(pub i32);

impl Day {
    /// The day of a civil date (proleptic Gregorian). `None` for a date that
    /// doesn't exist, such as February 30.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Day> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        // Howard Hinnant's days_from_civil.
        let y = if month <= 2 { year - 1 } else { year };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = month as i32;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Day(era * 146_097 + doe - 719_468))
    }

    /// The civil date of this day: (year, month, day).
    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + i32::from(m <= 2);
        (y, m, d)
    }

    /// Parses `YYYY-MM-DD`, the value of an HTML date input.
    pub fn parse(text: &str) -> Option<Day> {
        let mut parts = text.trim().splitn(3, '-');
        let y = parts.next()?.parse().ok()?;
        let m = parts.next()?.parse().ok()?;
        let d = parts.next()?.parse().ok()?;
        Day::from_ymd(y, m, d)
    }

    /// `YYYY-MM-DD`.
    pub fn iso(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }

    /// 0 = Saturday … 6 = Friday: the Egyptian week, which starts on Saturday.
    pub fn weekday(self) -> usize {
        // 1970-01-01 was a Thursday, index 5 in this week.
        (self.0 + 5).rem_euclid(7) as usize
    }

    /// The study day of a moment: local time shifted back by the rollover
    /// hour, so a session at 1 am still counts for the evening before.
    pub fn from_unix_ms(ms: i64, utc_offset_min: i32, rollover_hour: u32) -> Day {
        let local = ms + i64::from(utc_offset_min) * 60_000 - i64::from(rollover_hour) * 3_600_000;
        Day(local.div_euclid(86_400_000) as i32)
    }

    pub fn plus(self, days: i32) -> Day {
        Day(self.0 + days)
    }

    /// Days from `self` to `later` (negative if `later` is earlier).
    pub fn until(self, later: Day) -> i32 {
        later.0 - self.0
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Share of the learn window kept for the second pass.
pub const SECOND_PASS_SHARE: f32 = 0.3;
/// The second pass gets at least this many study days when the window has room.
pub const MIN_SECOND_PASS_DAYS: usize = 7;
/// Study days after learning on which a new item is expected back.
pub const REVIEW_LADDER: [usize; 4] = [1, 3, 7, 14];
/// Default daily cap on expected reviews.
pub const DEFAULT_REVIEW_CAP: u32 = 150;
/// Default length of dedicated, in weeks, and the range Mohamed can pick.
pub const DEFAULT_DEDICATED_WEEKS: u32 = 6;
pub const DEDICATED_WEEKS: std::ops::RangeInclusive<u32> = 4..=8;
/// Default qbank questions on a normal systems-phase day. Integration asks
/// twice that and dedicated four times (four 20-question blocks).
pub const DEFAULT_QBANK_PER_DAY: u32 = 20;
/// The highest daily qbank base the planner accepts.
pub const MAX_QBANK_PER_DAY: u32 = 120;
/// Days between self-assessment checkpoints before dedicated (about three
/// months).
pub const CHECKPOINT_EVERY: i32 = 91;
/// The Free 120 sits this many days before the exam (moved to a study day
/// between 3 and 7 days out).
pub const FREE_120_BEFORE: i32 = 5;
/// How far a suggested later exam day may move, in weeks.
pub const MAX_LATER_WEEKS: i32 = 26;
/// The furthest exam day the planner takes, in days from today: a typo like
/// 2207 would otherwise spread the items to a trickle over decades (#249).
pub const MAX_DAYS_AHEAD: i32 = 3 * 366;
/// The highest daily review cap the planner suggests or accepts.
pub const MAX_REVIEW_CAP: u32 = 1000;

/// An exam day the planner accepts: after today and within [`MAX_DAYS_AHEAD`].
pub fn exam_day_ok(today: Day, exam: Day) -> bool {
    exam > today && today.until(exam) <= MAX_DAYS_AHEAD
}

/// How much a study day takes, set per weekday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayLoad {
    /// Half a normal day's share.
    Light,
    #[default]
    Normal,
    /// One and a half times a normal day's share.
    Heavy,
}

impl DayLoad {
    /// The day's weight in half-days.
    fn weight(self) -> u32 {
        match self {
            DayLoad::Light => 1,
            DayLoad::Normal => 2,
            DayLoad::Heavy => 3,
        }
    }

    /// Scales a normal day's amount, rounding halves up.
    pub fn scale(self, normal: u32) -> u32 {
        (normal * self.weight()).div_ceil(2)
    }
}

/// Days off, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vacation {
    pub from: Day,
    pub to: Day,
}

impl Vacation {
    pub fn contains(&self, day: Day) -> bool {
        self.from <= day && day <= self.to
    }
}

/// What Mohamed chose for the new items he missed: spread over the next
/// days (the default, which the plan does by itself), or put on the next
/// catch-up day. Either choice can be flipped back (Undo) until the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rebalance {
    /// The day the choice was made; moving to the catch-up day holds until
    /// the catch-up day after it.
    pub on: Day,
    /// New items missed.
    pub missed: u32,
    /// Put them on the catch-up day instead of spreading them.
    pub to_catch_up: bool,
}

/// What Mohamed sets in the planner. LWW-synced as one settings row; every
/// field added after the first version has a serde default, so older rows
/// still load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSettings {
    /// The exam day. `None` until Mohamed sets it in the app.
    pub exam: Option<Day>,
    /// Study days of the week, indexed like [`Day::weekday`] (Saturday first).
    pub weekdays: [bool; 7],
    pub review_cap: u32,
    /// Length of dedicated, in weeks ([`DEDICATED_WEEKS`]).
    #[serde(default = "default_dedicated_weeks")]
    pub dedicated_weeks: u32,
    /// The day the plan started: the baseline self-assessment's day, and the
    /// day checkpoints count from. Set with the first exam day.
    #[serde(default)]
    pub started: Option<Day>,
    /// Load per weekday, indexed like [`Self::weekdays`].
    #[serde(default)]
    pub day_loads: [DayLoad; 7],
    /// One weekday kept free of new work for catching up.
    #[serde(default)]
    pub catch_up_day: Option<usize>,
    #[serde(default)]
    pub vacations: Vec<Vacation>,
    /// Qbank questions on a normal systems-phase day.
    #[serde(default = "default_qbank")]
    pub qbank_per_day: u32,
    #[serde(default)]
    pub rebalance: Option<Rebalance>,
}

fn default_dedicated_weeks() -> u32 {
    DEFAULT_DEDICATED_WEEKS
}

fn default_qbank() -> u32 {
    DEFAULT_QBANK_PER_DAY
}

impl Default for PlanSettings {
    fn default() -> Self {
        Self {
            exam: None,
            // Saturday to Thursday; Friday off.
            weekdays: [true, true, true, true, true, true, false],
            review_cap: DEFAULT_REVIEW_CAP,
            dedicated_weeks: DEFAULT_DEDICATED_WEEKS,
            started: None,
            day_loads: [DayLoad::Normal; 7],
            catch_up_day: None,
            vacations: Vec::new(),
            qbank_per_day: DEFAULT_QBANK_PER_DAY,
            rebalance: None,
        }
    }
}

impl PlanSettings {
    pub fn days_per_week(&self) -> usize {
        self.weekdays.iter().filter(|on| **on).count()
    }

    /// Sets the exam day; the first one also starts the plan today.
    pub fn set_exam(&mut self, exam: Day, today: Day) {
        self.exam = Some(exam);
        if self.started.is_none() {
            self.started = Some(today);
        }
    }

    /// Ticked in the week and not on vacation.
    pub fn is_study_day(&self, day: Day) -> bool {
        self.weekdays[day.weekday()] && !self.vacations.iter().any(|v| v.contains(day))
    }

    /// The weekly catch-up day, if it is one of the study days.
    pub fn is_catch_up_day(&self, day: Day) -> bool {
        self.catch_up_day.is_some_and(|w| w == day.weekday() && self.weekdays[w])
    }

    /// The first day of dedicated for an exam day.
    pub fn dedicated_from(&self, exam: Day) -> Day {
        let weeks = self.dedicated_weeks.clamp(*DEDICATED_WEEKS.start(), *DEDICATED_WEEKS.end());
        exam.plus(-(weeks as i32 * 7))
    }

    /// The next catch-up day from `from`, itself included, before `exam`.
    pub fn next_catch_up(&self, from: Day, exam: Day) -> Option<Day> {
        (0..from.until(exam))
            .map(|i| from.plus(i))
            .find(|d| self.is_catch_up_day(*d) && self.is_study_day(*d))
    }
}

/// One lecture's progress, in book order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LectureProgress {
    /// `cvs.L04`.
    pub id: String,
    /// Items in the app (rejected items left out).
    pub total: u32,
    /// Items answered at least once.
    pub seen: u32,
    /// Items answered at least twice, on different days.
    pub seen_twice: u32,
}

/// Everything the planner reads besides the settings.
#[derive(Debug, Clone, Copy)]
pub struct PlanInput<'a> {
    pub today: Day,
    pub lectures: &'a [LectureProgress],
    /// Reviews already due per calendar day from FSRS, today first (overdue
    /// cards count on today). Empty when nothing is scheduled yet.
    pub review_forecast: &'a [u32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Focus {
    /// New items, plus the day's reviews.
    Learn,
    /// Second pass: revisit whole lectures, plus reviews and any spill-over.
    Revisit,
    /// A self-assessment (see [`DayPlan::milestone`]).
    Mock,
    /// Dedicated: qbank blocks and the weakest topics.
    WeakTopics,
    /// The weekly catch-up day: reviews and whatever was missed, no new work.
    CatchUp,
    /// Not a study day (a weekday off or a vacation).
    Rest,
}

/// The three phases of the plan, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// First pass through every lecture.
    Systems,
    /// Second pass, with mixed qbank questions.
    Integration,
    /// The last weeks: qbank blocks, weak topics, self-assessments.
    Dedicated,
}

impl Phase {
    /// The phase's qbank questions in normal days' base amounts.
    fn qbank_factor(self) -> u32 {
        match self {
            Phase::Systems => 1,
            Phase::Integration => 2,
            Phase::Dedicated => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneKind {
    /// The first self-assessment, when the plan starts.
    Baseline,
    /// About every three months before dedicated.
    Checkpoint,
    /// Once a week in dedicated.
    Weekly,
    /// The Free 120, a few days before the exam.
    Free120,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Milestone {
    pub day: Day,
    pub kind: MilestoneKind,
}

/// The self-assessment days from the plan's start to the exam (see the
/// module docs). Each sits on a study day; one that finds none within a
/// week is left out. Without a start day (a plan saved before there were
/// milestones) only dedicated's weekly ones and the Free 120 are set, since
/// a baseline and checkpoints counted from today would move every day.
pub fn milestones(settings: &PlanSettings, today: Day, exam: Day) -> Vec<Milestone> {
    let Some(started) = settings.started.map(|d| d.min(exam)) else {
        let mut dated = settings.clone();
        dated.started = Some(today.min(settings.dedicated_from(exam)).plus(-7));
        return milestones(&dated, today, exam)
            .into_iter()
            .filter(|m| matches!(m.kind, MilestoneKind::Weekly | MilestoneKind::Free120))
            .collect();
    };
    let dedicated_from = settings.dedicated_from(exam);
    let snap =
        |d: Day, before: Day| (0..7).map(|i| d.plus(i)).find(|d| *d < before && settings.is_study_day(*d));
    let free120 = [0, 1, -1, 2, -2]
        .iter()
        .map(|o| exam.plus(-FREE_120_BEFORE + o))
        .find(|d| settings.is_study_day(*d) && *d > started)
        .unwrap_or(exam.plus(-FREE_120_BEFORE));
    let mut out = Vec::new();
    let baseline = snap(started, free120);
    if let Some(day) = baseline {
        out.push(Milestone { day, kind: MilestoneKind::Baseline });
    }
    let after_baseline = |d: Day| baseline.is_none_or(|b| d > b.plus(3));
    let mut at = started.plus(CHECKPOINT_EVERY);
    // A checkpoint too close to dedicated's first weekly one is dropped.
    while at < dedicated_from.plus(-14) {
        if let Some(day) = snap(at, dedicated_from).filter(|d| after_baseline(*d)) {
            out.push(Milestone { day, kind: MilestoneKind::Checkpoint });
        }
        at = at.plus(CHECKPOINT_EVERY);
    }
    let mut at = dedicated_from;
    while at < free120.plus(-3) {
        if let Some(day) = snap(at, free120.plus(-2)).filter(|d| after_baseline(*d)) {
            out.push(Milestone { day, kind: MilestoneKind::Weekly });
        }
        at = at.plus(7);
    }
    if free120 > started {
        out.push(Milestone { day: free120, kind: MilestoneKind::Free120 });
    }
    out
}

/// One day of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayPlan {
    pub day: Day,
    pub focus: Focus,
    pub phase: Phase,
    /// New items to learn.
    pub new: u32,
    /// Lectures the new items come from, in order.
    pub new_lectures: Vec<String>,
    /// Lectures to revisit as a whole (second pass).
    pub revisit: Vec<String>,
    /// Expected reviews: FSRS's forecast plus the plan's own new items.
    pub reviews: u32,
    /// Questions to do in the external qbank.
    pub qbank: u32,
    /// The self-assessment set for this day.
    pub milestone: Option<MilestoneKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    /// No exam date yet.
    NoExamDate,
    /// No study day ticked.
    NoStudyDays,
    /// The exam day is today or past.
    ExamDone,
    /// Inside dedicated: no new items left to plan.
    Dedicated,
    /// Everything fits before the first-pass deadline.
    OnTrack,
    /// Fits only by using the second pass for new items.
    Snug,
    /// `left_over` items don't fit before dedicated; `fix` says what would
    /// make them fit with this exam day, `later_exam` the first exam day
    /// (in whole weeks) that fits as the plan stands.
    Tight { left_over: u32, fix: TightFix, later_exam: Option<Day> },
}

/// What would make a tight plan fit, checked by re-planning (#251).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TightFix {
    /// This many more study days a week.
    MoreDays(u32),
    /// A daily review cap of this much (up to [`MAX_REVIEW_CAP`]).
    HigherCap(u32),
    /// Neither is enough before dedicated: high-yield first.
    HighYield,
    /// Already inside dedicated: no learn days are left to change.
    Dedicated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub status: PlanStatus,
    /// Every day from today to the day before the exam.
    pub days: Vec<DayPlan>,
    /// Study days from today to the day before the exam.
    pub study_days_left: u32,
    /// The first day of integration, if the plan still has one ahead.
    pub integration_from: Option<Day>,
    /// The first day of dedicated, if there is an exam date.
    pub dedicated_from: Option<Day>,
    pub total: u32,
    pub seen: u32,
    pub seen_twice: u32,
}

impl Plan {
    /// Today's entry (`None` once the exam is past or without a date).
    pub fn today(&self) -> Option<&DayPlan> {
        self.days.first()
    }

    /// The next day that isn't a rest day, today included.
    pub fn next_study_day(&self) -> Option<&DayPlan> {
        self.days.iter().find(|d| d.focus != Focus::Rest)
    }

    /// The next self-assessment, today included.
    pub fn next_milestone(&self) -> Option<Milestone> {
        self.days.iter().find_map(|d| d.milestone.map(|kind| Milestone { day: d.day, kind }))
    }
}

/// Builds the plan. See the module docs for the rules.
pub fn plan(settings: &PlanSettings, input: PlanInput) -> Plan {
    let total = input.lectures.iter().map(|l| l.total).sum();
    let seen = input.lectures.iter().map(|l| l.seen.min(l.total)).sum();
    let seen_twice = input.lectures.iter().map(|l| l.seen_twice.min(l.total)).sum();
    let mut out = Plan {
        status: PlanStatus::NoExamDate,
        days: Vec::new(),
        study_days_left: 0,
        integration_from: None,
        dedicated_from: None,
        total,
        seen,
        seen_twice,
    };
    let Some(exam) = settings.exam else { return out };
    let exam = exam.min(input.today.plus(MAX_DAYS_AHEAD));
    out.dedicated_from = Some(settings.dedicated_from(exam));
    if exam <= input.today {
        out.status = PlanStatus::ExamDone;
        return out;
    }
    if settings.days_per_week() == 0 {
        out.status = PlanStatus::NoStudyDays;
        return out;
    }
    let built = build(settings, input, exam);
    out.study_days_left = built.days.iter().filter(|d| d.focus != Focus::Rest).count() as u32;
    out.integration_from = built.integration_from;
    out.days = built.days;
    out.status = match built.left_over {
        0 if built.learn_days == 0 => PlanStatus::Dedicated,
        0 if built.used_second_pass => PlanStatus::Snug,
        0 => PlanStatus::OnTrack,
        left_over => PlanStatus::Tight {
            left_over,
            fix: if built.learn_days == 0 { TightFix::Dedicated } else { tight_fix(settings, input, exam) },
            later_exam: later_exam(settings, input, exam),
        },
    };
    out
}

/// How many weeks to suggest adding when the self-assessments say "not
/// ready yet" close to the exam.
pub const NOT_READY_WEEKS: i32 = 4;

/// A later exam day to suggest when recent self-assessments aren't there
/// yet: [`NOT_READY_WEEKS`] later, once dedicated is two weeks away or has
/// begun. Earlier low scores are expected and suggest nothing. `None` when
/// the move would leave the planner's range.
pub fn later_exam_if_not_ready(settings: &PlanSettings, today: Day) -> Option<Day> {
    let exam = settings.exam?;
    if exam <= today || today < settings.dedicated_from(exam).plus(-14) {
        return None;
    }
    Some(exam.plus(NOT_READY_WEEKS * 7)).filter(|d| exam_day_ok(today, *d))
}

struct Built {
    days: Vec<DayPlan>,
    learn_days: usize,
    left_over: u32,
    used_second_pass: bool,
    integration_from: Option<Day>,
}

fn build(settings: &PlanSettings, input: PlanInput, exam: Day) -> Built {
    let dedicated_from = settings.dedicated_from(exam);
    let marks = milestones(settings, input.today, exam);
    let mut days: Vec<DayPlan> = (0..input.today.until(exam))
        .map(|i| {
            let day = input.today.plus(i);
            let milestone = marks.iter().find(|m| m.day == day).map(|m| m.kind);
            let focus = if !settings.is_study_day(day) {
                Focus::Rest
            } else if milestone.is_some() {
                Focus::Mock
            } else if day >= dedicated_from {
                Focus::WeakTopics
            } else if settings.is_catch_up_day(day) {
                Focus::CatchUp
            } else {
                Focus::Learn
            };
            let phase = if day >= dedicated_from { Phase::Dedicated } else { Phase::Systems };
            DayPlan {
                day,
                focus,
                phase,
                new: 0,
                new_lectures: Vec::new(),
                revisit: Vec::new(),
                reviews: 0,
                qbank: 0,
                milestone,
            }
        })
        .collect();

    // Study days in index order; reviews on rest days roll to the next one.
    let study: Vec<usize> = (0..days.len()).filter(|&i| days[i].focus != Focus::Rest).collect();
    let mut load = vec![0u32; study.len()];
    let mut slot = 0;
    for i in 0..days.len() {
        while slot < study.len() && study[slot] < i {
            slot += 1;
        }
        let due = input.review_forecast.get(i).copied().unwrap_or(0);
        if let Some(l) = load.get_mut(slot.min(study.len().saturating_sub(1))) {
            *l += due;
        }
    }

    let learn: Vec<usize> = (0..study.len()).filter(|&s| days[study[s]].focus == Focus::Learn).collect();
    let second = second_pass_days(learn.len());
    let first = learn.len() - second;
    for &s in &learn[first..] {
        days[study[s]].focus = Focus::Revisit;
    }
    let integration_from = learn.get(first).map(|&s| days[study[s]].day);
    if let Some(from) = integration_from {
        for d in days.iter_mut().filter(|d| d.day >= from && d.phase == Phase::Systems) {
            d.phase = Phase::Integration;
        }
    }

    // Missed items Mohamed moved to the catch-up day go there first.
    let catch_up = settings
        .rebalance
        .filter(|r| r.to_catch_up)
        .and_then(|r| {
            settings
                .next_catch_up(r.on, exam)
                .filter(|d| *d >= input.today && *d < dedicated_from)
                .map(|d| (d, r.missed))
        })
        .and_then(|(d, n)| study.iter().position(|&i| days[i].day == d).map(|s| (s, n)));
    let mut slots: Vec<usize> = learn.clone();
    if let Some((s, _)) = catch_up {
        slots.push(s);
        slots.sort_unstable();
    }
    let weights: Vec<u32> =
        study.iter().map(|&i| settings.day_loads[days[i].day.weekday()].weight()).collect();
    let weight = |s: usize| weights[s];
    // Weights still ahead, per pass: [k] covers learn[k..first] / learn[k..].
    let mut ahead_first = vec![0u32; learn.len() + 1];
    let mut ahead_all = vec![0u32; learn.len() + 1];
    for k in (0..learn.len()).rev() {
        ahead_all[k] = ahead_all[k + 1] + weight(learn[k]);
        ahead_first[k] = if k < first { ahead_first[k + 1] + weight(learn[k]) } else { 0 };
    }

    // New items in lecture order: lectures already started come first.
    let mut queue: Vec<(String, u32)> = input
        .lectures
        .iter()
        .filter(|l| l.total > l.seen)
        .map(|l| (l.id.clone(), l.total - l.seen))
        .collect();
    queue.sort_by_key(|(id, _)| {
        let l = input.lectures.iter().position(|l| &l.id == id).unwrap_or(usize::MAX);
        (input.lectures[l].seen == 0, l)
    });
    let mut remaining: u32 = queue.iter().map(|(_, n)| n).sum();
    let mut reserved = catch_up.map_or(0, |(_, n)| n).min(remaining);
    let mut used_second_pass = false;
    for &s in &slots {
        if remaining == 0 {
            break;
        }
        let wanted = if catch_up.is_some_and(|(c, _)| c == s) {
            std::mem::take(&mut reserved).min(remaining)
        } else {
            let k = learn.binary_search(&s).unwrap_or_default();
            let in_first = k < first;
            let ahead = if in_first { ahead_first[k] } else { ahead_all[k] };
            (remaining - reserved.min(remaining)).saturating_mul(weight(s)).div_ceil(ahead.max(1))
        };
        // A day already at the cap learns nothing new; otherwise every day the
        // new items come back on must stay under it.
        let room = if load[s] >= settings.review_cap {
            0
        } else {
            REVIEW_LADDER
                .iter()
                .filter_map(|o| load.get(s + o))
                .map(|l| settings.review_cap.saturating_sub(*l))
                .min()
                .unwrap_or(settings.review_cap)
        };
        let new = wanted.min(room);
        if new == 0 {
            continue;
        }
        if days[study[s]].focus == Focus::Revisit {
            used_second_pass = true;
        }
        for o in REVIEW_LADDER {
            if let Some(l) = load.get_mut(s + o) {
                *l += new;
            }
        }
        remaining -= new;
        let d = &mut days[study[s]];
        d.new = new;
        let mut take = new;
        while take > 0 {
            let Some(head) = queue.first_mut() else { break };
            let n = head.1.min(take);
            d.new_lectures.push(head.0.clone());
            head.1 -= n;
            take -= n;
            if head.1 == 0 {
                queue.remove(0);
            }
        }
    }

    // Second pass: every lecture with items gets one revisit day, in book
    // order, spread evenly over the second-pass days.
    let lectures: Vec<&LectureProgress> = input.lectures.iter().filter(|l| l.total > 0).collect();
    let revisit_days = &learn[first..];
    if !revisit_days.is_empty() {
        for (n, l) in lectures.iter().enumerate() {
            let at = n * revisit_days.len() / lectures.len();
            days[study[revisit_days[at]]].revisit.push(l.id.clone());
        }
    }

    for (s, &i) in study.iter().enumerate() {
        let d = &mut days[i];
        d.reviews = load[s];
        if matches!(d.focus, Focus::Learn | Focus::Revisit | Focus::WeakTopics) {
            let normal = settings.qbank_per_day.min(MAX_QBANK_PER_DAY) * d.phase.qbank_factor();
            d.qbank = settings.day_loads[d.day.weekday()].scale(normal);
        }
    }
    Built { days, learn_days: learn.len(), left_over: remaining, used_second_pass, integration_from }
}

fn second_pass_days(learn_days: usize) -> usize {
    if learn_days == 0 {
        return 0;
    }
    let share = (learn_days as f32 * SECOND_PASS_SHARE).ceil() as usize;
    // Keep at least one first-pass day.
    share.max(MIN_SECOND_PASS_DAYS.min(learn_days / 2)).min(learn_days - 1)
}

/// More study days a week first (adding the free weekdays in order, Friday
/// last), then a higher review cap (in steps of 50), and only what
/// re-planning shows would actually fit.
fn tight_fix(settings: &PlanSettings, input: PlanInput, exam: Day) -> TightFix {
    let free: Vec<usize> = (0..7).filter(|&w| !settings.weekdays[w]).collect();
    let mut more = settings.clone();
    for (n, w) in free.iter().enumerate() {
        more.weekdays[*w] = true;
        if build(&more, input, exam).left_over == 0 {
            return TightFix::MoreDays(n as u32 + 1);
        }
    }
    let mut higher = settings.clone();
    let mut cap = (settings.review_cap / 50 + 1) * 50;
    while cap <= MAX_REVIEW_CAP {
        higher.review_cap = cap;
        if build(&higher, input, exam).left_over == 0 {
            return TightFix::HigherCap(cap);
        }
        cap += 50;
    }
    TightFix::HighYield
}

/// The first exam day, whole weeks later, on which everything fits with
/// dedicated kept at full length.
fn later_exam(settings: &PlanSettings, input: PlanInput, exam: Day) -> Option<Day> {
    (1..=MAX_LATER_WEEKS)
        .map(|w| exam.plus(w * 7))
        .take_while(|d| exam_day_ok(input.today, *d))
        .find(|d| build(settings, input, *d).left_over == 0)
}

#[cfg(test)]
mod tests;
