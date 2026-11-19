//! Lesson text: Egyptian Arabic with English terms. `steps_core::lesson`
//! splits it into paragraphs, bullets, bold and Latin runs; this draws them,
//! each Latin run isolated left-to-right so "Stanford type A" never flips.

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

/// The `dir` for a box holding one piece of lesson text inside English
/// surroundings (an explanation under English options, a mnemonic line):
/// left to right only with no Arabic letter in it, as `Rich` decides a
/// paragraph; any Arabic and it reads right to left, its English terms
/// isolated by `Line`.
pub fn line_dir(text: &str) -> &'static str {
    dir(&spans(text)).unwrap_or("rtl")
}

fn runs(spans: Vec<Span>) -> Element {
    if dir(&spans).is_some() {
        // All English: one isolate, its own full stop included, so a bullet
        // keeps the list's side but reads "… RNA." and never ".Genome …".
        return rsx! {
            bdi { dir: "ltr", lang: "en", {styled(spans, true)} }
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
        for (i , run) in key_runs(pieces).into_iter().enumerate() {
            match run {
                Run::One(piece) => draw(piece, i, true),
                // One key term made of English and Arabic ("immunosuppressive
                // و antineoplastic"): one marker around all of it, not a pill
                // per word (UX-44).
                Run::Key(pieces) => rsx! {
                    mark { key: "{i}",
                        for (j , piece) in pieces.into_iter().enumerate() {
                            {draw(piece, j, false)}
                        }
                    }
                },
            }
        }
    }
}

/// One piece of a mixed line; `marks` draws its key spans as markers (off
/// inside a marker around a whole key run).
fn draw(piece: Piece, i: usize, marks: bool) -> Element {
    match piece {
        Piece::Arabic(g) => rsx! { span { key: "{i}", {styled(g, marks)} } },
        // An English term is its own left-to-right box: a short one
        // ("vitamin A and") stays on one line, a longer one (a book
        // sentence) wraps top to bottom instead of its halves trading
        // places with the Arabic around it. Neither is ever wider than
        // the line. Its "الـ" stays beside it.
        Piece::Term { article, g, long } => {
            let class = if long { "ls-en" } else { "ls-nw" };
            if article.is_empty() {
                rsx! {
                    bdi { key: "{i}", dir: "ltr", lang: "en", class, {styled(g, marks)} }
                }
            } else {
                rsx! {
                    span { key: "{i}", class: "ls-en-glued",
                        span { {styled(article, marks)} }
                        bdi { dir: "ltr", lang: "en", class, {styled(g, marks)} }
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

impl Piece {
    fn spans(&self) -> impl Iterator<Item = &Span> {
        let (a, b): (&[Span], &[Span]) = match self {
            Piece::Arabic(g) => (g, &[]),
            Piece::Term { article, g, .. } => (article, g),
        };
        a.iter().chain(b)
    }

    /// Every word in it is a key term.
    fn is_key(&self) -> bool {
        self.spans().next().is_some() && self.spans().all(|s| s.key || s.text.trim().is_empty())
    }

    /// Only spaces or a list comma: what lies between the words of one key
    /// term ("ankyrin، band 3"), which `spans` leaves unmarked.
    fn is_gap(&self) -> bool {
        self.spans().all(|s| !s.key && s.text.chars().all(|c| c.is_whitespace() || "،,".contains(c)))
    }
}

/// A run of pieces drawn together.
enum Run {
    One(Piece),
    /// Two or more pieces of one key term, with the gaps between them.
    Key(Vec<Piece>),
}

/// Joins the pieces of one key term across scripts: from a key piece through
/// key pieces and the gaps between them, to the last key piece.
fn key_runs(pieces: Vec<Piece>) -> Vec<Run> {
    let flags: Vec<(bool, bool)> = pieces.iter().map(|p| (p.is_key(), p.is_gap())).collect();
    let mut pieces = pieces.into_iter();
    let mut out = Vec::new();
    let mut i = 0;
    while i < flags.len() {
        let mut end = i;
        if flags[i].0 {
            let mut j = i + 1;
            while j < flags.len() && (flags[j].0 || flags[j].1) {
                if flags[j].0 {
                    end = j;
                }
                j += 1;
            }
        }
        if end > i {
            out.push(Run::Key(pieces.by_ref().take(end - i + 1).collect()));
        } else {
            out.extend(pieces.next().map(Run::One));
        }
        i = end + 1;
    }
    out
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

fn styled(spans: Vec<Span>, marks: bool) -> Element {
    rsx! {
        for (i , s) in spans.into_iter().enumerate() {
            if s.key && marks {
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
/// Authors write either "→" (as in English) or "←" (as it is seen, the way
/// docs/LESSONS.md asks); both come out "←", never flipped back (#217).
/// Arrows inside an English isolate keep their direction.
fn flip_arrows(groups: &mut [(bool, Vec<Span>)]) {
    for (latin, g) in groups.iter_mut() {
        if *latin {
            continue;
        }
        for s in g {
            if s.text.contains(['→', '⇒']) {
                s.text = s
                    .text
                    .chars()
                    .map(|c| match c {
                        '→' => '←',
                        '⇒' => '⇐',
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
        // Written the way it is seen, it stays that way (#217).
        let h = html("زيادة الـ preload ← زيادة الـ stroke volume ⇐ ضغط أعلى");
        assert!(h.contains('←') && h.contains('⇐') && !h.contains('→') && !h.contains('⇒'), "{h}");
        // An English line keeps its own order, full stop and arrows.
        let h = html("- Tamponade → pericardiocentesis, then drainage.");
        assert!(h.contains(r#"<bdi dir="ltr" lang="en">"#) && h.contains("<span>.</span></bdi>"), "{h}");
        assert!(h.contains('→') && !h.contains('\u{200A}'), "{h}");
        // A book sentence inside an Arabic line is one box with its comma;
        // "الـ" stays beside a longer term's box.
        let h = html(
            "**Tamponade is compression of the heart by fluid**، و **diastolic pressures equalize** بيوضّح",
        );
        assert!(h.contains(r#"class="ls-en""#) && h.contains("<span>،</span></bdi>"), "{h}");
        let h = html("من الـ intrapericardial pressure للـ تجويف");
        assert!(h.contains("ls-en-glued") && h.contains("<span>الـ\u{a0}</span>"), "{h}");
        let h = html("Genome: single-stranded RNA.\n\nتمام كده.");
        assert!(h.contains(r#"<p dir="ltr">"#), "an English paragraph runs left to right: {h}");
    }

    #[test]
    fn one_key_term_across_scripts_is_one_marker() {
        // bio.L01: «immunosuppressive و antineoplastic و antibiotic» (UX-44).
        let h = html("الدوا ده ==immunosuppressive و antineoplastic و antibiotic== في نفس الوقت");
        assert_eq!(h.matches("<mark").count(), 1, "{h}");
        let inside = h.split("<mark").nth(1).unwrap().split("</mark>").next().unwrap();
        for word in ["immunosuppressive", "antineoplastic", "antibiotic", " و "] {
            assert!(inside.contains(word), "{word} inside the one marker: {h}");
        }
        // Arabic and English in one key, with its article; and a list
        // comma between the words of one key.
        assert_eq!(html("ده ==الـ amino acid الغلط== كده").matches("<mark").count(), 1);
        assert_eq!(html("فيه ==ankyrin، band 3، spectrin== في الغشا").matches("<mark").count(), 1);
        // Two key terms written apart stay two markers.
        assert_eq!(html("بيعلّي ==lactate== و ==triglycerides== مع بعض").matches("<mark").count(), 2);
        // A key word alone, and an English line, as before.
        assert_eq!(html("الـ ==preload== بيزيد").matches("<mark").count(), 1);
        assert_eq!(html("The ==preload== rises.").matches("<mark").count(), 1);
    }

    #[test]
    fn a_line_reads_left_to_right_only_with_no_arabic() {
        assert_eq!(line_dir("Right: a PFO lets a paradoxical embolus through."), "ltr");
        assert_eq!(line_dir("الـ PFO بيعدّي الـ embolus للناحية الشمال."), "rtl");
        assert_eq!(line_dir("Glycosylase يشيل الـ base البايظة ويعمل AP site"), "rtl");
        assert_eq!(line_dir(""), "rtl");
    }
}
