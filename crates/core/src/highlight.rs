//! Adapted from batna-app main: crates/core/src/highlight.rs (#770).
//!
//! Mohamed's own highlights in a lesson (2026-11-08): he selects words in a
//! step and marks them in a colour, as a highlight or an underline, shaped as
//! a pill or a rectangle. Each one is saved as its own synced setting
//! (`hl.<lesson>.<id>`), so two devices adding highlights never overwrite
//! each other; removing one writes a tombstone.
//!
//! A highlight is anchored by its text, not by DOM offsets: the quoted words
//! plus a little text before and after, so it finds its place again after
//! the step is re-rendered or its layout changes. [`locate`] is the rule the
//! lesson script (`learn/highlight.js`) follows.

use serde::{Deserialize, Serialize};

/// How many characters of context are kept on each side of the quote.
pub const CONTEXT: usize = 32;

/// The longest quote kept (characters). A sync op has a size cap and a
/// highlight is a few words or a sentence, not a page.
pub const MAX_QUOTE: usize = 600;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Colour {
    #[default]
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
}

impl Colour {
    pub const ALL: [Self; 5] = [Self::Yellow, Self::Green, Self::Blue, Self::Pink, Self::Orange];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Pink => "pink",
            Self::Orange => "orange",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// A filled band behind the words.
    #[default]
    Fill,
    /// A thick line under the words.
    Underline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// Rounded ends, like the lesson's key-term pill.
    #[default]
    Pill,
    /// Square corners, like a highlighter pen.
    Rect,
}

/// One highlight in one step of one lesson.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlight {
    /// Unique per highlight (random, made on the device that added it).
    pub id: String,
    /// The step, 0-based.
    pub step: usize,
    /// The highlighted words, exactly as shown.
    pub quote: String,
    /// Up to [`CONTEXT`] characters just before the quote in the step.
    #[serde(default)]
    pub prefix: String,
    /// Up to [`CONTEXT`] characters just after it.
    #[serde(default)]
    pub suffix: String,
    /// Where the quote started in the step's text (characters), as a last
    /// tie-break between equal matches.
    #[serde(default)]
    pub at: usize,
    #[serde(default)]
    pub colour: Colour,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub shape: Shape,
}

/// What a `hl.*` setting holds: a highlight, or the tombstone of a removed one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Saved {
    Gone { gone: bool },
    Live(Highlight),
}

/// The settings key prefix for a lesson's highlights.
pub fn prefix(lesson: &str) -> String {
    format!("hl.{lesson}.")
}

/// The settings key for one highlight.
pub fn key(lesson: &str, id: &str) -> String {
    format!("hl.{lesson}.{id}")
}

impl Highlight {
    /// Trims what the script sent to the stored limits; `None` when there is
    /// nothing to keep (an empty or whitespace-only selection, a bad id).
    pub fn cleaned(mut self) -> Option<Self> {
        let ok_id = !self.id.is_empty()
            && self.id.len() <= 40
            && self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !ok_id || self.quote.trim().is_empty() {
            return None;
        }
        self.quote = self.quote.chars().take(MAX_QUOTE).collect();
        let n = self.prefix.chars().count();
        self.prefix = self.prefix.chars().skip(n.saturating_sub(CONTEXT)).collect();
        self.suffix = self.suffix.chars().take(CONTEXT).collect();
        Some(self)
    }
}

/// Where `h` sits in a step whose visible text is `text`, as a character
/// range. Of every place the quote occurs, the one whose surrounding text
/// matches the saved context best wins; equal matches go to the one nearest
/// the saved position. `None` when the quote is no longer in the step.
pub fn locate(text: &str, h: &Highlight) -> Option<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let quote: Vec<char> = h.quote.chars().collect();
    let prefix: Vec<char> = h.prefix.chars().collect();
    let suffix: Vec<char> = h.suffix.chars().collect();
    if quote.is_empty() || quote.len() > chars.len() {
        return None;
    }
    let mut best: Option<(usize, usize, usize)> = None; // (score, distance, start)
    for start in 0..=chars.len() - quote.len() {
        if chars[start..start + quote.len()] != quote[..] {
            continue;
        }
        let before = prefix.iter().rev().zip(chars[..start].iter().rev()).take_while(|(a, b)| a == b).count();
        let after =
            suffix.iter().zip(chars[start + quote.len()..].iter()).take_while(|(a, b)| a == b).count();
        let score = before + after;
        let distance = start.abs_diff(h.at);
        let better = match best {
            None => true,
            Some((s, d, _)) => score > s || (score == s && distance < d),
        };
        if better {
            best = Some((score, distance, start));
        }
    }
    best.map(|(_, _, start)| (start, start + quote.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(quote: &str, prefix: &str, suffix: &str, at: usize) -> Highlight {
        Highlight {
            id: "a1".into(),
            step: 0,
            quote: quote.into(),
            prefix: prefix.into(),
            suffix: suffix.into(),
            at,
            colour: Colour::Yellow,
            style: Style::Fill,
            shape: Shape::Pill,
        }
    }

    // Mixed Arabic + English lesson text, as the lessons are written.
    const TEXT: &str = "WPW فيه accessory pathway اسمه bundle of Kent، و ECG: delta wave، short PR. PR قصير عشان الـ impulse بيعدّي بدري.";

    #[test]
    fn finds_the_quote_by_its_context() {
        // "PR" occurs twice; the context picks the second one.
        let second = h("PR", "short PR. ", " قصير", 0);
        let (s, e) = locate(TEXT, &second).unwrap();
        let chars: Vec<char> = TEXT.chars().collect();
        assert_eq!(chars[s..e].iter().collect::<String>(), "PR");
        assert!(chars[..s].iter().collect::<String>().ends_with("short PR. "));
        let first = h("PR", "short ", ". PR", 99);
        let (s1, _) = locate(TEXT, &first).unwrap();
        assert!(s1 < s);
    }

    #[test]
    fn falls_back_to_the_nearest_place_then_gives_up() {
        let near_end = h("PR", "", "", 80);
        let (s, _) = locate(TEXT, &near_end).unwrap();
        assert!(s > 60, "{s}");
        assert_eq!(locate(TEXT, &h("aortic stenosis", "", "", 0)), None);
        assert_eq!(locate("", &h("x", "", "", 0)), None);
    }

    #[test]
    fn saved_round_trips_and_tombstones_parse() {
        let live = h("delta wave", "", "", 3);
        let json = serde_json::to_string(&Saved::Live(live.clone())).unwrap();
        assert_eq!(serde_json::from_str::<Saved>(&json).unwrap(), Saved::Live(live));
        assert_eq!(serde_json::from_str::<Saved>(r#"{"gone":true}"#).unwrap(), Saved::Gone { gone: true });
        // An older row without style fields still loads with the defaults.
        let old: Saved = serde_json::from_str(r#"{"id":"x","step":2,"quote":"PR"}"#).unwrap();
        let Saved::Live(old) = old else { panic!() };
        assert_eq!((old.colour, old.style, old.shape), (Colour::Yellow, Style::Fill, Shape::Pill));
        assert_eq!(key("cvs.L01.lesson2", "x"), "hl.cvs.L01.lesson2.x");
        assert!(key("cvs.L01.lesson2", "x").starts_with(&prefix("cvs.L01.lesson2")));
    }

    #[test]
    fn cleaning_trims_and_rejects() {
        let long = h(&"x".repeat(900), &"p".repeat(50), &"s".repeat(50), 0).cleaned().unwrap();
        assert_eq!((long.quote.len(), long.prefix.len(), long.suffix.len()), (MAX_QUOTE, CONTEXT, CONTEXT));
        assert!(h("   ", "", "", 0).cleaned().is_none());
        let mut bad = h("PR", "", "", 0);
        bad.id = "../x".into();
        assert!(bad.cleaned().is_none());
    }
}
