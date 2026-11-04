//! Book page references: every "ص N" chip must point at the page that holds
//! the fact, and a quote must be that page's own words about it (Mohamed,
//! 2026-10-29: "the page reference feels random").
//!
//! A reference is an item's `src` (`cvs.L04.009`) or a lesson step's
//! `page:`/`quote:` (`cvs.L04.lesson4#4`, steps counted from 1).
//!
//! The book never goes in git, so the check has two halves:
//!
//! - `steps-tools refs [--lecture cvs.L04] [--book DIR] [--ocr DIR]… [--checked DIR] [--write]`
//!   runs where the book text is (by default /mnt/project-files/steps/courses/<course>/text).
//!   Every reference carries a quote (Mohamed, 2026-10-29: every chip shows
//!   the book's line), except a lesson figure showing the book's own figure.
//!   The quote must be on the cited page in the text layer or OCR, or listed
//!   as read off the page image (`ref<TAB>page<TAB>quote-ok<TAB>the quote`);
//!   and the page must carry enough of the reference's terms (no other page
//!   far more), or a person checked it (`ref<TAB>page<TAB>ok<TAB>reason`),
//!   both in `--checked DIR/LNN.tsv`.
//!   `--write` records the result in `content/book-refs/LNN.tsv`.
//! - `steps-tools validate` (CI) needs no book: for each lecture that has a
//!   lock file, every reference must match a lock row (same page, same quote),
//!   so a changed page or quote fails until it is checked again, every
//!   reference needs a quote, and every quote must share a term with its
//!   item or step.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

const OCR: [&str; 2] = ["/mnt/project-files/steps/pagerefs/ocr", "/mnt/project-files/steps/pagerefs/ocr-all"];
const CHECKED: &str = "/mnt/project-files/steps/pagerefs/verdicts";
const LOCKS: &str = "content/book-refs";
/// Each course keeps its First Aid text, page checks and locks per course.
const COURSES: &str = "/mnt/project-files/steps/courses";

/// Where a course's references are locked:
/// `content/book-refs/<course>/`.
fn lock_dir(root: &Path, course: &str) -> PathBuf {
    root.join(LOCKS).join(course)
}

/// A course's source text by page or slide, unless `--book` names one:
/// `courses/<course>/text/` (First Aid, split per lecture).
fn source_dir(given: Option<&str>, course: &str) -> PathBuf {
    match given {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(COURSES).join(course).join("text"),
    }
}

/// A cited page must carry this share of the reference's weighted terms…
const MIN_SCORE: f64 = 0.3;
/// …and no page of the lecture may carry this much more.
const MAX_GAP: f64 = 0.2;
/// Terms a quote must share with its item or step. Lesson steps are mostly
/// Arabic around a few English terms, so one is the floor; terms match on
/// their first [`STEM`] letters ("transplant" ~ "transplantation").
const QUOTE_SHARED: usize = 1;
const STEM: usize = 6;

const STOP: &[&str] = &[
    "the",
    "and",
    "for",
    "with",
    "that",
    "this",
    "from",
    "are",
    "was",
    "were",
    "has",
    "have",
    "had",
    "not",
    "but",
    "can",
    "its",
    "into",
    "than",
    "then",
    "they",
    "them",
    "their",
    "there",
    "which",
    "who",
    "whom",
    "what",
    "when",
    "where",
    "while",
    "will",
    "would",
    "should",
    "could",
    "may",
    "might",
    "also",
    "more",
    "most",
    "less",
    "such",
    "other",
    "some",
    "any",
    "all",
    "each",
    "both",
    "only",
    "very",
    "over",
    "under",
    "about",
    "after",
    "before",
    "between",
    "during",
    "through",
    "per",
    "via",
    "due",
    "one",
    "two",
    "three",
    "use",
    "used",
    "using",
    "usually",
    "often",
    "mainly",
    "patients",
    "patient",
    "disease",
    "cases",
    "case",
    "include",
    "includes",
    "including",
    "seen",
    "common",
    "commonly",
    "cause",
    "causes",
    "caused",
    "associated",
    "based",
    "level",
    "levels",
    "high",
    "low",
    "increased",
    "decreased",
    "normal",
];

/// Keys whose values are not the reference's own words.
const SKIP_KEYS: &[&str] =
    &["id", "kind", "status", "src", "topic", "tags", "page", "quote", "image", "pages"];

#[derive(Debug, Clone, PartialEq)]
pub struct Ref {
    pub id: String,
    pub page: u32,
    pub quote: Option<String>,
    /// Everything the item or step says, for its terms.
    pub text: String,
    /// A lesson figure step showing the book's own figure (`image.book`):
    /// the picture is its proof, and a figure-only page has no line to quote.
    pub book_figure: bool,
}

/// One row of `content/book-refs/LNN.tsv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockRow {
    pub page: u32,
    pub quote: String,
    /// `quote`, `text` or `checked`.
    pub basis: String,
    pub note: String,
}

/// Lowercase letters and digits only: the quote as the page has it, whatever
/// the case, punctuation, line breaks or glued words ("factorsinclude").
pub fn squash(s: &str) -> String {
    let s = s.replace('\u{fb01}', "fi").replace('\u{fb02}', "fl");
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Drops the "o" sub-bullet marker that starts a line ("    o   Fatigue."),
/// so a quote spanning several bullets still matches the page text.
pub fn drop_o_bullets(t: &str) -> String {
    t.lines()
        .map(|l| {
            let s = l.trim_start();
            match s.strip_prefix('o') {
                Some(rest) if rest.starts_with(char::is_whitespace) => rest,
                _ => l,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A short stable hash of a quote, so the lock notices any change to it.
pub fn quote_hash(quote: Option<&str>) -> String {
    match quote.map(squash).filter(|q| !q.is_empty()) {
        None => "-".into(),
        Some(q) => {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in q.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            format!("{h:016x}")
        }
    }
}

/// The English terms of a text: words of 3+ characters that start with a
/// letter (hyphens kept), and numbers, minus very common words.
pub fn terms(s: &str) -> BTreeSet<String> {
    let s = s.replace('\u{fb01}', "fi").replace('\u{fb02}', "fl").to_lowercase();
    let chars: Vec<char> = s.chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_lowercase() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_lowercase() || chars[i].is_ascii_digit() || chars[i] == '-')
            {
                i += 1;
            }
            let w: String = chars[start..i].iter().collect();
            if w.chars().count() >= 3 && !STOP.contains(&w.as_str()) {
                out.insert(w);
            }
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            out.insert(chars[start..i].iter().collect());
        } else {
            i += 1;
        }
    }
    out
}

fn strings(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => {
            out.push_str(s);
            out.push('\n');
        }
        Value::Sequence(seq) => seq.iter().for_each(|x| strings(x, out)),
        Value::Mapping(m) => {
            for (k, x) in m {
                if !k.as_str().is_some_and(|k| SKIP_KEYS.contains(&k)) {
                    strings(x, out);
                }
            }
        }
        _ => {}
    }
}

fn text_of(v: &Value) -> String {
    let mut s = String::new();
    strings(v, &mut s);
    s
}

fn page_of(v: &Value) -> Option<u32> {
    v.as_u64().and_then(|p| u32::try_from(p).ok())
}

/// Every page reference of lecture `num` (`L01`) in `chapter` (`cvs`).
pub fn collect(root: &Path, chapter: &str, num: &str) -> Result<Vec<Ref>, String> {
    let mut out = Vec::new();
    let items = root.join("content").join(chapter).join(format!("{num}.yaml"));
    let doc: Value = crate::read_yaml(&items)?;
    for item in doc.get("items").and_then(Value::as_sequence).into_iter().flatten() {
        let (Some(id), Some(src)) = (item.get("id").and_then(Value::as_str), item.get("src")) else {
            continue;
        };
        // Only First Aid's own pages are checked against its text here; another
        // source (`src.book`) gets its own text folder when it arrives (docs/SOURCES.md).
        if src.get("book").and_then(Value::as_str).is_some_and(|b| b != steps_core::content::PRIMARY_SOURCE) {
            continue;
        }
        let Some(page) = src.get("page").and_then(page_of).filter(|p| *p > 0) else {
            continue;
        };
        let quote = src.get("quote").and_then(Value::as_str).map(String::from);
        out.push(Ref { id: id.into(), page, quote, text: text_of(item), book_figure: false });
    }
    let lessons = root.join("content/lessons").join(chapter).join(format!("{num}.yaml"));
    if lessons.exists() {
        let doc: Value = crate::read_yaml(&lessons)?;
        for lesson in doc.get("lessons").and_then(Value::as_sequence).into_iter().flatten() {
            let id = lesson.get("id").and_then(Value::as_str).unwrap_or_default();
            for (i, step) in
                lesson.get("steps").and_then(Value::as_sequence).into_iter().flatten().enumerate()
            {
                let Some(page) = step.get("page").and_then(page_of) else {
                    continue;
                };
                let quote = step.get("quote").and_then(Value::as_str).map(String::from);
                let book_figure = step.get("kind").and_then(Value::as_str) == Some("figure")
                    && step.get("image").and_then(|i| i.get("book")).and_then(Value::as_bool) == Some(true);
                out.push(Ref {
                    id: format!("{id}#{}", i + 1),
                    page,
                    quote,
                    text: text_of(step),
                    book_figure,
                });
            }
        }
    }
    Ok(out)
}

/// `ref<TAB>page<TAB>quote-hash<TAB>basis[<TAB>note]`; `#` lines are comments.
pub fn read_lock(path: &Path) -> Result<BTreeMap<String, LockRow>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let page = f.get(1).and_then(|p| p.parse().ok());
        match (f.len() >= 4, page) {
            (true, Some(page)) => {
                let row = LockRow {
                    page,
                    quote: f[2].into(),
                    basis: f[3].into(),
                    note: f.get(4).copied().unwrap_or_default().into(),
                };
                if out.insert(f[0].to_string(), row).is_some() {
                    return Err(format!("{}:{}: {} is listed twice", path.display(), n + 1, f[0]));
                }
            }
            _ => return Err(format!("{}:{}: expected ref, page, quote hash, basis", path.display(), n + 1)),
        }
    }
    Ok(out)
}

/// Whether a quote shares enough terms with what its reference says.
pub fn quote_relates(r: &Ref) -> bool {
    let Some(q) = &r.quote else { return true };
    let stem = |w: String| w.chars().take(STEM).collect::<String>();
    let q: BTreeSet<String> = terms(q).into_iter().map(stem).collect();
    let t: BTreeSet<String> = terms(&r.text).into_iter().map(stem).collect();
    // A joke or analogy says too little in English to compare.
    q.is_empty() || t.len() < 3 || q.intersection(&t).count() >= QUOTE_SHARED
}

/// The book-free half, run by `validate`: lock rows match the content, and
/// quotes relate to their items. Unwritten lectures have no content or lock yet.
pub fn check_locks(root: &Path, lectures: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for id in lectures {
        let (chapter, num) = id.split_once('.').unwrap_or((id, ""));
        let lock_path = lock_dir(root, chapter).join(format!("{num}.tsv"));
        let content_path = root.join("content").join(chapter).join(format!("{num}.yaml"));
        // Index entries for lectures not yet authored are permitted; once
        // content exists, a missing page-reference lock must fail validation.
        if !content_path.exists() {
            if lock_path.exists() {
                problems.push(format!(
                    "{}: lock exists but lecture content is missing",
                    crate::rel(root, &lock_path)
                ));
            }
            continue;
        }
        if !lock_path.exists() {
            problems.push(format!(
                "{}: missing page-reference lock; run steps-tools refs --lecture {id} --write",
                crate::rel(root, &lock_path)
            ));
            continue;
        }
        let at = crate::rel(root, &lock_path);
        let lock = match read_lock(&lock_path) {
            Ok(l) => l,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        let refs = match collect(root, chapter, num) {
            Ok(r) => r,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        let again = "check it against the book again: steps-tools refs --lecture";
        let mut seen = BTreeSet::new();
        for r in &refs {
            seen.insert(r.id.as_str());
            match lock.get(&r.id) {
                None => problems
                    .push(format!("{at}: {} (p{}) was never checked; {again} {id} --write", r.id, r.page)),
                Some(row) if row.page != r.page => problems.push(format!(
                    "{at}: {} now cites p{}, checked at p{}; {again} {id} --write",
                    r.id, r.page, row.page
                )),
                Some(row) if row.quote != quote_hash(r.quote.as_deref()) => problems.push(format!(
                    "{at}: {}'s quote changed since it was checked; {again} {id} --write",
                    r.id
                )),
                Some(_) => {}
            }
            if r.quote.is_none() && !r.book_figure {
                problems.push(format!(
                    "{at}: {} has no quote; every page chip shows the book's line that holds its fact (pagerefs/pick_quotes.py proposes one)",
                    r.id
                ));
            }
            if !quote_relates(r) {
                problems.push(format!(
                    "{at}: {}'s quote shares no term with what it says; quote the line that holds its fact",
                    r.id
                ));
            }
        }
        for stale in lock.keys().filter(|k| !seen.contains(k.as_str())) {
            problems.push(format!("{at}: {stale} is no longer in the content; {again} {id} --write"));
        }
    }
    problems
}

/// Printed page (or slide) → text for one lecture: `LNN.txt`, split at
/// `=== book page N` or `=== slide N` lines, plus any OCR files.
fn book_pages(book: &Path, ocr: &[PathBuf], num: &str) -> Result<BTreeMap<u32, String>, String> {
    let path = book.join(format!("{num}.txt"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out: BTreeMap<u32, String> = BTreeMap::new();
    let mut cur = None;
    for line in text.lines() {
        // pdftotext ends each page with a form feed, which can lead the next marker.
        let line = line.trim_start_matches('\u{c}');
        if let Some(rest) = line.strip_prefix("=== book page ").or_else(|| line.strip_prefix("=== slide ")) {
            cur = rest.split_whitespace().next().and_then(|p| p.parse().ok());
            if let Some(p) = cur {
                out.entry(p).or_default();
            }
        } else if let Some(p) = cur {
            let page = out.entry(p).or_default();
            page.push_str(line);
            page.push('\n');
        }
    }
    for (p, page) in out.iter_mut() {
        for dir in ocr {
            if let Ok(t) = std::fs::read_to_string(dir.join(format!("{p}.txt"))) {
                page.push('\n');
                page.push_str(&t);
            }
        }
    }
    Ok(out)
}

/// `ref<TAB>page<TAB>ok…` lines a person wrote after reading the page.
fn read_checked(path: &Path) -> HashMap<String, (u32, String)> {
    let mut out = HashMap::new();
    for line in std::fs::read_to_string(path).unwrap_or_default().lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if line.starts_with('#') || f.len() < 3 || f[2] != "ok" {
            continue;
        }
        if let Ok(p) = f[1].parse() {
            out.insert(f[0].to_string(), (p, f.get(3).copied().unwrap_or_default().to_string()));
        }
    }
    out
}

/// `ref<TAB>page<TAB>quote-ok<TAB>the quote` lines: a quote a person read off
/// the page image (a figure or table the text layer and OCR miss or garble).
/// It counts only while the quote is exactly that text.
fn read_image_quotes(path: &Path) -> HashMap<String, (u32, String)> {
    let mut out = HashMap::new();
    for line in std::fs::read_to_string(path).unwrap_or_default().lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if line.starts_with('#') || f.len() < 4 || f[2] != "quote-ok" {
            continue;
        }
        if let Ok(p) = f[1].parse() {
            out.insert(f[0].to_string(), (p, squash(f[3])));
        }
    }
    out
}

/// How well each page carries a reference's terms: the idf-weighted share.
fn scores(
    text: &str,
    pages: &BTreeMap<u32, BTreeSet<String>>,
    df: &HashMap<String, usize>,
) -> BTreeMap<u32, f64> {
    let n = pages.len() as f64;
    let weights: Vec<(String, f64)> = terms(text)
        .into_iter()
        .filter_map(|w| df.get(&w).map(|d| (w, ((n + 1.0) / *d as f64).ln())))
        .collect();
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    pages
        .iter()
        .map(|(p, t)| {
            let got: f64 = weights.iter().filter(|(w, _)| t.contains(w)).map(|(_, w)| w).sum();
            (*p, if total > 0.0 { got / total } else { 0.0 })
        })
        .collect()
}

pub fn run(root: &Path, args: &[String]) -> Result<String, Vec<String>> {
    let index: steps_core::content::Index =
        crate::read_yaml(&root.join("content/index.yaml")).map_err(|e| vec![e])?;
    let only = crate::flag(args, "--lecture");
    let book_flag = crate::flag(args, "--book");
    let ocr: Vec<PathBuf> = {
        let given: Vec<PathBuf> =
            args.windows(2).filter(|w| w[0] == "--ocr").map(|w| PathBuf::from(&w[1])).collect();
        if given.is_empty() { OCR.iter().map(PathBuf::from).collect() } else { given }
    };
    let checked_flag = crate::flag(args, "--checked");
    let write = args.iter().any(|a| a == "--write");

    let mut problems = Vec::new();
    let mut summary = String::new();
    for entry in index.lectures.iter().filter(|e| only.is_none_or(|o| o == e.id)) {
        let (chapter, num) = entry.id.split_once('.').unwrap_or((&entry.id, ""));
        if !root.join("content").join(chapter).join(format!("{num}.yaml")).exists() {
            continue;
        }
        // Slide text already holds the OCR of image-only slides.
        let ocr: &[PathBuf] = if chapter == "cvs" { &ocr } else { &[] };
        let pages = book_pages(&source_dir(book_flag, chapter), ocr, num).map_err(|e| vec![e])?;
        let checked_dir = match checked_flag {
            Some(dir) => PathBuf::from(dir),
            None if chapter == "cvs" => PathBuf::from(CHECKED),
            None => Path::new(COURSES).join(chapter).join("verdicts"),
        };
        let page_terms: BTreeMap<u32, BTreeSet<String>> = pages.iter().map(|(p, t)| (*p, terms(t))).collect();
        let squashed: BTreeMap<u32, String> =
            pages.iter().map(|(p, t)| (*p, squash(&drop_o_bullets(t)))).collect();
        let mut df: HashMap<String, usize> = HashMap::new();
        for t in page_terms.values() {
            for w in t {
                *df.entry(w.clone()).or_default() += 1;
            }
        }
        let lock_path = lock_dir(root, chapter).join(format!("{num}.tsv"));
        let old =
            if lock_path.exists() { read_lock(&lock_path).map_err(|e| vec![e])? } else { BTreeMap::new() };
        let checked = read_checked(&checked_dir.join(format!("{num}.tsv")));
        let image_quotes = read_image_quotes(&checked_dir.join(format!("{num}.tsv")));
        let refs = collect(root, chapter, num).map_err(|e| vec![e])?;

        let mut rows = Vec::new();
        let mut bad = Vec::new();
        let mut by_basis: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &refs {
            let qh = quote_hash(r.quote.as_deref());
            let Some(page_text) = squashed.get(&r.page) else {
                bad.push(format!("{}: p{} is not in {num}'s book text", r.id, r.page));
                continue;
            };
            let mut from_image = false;
            if let Some(q) = &r.quote {
                let q = squash(q);
                from_image = image_quotes.get(&r.id).is_some_and(|(p, iq)| *p == r.page && *iq == q);
                if !page_text.contains(&q) && !from_image {
                    let elsewhere: Vec<String> = squashed
                        .iter()
                        .filter(|(_, t)| t.contains(&q))
                        .map(|(p, _)| format!("p{p}"))
                        .collect();
                    let hint =
                        if elsewhere.is_empty() { "not in the lecture".into() } else { elsewhere.join(", ") };
                    bad.push(format!("{}: quote is not on p{} ({hint})", r.id, r.page));
                    continue;
                }
            }
            let sc = scores(&r.text, &page_terms, &df);
            let cited = sc.get(&r.page).copied().unwrap_or(0.0);
            let (best_page, best) =
                sc.iter().fold((r.page, cited), |a, (p, s)| if *s > a.1 { (*p, *s) } else { a });
            let human = old
                .get(&r.id)
                .filter(|row| row.basis == "checked" && row.page == r.page && row.quote == qh)
                .map(|row| row.note.clone())
                .or_else(|| checked.get(&r.id).filter(|(p, _)| *p == r.page).map(|(_, n)| n.clone()));
            if r.quote.is_none() && !r.book_figure {
                bad.push(format!(
                    "{}: has no quote; quote the line of p{} that holds its fact",
                    r.id, r.page
                ));
                continue;
            }
            // A quote proves the line is on the page; the page itself must
            // still be where the reference's fact is.
            let (basis, note) = if from_image {
                ("image", String::new())
            } else if cited >= MIN_SCORE && best - cited < MAX_GAP {
                (if r.quote.is_some() { "quote" } else { "figure" }, String::new())
            } else if let Some(n) = human {
                ("checked", n)
            } else {
                bad.push(format!(
                    "{}: p{} carries {:.0}% of its terms, p{best_page} {:.0}%; read the page, fix it, or list it in {}",
                    r.id,
                    r.page,
                    cited * 100.0,
                    best * 100.0,
                    checked_dir.join(format!("{num}.tsv")).display()
                ));
                continue;
            };
            *by_basis.entry(basis).or_default() += 1;
            rows.push(format!("{}\t{}\t{qh}\t{basis}\t{}", r.id, r.page, note.replace(['\t', '\n'], " ")));
        }
        let _ = writeln!(
            summary,
            "{}: {} refs, {} quoted, {} book figures, {} checked by hand, {} to look at",
            entry.id,
            refs.len(),
            by_basis.get("quote").unwrap_or(&0),
            by_basis.get("figure").unwrap_or(&0),
            by_basis.get("checked").unwrap_or(&0),
            bad.len()
        );
        if bad.is_empty() && write {
            let mut out = format!(
                "# {} page references, checked against the book by `steps-tools refs --lecture {} --write`.\n\
                 # ref\tpage\tquote hash\tbasis (quote | image | figure | checked)\tnote\n",
                entry.id, entry.id
            );
            for row in rows {
                out.push_str(&row);
                out.push('\n');
            }
            std::fs::create_dir_all(lock_dir(root, chapter)).map_err(|e| vec![e.to_string()])?;
            std::fs::write(&lock_path, out).map_err(|e| vec![format!("{}: {e}", lock_path.display())])?;
        }
        problems.extend(bad.into_iter().map(|b| format!("{}: {b}", entry.id)));
    }
    if problems.is_empty() {
        Ok(summary.trim_end().to_string())
    } else {
        problems.push(summary.trim_end().to_string());
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squash_ignores_case_punctuation_and_glued_words() {
        assert_eq!(
            squash("Equilibration of diastolic pressures"),
            squash("Equilibration of diastolicpressures")
        );
        assert_eq!(squash("(>10 mm Hg) drop"), "10mmhgdrop");
        assert_eq!(quote_hash(None), "-");
        assert_eq!(quote_hash(Some("A b")), quote_hash(Some("a, B")));
        assert_ne!(quote_hash(Some("type A")), quote_hash(Some("type B")));
    }

    #[test]
    fn o_bullets_do_not_break_a_quote() {
        let page = "Findings:\n    o   Hypotension.\n    o   Distant heart sounds.\none line";
        assert!(
            squash(&drop_o_bullets(page))
                .contains(&squash("Findings:\n  Hypotension.\n  Distant heart sounds."))
        );
        assert!(squash(&drop_o_bullets(page)).contains("oneline"));
    }

    #[test]
    fn terms_keep_numbers_and_drop_common_words() {
        let t = terms("In the patients, pulsus paradoxus is a fall > 10 mm Hg (type A)");
        assert!(t.contains("pulsus") && t.contains("10") && t.contains("paradoxus"));
        assert!(!t.contains("the") && !t.contains("patients") && !t.contains("in"));
    }

    fn r(quote: &str, text: &str) -> Ref {
        Ref { id: "x".into(), page: 305, quote: Some(quote.into()), text: text.into(), book_figure: false }
    }

    #[test]
    fn a_quote_about_another_fact_does_not_relate() {
        let stanford =
            "Stanford type A: ascending aorta, surgery. Stanford type B: descending only, beta-blockers.";
        assert!(!quote_relates(&r("Hypertension is the strongest risk factor for dissection", stanford)));
        assert!(quote_relates(&r("type A involves the ascending aorta; treatment is surgery", stanford)));
        assert!(quote_relates(&r("diastolic RV collapse", "the echo shows diastolic RV collapse")));
        assert!(quote_relates(&r("treated with surgery", "النوع A علاجه surgery")));
    }

    #[test]
    fn the_lock_catches_changed_pages_and_quotes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("content/cvs")).unwrap();
        std::fs::create_dir_all(root.join(LOCKS).join("cvs")).unwrap();
        let item = |page: u32, quote: &str| {
            format!(
                "items:\n  - id: cvs.L04.001\n    kind: card\n    front: Which part of the aorta does Stanford type A involve?\n    back: The ascending aorta.\n    src: {{ page: {page}, quote: \"{quote}\" }}\n"
            )
        };
        let quote = "involves Ascending aorta";
        std::fs::write(root.join("content/cvs/L04.yaml"), item(305, quote)).unwrap();
        let lecture = vec!["cvs.L04".to_string()];
        std::fs::write(
            root.join(LOCKS).join("cvs").join("L04.tsv"),
            format!("# header\ncvs.L04.001\t305\t{}\tquote\t\n", quote_hash(Some(quote))),
        )
        .unwrap();
        let p = check_locks(root, &lecture);
        assert!(p.is_empty(), "{p:#?}");

        std::fs::write(root.join("content/cvs/L04.yaml"), item(306, quote)).unwrap();
        let p = check_locks(root, &lecture);
        assert!(p.iter().any(|p| p.contains("now cites p306, checked at p305")), "{p:#?}");

        std::fs::write(root.join("content/cvs/L04.yaml"), item(305, "involves only descending aorta"))
            .unwrap();
        let p = check_locks(root, &lecture);
        assert!(p.iter().any(|p| p.contains("quote changed")), "{p:#?}");
    }

    #[test]
    fn the_book_check_writes_a_lock_and_flags_wrong_pages() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let book = root.join("book");
        std::fs::create_dir_all(root.join("content/cvs")).unwrap();
        std::fs::create_dir_all(&book).unwrap();
        std::fs::write(
            root.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [317, 319], chunk: 6 }\n",
        )
        .unwrap();
        std::fs::write(
            book.join("L04.txt"),
            "=== book page 317 (PDF page 339) ===\nBeck triad (hypotension, distended neck veins, distant heart sounds).\n\
             === book page 319 (PDF page 341) ===\nRheumatic heart disease affects heart valves,\nmitral > aortic >> tricuspid.\n",
        )
        .unwrap();
        let yaml = |page: u32, quote: &str| {
            format!(
                "items:\n  - id: cvs.L04.001\n    kind: card\n    front: Which valve does rheumatic heart disease affect most?\n    back: Mitral.\n    src: {{ page: {page}{quote} }}\n  - id: cvs.L04.002\n    kind: card\n    front: What is Beck triad?\n    back: Hypotension, distended neck veins, distant heart sounds.\n    src: {{ page: 317, quote: \"hypotension, distended neck veins, distant heart sounds\" }}\n"
            )
        };
        let args = |extra: &[&str]| {
            let mut a: Vec<String> =
                ["--book", book.to_str().unwrap(), "--ocr", "/nonexistent", "--checked", "/nonexistent"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            a.extend(extra.iter().map(|s| s.to_string()));
            a
        };
        let mitral = ", quote: \"mitral > aortic >> tricuspid\"";
        std::fs::write(root.join("content/cvs/L04.yaml"), yaml(319, "")).unwrap();
        let p = run(root, &args(&["--write"])).unwrap_err();
        assert!(p.iter().any(|p| p.contains("cvs.L04.001: has no quote")), "{p:#?}");
        assert!(!root.join(LOCKS).join("cvs").join("L04.tsv").exists());

        std::fs::write(root.join("content/cvs/L04.yaml"), yaml(317, mitral)).unwrap();
        let p = run(root, &args(&["--write"])).unwrap_err();
        assert!(p.iter().any(|p| p.contains("cvs.L04.001: quote is not on p317 (p319)")), "{p:#?}");

        std::fs::write(root.join("content/cvs/L04.yaml"), yaml(319, mitral)).unwrap();
        run(root, &args(&["--write"])).unwrap();
        let lock = read_lock(&root.join(LOCKS).join("cvs").join("L04.tsv")).unwrap();
        assert_eq!(lock["cvs.L04.001"].basis, "quote");
        assert_eq!(lock["cvs.L04.002"].basis, "quote");
        assert!(check_locks(root, &["cvs.L04".into()]).is_empty());
    }

    #[test]
    fn every_course_keeps_its_own_text_and_locks() {
        let root = Path::new("/r");
        assert_eq!(lock_dir(root, "cvs"), Path::new("/r/content/book-refs/cvs"));
        assert_eq!(lock_dir(root, "phs"), Path::new("/r/content/book-refs/phs"));
        assert_eq!(source_dir(None, "cvs"), Path::new(COURSES).join("cvs/text"));
        assert_eq!(source_dir(None, "phs"), Path::new(COURSES).join("phs/text"));
        assert_eq!(source_dir(Some("/x"), "phs"), Path::new("/x"));

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("L02.txt"),
            "=== book page 287 ===\nHeart anatomy\n\u{c}=== book page 288 ===\nCardiac output\n",
        )
        .unwrap();
        let pages = book_pages(dir.path(), &[], "L02").unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[&288].contains("Cardiac output"));
        assert!(!pages[&287].contains("book page 288"));
    }

    #[test]
    fn terms_keep_decimals_whole() {
        let t = terms("A QRS wider than 0.12 s, a PR over 0.2");
        assert!(t.contains("0.12") && t.contains("0.2") && t.contains("qrs"), "{t:?}");
    }

    /// A tree with the lecture `cvs.L04` written from `yaml`, and its lessons when given.
    fn tree(yaml: &str, lessons: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("content/cvs")).unwrap();
        std::fs::create_dir_all(root.join(LOCKS).join("cvs")).unwrap();
        std::fs::write(
            root.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [305, 319], chunk: 6 }\n",
        )
        .unwrap();
        std::fs::write(root.join("content/cvs/L04.yaml"), yaml).unwrap();
        if let Some(l) = lessons {
            std::fs::create_dir_all(root.join("content/lessons/cvs")).unwrap();
            std::fs::write(root.join("content/lessons/cvs/L04.yaml"), l).unwrap();
        }
        dir
    }

    const ITEMS: &str = "items:
  - id: cvs.L04.001
    kind: card
    front: Which valve does rheumatic heart disease affect most?
    back: The mitral valve.
    src: { page: 319, quote: \"mitral > aortic >> tricuspid\" }
  - id: cvs.L04.002
    kind: card
    front: What does Pathoma say about the mitral valve?
    back: Same as First Aid.
    src: { book: pathoma, page: 77 }
  - id: cvs.L04.003
    kind: card
    front: No page yet
    back: x
    src: { page: 0 }
  - { id: cvs.L04.004, kind: card, front: No src, back: x }
";

    const LESSONS: &str = "lessons:
  - id: cvs.L04.lesson1
    steps:
      - { kind: explain, text: الـ mitral valve أكتر صمام بيتأثر, page: 319, quote: \"mitral > aortic >> tricuspid\" }
      - { kind: explain, text: من غير صفحة }
      - { kind: figure, image: { file: cvs/p317.png, book: true }, caption: Beck triad, page: 317 }
";

    #[test]
    fn collect_takes_first_aid_pages_and_lesson_steps_only() {
        let dir = tree(ITEMS, Some(LESSONS));
        let refs = collect(dir.path(), "cvs", "L04").unwrap();
        let ids: Vec<&str> = refs.iter().map(|r| r.id.as_str()).collect();
        // Pathoma's page, page 0, a missing src and a step without a page are not book refs.
        assert_eq!(ids, ["cvs.L04.001", "cvs.L04.lesson1#1", "cvs.L04.lesson1#3"]);
        assert!(refs[0].text.contains("mitral valve") && !refs[0].text.contains("tricuspid"), "{refs:?}");
        assert!(refs[2].book_figure && refs[2].quote.is_none());
        assert!(collect(dir.path(), "cvs", "L09").is_err(), "no such lecture file");
    }

    #[test]
    fn a_lock_file_must_be_well_formed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("L04.tsv");
        std::fs::write(&path, "# ref\tpage\n\ncvs.L04.001\t319\tabc\tquote\tread on p319\n").unwrap();
        let lock = read_lock(&path).unwrap();
        assert_eq!(lock["cvs.L04.001"].note, "read on p319");
        std::fs::write(&path, "cvs.L04.001\t319\tabc\tquote\ncvs.L04.001\t319\tabc\tquote\n").unwrap();
        assert!(read_lock(&path).unwrap_err().ends_with(":2: cvs.L04.001 is listed twice"));
        std::fs::write(&path, "cvs.L04.001\tp319\tabc\tquote\n").unwrap();
        assert!(read_lock(&path).unwrap_err().contains(":1: expected ref, page, quote hash, basis"));
        assert!(read_lock(&dir.path().join("none.tsv")).is_err());
    }

    #[test]
    fn check_locks_names_every_way_a_lock_goes_stale() {
        let dir = tree(ITEMS, Some(LESSONS));
        let root = dir.path();
        let lock = root.join(LOCKS).join("cvs/L04.tsv");
        let lectures = ["cvs.L04".to_string(), "cvs.L05".to_string()];
        // A lecture with content needs a lock; one without content needs none.
        let p = check_locks(root, &lectures);
        assert_eq!(p.len(), 1, "{p:#?}");
        assert!(
            p[0].contains("L04.tsv: missing page-reference lock; run steps-tools refs --lecture cvs.L04")
        );
        // A lock for a lecture that has no content.
        std::fs::write(root.join(LOCKS).join("cvs/L05.tsv"), "").unwrap();
        // A broken lock.
        std::fs::write(&lock, "nonsense\n").unwrap();
        let p = check_locks(root, &lectures);
        assert!(
            p.iter().any(|m| m.contains("L05.tsv: lock exists but lecture content is missing")),
            "{p:#?}"
        );
        assert!(p.iter().any(|m| m.contains("expected ref, page, quote hash")), "{p:#?}");
        // A lock that misses a ref, lists a stale one, and a lesson step with no quote.
        let mitral = quote_hash(Some("mitral > aortic >> tricuspid"));
        std::fs::write(
            &lock,
            format!(
                "cvs.L04.001\t319\t{mitral}\tquote\t\ncvs.L04.lesson1#3\t317\t-\tfigure\t\ncvs.L04.099\t310\t-\tquote\t\n"
            ),
        )
        .unwrap();
        let p = check_locks(root, &lectures[..1]);
        assert!(p.iter().any(|m| m.contains("cvs.L04.lesson1#1 (p319) was never checked")), "{p:#?}");
        assert!(p.iter().any(|m| m.contains("cvs.L04.099 is no longer in the content")), "{p:#?}");
        assert!(!p.iter().any(|m| m.contains("lesson1#3 has no quote")), "a book figure needs none: {p:#?}");
        // A step that cites a page with no quote, and a quote about another fact.
        let lessons = "lessons:
  - id: cvs.L04.lesson1
    steps:
      - { kind: explain, text: Pulsus paradoxus is a fall in systolic pressure on inspiration in tamponade, page: 317 }
      - { kind: explain, text: Beck triad points to cardiac tamponade after chest trauma, page: 317, quote: Kussmaul sign }
";
        std::fs::write(root.join("content/lessons/cvs/L04.yaml"), lessons).unwrap();
        let p = check_locks(root, &lectures[..1]);
        assert!(p.iter().any(|m| m.contains("cvs.L04.lesson1#1 has no quote")), "{p:#?}");
        assert!(p.iter().any(|m| m.contains("lesson1#2's quote shares no term")), "{p:#?}");
        // Content that no longer parses.
        std::fs::write(root.join("content/cvs/L04.yaml"), "items: [").unwrap();
        let p = check_locks(root, &lectures[..1]);
        assert_eq!(p.len(), 1, "{p:#?}");
        assert!(p[0].contains("L04.yaml"), "{p:#?}");
    }

    #[test]
    fn ocr_and_hand_checks_join_the_book_text() {
        let dir = tempfile::tempdir().unwrap();
        let (book, ocr) = (dir.path().join("book"), dir.path().join("ocr"));
        std::fs::create_dir_all(&book).unwrap();
        std::fs::create_dir_all(&ocr).unwrap();
        std::fs::write(book.join("L04.txt"), "before any page\n=== book page 317 ===\nBeck triad\n").unwrap();
        std::fs::write(ocr.join("317.txt"), "Electrical alternans").unwrap();
        let pages = book_pages(&book, &[ocr], "L04").unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[&317].contains("Beck triad") && pages[&317].contains("Electrical alternans"));
        assert!(book_pages(&book, &[], "L09").is_err());

        let tsv = dir.path().join("L04.tsv");
        std::fs::write(
            &tsv,
            "# ref\tpage\tverdict\n\
             cvs.L04.001\t319\tok\tthe table on p319\n\
             cvs.L04.002\tp317\tok\n\
             cvs.L04.003\t317\tquote-ok\tPulsus Paradoxus!\n\
             cvs.L04.004\t317\tno\n",
        )
        .unwrap();
        let checked = read_checked(&tsv);
        assert_eq!(checked.len(), 1);
        assert_eq!(checked["cvs.L04.001"], (319, "the table on p319".to_string()));
        let quotes = read_image_quotes(&tsv);
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes["cvs.L04.003"], (317, "pulsusparadoxus".to_string()));
        assert!(read_checked(&dir.path().join("missing.tsv")).is_empty());
    }

    #[test]
    fn the_book_check_takes_image_quotes_and_hand_checks_and_names_the_rest() {
        let dir = tree(
            "items:
  - id: cvs.L04.001
    kind: card
    front: What does the echo show in cardiac tamponade?
    back: Diastolic collapse of the right ventricle.
    src: { page: 317, quote: \"RV diastolic collapse\" }
  - id: cvs.L04.002
    kind: card
    front: Which valve does rheumatic heart disease affect most?
    back: Mitral.
    src: { page: 317, quote: \"Beck triad\" }
  - id: cvs.L04.003
    kind: card
    front: Which valve does rheumatic fever scar most often?
    back: Mitral.
    src: { page: 305, quote: \"Aortic dissection\" }
  - id: cvs.L04.004
    kind: card
    front: Rheumatic valve disease
    back: Mitral first.
    src: { page: 311 }
  - id: cvs.L04.005
    kind: card
    front: Which murmur does rheumatic mitral stenosis give?
    back: A diastolic rumble after an opening snap.
    src: { page: 317, quote: \"Beck triad\" }
",
            None,
        );
        let root = dir.path();
        let (book, checked) = (root.join("book"), root.join("checked"));
        std::fs::create_dir_all(&book).unwrap();
        std::fs::create_dir_all(&checked).unwrap();
        std::fs::write(
            book.join("L04.txt"),
            "=== book page 305 ===\nAortic dissection: tearing chest pain, hypertension.\n\
             === book page 317 ===\nCardiac tamponade. Beck triad: hypotension, distended neck veins.\n\
             === book page 319 ===\nRheumatic fever: mitral valve, rheumatic heart disease, valve scarring.\n",
        )
        .unwrap();
        // 001's quote is in the echo figure (read off the image); 005 was read by a person.
        std::fs::write(
            checked.join("L04.tsv"),
            "cvs.L04.001\t317\tquote-ok\tRV diastolic collapse\ncvs.L04.005\t317\tok\tthe murmur table\n",
        )
        .unwrap();
        let args: Vec<String> = [
            "--lecture",
            "cvs.L04",
            "--book",
            book.to_str().unwrap(),
            "--ocr",
            "/nonexistent",
            "--checked",
            checked.to_str().unwrap(),
            "--write",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = run(root, &args).unwrap_err();
        assert!(p.iter().any(|m| m.contains("cvs.L04.002: p317 carries")), "{p:#?}");
        assert!(p.iter().any(|m| m.contains("p319")), "the better page is named: {p:#?}");
        assert!(p.iter().any(|m| m.contains("cvs.L04.003: p305 carries")), "{p:#?}");
        assert!(p.iter().any(|m| m.contains("cvs.L04.004: p311 is not in L04's book text")), "{p:#?}");
        assert!(!p.iter().any(|m| m.contains("cvs.L04.001") || m.contains("cvs.L04.005:")), "{p:#?}");
        let summary = p.last().unwrap();
        assert_eq!(summary, "cvs.L04: 5 refs, 0 quoted, 0 book figures, 1 checked by hand, 3 to look at");
        assert!(!root.join(LOCKS).join("cvs/L04.tsv").exists(), "nothing is written while refs are wrong");

        // Without the wrong ones, the lock records how each was proven.
        let yaml = std::fs::read_to_string(root.join("content/cvs/L04.yaml")).unwrap();
        let keep: String = yaml
            .split("  - id: ")
            .filter(|c| {
                !c.starts_with("cvs.L04.00") || c.starts_with("cvs.L04.001") || c.starts_with("cvs.L04.005")
            })
            .collect::<Vec<_>>()
            .join("  - id: ");
        std::fs::write(root.join("content/cvs/L04.yaml"), keep).unwrap();
        let out = run(root, &args).unwrap();
        assert!(out.contains("2 refs"), "{out}");
        let lock = read_lock(&root.join(LOCKS).join("cvs/L04.tsv")).unwrap();
        assert_eq!(lock["cvs.L04.001"].basis, "image");
        assert_eq!(
            (lock["cvs.L04.005"].basis.as_str(), lock["cvs.L04.005"].note.as_str()),
            ("checked", "the murmur table")
        );
        // A hand check already in the lock still counts without the checked file.
        std::fs::remove_file(checked.join("L04.tsv")).unwrap();
        std::fs::write(checked.join("L04.tsv"), "cvs.L04.001\t317\tquote-ok\tRV diastolic collapse\n")
            .unwrap();
        assert!(run(root, &args).is_ok());
        // A quote nowhere in the lecture.
        std::fs::write(
            root.join("content/cvs/L04.yaml"),
            "items:\n  - { id: cvs.L04.001, kind: card, front: a, back: b, src: { page: 317, quote: Kussmaul sign } }\n",
        )
        .unwrap();
        let p = run(root, &args).unwrap_err();
        assert!(p.iter().any(|m| m.contains("quote is not on p317 (not in the lecture)")), "{p:#?}");
        // No index: nothing to check.
        std::fs::remove_file(root.join("content/index.yaml")).unwrap();
        assert!(run(root, &args).is_err());
    }
}
