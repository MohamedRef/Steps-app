//! Pure answer appearance and table-cell sizing decisions.

use crate::i18n::learn as tx;

/// An answer choice's look (parts.css `button.answer`) and its mark in
/// words: picked before Check, then his pick right or wrong and the right
/// one he missed. Other options stay plain (and dimmed once checked).
pub(super) fn answer_state(
    checked: bool,
    picked: Option<usize>,
    i: usize,
    answer: usize,
) -> (Option<&'static str>, Option<&'static str>) {
    match (checked, picked) {
        (false, Some(p)) if p == i => (Some("selected"), None),
        (false, _) => (None, None),
        (true, Some(p)) if p == i && i == answer => (Some("correct"), Some(tx::MARK_MINE_RIGHT)),
        (true, Some(p)) if p == i => (Some("wrong"), Some(tx::MARK_MINE_WRONG)),
        (true, _) if i == answer => (Some("missed"), Some(tx::MARK_RIGHT)),
        (true, _) => (None, None),
    }
}

/// A short value ("8–11", "F0–F1", "> 12.5") that must not break inside a
/// table cell.
pub(super) fn is_value(cell: &str) -> bool {
    cell.chars().count() <= 14 && !cell.contains(' ') || cell.chars().count() <= 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_feedback_is_textual_as_well_as_colour() {
        assert_eq!(answer_state(false, Some(1), 1, 2), (Some("selected"), None));
        assert_eq!(answer_state(true, Some(1), 1, 2), (Some("wrong"), Some(tx::MARK_MINE_WRONG)));
        assert_eq!(answer_state(true, Some(1), 2, 2), (Some("missed"), Some(tx::MARK_RIGHT)));
        assert_eq!(answer_state(true, Some(2), 2, 2), (Some("correct"), Some(tx::MARK_MINE_RIGHT)));
    }

    #[test]
    fn short_table_values_do_not_wrap() {
        assert!(is_value("F0–F1"));
        assert!(is_value(">12.5"));
        assert!(!is_value("a full descriptive diagnosis"));
    }
}
