//! Answer cues in the written MCQs and in the lessons' check questions:
//! hints that point at the key without any medicine. The key that is the
//! single longest option, distractors padded with a final "only" or "alone",
//! and where the key was written. `steps-tools validate` prints these per
//! lecture (MCQs) and per course (lesson checks) so they cannot drift back.

use crate::content::{Body, Lecture};
use crate::lesson::{LessonFile, StepBody};

/// The bank-wide limit on MCQs whose key is the single longest option. With
/// five options chance is about 20%.
pub const KEY_LONGEST_LIMIT: f64 = 0.25;

/// Words that, at the end of a distractor, mark it as padded to even out
/// lengths ("Prednisone alone"). They are never the key, so they give it away.
const PAD_WORDS: [&str; 2] = ["only", "alone"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cues {
    /// MCQs counted.
    pub mcqs: usize,
    /// MCQs whose key is strictly longer than every other option.
    pub key_longest: usize,
    /// Options, not the key, ending in a padding word: (item id, option index).
    pub padded: Vec<(String, usize)>,
    /// How many keys were written at A, B, C, D, E.
    pub key_at: [usize; 5],
}

impl Cues {
    pub fn add(&mut self, other: &Cues) {
        self.mcqs += other.mcqs;
        self.key_longest += other.key_longest;
        self.padded.extend(other.padded.iter().cloned());
        for (a, b) in self.key_at.iter_mut().zip(other.key_at) {
            *a += b;
        }
    }

    /// The share of MCQs whose key is the single longest option, 0–1.
    pub fn key_longest_share(&self) -> f64 {
        if self.mcqs == 0 { 0.0 } else { self.key_longest as f64 / self.mcqs as f64 }
    }
}

impl Cues {
    /// Counts one question: its options, the key's index, and the id a
    /// padded option is reported under.
    fn count(&mut self, id: &str, options: &[String], answer: usize) {
        let Some(key) = options.get(answer) else {
            return;
        };
        self.mcqs += 1;
        if let Some(n) = self.key_at.get_mut(answer) {
            *n += 1;
        }
        let len = |s: &str| s.trim().chars().count();
        if options.len() > 1 && options.iter().enumerate().all(|(i, o)| i == answer || len(o) < len(key)) {
            self.key_longest += 1;
        }
        for (i, o) in options.iter().enumerate() {
            if i != answer && padded(o) {
                self.padded.push((id.to_string(), i));
            }
        }
    }
}

/// The cues in one lecture's MCQs.
pub fn lecture(lec: &Lecture) -> Cues {
    let mut out = Cues::default();
    for item in &lec.items {
        if let Body::Mcq { options, answer, .. } = &item.body {
            out.count(&item.id, options, *answer);
        }
    }
    out
}

/// The cues in one lesson file's `check` steps (issue #746). A padded
/// option is reported as `<lesson id> step <n>`.
pub fn lessons(file: &LessonFile) -> Cues {
    let mut out = Cues::default();
    for lesson in &file.lessons {
        for (k, step) in lesson.steps.iter().enumerate() {
            if let StepBody::Check { options, answer, .. } = &step.body {
                out.count(&format!("{} step {}", lesson.id, k + 1), options, *answer);
            }
        }
    }
    out
}

/// "Prednisone alone", "Stratum corneum only".
pub fn padded(option: &str) -> bool {
    let last =
        option.trim().trim_end_matches(|c: char| c.is_ascii_punctuation()).rsplit(' ').next().unwrap_or("");
    option.trim().contains(' ') && PAD_WORDS.iter().any(|w| last.eq_ignore_ascii_case(w))
}

#[cfg(test)]
#[path = "cues_tests.rs"]
mod tests;
