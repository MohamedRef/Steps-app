//! The exam planner: spreads the items Mohamed hasn't seen yet over the study
//! days left before his exam, so that every lecture is seen at least twice
//! and the last days stay free for mock exams and weak topics.
//!
//! The plan is a pure function of today's progress, so it is recomputed every
//! day: a missed day never piles up as "debt", it only nudges the next days'
//! targets up a little.
//!
//! Rules, stated once:
//! - **Final window:** the last [`PlanSettings::final_days`] days before the
//!   exam bring no new items. Study days there alternate a mock exam and a
//!   weak-topics session.
//! - **Learn window:** the study days before that. New items end at the
//!   *first-pass deadline*, which leaves the last part of the window
//!   ([`SECOND_PASS_SHARE`], at least [`MIN_SECOND_PASS_DAYS`] study days when
//!   the window allows) for a second pass, where every lecture gets a revisit
//!   session.
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
/// Default length of the final window, in days.
pub const DEFAULT_FINAL_DAYS: u32 = 10;
/// The furthest exam day the planner takes, in days from today: a typo like
/// 2207 would otherwise spread the items to a trickle over decades (#249).
pub const MAX_DAYS_AHEAD: i32 = 3 * 366;
/// The highest daily review cap the planner suggests or accepts.
pub const MAX_REVIEW_CAP: u32 = 1000;

/// An exam day the planner accepts: after today and within [`MAX_DAYS_AHEAD`].
pub fn exam_day_ok(today: Day, exam: Day) -> bool {
    exam > today && today.until(exam) <= MAX_DAYS_AHEAD
}

/// What Mohamed sets in the planner. LWW-synced as one settings row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSettings {
    /// The exam day. `None` until Mohamed sets it in the app.
    pub exam: Option<Day>,
    /// Study days of the week, indexed like [`Day::weekday`] (Saturday first).
    pub weekdays: [bool; 7],
    pub review_cap: u32,
    pub final_days: u32,
}

impl Default for PlanSettings {
    fn default() -> Self {
        Self {
            exam: None,
            // Saturday to Thursday; Friday off.
            weekdays: [true, true, true, true, true, true, false],
            review_cap: DEFAULT_REVIEW_CAP,
            final_days: DEFAULT_FINAL_DAYS,
        }
    }
}

impl PlanSettings {
    pub fn days_per_week(&self) -> usize {
        self.weekdays.iter().filter(|on| **on).count()
    }

    fn is_study_day(&self, day: Day) -> bool {
        self.weekdays[day.weekday()]
    }
}

/// One lecture's progress, in book order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LectureProgress {
    /// `hb.L01`.
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
    /// Final window: a mock exam.
    Mock,
    /// Final window: the weakest topics.
    WeakTopics,
    /// Not a study day.
    Rest,
}

/// One day of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayPlan {
    pub day: Day,
    pub focus: Focus,
    /// New items to learn.
    pub new: u32,
    /// Lectures the new items come from, in order.
    pub new_lectures: Vec<String>,
    /// Lectures to revisit as a whole (second pass).
    pub revisit: Vec<String>,
    /// Expected reviews: FSRS's forecast plus the plan's own new items.
    pub reviews: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    /// No exam date yet.
    NoExamDate,
    /// No study day ticked.
    NoStudyDays,
    /// The exam day is today or past.
    ExamDone,
    /// Inside the final window: no new items left to plan.
    FinalWindow,
    /// Everything fits before the first-pass deadline.
    OnTrack,
    /// Fits only by using the second pass for new items.
    Snug,
    /// `left_over` items don't fit before the final window; `fix` says what
    /// would make them fit.
    Tight { left_over: u32, fix: TightFix },
}

/// What would make a tight plan fit, checked by re-planning (#251).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TightFix {
    /// This many more study days a week.
    MoreDays(u32),
    /// A daily review cap of this much (up to [`MAX_REVIEW_CAP`]).
    HigherCap(u32),
    /// Neither is enough before the final window: high-yield first.
    HighYield,
    /// Already inside the final window: no learn days are left to change.
    FinalWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub status: PlanStatus,
    /// Every day from today to the day before the exam.
    pub days: Vec<DayPlan>,
    /// Study days from today to the day before the exam.
    pub study_days_left: u32,
    /// The first day of the final window, if there is an exam date.
    pub final_from: Option<Day>,
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
        final_from: None,
        total,
        seen,
        seen_twice,
    };
    let Some(exam) = settings.exam else { return out };
    let exam = exam.min(input.today.plus(MAX_DAYS_AHEAD));
    out.final_from = Some(exam.plus(-(settings.final_days as i32)));
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
    out.days = built.days;
    out.status = match built.left_over {
        0 if built.learn_days == 0 => PlanStatus::FinalWindow,
        0 if built.used_second_pass => PlanStatus::Snug,
        0 => PlanStatus::OnTrack,
        left_over if built.learn_days == 0 => PlanStatus::Tight { left_over, fix: TightFix::FinalWindow },
        left_over => PlanStatus::Tight { left_over, fix: tight_fix(settings, input, exam) },
    };
    out
}

struct Built {
    days: Vec<DayPlan>,
    learn_days: usize,
    left_over: u32,
    used_second_pass: bool,
}

fn build(settings: &PlanSettings, input: PlanInput, exam: Day) -> Built {
    let final_from = exam.plus(-(settings.final_days as i32));
    let mut days: Vec<DayPlan> = (0..input.today.until(exam))
        .map(|i| {
            let day = input.today.plus(i);
            let focus = if !settings.is_study_day(day) {
                Focus::Rest
            } else if day >= final_from {
                Focus::Mock // alternated below
            } else {
                Focus::Learn
            };
            DayPlan { day, focus, new: 0, new_lectures: Vec::new(), revisit: Vec::new(), reviews: 0 }
        })
        .collect();

    // Final window: mock first, then weak topics, alternating.
    for (n, d) in days.iter_mut().filter(|d| d.focus == Focus::Mock).enumerate() {
        if n % 2 == 1 {
            d.focus = Focus::WeakTopics;
        }
    }

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
    let mut used_second_pass = false;
    for (k, &s) in learn.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        let in_first = k < first;
        let days_left = if in_first { first - k } else { learn.len() - k } as u32;
        let wanted = remaining.div_ceil(days_left);
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
        if !in_first {
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
        days[i].reviews = load[s];
    }
    Built { days, learn_days: learn.len(), left_over: remaining, used_second_pass }
}

fn second_pass_days(learn_days: usize) -> usize {
    if learn_days == 0 {
        return 0;
    }
    let share = (learn_days as f32 * SECOND_PASS_SHARE).ceil() as usize;
    // Keep at least one first-pass day.
    share.max(MIN_SECOND_PASS_DAYS.min(learn_days / 2)).min(learn_days - 1)
}

/// How many more study days a week would fit every new item, adding the
/// free weekdays in order (Friday last).
/// More study days first, then a higher review cap (in steps of 50), and
/// only what re-planning shows would actually fit.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> Day {
        Day::from_ymd(y, m, d).unwrap()
    }

    fn lecture(id: &str, total: u32, seen: u32) -> LectureProgress {
        LectureProgress { id: id.into(), total, seen, seen_twice: 0 }
    }

    fn settings(exam: Day) -> PlanSettings {
        PlanSettings { exam: Some(exam), ..PlanSettings::default() }
    }

    #[test]
    fn civil_dates_round_trip() {
        assert_eq!(day(1970, 1, 1), Day(0));
        assert_eq!(day(2000, 3, 1).ymd(), (2000, 3, 1));
        for n in [-800_000, -1, 0, 1, 10_957, 20_000, 20_454, 30_000] {
            let (y, m, d) = Day(n).ymd();
            assert_eq!(Day::from_ymd(y, m, d), Some(Day(n)));
        }
        assert_eq!(Day::from_ymd(2027, 2, 29), None);
        assert_eq!(Day::from_ymd(2028, 2, 29).map(Day::iso).as_deref(), Some("2028-02-29"));
        assert_eq!(Day::parse("2027-01-20"), Some(day(2027, 1, 20)));
        assert_eq!(Day::parse("2027-13-01"), None);
    }

    #[test]
    fn weekdays_start_on_saturday() {
        assert_eq!(day(2026, 10, 31).weekday(), 0); // a Saturday
        assert_eq!(day(2026, 10, 30).weekday(), 6); // a Friday
        assert_eq!(day(2027, 1, 1).weekday(), 6); // a Friday
    }

    #[test]
    fn study_day_rolls_over_at_four() {
        // 2026-10-27 01:30 in Cairo (UTC+2) still counts for the 26th.
        let ms = (day(2026, 10, 26).0 as i64) * 86_400_000 + 23 * 3_600_000 + 30 * 60_000;
        assert_eq!(Day::from_unix_ms(ms, 120, 4), day(2026, 10, 26));
        let ms_after = ms + 3 * 3_600_000; // 04:30 Cairo
        assert_eq!(Day::from_unix_ms(ms_after, 120, 4), day(2026, 10, 27));
    }

    #[test]
    fn no_date_no_plan() {
        let p =
            plan(&PlanSettings::default(), PlanInput { today: Day(0), lectures: &[], review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::NoExamDate);
        assert!(p.days.is_empty());
    }

    #[test]
    fn exam_past_or_today() {
        let today = day(2027, 1, 20);
        let p = plan(&settings(today), PlanInput { today, lectures: &[], review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::ExamDone);
    }

    #[test]
    fn no_weekday_ticked() {
        let mut s = settings(day(2027, 1, 20));
        s.weekdays = [false; 7];
        let p = plan(&s, PlanInput { today: day(2026, 10, 27), lectures: &[], review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::NoStudyDays);
    }

    #[test]
    fn spreads_new_items_and_keeps_the_final_window_free() {
        let today = day(2026, 10, 27);
        let exam = day(2027, 1, 20);
        let lectures: Vec<_> = (1..=20).map(|n| lecture(&format!("hb.L{n:02}"), 75, 0)).collect();
        let p = plan(&settings(exam), PlanInput { today, lectures: &lectures, review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::OnTrack);
        assert_eq!(p.days.len(), today.until(exam) as usize);
        let final_from = exam.plus(-10);
        assert_eq!(p.final_from, Some(final_from));
        let new: u32 = p.days.iter().map(|d| d.new).sum();
        assert_eq!(new, 1500);
        // No new items in the final window or the second pass, nor on Fridays.
        for d in &p.days {
            if d.day >= final_from || d.day.weekday() == 6 || d.focus == Focus::Revisit {
                assert_eq!(d.new, 0, "{d:?}");
            }
            assert!(d.reviews <= 150, "{d:?}");
        }
        // The final window alternates mock and weak topics, starting with a mock.
        let finals: Vec<Focus> = p
            .days
            .iter()
            .filter(|d| d.day >= final_from && d.focus != Focus::Rest)
            .map(|d| d.focus)
            .collect();
        assert_eq!(finals.first(), Some(&Focus::Mock));
        assert!(finals.windows(2).all(|w| w[0] != w[1]));
        // Every lecture gets a revisit, after its own first pass.
        for l in &lectures {
            let revisit = p.days.iter().find(|d| d.revisit.contains(&l.id)).expect("revisit");
            let last_new = p.days.iter().rfind(|d| d.new_lectures.contains(&l.id)).expect("learned");
            assert!(revisit.day > last_new.day, "{}", l.id);
        }
        // Lectures are learned in book order.
        let order: Vec<&String> = p.days.iter().flat_map(|d| &d.new_lectures).collect();
        assert_eq!(order.first().map(|s| s.as_str()), Some("hb.L01"));
        assert_eq!(order.last().map(|s| s.as_str()), Some("hb.L20"));
    }

    #[test]
    fn started_lectures_come_first() {
        let today = day(2026, 10, 27);
        let lectures = [lecture("hb.L01", 50, 0), lecture("hb.L02", 50, 10)];
        let p =
            plan(&settings(day(2027, 1, 20)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
        assert_eq!(p.days.iter().find(|d| d.new > 0).unwrap().new_lectures[0], "hb.L02");
        assert_eq!(p.seen, 10);
    }

    #[test]
    fn heavy_load_says_tight_and_suggests_days() {
        let today = day(2026, 10, 27);
        let mut s = settings(day(2026, 12, 15));
        s.weekdays = [true, false, true, false, true, false, false];
        let lectures = [lecture("hb.L01", 3000, 0)];
        let p = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
        let PlanStatus::Tight { left_over, fix } = p.status else { panic!("{:?}", p.status) };
        assert!(left_over > 0);
        if let TightFix::MoreDays(n) = fix {
            assert!((1..=4).contains(&n));
            let mut more = s.clone();
            let free: Vec<usize> = (0..7).filter(|&w| !s.weekdays[w]).collect();
            for w in &free[..n as usize] {
                more.weekdays[*w] = true;
            }
            assert_ne!(
                plan(&more, PlanInput { today, lectures: &lectures, review_forecast: &[] }).status,
                p.status
            );
        }
    }

    #[test]
    fn suggests_a_higher_cap_only_when_it_fits() {
        let today = day(2026, 10, 27);
        let mut s = settings(day(2026, 12, 15));
        s.weekdays = [true; 7];
        s.review_cap = 60;
        let input =
            |l: &[LectureProgress]| plan(&s, PlanInput { today, lectures: l, review_forecast: &[] }).status;
        let some = [lecture("hb.L01", 600, 0)];
        let PlanStatus::Tight { fix: TightFix::HigherCap(cap), .. } = input(&some) else {
            panic!("{:?}", input(&some))
        };
        assert!(cap > 60 && cap <= MAX_REVIEW_CAP && cap % 50 == 0);
        let mut raised = s.clone();
        raised.review_cap = cap;
        assert!(!matches!(
            plan(&raised, PlanInput { today, lectures: &some, review_forecast: &[] }).status,
            PlanStatus::Tight { .. }
        ));
        let huge = [lecture("hb.L01", 100_000, 0)];
        assert!(matches!(input(&huge), PlanStatus::Tight { fix: TightFix::HighYield, .. }));
    }

    #[test]
    fn far_exam_days_are_clamped() {
        let today = day(2026, 10, 27);
        assert!(exam_day_ok(today, day(2027, 1, 20)));
        assert!(!exam_day_ok(today, today));
        assert!(!exam_day_ok(today, day(2207, 1, 20)));
        let lectures = [lecture("hb.L01", 100, 0)];
        let p =
            plan(&settings(day(9999, 1, 1)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
        assert_eq!(p.days.len(), MAX_DAYS_AHEAD as usize);
    }

    #[test]
    fn forecast_reviews_count_against_the_cap() {
        let today = day(2026, 10, 27);
        let lectures = [lecture("hb.L01", 300, 0)];
        let s = settings(day(2027, 1, 20));
        let quiet = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
        let busy_forecast = vec![140; 30];
        let busy = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &busy_forecast });
        let first_new = |p: &Plan| p.days.iter().position(|d| d.new > 0).unwrap();
        assert!(first_new(&busy) >= first_new(&quiet));
        for d in &busy.days {
            assert!(d.reviews <= 150 || d.new == 0, "{d:?}");
        }
    }

    #[test]
    fn rest_day_reviews_roll_to_the_next_study_day() {
        let friday = day(2026, 10, 30);
        let s = settings(day(2027, 1, 20));
        let p = plan(&s, PlanInput { today: friday, lectures: &[], review_forecast: &[20, 5] });
        assert_eq!(p.days[0].focus, Focus::Rest);
        assert_eq!(p.days[1].reviews, 25);
        assert_eq!(p.next_study_day().map(|d| d.day), Some(friday.plus(1)));
    }

    #[test]
    fn inside_the_final_window_only_mocks_and_weak_topics() {
        let exam = day(2027, 1, 20);
        let today = exam.plus(-6);
        let lectures = [lecture("hb.L01", 40, 10)];
        let p = plan(&settings(exam), PlanInput { today, lectures: &lectures, review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::Tight { left_over: 30, fix: TightFix::FinalWindow });
        assert!(p.days.iter().all(|d| matches!(d.focus, Focus::Mock | Focus::WeakTopics | Focus::Rest)));
        let done = [lecture("hb.L01", 40, 40)];
        let p = plan(&settings(exam), PlanInput { today, lectures: &done, review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::FinalWindow);
    }

    #[test]
    fn everything_seen_means_reviews_only() {
        let today = day(2026, 10, 27);
        let lectures = [LectureProgress { id: "hb.L01".into(), total: 40, seen: 40, seen_twice: 40 }];
        let p =
            plan(&settings(day(2027, 1, 20)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
        assert_eq!(p.status, PlanStatus::OnTrack);
        assert!(p.days.iter().all(|d| d.new == 0));
        assert_eq!(p.seen_twice, 40);
    }
}
