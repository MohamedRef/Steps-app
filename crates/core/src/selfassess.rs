//! Mohamed's external self-assessments: NBME CBSSA forms, UWorld UWSA, the
//! Free 120 and others he takes outside Steps. He types the score in; Steps
//! keeps the log (the synced `self_assessments` table) and reads it with one
//! plain rule, never an invented pass probability.
//!
//! The rule (from the 2026-11-04 study-method research,
//! `steps-app-changes.md` item 5): the last two NBME forms at or above
//! [`READY_PERCENT`] with a flat or rising trend look ready. This is a
//! community and tutor threshold, not an official cut-off. Only NBME forms
//! count, because UWSA and other banks use their own scales.
//!
//! Where the numbers come from (none of them is First Aid or the USMLE):
//! - 68%: tutor and community readiness lines of about 65% twice to 70%
//!   (Mehlman Medical, https://step1.mehlmanmedical.com/, 2025; Elite Medical
//!   Prep, "NBME 25-30: best predictor for passing Step 1", 2025).
//! - 62-68% "low pass" band: the same sources' borderline zone.
//! - Why CBSSA at all: it explained 67% of Step 1 score variance
//!   (Morrison et al., Acad Med 2010, https://pubmed.ncbi.nlm.nih.gov/20881715).
//! - How to read a score range: NBME CBSSA/CBSE score guidance, Sept 2026,
//!   https://www.nbme.org/wp-content/uploads/2024/10/CBSSA_CBSE_Guidance-f43.pdf

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::plan::Day;

/// Every entry's id starts with this.
pub const PREFIX: &str = "sa.";
/// Two recent NBME forms at or above this look ready.
pub const READY_PERCENT: u8 = 68;
/// The "low pass" zone the chart shades: near the line, not clear of it.
pub const LOW_PASS_BAND: RangeInclusive<u8> = 62..=68;
/// A drop of this many points or fewer still counts as a flat trend.
pub const FLAT_TOLERANCE: u8 = 2;

pub const MAX_FORM: usize = 40;
pub const MAX_NOTE: usize = 300;

/// Which test it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An NBME CBSSA form (the official practice exams).
    Nbme,
    /// A UWorld Self-Assessment: practice, its % is not NBME's.
    Uwsa,
    /// The USMLE's free 120 sample questions.
    Free120,
    /// An AMBOSS self-assessment.
    Amboss,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Nbme, Kind::Uwsa, Kind::Free120, Kind::Amboss, Kind::Other];

    /// Whether the readiness rule reads this kind (NBME forms only).
    pub fn counts_for_readiness(self) -> bool {
        self == Kind::Nbme
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Nbme => "nbme",
            Kind::Uwsa => "uwsa",
            Kind::Free120 => "free120",
            Kind::Amboss => "amboss",
            Kind::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// The test's own name, as it is printed (English, shown in `<bdi>`).
    pub fn label(self) -> &'static str {
        match self {
            Kind::Nbme => "NBME",
            Kind::Uwsa => "UWSA",
            Kind::Free120 => "Free 120",
            Kind::Amboss => "AMBOSS SA",
            Kind::Other => "Other",
        }
    }
}

/// One test he took, as stored (JSON in `self_assessments.value`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfAssessment {
    /// `sa.<random>`.
    pub id: String,
    pub kind: Kind,
    /// The form's name or number ("30", "UWSA 2"), when he noted one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// The day he took it.
    #[serde(with = "iso_day")]
    pub day: Day,
    /// Percent correct (NBME's EPC for a CBSSA).
    pub percent: u8,
    /// The pass probability printed on the NBME report, if he has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_probability: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    PercentTooHigh,
    ProbabilityTooHigh,
    FormTooLong,
    NoteTooLong,
    InTheFuture,
}

impl SelfAssessment {
    /// Trims the text fields and drops empty ones.
    pub fn tidy(mut self) -> Self {
        let clean = |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.form = clean(self.form);
        self.note = clean(self.note);
        self
    }

    /// The first problem that stops it being saved, given today's date.
    pub fn problem(&self, today: Day) -> Option<Problem> {
        if self.percent > 100 {
            Some(Problem::PercentTooHigh)
        } else if self.pass_probability.is_some_and(|p| p > 100) {
            Some(Problem::ProbabilityTooHigh)
        } else if self.form.as_ref().is_some_and(|f| f.chars().count() > MAX_FORM) {
            Some(Problem::FormTooLong)
        } else if self.note.as_ref().is_some_and(|n| n.chars().count() > MAX_NOTE) {
            Some(Problem::NoteTooLong)
        } else if self.day > today {
            Some(Problem::InTheFuture)
        } else {
            None
        }
    }

    /// "NBME 30", "UWSA 2", "Free 120".
    pub fn title(&self) -> String {
        match &self.form {
            Some(f) if self.kind != Kind::Other && !f.starts_with(self.kind.label()) => {
                format!("{} {f}", self.kind.label())
            }
            Some(f) => f.clone(),
            None => self.kind.label().to_string(),
        }
    }
}

pub fn is_self_assessment(id: &str) -> bool {
    id.starts_with(PREFIX)
}

/// Oldest first, by day and then by when it was entered.
pub fn sorted(entries: &[SelfAssessment]) -> Vec<SelfAssessment> {
    let mut v = entries.to_vec();
    v.sort_by(|a, b| a.day.cmp(&b.day).then(a.created_ms.cmp(&b.created_ms)).then_with(|| a.id.cmp(&b.id)));
    v
}

/// The scores the readiness rule reads: NBME forms, as `(day, percent)`.
pub fn readiness_points(entries: &[SelfAssessment]) -> Vec<(Day, u8)> {
    sorted(entries).iter().filter(|e| e.kind.counts_for_readiness()).map(|e| (e.day, e.percent)).collect()
}

/// What the last NBME forms say. Never a probability: the words only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// No NBME form logged yet.
    NoScores,
    /// One form: a trend needs a second.
    OneScore,
    /// The last two at or above [`READY_PERCENT`], flat or rising.
    Ready,
    /// The last two at least in the low-pass band, or high but falling.
    Close,
    /// Below the band: moving the date is a plan, not a failure.
    NotYet,
}

/// Reads NBME-scale `(day, percent)` points in any order (stable for
/// points on the same day).
pub fn readiness(points: &[(Day, u8)]) -> Readiness {
    let mut p = points.to_vec();
    p.sort_by_key(|(day, _)| *day);
    let [.., (_, prev), (_, last)] = p.as_slice() else {
        return if p.is_empty() { Readiness::NoScores } else { Readiness::OneScore };
    };
    let (prev, last) = (*prev, *last);
    let flat_or_rising = last + FLAT_TOLERANCE >= prev;
    if prev >= READY_PERCENT && last >= READY_PERCENT && flat_or_rising {
        Readiness::Ready
    } else if prev >= *LOW_PASS_BAND.start() && last >= *LOW_PASS_BAND.start() {
        Readiness::Close
    } else {
        Readiness::NotYet
    }
}

/// The change from the second-last to the last point, in points.
pub fn trend(points: &[(Day, u8)]) -> Option<i16> {
    let mut p = points.to_vec();
    p.sort_by_key(|(day, _)| *day);
    match p.as_slice() {
        [.., (_, prev), (_, last)] => Some(i16::from(*last) - i16::from(*prev)),
        _ => None,
    }
}

/// Stores a [`Day`] as `YYYY-MM-DD`, so exports read plainly.
mod iso_day {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    use crate::plan::Day;

    pub fn serialize<S: Serializer>(day: &Day, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&day.iso())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Day, D::Error> {
        let text = String::deserialize(d)?;
        Day::parse(&text).ok_or_else(|| D::Error::custom(format!("not a date: {text}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> Day {
        Day::parse(s).unwrap()
    }

    fn entry(kind: Kind, on: &str, percent: u8) -> SelfAssessment {
        SelfAssessment {
            id: format!("sa.{on}"),
            kind,
            form: None,
            day: day(on),
            percent,
            pass_probability: None,
            note: None,
            created_ms: 0,
        }
    }

    fn pts(v: &[(&str, u8)]) -> Vec<(Day, u8)> {
        v.iter().map(|(d, p)| (day(d), *p)).collect()
    }

    #[test]
    fn readiness_follows_the_rule() {
        assert_eq!(readiness(&[]), Readiness::NoScores);
        assert_eq!(readiness(&pts(&[("2027-06-01", 75)])), Readiness::OneScore);
        // Two at or above 68, rising or flat.
        assert_eq!(readiness(&pts(&[("2027-06-01", 68), ("2027-06-08", 70)])), Readiness::Ready);
        assert_eq!(readiness(&pts(&[("2027-06-01", 72), ("2027-06-08", 70)])), Readiness::Ready);
        // High but falling more than the tolerance.
        assert_eq!(readiness(&pts(&[("2027-06-01", 78), ("2027-06-08", 70)])), Readiness::Close);
        // In the low-pass band.
        assert_eq!(readiness(&pts(&[("2027-06-01", 62), ("2027-06-08", 66)])), Readiness::Close);
        assert_eq!(readiness(&pts(&[("2027-06-01", 70), ("2027-06-08", 64)])), Readiness::Close);
        // Below it.
        assert_eq!(readiness(&pts(&[("2027-06-01", 58), ("2027-06-08", 69)])), Readiness::NotYet);
        assert_eq!(readiness(&pts(&[("2027-06-01", 70), ("2027-06-08", 61)])), Readiness::NotYet);
    }

    #[test]
    fn only_the_last_two_count_and_order_is_by_day() {
        // Out of order: the June 8 score is the last one.
        let p = pts(&[("2027-06-08", 71), ("2027-01-01", 40), ("2027-06-01", 69)]);
        assert_eq!(readiness(&p), Readiness::Ready);
        assert_eq!(trend(&p), Some(2));
        assert_eq!(trend(&p[..1]), None);
    }

    #[test]
    fn only_nbme_forms_feed_the_rule() {
        let log = vec![
            entry(Kind::Nbme, "2027-06-01", 69),
            entry(Kind::Uwsa, "2027-06-03", 50),
            entry(Kind::Free120, "2027-06-05", 55),
            entry(Kind::Nbme, "2027-06-08", 70),
        ];
        let points = readiness_points(&log);
        assert_eq!(points, pts(&[("2027-06-01", 69), ("2027-06-08", 70)]));
        assert_eq!(readiness(&points), Readiness::Ready);
        assert!(Kind::ALL.iter().filter(|k| k.counts_for_readiness()).eq([&Kind::Nbme]));
    }

    #[test]
    fn kinds_round_trip_by_name() {
        for k in Kind::ALL {
            assert_eq!(Kind::parse(k.as_str()), Some(k));
            assert_eq!(serde_json::to_value(k).unwrap(), k.as_str());
        }
        assert_eq!(Kind::parse("nope"), None);
    }

    #[test]
    fn stored_as_plain_json_with_an_iso_day() {
        let mut e = entry(Kind::Nbme, "2027-06-01", 69);
        e.form = Some("30".into());
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["day"], "2027-06-01");
        assert_eq!(json["kind"], "nbme");
        assert!(json.get("note").is_none());
        let back: SelfAssessment = serde_json::from_value(json).unwrap();
        assert_eq!(back, e);
        let bad = serde_json::json!({"id": "sa.x", "kind": "nbme", "day": "2027-02-30", "percent": 5, "created_ms": 0});
        assert!(serde_json::from_value::<SelfAssessment>(bad).is_err());
    }

    #[test]
    fn problems_and_tidy() {
        let today = day("2027-06-10");
        let ok = entry(Kind::Nbme, "2027-06-10", 100);
        assert_eq!(ok.problem(today), None);
        assert_eq!(
            SelfAssessment { percent: 101, ..ok.clone() }.problem(today),
            Some(Problem::PercentTooHigh)
        );
        assert_eq!(
            SelfAssessment { pass_probability: Some(101), ..ok.clone() }.problem(today),
            Some(Problem::ProbabilityTooHigh)
        );
        assert_eq!(
            SelfAssessment { form: Some("x".repeat(MAX_FORM + 1)), ..ok.clone() }.problem(today),
            Some(Problem::FormTooLong)
        );
        assert_eq!(
            SelfAssessment { note: Some("x".repeat(MAX_NOTE + 1)), ..ok.clone() }.problem(today),
            Some(Problem::NoteTooLong)
        );
        assert_eq!(entry(Kind::Nbme, "2027-06-11", 60).problem(today), Some(Problem::InTheFuture));
        let t = SelfAssessment { form: Some("  ".into()), note: Some(" hi ".into()), ..ok }.tidy();
        assert_eq!((t.form, t.note.as_deref()), (None, Some("hi")));
    }

    #[test]
    fn titles() {
        let mut e = entry(Kind::Nbme, "2027-06-01", 69);
        assert_eq!(e.title(), "NBME");
        e.form = Some("30".into());
        assert_eq!(e.title(), "NBME 30");
        e.kind = Kind::Uwsa;
        e.form = Some("UWSA 2".into());
        assert_eq!(e.title(), "UWSA 2");
        e.kind = Kind::Other;
        e.form = Some("Kaplan diag".into());
        assert_eq!(e.title(), "Kaplan diag");
        assert!(is_self_assessment("sa.1") && !is_self_assessment("own.1"));
    }

    #[test]
    fn sorted_by_day_then_entry_time() {
        let mut a = entry(Kind::Nbme, "2027-06-01", 60);
        a.created_ms = 5;
        let mut b = entry(Kind::Uwsa, "2027-06-01", 61);
        b.id = "sa.b".into();
        b.created_ms = 1;
        let c = entry(Kind::Nbme, "2027-05-01", 50);
        let s = sorted(&[a.clone(), b.clone(), c.clone()]);
        assert_eq!(s, vec![c, b, a]);
    }
}
