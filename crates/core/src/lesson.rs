//! Lessons, `steps.lesson/v1`: the teaching layer on top of the study items.
//!
//! One YAML file per lecture (`content/lessons/hb/L01.yaml`) holds a few
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
    /// `hb.L01`, the lecture these lessons teach.
    pub lecture: String,
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    /// `hb.L01.lesson1`. Never reused or renumbered.
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

pub const SHARED_STEP_KEYS: &[&str] = &["kind", "page", "quote"];

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

// ---------- the text markup ----------

/// A run of lesson text with one look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    /// One of the step's few key terms (`==term==`): drawn with a marker.
    pub key: bool,
    /// Latin script (English terms, numbers with units): isolated left-to-right.
    pub latin: bool,
}

/// A block of lesson text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Para(Vec<Span>),
    Bullets(Vec<Vec<Span>>),
    /// Lines written `1. `, `2. `…: drawn as a numbered list, counted from 1.
    Numbered(Vec<Vec<Span>>),
}

/// A list line: whether it is numbered, and its text. `- item`, or a number
/// then `. ` or `) ` (`1. item`, `2) item`).
fn list_item(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix("- ") {
        return Some((false, rest));
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return None;
    }
    let rest = &line[digits..];
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")).map(|r| (true, r))
}

/// Splits lesson text into paragraphs (blank lines) and lists (lines
/// starting `- `, or `1. ` for a numbered one), with `**bold**`,
/// `==key terms==` and Latin runs marked.
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let flush = |out: &mut Vec<Block>, list: &mut Option<(bool, Vec<Vec<Span>>)>| {
        if let Some((numbered, items)) = list.take() {
            out.push(if numbered { Block::Numbered(items) } else { Block::Bullets(items) });
        }
    };
    for chunk in text.split("\n\n").map(str::trim).filter(|c| !c.is_empty()) {
        let mut para: Vec<Span> = Vec::new();
        let mut list: Option<(bool, Vec<Vec<Span>>)> = None;
        for l in chunk.lines().map(str::trim).filter(|l| !l.is_empty()) {
            match list_item(l) {
                Some((numbered, item)) => {
                    if !para.is_empty() {
                        out.push(Block::Para(std::mem::take(&mut para)));
                    }
                    if list.as_ref().is_some_and(|(n, _)| *n != numbered) {
                        flush(&mut out, &mut list);
                    }
                    list.get_or_insert_with(|| (numbered, Vec::new())).1.push(spans(item));
                }
                None => {
                    flush(&mut out, &mut list);
                    if !para.is_empty() {
                        para.push(Span { text: " ".into(), bold: false, key: false, latin: false });
                    }
                    para.extend(spans(l));
                }
            }
        }
        if !para.is_empty() {
            out.push(Block::Para(para));
        }
        flush(&mut out, &mut list);
    }
    out
}

/// One line of lesson text as spans: `**bold**`, `==key term==`, and runs
/// of Latin script apart from Arabic so the UI can isolate them
/// (`<bdi dir="ltr">`). Bold on punctuation alone (the colon after
/// "**(SECs)**:") is dropped, so it doesn't stand apart from its term.
pub fn spans(line: &str) -> Vec<Span> {
    // Script runs come from the whole line, so bold can't split an English
    // phrase ("**Genome:** single-stranded") into runs that flip.
    let mut plain = String::new();
    let mut look: Vec<(bool, bool)> = Vec::new();
    let (mut bold, mut key) = (false, false);
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        if let Some(r) = rest.strip_prefix("**") {
            bold = !bold;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("==") {
            key = !key;
            rest = r;
        } else {
            plain.push(c);
            look.push((bold, key));
            rest = &rest[c.len_utf8()..];
        }
    }
    let mut out: Vec<Span> = Vec::new();
    let mut at = 0;
    for (text, latin) in script_runs(&plain) {
        for c in text.chars() {
            let (bold, key) = look[at];
            at += 1;
            match out.last_mut() {
                Some(last) if last.bold == bold && last.key == key && last.latin == latin => {
                    last.text.push(c)
                }
                _ => out.push(Span { text: c.to_string(), bold, key, latin }),
            }
        }
    }
    for s in &mut out {
        if s.text.chars().all(|c| !c.is_alphanumeric()) {
            s.bold = false;
            s.key = false;
        }
    }
    // Neighbours that now look the same join up again.
    let mut joined: Vec<Span> = Vec::with_capacity(out.len());
    for s in out {
        match joined.last_mut() {
            Some(l) if l.bold == s.bold && l.key == s.key && l.latin == s.latin => l.text.push_str(&s.text),
            _ => joined.push(s),
        }
    }
    joined
}

/// Runs of Latin and non-Latin text. Spaces, digits and punctuation between
/// two Latin words stay in the Latin run ("Child-Pugh class C"), so a term
/// is isolated whole; elsewhere they go with the Arabic around them.
fn script_runs(text: &str) -> Vec<(String, bool)> {
    #[derive(Clone, Copy, PartialEq)]
    enum C {
        Latin,
        Arabic,
        Neutral,
    }
    let class = |c: char| {
        if c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c) || "µβαγκ".contains(c) {
            C::Latin
        } else if ('\u{0600}'..='\u{06FF}').contains(&c) || ('\u{0750}'..='\u{077F}').contains(&c) {
            C::Arabic
        } else {
            C::Neutral
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let strong: Vec<C> = chars.iter().map(|&c| class(c)).collect();
    let mut resolved = strong.clone();
    let mixed = strong.contains(&C::Arabic);
    // Each run of neutral chars (spaces, digits, signs) between two strong
    // chars: between two Latin words it is Latin ("Child-Pugh class C").
    // Right after a Latin word, the numbers, units and closing brackets that
    // continue it are Latin ("Zone 3", "PaO2 <80", "(MELD)"), but not the
    // sentence's own punctuation. Right before one, an opening bracket or
    // quote is Latin ("(morphology").
    let mut i = 0;
    while i < chars.len() {
        if strong[i] != C::Neutral {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && strong[i] == C::Neutral {
            i += 1;
        }
        let before = start.checked_sub(1).map(|k| strong[k]);
        let after = strong.get(i).copied();
        let run = start..i;
        for k in run.clone() {
            resolved[k] = C::Arabic;
        }
        // In an Arabic line, "label: value" and "cause → effect" are two
        // terms the line reads in Arabic order ("الـ edge: sharp" reads edge,
        // then sharp); one isolate would put "sharp" next to "الـ" and
        // "edge" far off.
        let separator = || {
            let t: String = chars[run.clone()].iter().collect();
            mixed
                && t.ends_with(' ')
                && (matches!(t.trim(), ":" | ";")
                    || t.starts_with(' ') && matches!(t.trim(), "→" | "⇒" | "←"))
        };
        if before == Some(C::Latin) && after == Some(C::Latin) && !separator() {
            for k in run {
                resolved[k] = C::Latin;
            }
            continue;
        }
        if before == Some(C::Latin) {
            let continues = |c: char| c.is_ascii_digit() || c == ' ' || "%()[]+-−–≥≤<>/.×±°'\"·,".contains(c);
            let mut end = start;
            while end < i && continues(chars[end]) {
                end += 1;
            }
            // Brackets stay only in pairs: an opening one must close inside
            // the run ("Low risk (≤ 2%)"), a closing one must close a bracket
            // the Latin term opened ("(morphology)"). The others are Arabic's
            // ("(مع albumin 20%)").
            let mut term = start;
            while term > 0 && resolved[term - 1] == C::Latin {
                term -= 1;
            }
            let mut opened = chars[term..start].iter().fold(0usize, |d, &c| match c {
                '(' | '[' => d + 1,
                ')' | ']' => d.saturating_sub(1),
                _ => d,
            });
            let mut open: Vec<usize> = Vec::new();
            for (k, &c) in chars.iter().enumerate().take(end).skip(start) {
                match c {
                    '(' | '[' => open.push(k),
                    ')' | ']' if open.pop().is_none() => {
                        if opened == 0 {
                            end = k;
                            break;
                        }
                        opened -= 1;
                    }
                    _ => {}
                }
            }
            if let Some(&first) = open.first() {
                end = first;
            }
            // Don't take the sentence's punctuation or trailing spaces.
            while end > start && " .,-–·".contains(chars[end - 1]) {
                end -= 1;
            }
            for r in &mut resolved[start..end] {
                *r = C::Latin;
            }
        }
        if after == Some(C::Latin) {
            // A number that starts the term is part of it too ("20 cm",
            // "1.5 ml/kg/h", "3–4 L").
            let mut from = i;
            while from > start
                && (chars[from - 1].is_ascii_digit() || " .,–-×/~<>≥≤±%([\"'«".contains(chars[from - 1]))
            {
                from -= 1;
            }
            while from < i && !(chars[from].is_ascii_digit() || "<>≥≤±~([\"'«".contains(chars[from])) {
                from += 1;
            }
            for r in &mut resolved[from..i] {
                *r = C::Latin;
            }
        }
    }
    // Numbers with a sign, a range or a ratio ("≤ 2%", "> 3", "5–6",
    // "1/3") read wrong in an Arabic line: bidi mirrors "≤" into "≥" and
    // turns "5–6" into "6–5". They are isolated left to right too.
    let mut k = 0;
    while k < chars.len() {
        if !chars[k].is_ascii_digit() || resolved[k] == C::Latin {
            k += 1;
            continue;
        }
        // A sign right before the number, maybe with one space ("≤ 2").
        let mut start = k;
        let mut signed = false;
        let mut j = k;
        if j > 0 && chars[j - 1] == ' ' {
            j -= 1;
        }
        while j > 0 && "<>≤≥±~=+−".contains(chars[j - 1]) && resolved[j - 1] != C::Latin {
            j -= 1;
            signed = true;
        }
        if signed {
            start = j;
        }
        let mut end = k;
        let mut joined = false;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
            if end + 1 < chars.len() && "–-/:×".contains(chars[end]) && chars[end + 1].is_ascii_digit() {
                joined = true;
                end += 1;
            } else if end + 1 < chars.len() && chars[end] == '.' && chars[end + 1].is_ascii_digit() {
                end += 1;
            } else if let Some(skip) = spaced_operator(&chars[end..]) {
                // A sum or product written with spaces: "45 + 55 = 100".
                joined = true;
                end += skip;
            }
        }
        let percent = chars.get(end) == Some(&'%');
        if percent {
            end += 1;
        }
        if signed || joined || percent {
            for r in &mut resolved[start..end] {
                *r = C::Latin;
            }
        }
        k = end;
    }
    // A bracket pair goes into an isolate only when all of it does: in
    // "(black أو brown)" the brackets stay Arabic, so they mirror right.
    let mut open: Vec<usize> = Vec::new();
    for k in 0..chars.len() {
        match chars[k] {
            '(' | '[' => open.push(k),
            ')' | ']' => {
                if let Some(o) = open.pop()
                    && resolved[o..=k].iter().any(|r| *r != C::Latin)
                {
                    resolved[o] = C::Arabic;
                    resolved[k] = C::Arabic;
                }
            }
            _ => {}
        }
    }
    let mut out: Vec<(String, bool)> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        let latin = resolved[i] == C::Latin;
        match out.last_mut() {
            Some((s, l)) if *l == latin => s.push(c),
            _ => out.push((c.to_string(), latin)),
        }
    }
    out
}

/// The length of " + " (or "+", " = ", " × ", "−", "/") in front of a
/// digit at the start of `rest`, so "45 + 55 = 100" reads as one formula.
fn spaced_operator(rest: &[char]) -> Option<usize> {
    let mut k = 0;
    if rest.get(k) == Some(&' ') {
        k += 1;
    }
    if !rest.get(k).is_some_and(|c| "+=×−÷".contains(*c)) {
        return None;
    }
    k += 1;
    if rest.get(k) == Some(&' ') {
        k += 1;
    }
    rest.get(k).is_some_and(|c| c.is_ascii_digit()).then_some(k)
}

// ---------- checks ----------

/// What the validator needs to know about the lecture a lesson file teaches.
pub struct LessonContext<'a> {
    pub lecture: &'a Lecture,
    /// Ids Mohamed approved (`content/approved.txt`).
    pub approved: Option<&'a BTreeSet<String>>,
    /// The keys written on each step, by `(lesson index, step index)`.
    pub raw_step_keys: Vec<((usize, usize), Vec<String>)>,
}

/// An image file in git can't be bigger than this (WebP, about 800 px wide).
pub const MAX_IMAGE_BYTES: usize = 150 * 1024;

/// Image files a figure may use, by extension, with their MIME type.
pub const IMAGE_TYPES: [(&str, &str); 4] =
    [("webp", "image/webp"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")];

/// Problems with a figure's picture (the file itself is checked by [`embed_images`]).
pub fn picture_problems(p: &Picture) -> Vec<String> {
    let mut out = Vec::new();
    if !p.data.is_empty() {
        out.push("image.data is filled in by the pack build; don't write it".into());
    }
    let file = &p.file;
    let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if !IMAGE_TYPES.iter().any(|(e, _)| *e == ext) {
        out.push(format!("image file {file:?} must be .webp, .jpg or .png"));
    }
    if file.is_empty() || file.contains("..") || file.starts_with('/') || file.contains('\\') {
        out.push(format!("image file {file:?} must be a path under content/lessons/img/"));
    }
    let has = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    if p.book {
        if has(&p.credit) || has(&p.license) || has(&p.source) {
            out.push("a book figure takes no credit, license or source (the step's page cites it)".into());
        }
        return out;
    }
    if !has(&p.credit) || !has(&p.license) || !has(&p.source) {
        out.push("an image needs credit, license and source (or book: true for a book figure)".into());
    }
    if let Some(l) = &p.license
        && !license_ok(l)
    {
        out.push(format!(
            "license {l:?}: use public domain, CC0 or a CC BY licence without ND (no-derivatives)"
        ));
    }
    if let Some(s) = &p.source
        && !s.starts_with("https://")
    {
        out.push("source must be the https:// page the image came from".into());
    }
    out
}

/// Public domain, CC0, or CC BY (also -SA and -NC: Steps is personal and
/// free), but never -ND, since images get resized and cropped.
fn license_ok(l: &str) -> bool {
    let u = l.to_ascii_uppercase().replace('-', " ");
    let open = u.starts_with("PUBLIC DOMAIN")
        || u.starts_with("PD")
        || u.starts_with("CC0")
        || u.starts_with("CC BY");
    open && !u.split_whitespace().any(|w| w == "ND")
}

/// Fills in every open-licence picture's `data` from its file under
/// `content/lessons/img/` (`read` is handed that relative path). Returns
/// the problems: missing files and files that are too big.
pub fn embed_images(files: &mut [LessonFile], mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Vec<String> {
    use base64::Engine;
    let mut out = Vec::new();
    for f in files.iter_mut() {
        for l in &mut f.lessons {
            for (k, step) in l.steps.iter_mut().enumerate() {
                let StepBody::Figure { image: Some(p), .. } = &mut step.body else { continue };
                let file = p.file.clone();
                let at = format!("{} step {}", l.id, k + 1);
                let Some(bytes) = read(&file) else {
                    out.push(format!("{at}: content/lessons/img/{file} is missing"));
                    continue;
                };
                if bytes.len() > MAX_IMAGE_BYTES {
                    out.push(format!(
                        "{at}: {file} is {} KB; keep images under {} KB (WebP, about 800 px wide)",
                        bytes.len() / 1024,
                        MAX_IMAGE_BYTES / 1024
                    ));
                }
                let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                let mime = IMAGE_TYPES.iter().find(|(e, _)| *e == ext).map_or("image/webp", |(_, m)| m);
                p.data = format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                );
            }
        }
    }
    out
}

/// Key terms (`==term==`) per step: the marker only helps when it's rare.
pub const MAX_KEY_TERMS: usize = 3;

/// Figures can't be bigger than this (they are drawn, not pasted).
pub const MAX_SVG_BYTES: usize = 24 * 1024;

/// Checks a lesson file against the rules in `docs/LESSONS.md`.
pub fn validate(file: &LessonFile, cx: &LessonContext<'_>) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut bad = |at: &str, message: String| out.push(Problem { at: at.to_string(), message });
    let lec = cx.lecture;
    let lid = file.lecture.as_str();
    if file.schema != SCHEMA {
        bad(lid, format!("schema is {:?}, expected {SCHEMA:?}", file.schema));
    }
    if file.lecture != lec.lecture {
        bad(lid, format!("lessons are for {lid} but the lecture is {}", lec.lecture));
    }
    if file.lessons.is_empty() {
        bad(lid, "no lessons".into());
    }
    let items: BTreeSet<&str> = lec.items.iter().map(|i| i.id.as_str()).collect();
    let topics: BTreeSet<&str> = lec.topics.iter().map(|t| t.id.as_str()).collect();
    let [first, last] = lec.book.pages;
    let mut ids = BTreeSet::new();
    for (li, lesson) in file.lessons.iter().enumerate() {
        let at = lesson.id.as_str();
        if !ids.insert(at) {
            bad(at, "lesson id is used twice".into());
        }
        match at.strip_prefix(lid).and_then(|r| r.strip_prefix(".lesson")) {
            Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0') => {}
            _ => bad(at, format!("lesson id must be {lid}.lessonN")),
        }
        if lesson.title.trim().is_empty() || lesson.hook.trim().is_empty() {
            bad(at, "title and hook are required".into());
        }
        if !(2..=15).contains(&lesson.minutes) {
            bad(at, format!("minutes is {}: keep a lesson between 2 and 15 minutes", lesson.minutes));
        }
        let [a, b] = lesson.pages;
        if a > b || a < first || b > last {
            bad(at, format!("pages {a}-{b} are not inside the lecture ({first}-{last})"));
        }
        for t in &lesson.topics {
            if !topics.contains(t.as_str()) {
                bad(at, format!("topic `{t}` is not one of the lecture's topics"));
            }
        }
        if lesson.recap.is_empty() {
            bad(at, "recap is empty: end with the take-aways".into());
        }
        if lesson.steps.len() < 3 {
            bad(at, "a lesson needs at least 3 steps".into());
        }
        if !lesson.steps.iter().any(|s| s.body.is_question()) {
            bad(at, "a lesson needs at least one check, order, match, sort or case step".into());
        }
        match lesson.status {
            Status::Approved if !cx.approved.is_some_and(|a| a.contains(at)) => {
                bad(at, "status approved, but Mohamed hasn't approved it (content/approved.txt)".into());
            }
            _ => {}
        }
        for id in &lesson.practice {
            if !items.contains(id.as_str()) {
                bad(at, format!("practice item {id} is not in {lid}"));
            }
        }
        for (si, step) in lesson.steps.iter().enumerate() {
            let sat = format!("{at} step {}", si + 1);
            if let Some((_, keys)) = cx.raw_step_keys.iter().find(|(k, _)| *k == (li, si)) {
                let own = allowed_keys(step.body.kind()).unwrap_or(&[]);
                for k in keys {
                    if !own.contains(&k.as_str()) && !SHARED_STEP_KEYS.contains(&k.as_str()) {
                        bad(&sat, format!("unknown key `{k}` for a {} step", step.body.kind()));
                    }
                }
            }
            if let Some(p) = step.page
                && (p < a || p > b)
            {
                bad(&sat, format!("page {p} is outside the lesson's pages {a}-{b}"));
            }
            if step.quote.is_some() && step.page.is_none() {
                bad(&sat, "a quote needs its page".into());
            }
            if let StepBody::Figure { image: Some(p), .. } = &step.body
                && p.book
                && step.page.is_none()
            {
                bad(&sat, "a book figure needs its page".into());
            }
            for m in step_problems(&step.body, &items) {
                bad(&sat, m);
            }
        }
    }
    out
}

/// `**` and `==` in every text a step shows, checked line by line the way
/// [`spans`] reads them, and the step's key terms counted together (#321).
fn markup_problems(body: &StepBody) -> Vec<String> {
    fn strings<'a>(v: &'a serde_json::Value, key: &str, out: &mut Vec<(String, &'a str)>) {
        match v {
            // Drawings and pictures aren't lesson text.
            serde_json::Value::String(s) if !matches!(key, "svg" | "image" | "alt" | "kind") => {
                out.push((key.to_string(), s))
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, key, out)),
            serde_json::Value::Object(o) => o
                .iter()
                .filter(|(k, _)| !matches!(k.as_str(), "svg" | "image"))
                .for_each(|(k, x)| strings(x, k, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    let Ok(v) = serde_json::to_value(body) else { return out };
    let mut texts = Vec::new();
    strings(&v, "", &mut texts);
    let mut keys = 0;
    for (field, text) in texts {
        for line in text.lines() {
            if line.contains("===") {
                out.push(format!("{field}: `===` (write `==term==` with no extra =)"));
            }
            if line.matches("**").count() % 2 == 1 {
                out.push(format!("{field}: `**` is not closed on its line"));
            }
            let marks = line.matches("==").count();
            if marks % 2 == 1 {
                out.push(format!("{field}: `==` is not closed on its line"));
            }
            keys += marks / 2;
        }
    }
    if keys > MAX_KEY_TERMS {
        out.push(format!(
            "{keys} key terms (`==`): keep the marker for the {MAX_KEY_TERMS} that matter most"
        ));
    }
    out
}

fn step_problems(body: &StepBody, items: &BTreeSet<&str>) -> Vec<String> {
    let mut out = markup_problems(body);
    let empty = |s: &str| s.trim().is_empty();
    match body {
        StepBody::Explain { text, .. } | StepBody::Aside { text, .. } => {
            if empty(text) {
                out.push("text is empty".into());
            }
        }
        StepBody::Figure { svg, image, caption, alt, .. } => {
            match image {
                None => out.extend(svg_problems(svg)),
                Some(_) if !svg.trim().is_empty() => {
                    out.push("a figure has either svg or image, not both".into())
                }
                Some(p) => out.extend(picture_problems(p)),
            }
            if empty(caption) || empty(alt) {
                out.push("a figure needs a caption and an alt text".into());
            }
        }
        StepBody::Reveal { prompt, answer } => {
            if empty(prompt) || empty(answer) {
                out.push("prompt and answer are required".into());
            }
        }
        StepBody::Mnemonic { word, parts, .. } => {
            if parts.len() < 2 {
                out.push("a mnemonic needs at least 2 parts".into());
            }
            // The keys spell the word, in order (ignoring spaces and case).
            let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            let spelled: String = parts.iter().map(|p| norm(&p.key)).collect();
            if spelled != norm(word) {
                out.push(format!("the keys spell {spelled:?}, not the word {word:?}"));
            }
        }
        StepBody::Update { text, reference } => {
            if empty(text) || empty(reference) {
                out.push("an update needs text and ref (guideline and year)".into());
            } else if !reference.chars().any(|c| c.is_ascii_digit()) {
                out.push("ref must name the guideline's year".into());
            }
        }
        StepBody::Check { options, answer, why, .. } => {
            if !(3..=5).contains(&options.len()) {
                out.push(format!("a check has {} options; use 3 to 5", options.len()));
            }
            if *answer >= options.len() {
                out.push("answer is not one of the options".into());
            }
            if why.len() != options.len() {
                out.push("why needs one line per option".into());
            }
            if options.iter().collect::<BTreeSet<_>>().len() != options.len() {
                out.push("two options are the same".into());
            }
        }
        StepBody::Order { steps, .. } => {
            if !(3..=8).contains(&steps.len()) {
                out.push("order needs 3 to 8 steps".into());
            }
        }
        StepBody::Match { pairs, .. } => {
            if !(3..=6).contains(&pairs.len()) {
                out.push("match needs 3 to 6 pairs".into());
            }
            let rights: BTreeSet<&String> = pairs.iter().map(|p| &p[1]).collect();
            if rights.len() != pairs.len() {
                out.push("two pairs have the same right side".into());
            }
        }
        StepBody::Sort { bins, cards, .. } => {
            if !(2..=4).contains(&bins.len()) {
                out.push("sort needs 2 to 4 bins".into());
            }
            if !(3..=10).contains(&cards.len()) {
                out.push("sort needs 3 to 10 cards".into());
            }
            if cards.iter().any(|c| c.bin >= bins.len()) {
                out.push("a card's bin is out of range".into());
            }
            if (0..bins.len()).any(|b| !cards.iter().any(|c| c.bin == b)) {
                out.push("every bin needs a card".into());
            }
        }
        StepBody::Case { steps, .. } => {
            if steps.is_empty() {
                out.push("a case needs steps".into());
            }
            for (i, s) in steps.iter().enumerate() {
                match s.answer {
                    Some(a) if a >= s.options.len() => {
                        out.push(format!("case step {}: answer out of range", i + 1))
                    }
                    None if !s.options.is_empty() => {
                        out.push(format!("case step {}: options without an answer", i + 1))
                    }
                    _ => {}
                }
            }
        }
        StepBody::Table { columns, rows, hide, .. } => {
            if columns.len() < 2 || rows.is_empty() {
                out.push("a table needs 2+ columns and a row".into());
            }
            if rows.iter().any(|r| r.len() != columns.len()) {
                out.push("every row needs one cell per column".into());
            }
            if hide.iter().any(|[r, c]| *r >= rows.len() || *c >= columns.len()) {
                out.push("a hidden cell is outside the table".into());
            }
        }
        StepBody::Try { items: ids } => {
            if ids.is_empty() || ids.len() > 5 {
                out.push("try 1 to 5 items".into());
            }
            for id in ids {
                if !items.contains(id.as_str()) {
                    out.push(format!("item {id} is not in this lecture"));
                }
            }
        }
    }
    out
}

/// A figure must be a plain, self-contained drawing in theme colours: no
/// scripts, event handlers, links, embedded images or fixed colours (so it
/// reads in Day, Night and true black alike).
pub fn svg_problems(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let s = svg.trim();
    if !s.starts_with("<svg") || !s.ends_with("</svg>") {
        out.push("svg must be one <svg>…</svg> element".into());
    }
    if !s.contains("viewBox") {
        out.push("svg needs a viewBox (it scales to the screen)".into());
    }
    if s.len() > MAX_SVG_BYTES {
        out.push(format!("svg is {} KB; keep figures under {} KB", s.len() / 1024, MAX_SVG_BYTES / 1024));
    }
    let lower = s.to_ascii_lowercase();
    for banned in
        ["<script", "<image", "<foreignobject", "<iframe", "javascript:", "href=\"http", "@import", "<style"]
    {
        if lower.contains(banned) {
            out.push(format!("svg may not contain `{banned}`"));
        }
    }
    // on…= event handler attributes.
    for (i, w) in lower.match_indices(" on") {
        let rest = &lower[i + w.len()..];
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        if !name.is_empty() && rest[name.len()..].trim_start().starts_with('=') {
            out.push(format!("svg may not have event handlers (`on{name}`)"));
            break;
        }
    }
    for attr in ["fill=\"", "stroke=\"", "stop-color=\"", "color=\""] {
        for (i, _) in s.match_indices(attr) {
            // `stroke="` also matches inside `-stroke="`… only whole attributes.
            if i > 0 && !s.as_bytes()[i - 1].is_ascii_whitespace() {
                continue;
            }
            let value: String = s[i + attr.len()..].chars().take_while(|c| *c != '"').collect();
            let ok = value.starts_with("var(--")
                || value.starts_with("url(#")
                || matches!(value.as_str(), "none" | "currentColor" | "transparent");
            if !ok {
                out.push(format!("svg colour `{attr}{value}\"`: use var(--token), currentColor or none"));
            }
        }
    }
    for fixed in ["rgb(", "rgba(", "hsl(", "#"] {
        if fixed == "#" {
            // `#` is fine in url(#id) and href="#id"; a hex colour isn't.
            let hex = s.match_indices('#').any(|(i, _)| {
                let tail: String = s[i + 1..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                let before = &s[..i];
                matches!(tail.len(), 3 | 6 | 8) && !before.ends_with("url(") && !before.ends_with("href=\"")
            });
            if hex {
                out.push("svg has a hex colour: use var(--token) or a class".into());
            }
        } else if lower.contains(fixed) {
            out.push(format!("svg has a fixed colour `{fixed}…)`: use var(--token)"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn shown_text_covers_what_the_step_shows_and_svg_words_only() {
        let svg = r#"<svg viewBox="0 0 10 10"><text x="1" class="t-sm">HVPG &amp; MELD</text></svg>"#;
        let step = Step {
            body: StepBody::Figure {
                title: Some("Portal pressure".into()),
                svg: svg.into(),
                image: None,
                caption: "TIPS lowers it".into(),
                alt: "A gauge".into(),
            },
            page: Some(3),
            quote: Some("not shown".into()),
        };
        let t = step.shown_text();
        assert_eq!(t[0], "Portal pressure");
        assert!(t[1].contains("HVPG & MELD") && !t[1].contains("viewBox") && !t[1].contains("t-sm"), "{t:?}");
        assert_eq!(t[2], "TIPS lowers it");
        assert!(!t.iter().any(|x| x.contains("not shown")));
    }

    use super::*;

    fn latin_runs(line: &str) -> Vec<String> {
        spans(line).into_iter().filter(|s| s.latin).map(|s| s.text).collect()
    }

    #[test]
    fn terms_are_isolated_whole() {
        assert_eq!(latin_runs("الـ Child-Pugh class C معناها"), ["Child-Pugh class C"]);
        assert_eq!(latin_runs("الـ HSCs بتتنشط"), ["HSCs"]);
        assert_eq!(latin_runs("لما PaO2 <80 mmHg يبقى"), ["PaO2 <80 mmHg"]);
        assert_eq!(latin_runs("كله عربي"), Vec::<String>::new());
        assert_eq!(latin_runs("Zone 3"), ["Zone 3"]);
        assert_eq!(latin_runs("الشكل (morphology): كده"), ["(morphology)"]);
        assert_eq!(latin_runs("الـ nodules. وبعدين"), ["nodules"]);
        assert_eq!(latin_runs("الـ MELD ≥ 21 هنا"), ["MELD ≥ 21"]);
        assert_eq!(latin_runs("لازم يتحققوا (all 5 must be fulfilled). تمام"), ["(all 5 must be fulfilled)"]);
        assert_eq!(latin_runs("Low risk (≤ 2%)"), ["Low risk (≤ 2%)"]);
        assert_eq!(latin_runs("Low risk (≤ 2%) يعني"), ["Low risk (≤ 2%)"]);
        assert_eq!(latin_runs("الـ MELD (يعني"), ["MELD"]);
        assert_eq!(latin_runs("طوله حوالي 20 cm تقريبا"), ["20 cm"]);
        assert_eq!(latin_runs("20 cm"), ["20 cm"]);
        assert_eq!(latin_runs("خليه 1.5 ml/kg/h كده"), ["1.5 ml/kg/h"]);
        assert_eq!(latin_runs("حوالي 3–4 L في اليوم"), ["3–4 L"]);
        assert_eq!(latin_runs("في 24 h"), ["24 h"]);
        assert_eq!(latin_runs("من 5–6 نقط"), ["5–6"]);
        assert_eq!(latin_runs("الـ OR 7.4 هنا"), ["OR 7.4"]);
        assert_eq!(latin_runs("الـ CA 19-9 مش"), ["CA 19-9"]);
        assert_eq!(latin_runs("الـ ECOG 0–1 بس"), ["ECOG 0–1"]);
        assert_eq!(latin_runs("أكتر من (>80 g/day) كده"), ["(>80 g/day)"]);
        assert_eq!(latin_runs("لو (PMN ≥250) يبقى"), ["(PMN ≥250)"]);
        assert_eq!(latin_runs("الـ BCLC 2022"), ["BCLC 2022"]);
        assert_eq!(latin_runs("لو ≥250 cells يبقى"), ["≥250 cells"]);
        assert_eq!(latin_runs("(مع albumin 20%) كده"), ["albumin 20%"]);
        assert_eq!(latin_runs("حوالي 1/3 العيانين و 40 واحد"), ["1/3"]);
        assert_eq!(latin_runs("فيه ≤ 2%، والـ عالي > 2%:"), ["≤ 2%", "> 2%"]);
        assert_eq!(latin_runs("لو أقل من < 3 شهور"), ["< 3"]);
        assert_eq!(latin_runs("حوالي 40% منهم"), ["40%"]);
        assert_eq!(latin_runs("لو FIB-4 < 1.3 ثم كده"), ["FIB-4 < 1.3"]);
        assert_eq!(latin_runs("الـ platelets ≥ 150 (يعني"), ["platelets ≥ 150"]);
        assert_eq!(latin_runs("المرحلة (F1) دي"), ["(F1)"]);
        assert_eq!(latin_runs("**WAJH**: ≥ 2 خلال **20 يوم** بعد"), ["WAJH", "≥ 2"]);
        assert_eq!(
            latin_runs(
                "ركّز في الأرقام كأزواج: **15–45** يخفّوا و **55–85** يزمنوا. الاتنين بيكمّلوا بعض لـ 100 (45 + 55 = 100، و 15 + 85 = 100)."
            ),
            ["15–45", "55–85", "45 + 55 = 100", "15 + 85 = 100"]
        );
        assert_eq!(latin_runs("لو عدّى 12 h فكّر"), ["12 h"]);
        assert_eq!(latin_runs("حوالي 2 mm/yr بس"), ["2 mm/yr"]);
        assert_eq!(latin_runs("الـ 25(OH)D واطي"), ["25(OH)D"]);
        assert_eq!(latin_runs("لونه (black أو brown) كده"), ["black", "brown"]);
        assert_eq!(latin_runs("(الـ P في PATCH) كده"), ["P", "PATCH"]);
        // "label: value" in an Arabic line: two terms in Arabic order.
        assert_eq!(
            latin_runs("الـ edge: **sharp**، وممكن يبقى **irregular**"),
            ["edge", "sharp", "irregular"]
        );
        assert_eq!(latin_runs("Hyperfiltration: GFR 120-180، من ضغط"), ["Hyperfiltration", "GFR 120-180"]);
        assert_eq!(
            latin_runs("TCMR: I tubulitis; II arteritis كده"),
            ["TCMR", "I tubulitis", "II arteritis"]
        );
        assert_eq!(latin_runs("Bleeding → ceftriaxone 1 g لمدة"), ["Bleeding", "ceftriaxone 1 g"]);
        assert_eq!(latin_runs("Bleeding → ceftriaxone"), ["Bleeding → ceftriaxone"]);
        // Not a separator: a ratio, a time, a list.
        assert_eq!(latin_runs("نسبة 1:2 هنا"), ["1:2"]);
        assert_eq!(
            latin_runs("الحجم: **shrunken, enlarged or normal**. تمام"),
            ["shrunken, enlarged or normal"]
        );
    }

    #[test]
    fn bold_does_not_split_terms() {
        let s = spans("**Genome:** single-stranded **positive-sense RNA**. تمام");
        let latin: Vec<_> = s.iter().filter(|s| s.latin).map(|s| (s.text.as_str(), s.bold)).collect();
        assert_eq!(latin, [("Genome", true), ("single-stranded ", false), ("positive-sense RNA", true)]);
        // All English: one phrase, the colon kept inside it.
        let s = spans("**Genome:** single-stranded RNA");
        assert_eq!(
            s.iter().filter(|s| s.latin).map(|s| s.text.as_str()).collect::<String>(),
            "Genome: single-stranded RNA"
        );
        let s = spans("حوالي **58 million** عايشين");
        assert_eq!(s.iter().filter(|s| s.latin).map(|s| s.text.as_str()).collect::<Vec<_>>(), ["58 million"]);
    }

    #[test]
    fn bold_and_blocks() {
        let s = spans("ده **مهم جدا** يا دكتور");
        assert_eq!(s.iter().filter(|s| s.bold).count(), 1);
        assert_eq!(s.iter().find(|s| s.bold).unwrap().text, "مهم جدا");
        let b = blocks("سطر أول\nكمّلة\n\n- نقطة A\n- نقطة B\n\nآخر");
        assert_eq!(b.len(), 3);
        assert!(matches!(&b[1], Block::Bullets(l) if l.len() == 2));
        // A list right under a lead-in line, with no blank line between.
        let b = blocks("الأسباب:\n- Viral\n- Alcohol");
        assert!(matches!(&b[..], [Block::Para(_), Block::Bullets(l)] if l.len() == 2));
        // Numbered lines make a numbered list, also under a lead-in line.
        let b = blocks("الخطوات:\n1. Resuscitate\n2. Risk-stratify (GBS)\n3) Endoscopy");
        assert!(matches!(&b[..], [Block::Para(_), Block::Numbered(l)] if l.len() == 3), "{b:?}");
        // A text line after a list starts a new paragraph.
        let b = blocks("- A\n- B\nبعدها\n1. C");
        assert!(
            matches!(&b[..], [Block::Bullets(a), Block::Para(_), Block::Numbered(c)] if a.len() == 2 && c.len() == 1),
            "{b:?}"
        );
        // A year or a dose at the start of a line is not a list.
        assert!(matches!(&blocks("2024 guidelines.")[..], [Block::Para(_)]));
        assert!(matches!(&blocks("100. mg")[..], [Block::Para(_)]));
    }

    #[test]
    fn key_terms_and_bare_punctuation() {
        let s = spans("دي ==portal hypertension== و **SAAG**");
        let split = StepBody::Explain { title: None, text: "دي ==portal\nhypertension== كده".into() };
        assert!(super::markup_problems(&split).iter().any(|p| p.contains("not closed")));
        let cap = StepBody::Reveal { prompt: "==a== ==b==".into(), answer: "==c== و ==d==".into() };
        assert!(super::markup_problems(&cap).iter().any(|p| p.contains("4 key terms")));
        let ok = StepBody::Reveal { prompt: "**x** ==y==".into(), answer: "45 + 55 = 100".into() };
        assert!(super::markup_problems(&ok).is_empty());
        assert!(super::license_ok("CC BY-SA 4.0") && super::license_ok("CC BY-NC-SA 3.0"));
        assert!(super::license_ok("Public domain") && !super::license_ok("CC BY-ND 4.0"));
        assert!(!super::license_ok("All rights reserved"));
        let key: Vec<_> = s.iter().filter(|s| s.key).map(|s| s.text.as_str()).collect();
        assert_eq!(key, ["portal hypertension"]);
        assert!(s.iter().any(|s| s.bold && s.text == "SAAG"));
        // "**(SECs):**" and "**(SECs)**:" look the same: the colon isn't bold.
        for line in ["**cells (SECs):** دي", "**cells (SECs)**: دي"] {
            let s = spans(line);
            assert!(s.iter().all(|s| !s.bold || s.text.chars().any(char::is_alphanumeric)), "{line}: {s:?}");
        }
    }

    #[test]
    fn svg_rules() {
        let good = r#"<svg viewBox="0 0 10 10"><rect class="fg-box" fill="var(--surface)" stroke="currentColor" width="5" height="5"/><path marker-end="url(#a)" d="M0 0"/></svg>"#;
        assert!(svg_problems(good).is_empty(), "{:?}", svg_problems(good));
        assert!(!svg_problems(r##"<svg viewBox="0 0 1 1"><rect fill="#ff0000"/></svg>"##).is_empty());
        assert!(!svg_problems(r#"<svg viewBox="0 0 1 1"><rect fill="red"/></svg>"#).is_empty());
        assert!(!svg_problems(r#"<svg viewBox="0 0 1 1" onload="x()"></svg>"#).is_empty());
        assert!(!svg_problems(r#"<svg viewBox="0 0 1 1"><script>1</script></svg>"#).is_empty());
        assert!(!svg_problems(r#"<svg><g/></svg>"#).is_empty());
    }

    #[test]
    fn steps_round_trip_through_yaml() {
        let yaml = r#"
kind: check
question: Which zone first?
options: [Zone 1, Zone 2, Zone 3]
answer: 0
why: [yes, no, no]
page: 3
"#;
        let step: Step = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(step.body.is_question());
        assert_eq!(step.page, Some(3));
        let back: Step = serde_yaml_ng::from_str(&serde_yaml_ng::to_string(&step).unwrap()).unwrap();
        assert_eq!(step, back);
    }
}
