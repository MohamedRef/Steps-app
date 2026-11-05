//! Checks a lecture against the content rules (docs/CONTENT.md). Pure: the
//! tools crate reads the files and passes the parsed lecture plus the raw
//! keys of each item.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::content::{self, Body, Lecture, Status};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The item id, or the lecture id for file-level problems.
    pub at: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.message)
    }
}

/// What the validator needs besides the lecture itself.
#[derive(Debug, Default)]
pub struct Context<'a> {
    /// The keys written on each item, by item id (from the raw YAML).
    pub raw_keys: BTreeMap<String, Vec<String>>,
    /// Item ids Mohamed approved (`content/approved.txt`).
    pub approved: Option<&'a BTreeSet<String>>,
}

pub fn lecture(lec: &Lecture, cx: &Context<'_>) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut bad = |at: &str, message: String| out.push(Problem { at: at.to_string(), message });
    let lid = lec.lecture.as_str();

    if lec.schema != content::SCHEMA {
        bad(lid, format!("schema is {:?}, expected {:?}", lec.schema, content::SCHEMA));
    }
    if !is_lecture_id(lid) {
        bad(lid, "lecture id must look like cvs.L04".into());
    }
    let [first, last] = lec.book.pages;
    if first == 0 || first > last {
        bad(lid, format!("book pages {first}-{last} are not a range"));
    }
    let topics: BTreeSet<&str> = lec.topics.iter().map(|t| t.id.as_str()).collect();
    if topics.len() != lec.topics.len() {
        bad(lid, "topic ids repeat".into());
    }

    let mut seen = BTreeSet::new();
    for item in &lec.items {
        let at = item.id.as_str();
        if !seen.insert(at) {
            bad(at, "id is used twice".into());
        }
        match at.strip_prefix(lid).and_then(|r| r.strip_prefix('.')) {
            Some(n) if n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()) => {}
            _ => bad(at, format!("id must be {lid}.NNN (three digits)")),
        }
        if let Some(keys) = cx.raw_keys.get(at) {
            let own = content::allowed_keys(item.body.kind()).unwrap_or(&[]);
            for k in keys {
                if !own.contains(&k.as_str()) && !content::SHARED_KEYS.contains(&k.as_str()) {
                    bad(at, format!("unknown key `{k}` for kind {}", item.body.kind()));
                }
            }
        }
        match &item.topic {
            Some(t) if !topics.is_empty() && !topics.contains(t.as_str()) => {
                bad(at, format!("topic `{t}` is not in the lecture's topics"));
            }
            None if !topics.is_empty() => bad(at, "topic is missing".into()),
            _ => {}
        }
        if item.src.is_primary() && (item.src.page < first || item.src.page > last) {
            bad(at, format!("src.page {} is outside the lecture (pages {first}-{last})", item.src.page));
        }
        if item.status == Status::Approved && !cx.approved.is_some_and(|a| a.contains(at)) {
            bad(
                at,
                "status approved: only Mohamed approves (add the id to content/approved.txt from his export)"
                    .into(),
            );
        }
        if let Some(u) = &item.update
            && (u.text.trim().is_empty() || u.reference.trim().is_empty())
        {
            bad(at, "update needs both text and ref".into());
        }
        for message in body_problems(&item.body) {
            bad(at, message);
        }
    }
    out
}

fn body_problems(body: &Body) -> Vec<String> {
    let mut p = Vec::new();
    let empty = |s: &str| s.trim().is_empty();
    match body {
        Body::Card { front, back } => {
            if empty(front) || empty(back) {
                p.push("card needs front and back".into());
            }
        }
        Body::Cloze { text, .. } => match content::cloze_blanks(text) {
            None => p.push("cloze braces are malformed; use {{c1::answer}}".into()),
            Some(b) if b.is_empty() => p.push("cloze has no {{cN::…}} blank".into()),
            Some(b) => {
                let nums: BTreeSet<u32> = b.iter().map(|(n, _)| *n).collect();
                if nums.len() > 3 {
                    p.push("cloze has more than 3 blanks (c1–c3)".into());
                }
            }
        },
        Body::Mcq { stem, options, answer, why } => {
            if empty(stem) {
                p.push("mcq stem is empty".into());
            }
            if !(4..=5).contains(&options.len()) {
                p.push(format!("mcq needs 4 or 5 options, has {}", options.len()));
            }
            if *answer >= options.len() {
                p.push(format!("mcq answer {answer} is out of range"));
            }
            if why.len() != options.len() {
                p.push("mcq needs one `why` line per option".into());
            }
            let distinct: BTreeSet<&str> = options.iter().map(|o| o.trim()).collect();
            if distinct.len() != options.len() {
                p.push("mcq options repeat".into());
            }
        }
        Body::Enumerate { prompt, points, pass, hook } => {
            if empty(prompt) || points.len() < 2 {
                p.push("enumerate needs a prompt and at least 2 points".into());
            }
            if *pass == 0 || *pass > points.len() {
                p.push(format!("enumerate pass {pass} must be 1..={}", points.len()));
            }
            if hook.as_deref().is_some_and(empty) {
                p.push("enumerate hook is empty".into());
            }
            if let Some(n) = stated_count(prompt).filter(|n| *n != points.len()) {
                p.push(format!("enumerate prompt asks for {n} but lists {} points", points.len()));
            }
        }
        Body::Essay { prompt, outline } => {
            if empty(prompt) || outline.is_empty() {
                p.push("essay needs a prompt and an outline".into());
            }
        }
        Body::Case { stem, steps } => {
            if empty(stem) || steps.is_empty() {
                p.push("case needs a stem and at least one step".into());
            }
            for (i, s) in steps.iter().enumerate() {
                match (s.options.len(), s.answer) {
                    (0, None) => {}
                    (0, Some(_)) => p.push(format!("case step {} has an answer but no options", i + 1)),
                    (_, None) => p.push(format!("case step {} has options but no answer", i + 1)),
                    (n, Some(a)) if a >= n => p.push(format!("case step {} answer is out of range", i + 1)),
                    _ => {}
                }
                if empty(&s.why) {
                    p.push(format!("case step {} needs `why`", i + 1));
                }
            }
        }
        Body::Table { columns, rows, blanks, .. } => {
            if columns.len() < 2 || rows.is_empty() {
                p.push("table needs at least 2 columns and 1 row".into());
            }
            for (i, r) in rows.iter().enumerate() {
                if r.len() != columns.len() {
                    p.push(format!("table row {} has {} cells, expected {}", i + 1, r.len(), columns.len()));
                }
            }
            if blanks.is_empty() {
                p.push("table has no blanks".into());
            }
            for [r, c] in blanks {
                if *r >= rows.len() || *c >= columns.len() || *c == 0 {
                    p.push(format!("table blank [{r}, {c}] is outside the grid (column 0 is the label)"));
                }
            }
        }
        Body::Order { steps, .. } => {
            if steps.len() < 3 {
                p.push("order needs at least 3 steps".into());
            }
        }
    }
    p
}

/// `cvs.L04`: a lowercase chapter slug, `.L`, two digits.
pub fn is_lecture_id(id: &str) -> bool {
    let Some((chapter, num)) = id.split_once(".L") else {
        return false;
    };
    !chapter.is_empty()
        && chapter.bytes().all(|b| b.is_ascii_lowercase())
        && num.len() == 2
        && num.bytes().all(|b| b.is_ascii_digit())
}

/// Checks `content/glossary.yaml` (the hover peeks, PARITY.md §6.3): the
/// schema, one entry per spelling, short plain expansions, and no `approved`
/// that Mohamed didn't give (`glossary.TERM` in `content/approved.txt`).
pub fn glossary(g: &crate::glossary::Glossary, approved: Option<&BTreeSet<String>>) -> Vec<Problem> {
    const MAX_WORDS: usize = 25;
    let mut out = Vec::new();
    let mut bad = |at: &str, message: String| out.push(Problem { at: at.to_string(), message });
    if g.schema != crate::glossary::SCHEMA {
        bad("glossary", format!("schema is {:?}, expected {:?}", g.schema, crate::glossary::SCHEMA));
    }
    // Abbreviations match exactly and drug names ignore case
    // (`Glossary::find`), so `HBIG` and `HBIg` may both be abbreviations,
    // but a drug name must not collide with anything in any case.
    let (mut exact, mut lower, mut drugs) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for e in &g.entries {
        let at = e.id();
        let drug = e.kind == crate::glossary::Kind::Drug;
        for s in e.spellings() {
            if s.trim().is_empty() || s.chars().any(char::is_whitespace) {
                bad(&at, format!("`{s}` must be one word (the pointer picks one word)"));
            }
            let low = s.to_lowercase();
            let clash =
                !exact.insert(s.to_string()) || drugs.contains(&low) || (drug && lower.contains(&low));
            if clash {
                bad(&at, format!("`{s}` is listed twice"));
            }
            lower.insert(low.clone());
            if drug {
                drugs.insert(low);
            }
        }
        let words = e.text.split_whitespace().count();
        if words == 0 {
            bad(&at, "text is empty".into());
        } else if words > MAX_WORDS {
            bad(&at, format!("text is {words} words; keep it to {MAX_WORDS} or fewer"));
        }
        if e.status == Status::Approved && !approved.is_some_and(|a| a.contains(&at)) {
            bad(&at, "status approved: only Mohamed approves (add the id to content/approved.txt)".into());
        }
    }
    // `plain:` words are capitals that aren't abbreviations; one that has an
    // entry would hide nothing and confuse the next reader.
    let mut plain = BTreeSet::new();
    for p in &g.plain {
        if exact.contains(p.as_str()) {
            bad("glossary.plain", format!("`{p}` has an entry, so it isn't plain"));
        }
        if !plain.insert(p.as_str()) {
            bad("glossary.plain", format!("`{p}` is listed twice"));
        }
    }
    out
}

/// The count an enumerate prompt states ("Enumerate the 3 D's …", "List
/// the 11 criteria …", "Name three causes"), if any: a number, in digits
/// or as a word from two to twelve, right after enumerate/list/name/give/
/// mention/state, optionally with "the" or "all" between. A mismatch with
/// the points usually means a YAML flow list split items at commas (#556).
fn stated_count(prompt: &str) -> Option<usize> {
    const WORDS: [&str; 11] =
        ["two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    let words: Vec<String> = prompt
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .collect();
    let verb = words
        .iter()
        .position(|w| matches!(w.as_str(), "enumerate" | "list" | "name" | "give" | "mention" | "state"))?;
    let mut rest = words[verb + 1..].iter().skip_while(|w| matches!(w.as_str(), "the" | "all"));
    let n = rest.next()?;
    n.parse().ok().or_else(|| WORDS.iter().position(|w| w == n).map(|i| i + 2))
}

#[cfg(test)]
mod tests;
