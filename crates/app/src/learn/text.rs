//! Lesson text: Egyptian Arabic with English terms. `steps_core::lesson`
//! splits it into paragraphs, bullets, bold and Latin runs; this draws them,
//! each Latin run isolated left-to-right so "Child-Pugh class C" never flips.

use dioxus::prelude::*;
use steps_core::lesson::{Block, Span, blocks, spans};

/// Paragraphs, bullet lists and numbered lists.
#[component]
pub fn Rich(text: String, #[props(default)] class: Option<&'static str>) -> Element {
    let blocks = blocks(&text);
    rsx! {
        div { class: class.unwrap_or("ls-rich"),
            for (i , b) in blocks.into_iter().enumerate() {
                match b {
                    Block::Para(s) => {
                        let d = dir(&s);
                        rsx! { p { key: "{i}", dir: d, {runs(s)} } }
                    }
                    Block::Bullets(items) => rsx! {
                        ul { key: "{i}",
                            for (j , s) in items.into_iter().enumerate() {
                                li { key: "{j}", {runs(s)} }
                            }
                        }
                    },
                    Block::Numbered(items) => rsx! {
                        ol { key: "{i}",
                            for (j , s) in items.into_iter().enumerate() {
                                li { key: "{j}", {runs(s)} }
                            }
                        }
                    },
                }
            }
        }
    }
}

/// One line, no paragraphs (option labels, recap lines, titles).
#[component]
pub fn Line(text: String) -> Element {
    runs(spans(&text))
}

/// A line with no Arabic letter at all (an English paragraph or bullet) runs
/// left-to-right; everything else follows the Arabic page.
fn dir(spans: &[Span]) -> Option<&'static str> {
    let arabic = spans.iter().any(|s| s.text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)));
    (!arabic && spans.iter().any(|s| s.latin)).then_some("ltr")
}

fn runs(spans: Vec<Span>) -> Element {
    if dir(&spans).is_some() {
        // All English: one isolate, its own full stop included, so a bullet
        // keeps the list's side but reads "… RNA." and never ".Genome …".
        return rsx! {
            bdi { dir: "ltr", lang: "en", {styled(spans)} }
        };
    }
    // Neighbouring Latin spans (a bold word inside an English phrase) share
    // one isolate, so the phrase keeps its order.
    let mut groups: Vec<(bool, Vec<Span>)> = Vec::new();
    for s in spans {
        match groups.last_mut() {
            Some((latin, g)) if *latin && s.latin => g.push(s),
            _ => groups.push((s.latin, vec![s])),
        }
    }
    glue_prefixes(&mut groups);
    let pieces = pieces(groups);
    rsx! {
        for (i , piece) in pieces.into_iter().enumerate() {
            match piece {
                Piece::Arabic(g) => rsx! { span { key: "{i}", {styled(g)} } },
                // An English term is its own left-to-right box: a short one
                // ("vitamin A and") stays on one line, a longer one (a book
                // sentence) wraps top to bottom instead of its halves trading
                // places with the Arabic around it. Neither is ever wider than
                // the line. Its "الـ" stays beside it.
                Piece::Term { article, g, long } => {
                    let class = if long { "ls-en" } else { "ls-nw" };
                    if article.is_empty() {
                        rsx! {
                            bdi { key: "{i}", dir: "ltr", lang: "en", class, {styled(g)} }
                        }
                    } else {
                        rsx! {
                            span { key: "{i}", class: "ls-en-glued",
                                span { {styled(article)} }
                                bdi { dir: "ltr", lang: "en", class, {styled(g)} }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What a mixed line is drawn from, in reading order.
enum Piece {
    Arabic(Vec<Span>),
    Term { article: Vec<Span>, g: Vec<Span>, long: bool },
}

/// Sorts the groups into pieces: each English run gets a box with the
/// "الـ" before it beside it. A whole sentence takes the punctuation after
/// it into its box too: it fills its lines, and a comma left outside would
/// wrap to the start of the next line. A shorter box keeps the punctuation
/// on its left, where the Arabic sentence puts it.
fn pieces(mut groups: Vec<(bool, Vec<Span>)>) -> Vec<Piece> {
    let long: Vec<bool> = groups.iter().map(|(latin, g)| *latin && !short(g)).collect();
    for k in 0..groups.len() {
        if !long[k] || !sentence(&groups[k].1) || groups.get(k + 1).is_none_or(|(latin, _)| *latin) {
            continue;
        }
        let first = &mut groups[k + 1].1[0];
        let n: usize = first.text.chars().take_while(|c| PUNCT.contains(*c)).map(char::len_utf8).sum();
        if n > 0 {
            let punct: String = first.text.drain(..n).collect();
            groups[k].1.push(Span { text: punct, bold: false, key: false, latin: true });
        }
    }
    space_punctuation(&mut groups);
    flip_arrows(&mut groups);
    let mut out: Vec<Piece> = Vec::new();
    for (k, (latin, g)) in groups.into_iter().enumerate() {
        if !latin {
            out.push(Piece::Arabic(g));
            continue;
        }
        // The glued article ("بالـ\u{a0}") moves out of the Arabic before,
        // to sit beside a term. A sentence box is as tall as its lines, so
        // the article stays at the end of the Arabic line before it.
        let mut article = Vec::new();
        if !sentence(&g)
            && let Some(Piece::Arabic(prev)) = out.last_mut()
            && let Some(last) = prev.last_mut()
            && last.text.ends_with('\u{a0}')
        {
            let body = &last.text[..last.text.len() - '\u{a0}'.len_utf8()];
            let at = body.rfind(' ').map_or(0, |i| i + 1);
            let word = last.text.split_off(at);
            article.push(Span { text: word, bold: last.bold, key: last.key, latin: false });
            if last.text.is_empty() {
                prev.pop();
            }
        }
        out.push(Piece::Term { article, g, long: long[k] });
    }
    out
}

/// Sentence punctuation that follows an English term in an Arabic line.
const PUNCT: &str = ".,;!?،؛؟";

fn styled(spans: Vec<Span>) -> Element {
    rsx! {
        for (i , s) in spans.into_iter().enumerate() {
            if s.key {
                mark { key: "{i}", "{s.text}" }
            } else if s.bold {
                strong { key: "{i}", "{s.text}" }
            } else {
                span { key: "{i}", "{s.text}" }
            }
        }
    }
}

/// In an Arabic line the full stop or comma after an English term belongs
/// to the Arabic sentence, so it sits on the term's left, right against its
/// first letter: "shrunken … normal." read ".shrunken". A hair space after
/// the term sets it apart. A colon stays close, as after an Arabic label.
fn space_punctuation(groups: &mut [(bool, Vec<Span>)]) {
    for k in 1..groups.len() {
        if !groups[k - 1].0 || groups[k].0 {
            continue;
        }
        if let Some(first) = groups[k].1.first_mut()
            && first.text.starts_with(|c: char| PUNCT.contains(c))
        {
            first.text.insert(0, '\u{200A}');
        }
    }
}

/// An arrow between the terms of an Arabic line points the way the line
/// reads: "cause → effect" is drawn right to left, so its arrow is "←".
/// Arrows inside an English isolate keep their direction.
fn flip_arrows(groups: &mut [(bool, Vec<Span>)]) {
    for (latin, g) in groups.iter_mut() {
        if *latin {
            continue;
        }
        for s in g {
            if s.text.contains(['→', '←', '⇒', '⇐']) {
                s.text = s
                    .text
                    .chars()
                    .map(|c| match c {
                        '→' => '←',
                        '←' => '→',
                        '⇒' => '⇐',
                        '⇐' => '⇒',
                        c => c,
                    })
                    .collect();
            }
        }
    }
}

/// "الـ edge", "بالـ MRI": the Arabic article is written apart from the
/// English word it belongs to. The space after it becomes a no-break space,
/// so the line never wraps between "الـ" and its word.
fn glue_prefixes(groups: &mut [(bool, Vec<Span>)]) {
    for k in 1..groups.len() {
        if !groups[k].0 || groups[k - 1].0 {
            continue;
        }
        let text: String = groups[k - 1].1.iter().map(|s| s.text.as_str()).collect();
        let article = text.ends_with("\u{640} ") || text == "ال " || text.ends_with(" ال ");
        if article && let Some(last) = groups[k - 1].1.last_mut() {
            last.text.pop();
            last.text.push('\u{a0}');
        }
    }
}

/// Seven words or more: a sentence from the book, not a term.
fn sentence(g: &[Span]) -> bool {
    g.iter().map(|s| s.text.split_whitespace().count()).sum::<usize>() >= 7
}

/// Up to three words and 32 letters: kept whole on its line.
fn short(g: &[Span]) -> bool {
    let len: usize = g.iter().map(|s| s.text.trim().chars().count()).sum();
    let words: usize = g.iter().map(|s| s.text.split_whitespace().count()).sum();
    len <= 32 && words <= 3
}

/// Mostly-English text (an option, a table cell) runs left-to-right.
pub fn is_latin(text: &str) -> bool {
    let latin = text.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let arabic = text.chars().filter(|c| ('\u{0600}'..='\u{06FF}').contains(c)).count();
    latin > arabic
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(text: &str) -> String {
        let text = text.to_string();
        dioxus::ssr::render_element(rsx! { Rich { text } })
    }

    #[test]
    fn mixed_lines_read_in_arabic_order() {
        // The screenshot of 2026-11-01: "الـ edge: sharp".
        let h = html("- الـ edge: **sharp**، وممكن يبقى **irregular**");
        assert!(h.contains("الـ\u{a0}</span>"), "الـ stays with its word: {h}");
        assert!(h.contains(">edge</span></bdi>"), "label and value are separate terms: {h}");
        assert!(h.contains("\u{200A}، وممكن"), "the comma is set apart from the term: {h}");
        // Arrows between terms point the way the line reads.
        let h = html("الـ compensated → الـ first decompensation (بالراحة أو فجأة) → decompensated.");
        assert!(h.contains('←') && !h.contains('→'), "{h}");
        // An English line keeps its own order, full stop and arrows.
        let h = html("- Bleeding → ceftriaxone, then norfloxacin.");
        assert!(h.contains(r#"<bdi dir="ltr" lang="en">"#) && h.contains("<span>.</span></bdi>"), "{h}");
        assert!(h.contains('→') && !h.contains('\u{200A}'), "{h}");
        // A book sentence inside an Arabic line is one box with its comma;
        // "الـ" stays beside a longer term's box.
        let h = html(
            "**Ascites is due to the accumulation of endogenous vasodilators**، و **PH acts as a localizing factor** بيوجّه",
        );
        assert!(h.contains(r#"class="ls-en""#) && h.contains("<span>،</span></bdi>"), "{h}");
        let h = html("من الـ intravascular compartment pressure للـ تجويف");
        assert!(h.contains("ls-en-glued") && h.contains("<span>الـ\u{a0}</span>"), "{h}");
        let h = html("Genome: single-stranded RNA.\n\nتمام كده.");
        assert!(h.contains(r#"<p dir="ltr">"#), "an English paragraph runs left to right: {h}");
    }
}
