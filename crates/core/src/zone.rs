//! Mohamed's time zone, and the one rule for "which day is it" (issues #36
//! and #40). Study days start at 4 am local time (`srs::ROLLOVER_HOUR`), so
//! a session at 1 am still counts for the evening before; calendar dates
//! (file names, the history list, the exam date) start at midnight.
//!
//! Each moment uses the UTC offset that was in force *then*: an answer
//! given in summer time stays on its study day after the clocks go back.
//! The app builds the offset table from the device's time zone (`store::zone`);
//! core stays pure.

use crate::plan::Day;
use crate::srs::ROLLOVER_HOUR;

const DAY_MS: i64 = 86_400_000;

/// UTC offsets over time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone {
    /// Minutes east of UTC before the first change (or always, with none).
    pub base: i32,
    /// `(from_ms, minutes east of UTC)`, sorted by time.
    pub changes: &'static [(i64, i32)],
}

impl Zone {
    pub const fn fixed(minutes: i32) -> Self {
        Self { base: minutes, changes: &[] }
    }

    /// Minutes east of UTC at `ms`.
    pub fn offset_at(&self, ms: i64) -> i32 {
        match self.changes.partition_point(|(from, _)| *from <= ms) {
            0 => self.base,
            i => self.changes[i - 1].1,
        }
    }

    /// Local wall-clock ms at `ms`.
    pub fn local_ms(&self, ms: i64) -> i64 {
        ms + i64::from(self.offset_at(ms)) * 60_000
    }

    /// The day `ms` falls on when days start at `rollover_hour` local time.
    pub fn day_from(&self, ms: i64, rollover_hour: u8) -> i64 {
        (self.local_ms(ms) - i64::from(rollover_hour) * 3_600_000).div_euclid(DAY_MS)
    }

    /// UTC ms at which day `day` starts (at `rollover_hour` local time).
    pub fn day_start_from(&self, day: i64, rollover_hour: u8) -> i64 {
        let local = day * DAY_MS + i64::from(rollover_hour) * 3_600_000;
        // The offset at the start may differ from a guess a few hours off; two rounds settle it.
        let mut at = local - i64::from(self.base) * 60_000;
        for _ in 0..2 {
            at = local - i64::from(self.offset_at(at)) * 60_000;
        }
        at
    }

    /// The study day (from 4 am local) of `ms`.
    pub fn study_day(&self, ms: i64) -> Day {
        Day(self.day_from(ms, ROLLOVER_HOUR) as i32)
    }

    /// The calendar date (from midnight local) of `ms`.
    pub fn calendar_day(&self, ms: i64) -> Day {
        Day(self.day_from(ms, 0) as i32)
    }
}

impl From<i32> for Zone {
    fn from(minutes: i32) -> Self {
        Self::fixed(minutes)
    }
}

impl Default for Zone {
    /// Cairo standard time.
    fn default() -> Self {
        Self::fixed(120)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Egypt: summer time (UTC+3) ends 2026-10-29 at 24:00 local (21:00 UTC).
    const END_OF_SUMMER: i64 = 1_793_307_600_000; // 2026-10-29T21:00:00Z
    static CAIRO: [(i64, i32); 1] = [(END_OF_SUMMER, 120)];
    const ZONE: Zone = Zone { base: 180, changes: &CAIRO };

    #[test]
    fn past_answers_keep_their_day_after_the_clocks_go_back() {
        // 2026-10-20 04:30 summer time = 01:30 UTC: already that morning's study day.
        let morning = 1_792_459_800_000;
        let evening = morning + 15 * 3_600_000;
        assert_eq!(ZONE.study_day(morning), ZONE.study_day(evening));
        // Under a fixed +120 (today's offset) it would slip to the day before.
        assert_ne!(Zone::fixed(120).study_day(morning), Zone::fixed(120).study_day(evening));
    }

    #[test]
    fn offsets_and_day_starts_follow_the_change() {
        assert_eq!(ZONE.offset_at(END_OF_SUMMER - 1), 180);
        assert_eq!(ZONE.offset_at(END_OF_SUMMER), 120);
        for ms in [END_OF_SUMMER - 30 * 3_600_000, END_OF_SUMMER + 30 * 3_600_000, 1_790_000_000_000] {
            let day = ZONE.day_from(ms, 4);
            let start = ZONE.day_start_from(day, 4);
            assert!(start <= ms && ZONE.day_from(start, 4) == day && ZONE.day_from(start - 1, 4) == day - 1);
        }
    }

    #[test]
    fn study_and_calendar_days_differ_only_before_4_am() {
        let z = Zone::fixed(120);
        let one_am = 1_793_574_000_000; // 2026-11-02 01:00 Cairo (23:00 UTC)
        assert_eq!(z.calendar_day(one_am).0, z.study_day(one_am).0 + 1);
        assert_eq!(z.calendar_day(one_am + 4 * 3_600_000), z.study_day(one_am + 4 * 3_600_000));
    }
}
