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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Lecture;

    fn lec(yaml: &str) -> Lecture {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    const HEAD: &str = "schema: steps.content/v1\nlecture: cvs.L04\ntitle: Pathology\nbook: { chapter: 1, pages: [300, 320] }\n";

    #[test]
    fn the_sample_fixture_is_clean() {
        let raw = include_str!("../../../fixtures/content/sample.yaml");
        let l: Lecture = serde_yaml_ng::from_str(raw).unwrap();
        let problems = lecture(&l, &Context::default());
        assert!(problems.is_empty(), "{problems:#?}");
        let kinds: BTreeSet<_> = l.items.iter().map(|i| i.body.kind()).collect();
        assert_eq!(kinds.len(), 8, "the fixture should show every kind");
    }

    #[test]
    fn catches_the_common_mistakes() {
        let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 30 }}, status: unverified }}
  - {{ id: cvs.L04.001, kind: mcq, stem: s, options: [a, b, c], answer: 3, why: [x], src: {{ page: 302 }}, status: approved }}
  - {{ id: cvs.L1.2, kind: cloze, text: 'no blank', src: {{ page: 302 }}, status: unverified }}
"
        ));
        let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
        let has = |s: &str| msgs.iter().any(|m| m.contains(s));
        assert!(has("outside the lecture"), "{msgs:#?}");
        assert!(has("used twice"));
        assert!(has("4 or 5 options"));
        assert!(has("out of range"));
        assert!(has("one `why` line"));
        assert!(has("only Mohamed approves"));
        assert!(has("three digits"));
        assert!(has("no {{cN"));
    }

    #[test]
    fn enumerate_hooks_read_and_must_say_something() {
        let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: enumerate, prompt: p, points: [a, b], pass: 2, hook: 'حكاية', src: {{ page: 303 }}, status: unverified }}
  - {{ id: cvs.L04.002, kind: enumerate, prompt: p, points: [a, b], pass: 2, hook: ' ', src: {{ page: 303 }}, status: unverified }}
"
        ));
        assert!(matches!(&l.items[0].body, Body::Enumerate { hook: Some(h), .. } if h == "حكاية"));
        let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
        assert_eq!(msgs.len(), 1, "{msgs:#?}");
        assert!(msgs[0].contains("hook is empty"));
    }

    #[test]
    fn approved_is_fine_when_mohamed_approved_it() {
        let l = lec(&format!(
            "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 303 }}, status: approved }}\n"
        ));
        let ok: BTreeSet<String> = ["cvs.L04.001".to_string()].into();
        let cx = Context { approved: Some(&ok), ..Context::default() };
        assert!(lecture(&l, &cx).is_empty());
    }

    #[test]
    fn unknown_keys_are_flagged_and_long_quotes_are_not() {
        let l = lec(&format!(
            "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 303, quote: '{}' }}, status: unverified }}\n",
            "word ".repeat(120)
        ));
        let cx = Context {
            raw_keys: [("cvs.L04.001".to_string(), vec!["front".into(), "back".into(), "answer".into()])]
                .into(),
            ..Context::default()
        };
        let msgs: Vec<String> = lecture(&l, &cx).iter().map(|p| p.to_string()).collect();
        assert!(msgs.iter().any(|m| m.contains("unknown key `answer`")), "{msgs:#?}");
        // No length cap on quotes (Mohamed, 2026-10-29): a whole table is fine.
        assert!(!msgs.iter().any(|m| m.contains("quote")), "{msgs:#?}");
    }

    #[test]
    fn lecture_ids() {
        assert!(is_lecture_id("cvs.L04"));
        assert!(is_lecture_id("gi.L11"));
        assert!(!is_lecture_id("cvs.L1"));
        assert!(!is_lecture_id("CVS.L01"));
        assert!(!is_lecture_id("cvsL01"));
    }

    #[test]
    fn glossary_rules() {
        let g: crate::glossary::Glossary = serde_yaml_ng::from_str(
            "schema: steps.glossary/v1
entries:
  - { term: MI, kind: abbr, text: Myocardial infarction, status: unverified }
  - { term: mi, kind: drug, text: Duplicate spelling, status: unverified }
  - { term: two words, kind: drug, text: x, status: approved }",
        )
        .unwrap();
        let msgs: Vec<String> = glossary(&g, None).iter().map(|p| p.to_string()).collect();
        assert!(msgs.iter().any(|m| m.contains("listed twice")), "{msgs:#?}");
        assert!(msgs.iter().any(|m| m.contains("must be one word")));
        assert!(msgs.iter().any(|m| m.contains("only Mohamed approves")));
        let ok: BTreeSet<String> = ["glossary.two words".to_string()].into();
        assert_eq!(glossary(&g, Some(&ok)).len(), 2);
    }

    #[test]
    fn file_level_mistakes() {
        let l = lec("schema: steps.content/v0
lecture: CVS.4
title: Pathology
book: { chapter: 1, pages: [320, 300] }
topics:
  - { id: shock, title: Shock }
  - { id: shock, title: Shock again }
items:
  - { id: CVS.4.001, kind: card, topic: murmurs, front: ' ', back: b, src: { page: 310 }, status: unverified, update: { text: ' ', ref: AHA 2025 } }
  - { id: CVS.4.002, kind: card, front: a, back: b, src: { page: 310 }, status: unverified }
");
        let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
        for want in [
            "CVS.4: schema is \"steps.content/v0\", expected \"steps.content/v1\"",
            "CVS.4: lecture id must look like cvs.L04",
            "CVS.4: book pages 320-300 are not a range",
            "CVS.4: topic ids repeat",
            "CVS.4.001: topic `murmurs` is not in the lecture's topics",
            "CVS.4.002: topic is missing",
            "CVS.4.001: update needs both text and ref",
            "CVS.4.001: card needs front and back",
        ] {
            assert!(msgs.iter().any(|m| m == want), "{want}: {msgs:#?}");
        }
    }

    #[test]
    fn every_kind_has_its_own_shape_rules() {
        let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: cloze, text: 'Beck triad: {{{{c1::hypotension', src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.002, kind: cloze, text: '{{{{c1::a}}}} {{{{c2::b}}}} {{{{c3::c}}}} {{{{c4::d}}}}', src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.003, kind: mcq, stem: ' ', options: [Tamponade, Tamponade, MI, PE], answer: 0, why: [a, b, c, d], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.004, kind: enumerate, prompt: J♥NES, points: [Carditis], pass: 3, src: {{ page: 319 }}, status: unverified }}
  - {{ id: cvs.L04.005, kind: essay, prompt: Aortic dissection, outline: [], src: {{ page: 305 }}, status: unverified }}
  - id: cvs.L04.006
    kind: case
    stem: A 62-year-old man with tearing chest pain.
    steps:
      - {{ question: Diagnosis?, answer: 0, why: Dissection }}
      - {{ question: Next?, options: [CT, Echo], why: CT angiography }}
      - {{ question: Treatment?, options: [Surgery], answer: 4, why: ' ' }}
    src: {{ page: 305 }}
    status: unverified
  - {{ id: cvs.L04.007, kind: table, title: Shock, columns: [Type], rows: [[Cardiogenic, ↑]], blanks: [], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.008, kind: table, title: Shock, columns: [Type, PCWP], rows: [[Cardiogenic, ↑]], blanks: [[0, 0]], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.009, kind: order, prompt: Pulsus paradoxus, steps: [Inspiration, Less LV output], src: {{ page: 317 }}, status: unverified }}
"
        ));
        let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
        for want in [
            "cvs.L04.001: cloze braces are malformed",
            "cvs.L04.002: cloze has more than 3 blanks",
            "cvs.L04.003: mcq stem is empty",
            "cvs.L04.003: mcq options repeat",
            "cvs.L04.004: enumerate needs a prompt and at least 2 points",
            "cvs.L04.004: enumerate pass 3 must be 1..=1",
            "cvs.L04.005: essay needs a prompt and an outline",
            "cvs.L04.006: case step 1 has an answer but no options",
            "cvs.L04.006: case step 2 has options but no answer",
            "cvs.L04.006: case step 3 answer is out of range",
            "cvs.L04.006: case step 3 needs `why`",
            "cvs.L04.007: table needs at least 2 columns and 1 row",
            "cvs.L04.007: table row 1 has 2 cells, expected 1",
            "cvs.L04.007: table has no blanks",
            "cvs.L04.008: table blank [0, 0] is outside the grid",
            "cvs.L04.009: order needs at least 3 steps",
        ] {
            assert!(msgs.iter().any(|m| m.starts_with(want)), "{want}: {msgs:#?}");
        }
        let l = lec(&format!(
            "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: case, stem: ' ', steps: [], src: {{ page: 305 }}, status: unverified }}\n"
        ));
        let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
        assert_eq!(msgs, ["cvs.L04.001: case needs a stem and at least one step"]);
    }

    #[test]
    fn glossary_schema_text_and_plain_words() {
        let long = "word ".repeat(26);
        let g: crate::glossary::Glossary = serde_yaml_ng::from_str(&format!(
            "schema: steps.glossary/v0
entries:
  - {{ term: PDA, kind: abbr, text: '', status: unverified }}
  - {{ term: HOCM, kind: abbr, text: '{long}', status: unverified }}
plain: [PDA, NOT, NOT]"
        ))
        .unwrap();
        let msgs: Vec<String> = glossary(&g, None).iter().map(|p| p.to_string()).collect();
        for want in [
            "glossary: schema is \"steps.glossary/v0\", expected \"steps.glossary/v1\"",
            "glossary.PDA: text is empty",
            "glossary.HOCM: text is 26 words; keep it to 25 or fewer",
            "glossary.plain: `PDA` has an entry, so it isn't plain",
            "glossary.plain: `NOT` is listed twice",
        ] {
            assert!(msgs.iter().any(|m| m == want), "{want}: {msgs:#?}");
        }
    }
}
