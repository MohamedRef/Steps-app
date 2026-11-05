//! A lesson attempt part-way through, as saved on the device: where he
//! stands, the first result of every step, and the response that made it
//! (the option he picked, the order he built, the pairs he matched...).
//!
//! Each step moves through one state machine:
//! Unanswered → Draft → Checked (feedback) → Continue. Reopening the lesson,
//! switching between steps and pages, or restarting the app restores the
//! response before Check or Continue is decided, so a saved result is never
//! shown next to an empty, editable task.
//!
//! Responses name content by its authored index (option, step, pair, card,
//! bin), never by where it was drawn on screen, and carry a fingerprint of
//! the step: a response to a step that has since been edited is dropped and
//! the step shows its result as "answered before" with the answer key.

use serde::{Deserialize, Serialize};

use crate::lesson::{Lesson, StepBody};

/// The version written today. 0 = before responses were saved (only
/// `answers`, `skipped` and `tries`).
pub const VERSION: u32 = 2;

/// A lesson attempt part-way through (the synced setting `lessonpos.<id>`, written through the app's `Store::put_place`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    #[serde(default)]
    pub v: u32,
    /// When this attempt began: tells a retake of a finished lesson apart
    /// from a leftover of the attempt that finished it.
    #[serde(default)]
    pub started_ms: i64,
    pub at: usize,
    /// First result per step: Some(right). Reveals and Try steps record too
    /// (they gate Continue), but only question steps are scored.
    pub answers: Vec<Option<bool>>,
    pub skipped: Vec<bool>,
    /// Try steps part-way through: the cards already graded, so leaving and
    /// coming back never grades a card twice (#318).
    #[serde(default)]
    pub tries: Vec<TryLog>,
    /// The response per step (empty in records saved before v2).
    #[serde(default)]
    pub drafts: Vec<Saved>,
}

/// A Try step part-way through: the cards graded so far and how many were
/// "Again".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TryLog {
    pub graded: Vec<String>,
    pub again: u32,
}

/// One step's response and the fingerprint of the step it answers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sig: String,
    #[serde(default)]
    pub draft: Draft,
}

/// What he has done on one step. Indexes are authored indexes into the
/// step's content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Draft {
    /// Nothing yet (or a step without a response).
    #[default]
    None,
    /// The option picked; `checked` once Check ran on it.
    Check {
        pick: Option<usize>,
        checked: bool,
    },
    /// The steps placed so far, in his order.
    Order {
        placed: Vec<usize>,
        checked: bool,
    },
    /// The pairs matched (by pair index) and the wrong taps on the way.
    Match {
        done: Vec<usize>,
        misses: u32,
    },
    /// The bin each card is in, per card.
    Sort {
        put: Vec<Option<usize>>,
        checked: bool,
    },
    /// A mini case: the checked pick per part (a part without options is
    /// `Some(0)` once shown), the part he is on and its unchecked pick.
    /// `first` keeps each part's first checked pick: re-solving a part
    /// ("حاول تاني") changes `picks`, never `first`, so the score stays the
    /// first attempt's. Empty in drafts saved before it existed.
    Case {
        picks: Vec<Option<usize>>,
        at: usize,
        pick: Option<usize>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        first: Vec<Option<usize>>,
    },
    Reveal {
        open: bool,
    },
    /// Mnemonic letters opened.
    Mnemonic {
        shown: Vec<bool>,
    },
    /// Table cells opened, `[row, column]`.
    Table {
        open: Vec<[usize; 2]>,
    },
    /// A kind written by a newer version of the app.
    #[serde(other)]
    Unknown,
}

/// Where a step stands, derived from its result and its response. The
/// player shows each step from this and gates Continue on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    /// Nothing to answer (explain, figure, aside...).
    Plain,
    /// Waiting for a response; Continue stays off (Skip is offered).
    Open,
    /// A response in progress, not checked yet.
    Draft,
    /// Checked: feedback shows, the response is read-only.
    Done,
    /// A result from before responses were saved (or from an edited step):
    /// shown as "answered before" with the answer key, never as an empty task.
    AnsweredBefore,
    /// Skipped without an answer: he can still answer it.
    Skipped,
}

impl StepState {
    pub fn can_continue(self) -> bool {
        matches!(self, Self::Plain | Self::Done | Self::AnsweredBefore | Self::Skipped)
    }
}

/// FNV-1a over the step's content: stable across platforms and versions.
pub fn sig(body: &StepBody) -> String {
    if !has_draft(body) {
        return String::new();
    }
    let text = serde_json::to_string(body).unwrap_or_default();
    let h =
        text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    format!("{h:016x}")
}

/// Kinds that keep a response.
pub fn has_draft(body: &StepBody) -> bool {
    !matches!(
        body,
        StepBody::Explain { .. }
            | StepBody::Figure { .. }
            | StepBody::Aside { .. }
            | StepBody::Update { .. }
            | StepBody::Try { .. }
    )
}

/// Steps that gate Continue: questions, reveals and Try steps.
pub fn gates(body: &StepBody) -> bool {
    body.is_question() || matches!(body, StepBody::Reveal { .. } | StepBody::Try { .. })
}

impl Draft {
    /// Whether the response fits the step: the right kind, and every index
    /// inside its content.
    pub fn fits(&self, body: &StepBody) -> bool {
        let unique = |v: &[usize]| {
            let mut s = v.to_vec();
            s.sort_unstable();
            s.windows(2).all(|w| w[0] != w[1])
        };
        match (self, body) {
            (Self::None, _) => true,
            (Self::Check { pick, .. }, StepBody::Check { options, .. }) => {
                pick.is_none_or(|p| p < options.len())
            }
            (Self::Order { placed, .. }, StepBody::Order { steps, .. }) => {
                placed.len() <= steps.len() && placed.iter().all(|s| *s < steps.len()) && unique(placed)
            }
            (Self::Match { done, .. }, StepBody::Match { pairs, .. }) => {
                done.iter().all(|p| *p < pairs.len()) && unique(done)
            }
            (Self::Sort { put, .. }, StepBody::Sort { bins, cards, .. }) => {
                put.len() == cards.len() && put.iter().flatten().all(|b| *b < bins.len())
            }
            (Self::Case { picks, at, pick, first }, StepBody::Case { steps, .. }) => {
                (first.is_empty()
                    || first.len() == steps.len()
                        && first.iter().zip(steps).all(|(p, s)| p.is_none_or(|p| p < s.options.len().max(1))))
                    && picks.len() == steps.len()
                    && *at < steps.len()
                    && picks.iter().zip(steps).all(|(p, s)| p.is_none_or(|p| p < s.options.len().max(1)))
                    && pick.is_none_or(|p| p < steps[*at].options.len())
            }
            (Self::Reveal { .. }, StepBody::Reveal { .. }) => true,
            (Self::Mnemonic { shown }, StepBody::Mnemonic { parts, .. }) => shown.len() == parts.len(),
            (Self::Table { open }, StepBody::Table { rows, .. }) => {
                open.iter().all(|[r, c]| rows.get(*r).is_some_and(|row| *c < row.len()))
            }
            _ => false,
        }
    }

    /// The graded result of a checked response, or None while it is still
    /// a draft (or has nothing to grade).
    pub fn result(&self, body: &StepBody) -> Option<bool> {
        match (self, body) {
            (Self::Check { pick: Some(p), checked: true }, StepBody::Check { answer, .. }) => {
                Some(p == answer)
            }
            (Self::Order { placed, checked: true }, StepBody::Order { steps, .. })
                if placed.len() == steps.len() =>
            {
                Some(placed.iter().enumerate().all(|(pos, s)| pos == *s))
            }
            (Self::Match { done, misses }, StepBody::Match { pairs, .. }) if done.len() == pairs.len() => {
                Some(*misses == 0)
            }
            (Self::Sort { put, checked: true }, StepBody::Sort { cards, .. })
                if put.iter().all(Option::is_some) =>
            {
                Some(put.iter().zip(cards).all(|(p, c)| *p == Some(c.bin)))
            }
            (Self::Case { picks, first, .. }, StepBody::Case { steps, .. })
                if picks.iter().all(Option::is_some) =>
            {
                // The first checked pick per part is the one that counts.
                let counted = |i: usize| first.get(i).copied().flatten().or(picks[i]);
                Some(steps.iter().enumerate().all(|(i, s)| s.options.is_empty() || counted(i) == s.answer))
            }
            (Self::Reveal { open: true }, StepBody::Reveal { .. }) => Some(true),
            _ => None,
        }
    }

    /// Something entered but not checked yet.
    pub fn in_progress(&self) -> bool {
        match self {
            Self::Check { pick, checked } => pick.is_some() && !checked,
            Self::Order { placed, checked } => !placed.is_empty() && !checked,
            Self::Match { done, misses } => !done.is_empty() || *misses > 0,
            Self::Sort { put, checked } => put.iter().any(Option::is_some) && !checked,
            Self::Case { picks, pick, .. } => pick.is_some() || picks.iter().any(Option::is_some),
            _ => false,
        }
    }
}

/// Where one step stands.
pub fn state(body: &StepBody, answer: Option<bool>, skipped: bool, draft: Option<&Draft>) -> StepState {
    if !gates(body) {
        return StepState::Plain;
    }
    let checked = draft.is_some_and(|d| d.result(body).is_some());
    match (answer, draft) {
        _ if checked => StepState::Done,
        // Try steps keep their own log (`tries`): a result means done.
        (Some(_), _) if matches!(body, StepBody::Try { .. }) => StepState::Done,
        (Some(_), _) => StepState::AnsweredBefore,
        (None, Some(d)) if d.in_progress() => StepState::Draft,
        (None, _) if skipped => StepState::Skipped,
        (None, _) => StepState::Open,
    }
}

/// Whether a step's book quote waits until he has answered: a full quote
/// is often the very table or list the step asks for (Mohamed, 2026-11-02).
/// A question or reveal holds it until checked; a table with hidden cells,
/// until every cell is open.
pub fn quote_held(body: &StepBody, state: StepState, draft: Option<&Draft>) -> bool {
    if let StepBody::Table { hide, .. } = body {
        let open: &[[usize; 2]] = match draft {
            Some(Draft::Table { open }) => open,
            _ => &[],
        };
        return hide.iter().any(|c| !open.contains(c));
    }
    matches!(state, StepState::Open | StepState::Draft | StepState::Skipped)
}

impl Attempt {
    /// [`quote_held`] for step `k`.
    pub fn quote_held(&self, lesson: &Lesson, k: usize) -> bool {
        let draft = self.drafts.get(k).map(|s| &s.draft);
        quote_held(&lesson.steps[k].body, self.state(lesson, k), draft)
    }

    pub fn new(n: usize, started_ms: i64) -> Self {
        Self {
            v: VERSION,
            started_ms,
            at: 0,
            answers: vec![None; n],
            skipped: vec![false; n],
            tries: vec![TryLog::default(); n],
            drafts: vec![Saved::default(); n],
        }
    }

    /// The saved attempt made whole for `lesson`: or a fresh one when there
    /// is none, it doesn't fit the lesson any more, or the lesson was
    /// finished after it began (a leftover, not a retake).
    pub fn restore(lesson: &Lesson, saved: Option<Self>, finished_ms: Option<i64>, now_ms: i64) -> Self {
        let n = lesson.steps.len();
        let Some(mut a) = saved else { return Self::new(n, now_ms) };
        let fits = a.at < n && a.answers.len() == n && a.skipped.len() == n;
        let leftover = finished_ms.is_some_and(|done| a.started_ms <= done);
        if !fits || leftover {
            return Self::new(n, now_ms);
        }
        a.v = VERSION;
        a.tries.resize(n, TryLog::default());
        a.drafts.resize(n, Saved::default());
        for (k, step) in lesson.steps.iter().enumerate() {
            let s = &mut a.drafts[k];
            if s.draft != Draft::None && (s.sig != sig(&step.body) || !s.draft.fits(&step.body)) {
                *s = Saved::default();
            }
            // A reveal with a result was opened.
            if matches!(step.body, StepBody::Reveal { .. }) && a.answers[k].is_some() {
                *s = Saved { sig: sig(&step.body), draft: Draft::Reveal { open: true } };
            }
            // A response checked just before the app closed, its result not
            // yet written: the response is the record.
            if a.answers[k].is_none()
                && let Some(r) = s.draft.result(&step.body)
            {
                a.answers[k] = Some(r);
                a.skipped[k] = false;
            }
        }
        a
    }

    pub fn state(&self, lesson: &Lesson, k: usize) -> StepState {
        let draft = self.drafts.get(k).map(|s| &s.draft);
        state(&lesson.steps[k].body, self.answers[k], self.skipped[k], draft)
    }

    /// Keeps a step's response with the step's fingerprint ([`sig`]).
    pub fn put(&mut self, k: usize, sig: String, draft: Draft) {
        if let Some(s) = self.drafts.get_mut(k) {
            *s = Saved { sig, draft };
        }
    }

    /// Records a step's first result. Later results never replace it, so
    /// coming back to a step never grades it twice. Returns whether it counted.
    pub fn answer(&mut self, k: usize, right: bool) -> bool {
        match self.answers.get_mut(k) {
            Some(slot @ None) => {
                *slot = Some(right);
                // Answered after a skip: it counts.
                self.skipped[k] = false;
                true
            }
            _ => false,
        }
    }

    /// `(right, asked)` over the question steps he answered: reveals and Try
    /// steps don't score, and skips don't count against him.
    pub fn score(&self, lesson: &Lesson) -> (u32, u32) {
        let counted = || {
            self.answers
                .iter()
                .zip(&self.skipped)
                .zip(&lesson.steps)
                .filter(|((_, s), st)| st.body.is_question() && !**s)
                .filter_map(|((a, _), _)| *a)
        };
        (counted().filter(|r| *r).count() as u32, counted().count() as u32)
    }
}

#[cfg(test)]
mod tests;
