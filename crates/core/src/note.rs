//! Mohamed's own notes (research A5): a short free-text note on an item
//! (card, cloze, MCQ), a lesson step or a First Aid page. First Aid calls
//! itself an annotation hub; this is that margin, kept beside the book.
//!
//! Notes live in the synced `notes` table, one row per thing noted, keyed
//! by [`Anchor::row_id`], last writer wins. They never live in content: a
//! content-pack update that rewrites an item leaves its note alone, because
//! the note points at the item's id, not its text. Note text never leaves
//! the device except through Steps Cloud sync and his own exports.

use serde::{Deserialize, Serialize};

use crate::program::course_of;

/// Longest note, in characters. A note is a margin line, not a lesson.
pub const MAX_TEXT: usize = 2000;

/// What a note is written on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "on", content = "id", rename_all = "snake_case")]
pub enum Anchor {
    /// A content item or one of his own cards: `cvs.L04.012`, `own.…`.
    Item(String),
    /// A lesson step: `cvs.L04.lesson1#3`.
    Lesson(String),
    /// A printed First Aid page.
    Page(u32),
}

impl Anchor {
    /// The row's key in the `notes` table: one note per anchor.
    pub fn row_id(&self) -> String {
        match self {
            Anchor::Item(id) => format!("item:{id}"),
            Anchor::Lesson(id) => format!("lesson:{id}"),
            Anchor::Page(p) => format!("page:{p}"),
        }
    }

    /// The course an item or lesson step belongs to. A page has none of
    /// its own; the app finds it from the lecture page ranges.
    pub fn course(&self) -> Option<&str> {
        match self {
            Anchor::Item(id) if crate::own::is_own(id) => None,
            Anchor::Item(id) | Anchor::Lesson(id) => Some(course_of(id)),
            Anchor::Page(_) => None,
        }
    }
}

/// One note, as stored (JSON in `notes.value`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub anchor: Anchor,
    pub text: String,
    /// The printed page the anchor cited when the note was saved. The list
    /// prefers the item's current page and falls back to this one, so a
    /// note on an item a later pack removed still sorts where it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    pub created_ms: i64,
    pub edited_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    TooLong,
}

/// What saving `text` on an anchor does: an empty note is a delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Save {
    Put(Note),
    Delete,
}

/// The note to store for `text` typed on `anchor`, keeping when it was
/// first written. Text is trimmed; empty text deletes the note.
pub fn save(
    anchor: Anchor,
    text: &str,
    page: Option<u32>,
    existing: Option<&Note>,
    now_ms: i64,
) -> Result<Save, Problem> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Save::Delete);
    }
    if text.chars().count() > MAX_TEXT {
        return Err(Problem::TooLong);
    }
    let page = match &anchor {
        Anchor::Page(p) => Some(*p),
        _ => page.filter(|p| *p > 0).or(existing.and_then(|n| n.page)),
    };
    Ok(Save::Put(Note {
        anchor,
        text: text.to_string(),
        page,
        created_ms: existing.map_or(now_ms, |n| n.created_ms),
        edited_ms: now_ms,
    }))
}

/// A note in the list, with where it sits now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed<'a> {
    pub note: &'a Note,
    /// The printed page it sorts under; `None` sorts last.
    pub page: Option<u32>,
    pub course: Option<String>,
}

/// Every note that matches `query` (all its words, any case, in the note
/// or its anchor id) and `course`, sorted by printed page, then anchor.
/// `page_of` gives an anchor's current page (from the library); `course_of_page`
/// the course whose lectures hold a page.
pub fn list<'a>(
    notes: &'a [Note],
    query: &str,
    course: Option<&str>,
    page_of: impl Fn(&Anchor) -> Option<u32>,
    course_of_page: impl Fn(u32) -> Option<String>,
) -> Vec<Listed<'a>> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut out: Vec<Listed<'a>> = notes
        .iter()
        .filter(|n| {
            if words.is_empty() {
                return true;
            }
            let hay = format!("{} {}", n.text, n.anchor.row_id()).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|n| {
            let page = page_of(&n.anchor).or(n.page);
            let course = n.anchor.course().map(str::to_string).or_else(|| page.and_then(&course_of_page));
            Listed { note: n, page, course }
        })
        .filter(|l| course.is_none_or(|c| l.course.as_deref() == Some(c)))
        .collect();
    out.sort_by(|a, b| {
        (a.page.is_none(), a.page, &a.note.anchor).cmp(&(b.page.is_none(), b.page, &b.note.anchor))
    });
    out
}

/// The notes that belong on page `page`'s sheet: the page's own note first,
/// then notes on items and lesson steps citing it.
pub fn on_page(notes: &[Note], page: u32, page_of: impl Fn(&Anchor) -> Option<u32>) -> Vec<&Note> {
    let mut out: Vec<&Note> = notes.iter().filter(|n| page_of(&n.anchor).or(n.page) == Some(page)).collect();
    out.sort_by(|a, b| {
        let own = |n: &Note| !matches!(n.anchor, Anchor::Page(_));
        (own(a), &a.anchor).cmp(&(own(b), &b.anchor))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(anchor: Anchor, text: &str, page: Option<u32>) -> Note {
        Note { anchor, text: text.into(), page, created_ms: 1, edited_ms: 1 }
    }

    fn item(id: &str) -> Anchor {
        Anchor::Item(id.into())
    }

    #[test]
    fn anchors_have_stable_row_ids_and_json() {
        assert_eq!(item("cvs.L04.012").row_id(), "item:cvs.L04.012");
        assert_eq!(Anchor::Lesson("cvs.L04.l1#3".into()).row_id(), "lesson:cvs.L04.l1#3");
        assert_eq!(Anchor::Page(284).row_id(), "page:284");
        let n = note(Anchor::Page(284), "x", Some(284));
        let json = serde_json::to_value(&n).unwrap();
        assert_eq!(json["anchor"], serde_json::json!({"on": "page", "id": 284}));
        assert_eq!(serde_json::from_value::<Note>(json).unwrap(), n);
        // A note saved without a page reads back without one.
        let bare: Note = serde_json::from_str(
            r#"{"anchor":{"on":"item","id":"cvs.L01.001"},"text":"t","created_ms":1,"edited_ms":2}"#,
        )
        .unwrap();
        assert_eq!(bare.page, None);
    }

    #[test]
    fn courses_come_from_ids_except_pages_and_own_cards() {
        assert_eq!(item("renal.L03.004").course(), Some("renal"));
        assert_eq!(Anchor::Lesson("gi.L02.l1#2".into()).course(), Some("gi"));
        assert_eq!(item("own.0011").course(), None);
        assert_eq!(Anchor::Page(3).course(), None);
    }

    #[test]
    fn saving_trims_keeps_the_first_date_and_deletes_when_empty() {
        let first = match save(item("a.L01.1"), "  why ACEi cough?  ", Some(300), None, 10).unwrap() {
            Save::Put(n) => n,
            Save::Delete => panic!("a note with text is kept"),
        };
        assert_eq!(
            (first.text.as_str(), first.page, first.created_ms, first.edited_ms),
            ("why ACEi cough?", Some(300), 10, 10)
        );
        let again = match save(item("a.L01.1"), "bradykinin", None, Some(&first), 20).unwrap() {
            Save::Put(n) => n,
            Save::Delete => panic!(),
        };
        assert_eq!((again.page, again.created_ms, again.edited_ms), (Some(300), 10, 20));
        assert_eq!(save(item("a.L01.1"), "   \n ", None, Some(&first), 30), Ok(Save::Delete));
        assert_eq!(save(item("a.L01.1"), &"x".repeat(MAX_TEXT + 1), None, None, 1), Err(Problem::TooLong));
        let page = save(Anchor::Page(12), "p", Some(99), None, 1).unwrap();
        assert!(matches!(page, Save::Put(n) if n.page == Some(12)));
    }

    #[test]
    fn the_list_sorts_by_page_filters_by_course_and_searches() {
        let notes = vec![
            note(item("cvs.L04.002"), "Kussmaul sign", Some(310)),
            note(Anchor::Page(290), "draw the AP", None),
            note(item("renal.L03.001"), "RTA types", Some(600)),
            note(item("own.aa"), "my mnemonic", None),
            note(item("cvs.L01.009"), "old page", Some(999)),
        ];
        // cvs.L01.009 moved to page 285 in a later pack: the current page wins.
        let page_of = |a: &Anchor| match a {
            Anchor::Item(id) if id == "cvs.L01.009" => Some(285),
            Anchor::Page(p) => Some(*p),
            _ => None,
        };
        let course_of_page = |p: u32| (280..=330).contains(&p).then(|| "cvs".to_string());
        let all = list(&notes, "", None, page_of, course_of_page);
        let order: Vec<String> = all.iter().map(|l| l.note.anchor.row_id()).collect();
        assert_eq!(
            order,
            ["item:cvs.L01.009", "page:290", "item:cvs.L04.002", "item:renal.L03.001", "item:own.aa"]
        );
        assert_eq!(all[1].course.as_deref(), Some("cvs"));
        let cvs = list(&notes, "", Some("cvs"), page_of, course_of_page);
        assert_eq!(cvs.len(), 3);
        let found = list(&notes, "kussMAUL", None, page_of, course_of_page);
        assert_eq!(found.len(), 1);
        assert_eq!(list(&notes, "renal.L03", None, page_of, course_of_page).len(), 1);
        assert!(list(&notes, "zzz", None, page_of, course_of_page).is_empty());
    }

    #[test]
    fn a_page_sheet_shows_its_own_note_first() {
        let notes = vec![
            note(item("cvs.L01.003"), "item on 284", Some(284)),
            note(Anchor::Page(284), "page note", None),
            note(Anchor::Lesson("cvs.L01.l1#2".into()), "step", Some(284)),
            note(item("cvs.L01.004"), "elsewhere", Some(286)),
        ];
        let page_of = |a: &Anchor| match a {
            Anchor::Page(p) => Some(*p),
            _ => None,
        };
        let on = on_page(&notes, 284, page_of);
        let texts: Vec<&str> = on.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, ["page note", "item on 284", "step"]);
    }
}
