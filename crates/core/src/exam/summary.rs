//! The end of a paper on one screen (bank/result.rs): the score, the time used
//! of the time allowed and per question, how many were right, wrong, left
//! blank and marked, the score per course (system or discipline), and the
//! questions a new paper from this one takes.

use std::collections::BTreeMap;

use super::{Answer, Mode, Question, Score, time_limit_ms};
use crate::program::course_of;

/// How one question went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graded {
    Right,
    Wrong,
    /// Never answered (a timed paper's blank, a tutor question never checked).
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Right of all; a blank counts as wrong, as in [`super::score`].
    pub total: Score,
    /// Answered and wrong.
    pub wrong: u32,
    pub blank: u32,
    pub marked: u32,
    /// The paper's time: from start to the end, at most the time allowed.
    pub used_ms: u64,
    /// A timed paper's time; none for a tutor paper, which has no clock.
    pub allowed_ms: Option<u64>,
    /// The time on an answered question, on average: the answered rows of
    /// the time column. A question left blank (never opened, or opened and
    /// skipped) is not an answer's time. None when nothing was answered.
    pub per_question_ms: Option<u64>,
    /// Course id (`cvs`, `pharm`) → score.
    pub by_course: BTreeMap<String, Score>,
}

/// The numbers of a finished paper that took `took_ms` from start to end.
pub fn summary(questions: &[Question], answers: &[Answer], mode: Mode, took_ms: u64) -> Summary {
    let allowed_ms = mode.timed().then(|| time_limit_ms(questions.len()));
    let mut out = Summary {
        total: Score::default(),
        wrong: 0,
        blank: 0,
        marked: 0,
        used_ms: allowed_ms.map_or(took_ms, |limit| took_ms.min(limit)),
        allowed_ms,
        per_question_ms: None,
        by_course: BTreeMap::new(),
    };
    let blank = Answer::default();
    let (mut times, mut answered) = (0u64, 0u64);
    for (i, q) in questions.iter().enumerate() {
        let a = answers.get(i).unwrap_or(&blank);
        let graded = a.graded(q);
        out.total.add(graded == Graded::Right);
        out.wrong += u32::from(graded == Graded::Wrong);
        out.blank += u32::from(graded == Graded::Blank);
        out.marked += u32::from(a.flagged);
        out.by_course.entry(course_of(&q.lecture).to_string()).or_default().add(graded == Graded::Right);
        if graded != Graded::Blank {
            times += u64::from(a.took_ms);
            answered += 1;
        }
    }
    out.per_question_ms = times.checked_div(answered);
    out
}

/// The questions a new paper from this one takes, in the paper's order:
/// every one not answered right (wrong or blank), and every marked one.
pub fn retake(questions: &[Question], answers: &[Answer]) -> Vec<String> {
    questions
        .iter()
        .zip(answers)
        .filter(|(q, a)| a.graded(q) != Graded::Right || a.flagged)
        .map(|(q, _)| q.id.clone())
        .collect()
}
