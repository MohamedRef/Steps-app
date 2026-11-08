//! Lessons, `steps.lesson/v1`: the teaching layer on top of the study items.
//!
//! One YAML file per lecture (`content/lessons/cvs/L01.yaml`) holds a few
//! short lessons. A lesson is a list of steps: explanations in Egyptian
//! Arabic (medical terms stay English), drawn SVG figures, "why?" reveals,
//! mnemonics, asides (a tip, an exam trap, a joke), and small interactive
//! checks between them (choose, order, match, sort, a mini case, a table
//! with hidden cells). A `try` step and the lesson's `practice` list point at
//! the lecture's existing items, so a lesson hands over to spaced review.
//!
//! The rules for writing lessons are in `docs/LESSONS.md`; [`validate`]
//! enforces them.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::content::{CaseStep, Lecture, Status};
use crate::validate::Problem;

pub const SCHEMA: &str = "steps.lesson/v1";

/// The lessons of one lecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonFile {
    pub schema: String,
    /// `cvs.L04`, the lecture these lessons teach.
    pub lecture: String,
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    /// `cvs.L04.lesson1`. Never reused or renumbered.
    pub id: String,
    /// Egyptian Arabic.
    pub title: String,
    /// One line that makes him want to open it.
    pub hook: String,
    /// About how long it takes, in minutes.
    pub minutes: u32,
    /// Printed book pages it covers.
    pub pages: [u32; 2],
    /// Lecture topic ids it covers (for Today and the planner).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// "After this lesson you can…", in Arabic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<String>,
    pub steps: Vec<Step>,
    /// The take-aways shown at the end.
    pub recap: Vec<String>,
    /// Item ids of this lecture to practise afterwards (spaced review).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub practice: Vec<String>,
    pub status: Status,
}

/// One screen of a lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    #[serde(flatten)]
    pub body: StepBody,
    /// The printed book page this step teaches from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// The page's own words: as much as proves the step and gives the full
    /// reference (no length cap, Mohamed 2026-10-29).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    /// Memory hooks this step's topic already has elsewhere, shown under the
    /// step as a reminder (with a way back to the lesson that taught them):
    /// an enumerate item id (`cvs.L04.033`, its تحشيشة) or a mnemonic step
    /// as `<lesson id>:<WORD>` (`cvs.L04.lesson2:MINT`). See [`Reminder`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remind: Vec<String>,
}

/// One entry of a step's `remind` list.
// Adapted from batna-app main: crates/core/src/lesson/mod.rs (Reminder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reminder<'a> {
    /// An enumerate item's `hook`.
    Item(&'a str),
    /// A mnemonic step, by its lesson and its `word`.
    Mnemonic { lesson: &'a str, word: &'a str },
}

impl<'a> Reminder<'a> {
    /// `cvs.L04.033` or `cvs.L04.lesson2:MINT`; `None` when it is neither.
    pub fn parse(s: &'a str) -> Option<Self> {
        let s = s.trim();
        let id_like =
            |x: &str| !x.is_empty() && !x.contains(char::is_whitespace) && x.split('.').count() == 3;
        match s.split_once(':') {
            // A mnemonic's word may have spaces ("ABCD OF C").
            Some((lesson, word))
                if id_like(lesson) && lesson.contains(".lesson") && !word.trim().is_empty() =>
            {
                Some(Self::Mnemonic { lesson, word })
            }
            Some(_) => None,
            None if id_like(s) && !s.contains(".lesson") => Some(Self::Item(s)),
            None => None,
        }
    }
}

/// A picture in a figure step: an image file under `content/lessons/img/`.
/// Either an open-licence image (with `credit`, `license` and `source`), or
/// a figure cropped from the textbook (`book: true`; the step's `page`
/// cites it). Mohamed allowed book figures in the repo on 2026-10-28; the
/// book's text still never goes in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picture {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub book: bool,
    /// The file as a `data:` URL, filled in when the pack is built
    /// ([`embed_images`]); never written in YAML.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepBody {
    /// Prose in [`spans`] markup: paragraphs, `- ` bullets, `**bold**`.
    Explain {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        text: String,
    },
    /// A diagram drawn as inline SVG with theme colours only, or a picture
    /// ([`Picture`]): an open-licence image, or a figure from the book.
    Figure {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        svg: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<Picture>,
        caption: String,
        /// What the figure shows, for a screen reader.
        alt: String,
    },
    /// A question to think about first; the answer opens on a tap.
    Reveal { prompt: String, answer: String },
    /// Letters to remember a list by. Each letter opens its line on a tap.
    Mnemonic {
        /// The word or phrase the letters make (`"VAMPIRE"`).
        word: String,
        parts: Vec<MnemonicPart>,
        /// Why it sticks, in Arabic (the story, the picture, the joke).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hook: Option<String>,
    },
    /// A side note in a coloured box.
    Aside { tone: Tone, text: String },
    /// Where a current guideline differs from the book. The book stays the answer.
    Update {
        text: String,
        #[serde(rename = "ref")]
        reference: String,
    },
    /// Single best answer, feedback on every option.
    Check { question: String, options: Vec<String>, answer: usize, why: Vec<String> },
    /// Put the steps in order (written in the correct order).
    Order {
        prompt: String,
        steps: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// Match each left side to its right side (written matched).
    Match {
        prompt: String,
        pairs: Vec<[String; 2]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// Drop each card into its bin.
    Sort {
        prompt: String,
        bins: Vec<String>,
        cards: Vec<SortCard>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// A mini case: each step reveals data and asks one decision.
    Case { stem: String, steps: Vec<CaseStep> },
    /// A table; `hide` cells (`[row, column]`) open on a tap.
    Table {
        title: String,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hide: Vec<[usize; 2]>,
    },
    /// Practise these items of the lecture right here, graded into review.
    Try { items: Vec<String> },
}

impl StepBody {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Explain { .. } => "explain",
            Self::Figure { .. } => "figure",
            Self::Reveal { .. } => "reveal",
            Self::Mnemonic { .. } => "mnemonic",
            Self::Aside { .. } => "aside",
            Self::Update { .. } => "update",
            Self::Check { .. } => "check",
            Self::Order { .. } => "order",
            Self::Match { .. } => "match",
            Self::Sort { .. } => "sort",
            Self::Case { .. } => "case",
            Self::Table { .. } => "table",
            Self::Try { .. } => "try",
        }
    }

    /// Steps that ask something and count towards the lesson score.
    pub fn is_question(&self) -> bool {
        matches!(
            self,
            Self::Check { .. }
                | Self::Order { .. }
                | Self::Match { .. }
                | Self::Sort { .. }
                | Self::Case { .. }
        )
    }
}

impl Step {
    /// Every piece of text the step shows, in reading order: prose,
    /// questions, options, feedback, table cells, and the words drawn in
    /// an SVG figure. Not ids, file names, credits or the page quote. The
    /// "اختصارات" chip looks for abbreviations in these.
    pub fn shown_text(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut add = |s: &str| {
            if !s.trim().is_empty() {
                out.push(s.to_string());
            }
        };
        match &self.body {
            StepBody::Explain { title, text } => {
                title.iter().for_each(|t| add(t));
                add(text);
            }
            StepBody::Figure { title, svg, caption, .. } => {
                title.iter().for_each(|t| add(t));
                add(&svg_text(svg));
                add(caption);
            }
            StepBody::Reveal { prompt, answer } => {
                add(prompt);
                add(answer);
            }
            // The mnemonic's word is the letters themselves, not an abbreviation.
            StepBody::Mnemonic { parts, hook, .. } => {
                parts.iter().for_each(|p| add(&p.text));
                hook.iter().for_each(|h| add(h));
            }
            StepBody::Aside { text, .. } => add(text),
            StepBody::Update { text, reference } => {
                add(text);
                add(reference);
            }
            StepBody::Check { question, options, why, .. } => {
                add(question);
                options.iter().chain(why).for_each(|o| add(o));
            }
            StepBody::Order { prompt, steps, why } => {
                add(prompt);
                steps.iter().chain(why).for_each(|o| add(o));
            }
            StepBody::Match { prompt, pairs, why } => {
                add(prompt);
                pairs.iter().flatten().chain(why).for_each(|o| add(o));
            }
            StepBody::Sort { prompt, bins, cards, why } => {
                add(prompt);
                bins.iter().for_each(|b| add(b));
                cards.iter().for_each(|c| add(&c.text));
                why.iter().for_each(|w| add(w));
            }
            StepBody::Case { stem, steps } => {
                add(stem);
                for c in steps {
                    c.reveal.iter().for_each(|r| add(r));
                    add(&c.question);
                    c.options.iter().for_each(|o| add(o));
                    add(&c.why);
                }
            }
            StepBody::Table { title, columns, rows, .. } => {
                add(title);
                columns.iter().chain(rows.iter().flatten()).for_each(|c| add(c));
            }
            StepBody::Try { .. } => {}
        }
        out
    }
}

/// The words an SVG draws: its text with the tags (and their attributes)
/// taken out, so `fill="var(--ink)"` never reads as words.
pub fn svg_text(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len() / 4);
    let mut in_tag = false;
    for c in svg.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
}

/// Every key a step of each kind may have, shared keys included. Serde
/// can't reject unknown keys on a flattened enum, so [`validate`] checks.
pub fn allowed_keys(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "explain" => &["title", "text"],
        "figure" => &["title", "svg", "image", "caption", "alt"],
        "reveal" => &["prompt", "answer"],
        "mnemonic" => &["word", "parts", "hook"],
        "aside" => &["tone", "text"],
        "update" => &["text", "ref"],
        "check" => &["question", "options", "answer", "why"],
        "order" => &["prompt", "steps", "why"],
        "match" => &["prompt", "pairs", "why"],
        "sort" => &["prompt", "bins", "cards", "why"],
        "case" => &["stem", "steps"],
        "table" => &["title", "columns", "rows", "hide"],
        "try" => &["items"],
        _ => return None,
    })
}

/// Item kinds a lesson's `try` step (and the study player) can show.
pub const PLAYABLE: &[&str] = &["card", "cloze", "enumerate", "essay", "table", "order"];

pub const SHARED_STEP_KEYS: &[&str] = &["kind", "page", "quote", "remind"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MnemonicPart {
    /// The letter (or syllable) as it appears in the word.
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SortCard {
    pub text: String,
    /// Index into `bins`.
    pub bin: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// A practical pearl.
    Tip,
    /// "The examiner loves this": how it's asked.
    Exam,
    /// A classic mix-up.
    Trap,
    /// An everyday picture that explains the mechanism.
    Analogy,
    /// A light Egyptian joke that makes it stick (tahshish).
    Joke,
}

mod check;
mod markup;

pub use check::*;
#[cfg(test)]
use check::{license_ok, markup_problems, step_problems};
pub use markup::*;

#[cfg(test)]
mod tests;

/// The validator and the step helpers on a whole lesson of every step kind,
/// written on Cardiovascular › Pathology (`cvs.L04`, the fixture lecture).
#[cfg(test)]
mod lesson_file_tests;
