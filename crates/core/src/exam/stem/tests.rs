use super::*;

fn toggled(spans: &[Span], sel: (usize, usize)) -> Vec<Span> {
    let mut out = spans.to_vec();
    toggle(&mut out, Span(sel.0, sel.1));
    out
}

#[test]
fn a_selection_highlights_and_joins_what_it_touches() {
    assert_eq!(toggled(&[], (4, 9)), [Span(4, 9)]);
    // Apart: two highlights, in order.
    assert_eq!(toggled(&[Span(10, 12)], (2, 5)), [Span(2, 5), Span(10, 12)]);
    // Overlapping and touching ones become one.
    assert_eq!(toggled(&[Span(2, 5), Span(10, 12)], (4, 10)), [Span(2, 12)]);
    assert_eq!(toggled(&[Span(2, 5)], (5, 8)), [Span(2, 8)]);
    // Partly highlighted: the whole selection is highlighted.
    assert_eq!(toggled(&[Span(2, 5)], (3, 9)), [Span(2, 9)]);
    // Nothing selected, nothing changes.
    assert_eq!(toggled(&[Span(2, 5)], (7, 7)), [Span(2, 5)]);
    assert_eq!(toggled(&[Span(2, 5)], (9, 7)), [Span(2, 5)]);
}

#[test]
fn selecting_highlighted_words_again_takes_off_only_them() {
    assert_eq!(toggled(&[Span(4, 9)], (4, 9)), []);
    // The middle of a highlight: its two ends stay.
    assert_eq!(toggled(&[Span(2, 12)], (5, 8)), [Span(2, 5), Span(8, 12)]);
    // One end.
    assert_eq!(toggled(&[Span(2, 12), Span(20, 22)], (2, 6)), [Span(6, 12), Span(20, 22)]);
    assert_eq!(toggled(&[Span(2, 12)], (9, 12)), [Span(2, 9)]);
}

#[test]
fn pieces_follow_the_highlights_and_count_characters() {
    let stem = "A 34-year-old woman follows a strict vegan diet.";
    let at = stem.find("strict").unwrap();
    let spans = [Span(at, at + "strict vegan diet".len())];
    assert_eq!(
        pieces(stem, &spans),
        [("A 34-year-old woman follows a ", false), ("strict vegan diet", true), (".", false)]
    );
    assert_eq!(pieces(stem, &[]), [(stem, false)]);
    assert_eq!(pieces("", &[]), [("", false)]);
    // Characters, not bytes: "β" and "≥" are one character each.
    assert_eq!(pieces("β-hCG ≥ 5", &[Span(0, 5)]), [("β-hCG", true), (" ≥ 5", false)]);
    assert_eq!(pieces("β-hCG ≥ 5", &[Span(6, 7)]), [("β-hCG ", false), ("≥", true), (" 5", false)]);
    // A highlight past the end (the stem was edited since) is cut to it.
    assert_eq!(pieces("short", &[Span(3, 40), Span(50, 60)]), [("sho", false), ("rt", true)]);
    assert_eq!(pieces("short", &[Span(0, 5)]), [("short", true)]);
}

#[test]
fn spans_keep_a_small_form_in_the_saved_paper() {
    assert_eq!(serde_json::to_string(&[Span(2, 9)]).unwrap(), "[[2,9]]");
    assert_eq!(serde_json::from_str::<Vec<Span>>("[[2,9]]").unwrap(), [Span(2, 9)]);
}
