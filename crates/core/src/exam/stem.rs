//! The words of a stem he highlighted on a paper, as a question bank's test
//! interface lets him mark a phrase while he reads. A highlight is a range
//! of the stem's characters (Unicode scalar values, which is what the
//! page's script counts when it reads a selection), kept on the question's
//! [`super::Answer`] with the paper. The ranges stay sorted and apart:
//! highlights that overlap or touch become one.

use serde::{Deserialize, Serialize};

/// Characters `start..end` of a stem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Span(pub usize, pub usize);

/// Selecting words that are all highlighted already takes the highlight off
/// them (and only them); any other selection highlights it, joined with the
/// highlights it overlaps or touches. An empty selection changes nothing.
pub fn toggle(spans: &mut Vec<Span>, sel: Span) {
    let Span(start, end) = sel;
    if start >= end {
        return;
    }
    if spans.iter().any(|s| s.0 <= start && end <= s.1) {
        let mut kept = Vec::with_capacity(spans.len() + 1);
        for &s in spans.iter() {
            if s.1 <= start || end <= s.0 {
                kept.push(s);
                continue;
            }
            if s.0 < start {
                kept.push(Span(s.0, start));
            }
            if end < s.1 {
                kept.push(Span(end, s.1));
            }
        }
        *spans = kept;
    } else {
        let (mut from, mut to) = (start, end);
        spans.retain(|s| {
            let joins = s.0 <= to && from <= s.1;
            if joins {
                from = from.min(s.0);
                to = to.max(s.1);
            }
            !joins
        });
        spans.push(Span(from, to));
        spans.sort();
    }
}

/// `text` cut where highlights start and end: each piece with whether it is
/// highlighted, in order. Ranges past the end of the text are cut to it, so
/// a stem edited since (his ✎) never breaks the page.
pub fn pieces<'a>(text: &'a str, spans: &[Span]) -> Vec<(&'a str, bool)> {
    // Byte offsets of every character, and the end.
    let bounds: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
    let chars = bounds.len() - 1;
    let byte = |c: usize| bounds[c.min(chars)];
    let mut out = Vec::new();
    let mut at = 0;
    for s in spans {
        let (start, end) = (s.0.min(chars), s.1.min(chars));
        if start >= end || start < at {
            continue;
        }
        if at < start {
            out.push((&text[byte(at)..byte(start)], false));
        }
        out.push((&text[byte(start)..byte(end)], true));
        at = end;
    }
    if at < chars || out.is_empty() {
        out.push((&text[byte(at)..], false));
    }
    out
}

#[cfg(test)]
mod tests;
