//! Mohamed's own cards (PARITY 7.6–7.7): a passage shared from any app, or a
//! card he types in. They live in the synced `own_cards` table, study like
//! book cards in their own deck ("My own cards"), skip the ✓ / ✎ / ✗ check
//! (he wrote them), and export to Anki in their own subdeck.

use serde::{Deserialize, Serialize};

use crate::content::{Body, BookRef, Item, Lecture, Source, Status};

/// Every own card's id starts with this, so no book id can collide.
pub const PREFIX: &str = "own.";
/// The deck's lecture id in sessions and stats.
pub const LECTURE: &str = "own.cards";
pub const TITLE: &str = "My own cards";
pub const TAG: &str = "own";

pub const MAX_FRONT: usize = 600;
pub const MAX_BACK: usize = 3000;
pub const MAX_SOURCE: usize = 200;

/// One card, as stored (JSON in `own_cards.value`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnCard {
    /// `own.<random>`.
    pub id: String,
    pub front: String,
    pub back: String,
    /// The book lecture it belongs with (`cvs.L03`), for tags and filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lecture: Option<String>,
    /// Printed book page, when he noted one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Where it came from: the sharing app's subject line, a site, a lecture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub created_ms: i64,
}

pub fn is_own(item_id: &str) -> bool {
    item_id.starts_with(PREFIX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    EmptyFront,
    EmptyBack,
    FrontTooLong,
    BackTooLong,
    SourceTooLong,
}

impl OwnCard {
    /// Trims the fields and drops empty optional ones.
    pub fn tidy(mut self) -> Self {
        self.front = self.front.trim().to_string();
        self.back = self.back.trim().to_string();
        self.source = self.source.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.lecture = self.lecture.filter(|l| !l.is_empty());
        self.page = self.page.filter(|p| *p > 0);
        self
    }

    pub fn check(&self) -> Result<(), Problem> {
        match (self.front.trim(), self.back.trim()) {
            ("", _) => Err(Problem::EmptyFront),
            (_, "") => Err(Problem::EmptyBack),
            (f, _) if f.chars().count() > MAX_FRONT => Err(Problem::FrontTooLong),
            (_, b) if b.chars().count() > MAX_BACK => Err(Problem::BackTooLong),
            _ if self.source.as_deref().is_some_and(|s| s.chars().count() > MAX_SOURCE) => {
                Err(Problem::SourceTooLong)
            }
            _ => Ok(()),
        }
    }

    /// The study item: a plain card, approved because he wrote it.
    pub fn item(&self) -> Item {
        let mut tags = vec![TAG.to_string()];
        tags.extend(self.lecture.iter().cloned());
        Item {
            id: self.id.clone(),
            body: Body::Card { front: self.front.clone(), back: self.back.clone() },
            topic: None,
            src: Source { book: None, page: self.page.unwrap_or(0), quote: None },
            status: Status::Approved,
            update: None,
            tags,
        }
    }
}

/// At most `max` characters, cut at the last space when there is one in the
/// final fifth (so a word isn't split). True when anything was cut.
pub fn trim_at_word(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_string(), false);
    }
    let head: String = text.chars().take(max).collect();
    let cut = head.rfind(char::is_whitespace).filter(|&i| head[..i].chars().count() >= max * 4 / 5);
    (cut.map_or(head.clone(), |i| head[..i].trim_end().to_string()), true)
}

/// A draft from shared text: the passage becomes the back, and the front
/// starts with its first line as a prompt to rewrite into a question. True
/// when the passage was too long and its end was cut.
pub fn draft_from_shared(text: &str, subject: Option<&str>, id: String, now_ms: i64) -> (OwnCard, bool) {
    let text = text.trim();
    let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let (front, _) = trim_at_word(first, 160);
    let (back, trimmed) = trim_at_word(text, MAX_BACK);
    let card = OwnCard {
        id,
        front: if front == back { String::new() } else { front },
        back,
        lecture: None,
        page: None,
        source: subject.map(|s| trim_at_word(s.trim(), MAX_SOURCE).0),
        created_ms: now_ms,
    }
    .tidy();
    (card, trimmed)
}

/// The cards as one lecture, newest first, so sessions and the Anki export
/// treat them like any other deck.
pub fn lecture(cards: &[OwnCard]) -> Lecture {
    let mut cards: Vec<&OwnCard> = cards.iter().collect();
    cards.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then_with(|| a.id.cmp(&b.id)));
    Lecture {
        schema: crate::content::SCHEMA.to_string(),
        lecture: LECTURE.to_string(),
        title: TITLE.to_string(),
        author: None,
        book: BookRef { chapter: 0, pages: [0, 0] },
        topics: Vec::new(),
        items: cards.into_iter().map(OwnCard::item).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, at: i64) -> OwnCard {
        OwnCard {
            id: id.into(),
            front: "Q".into(),
            back: "A".into(),
            lecture: None,
            page: None,
            source: None,
            created_ms: at,
        }
    }

    #[test]
    fn own_items_are_approved_cards_with_an_own_tag() {
        let mut c = card("own.a", 1);
        c.lecture = Some("cvs.L03".into());
        let item = c.item();
        assert_eq!(item.status, Status::Approved);
        assert_eq!(item.body.kind(), "card");
        assert_eq!(item.tags, vec!["own".to_string(), "cvs.L03".to_string()]);
        assert!(is_own(&item.id) && !is_own("cvs.L04.001"));
    }

    #[test]
    fn checks_and_tidies() {
        let mut c = card("own.a", 1);
        c.front = "  ".into();
        assert_eq!(c.check(), Err(Problem::EmptyFront));
        c.front = "x".repeat(MAX_FRONT + 1);
        assert_eq!(c.check(), Err(Problem::FrontTooLong));
        c.front = "ok".into();
        c.back = String::new();
        assert_eq!(c.check(), Err(Problem::EmptyBack));
        let t = OwnCard {
            source: Some(" ".into()),
            page: Some(0),
            lecture: Some(String::new()),
            ..card("own.b", 2)
        }
        .tidy();
        assert_eq!((t.source, t.page, t.lecture), (None, None, None));
    }

    #[test]
    fn shared_text_becomes_a_draft() {
        let (d, trimmed) = draft_from_shared(
            "\n  SAAG ≥ 1.1 g/dL\nmeans portal hypertension.\n",
            Some("Chrome"),
            "own.x".into(),
            5,
        );
        assert!(!trimmed);
        assert_eq!(d.front, "SAAG ≥ 1.1 g/dL");
        assert_eq!(d.back, "SAAG ≥ 1.1 g/dL\nmeans portal hypertension.");
        assert_eq!(d.source.as_deref(), Some("Chrome"));
        // One line only: no front to suggest.
        assert_eq!(draft_from_shared("one line", None, "own.y".into(), 5).0.front, "");
    }

    #[test]
    fn long_shares_are_cut_at_a_word_and_say_so() {
        let text = "word ".repeat(1000);
        let (d, trimmed) = draft_from_shared(&text, Some(&"s".repeat(500)), "own.z".into(), 5);
        assert!(trimmed);
        assert!(
            d.back.chars().count() <= MAX_BACK && d.back.ends_with("word"),
            "{}",
            &d.back[d.back.len() - 10..]
        );
        assert_eq!(d.source.as_deref().map(|s| s.chars().count()), Some(MAX_SOURCE));
        let long_source = OwnCard { source: Some("x".repeat(MAX_SOURCE + 1)), ..card("own.s", 1) };
        assert_eq!(long_source.check(), Err(Problem::SourceTooLong));
        // No space near the end: a plain cut.
        assert_eq!(trim_at_word("abcdefghij", 4), ("abcd".into(), true));
        assert_eq!(trim_at_word("short", 10), ("short".into(), false));
    }

    #[test]
    fn lecture_is_newest_first() {
        let lec = lecture(&[card("own.old", 1), card("own.new", 9)]);
        assert_eq!(lec.lecture, LECTURE);
        assert_eq!(lec.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["own.new", "own.old"]);
    }
}
