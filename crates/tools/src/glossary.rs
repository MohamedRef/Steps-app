//! Where each course's abbreviations are explained (#744).
//!
//! `content/glossary/<course>.yaml` holds every abbreviation the course's
//! lessons and items show, with the meaning that course intends. The shared
//! `content/glossary.yaml` holds terms Steps uses and nothing else.
//! `steps-tools validate` therefore:
//!
//! - refuses an abbreviation a course shows that only another glossary file
//!   explains ([`outside_own_file`]): the app would show that file's
//!   meaning, which may belong to another field;
//! - prints how many shared entries nothing in the content or the UI text
//!   uses ([`unused_shared`]), so the shared file doesn't fill with leftovers.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use steps_core::content::{Body, Item, Lecture, OutlineEntry};
use steps_core::glossary::{Entry, Glossary, Kind};
use steps_core::lesson::LessonFile;

/// How many unused shared terms the report names before "…".
const NAMED: usize = 12;

/// The words an item shows: its prompt, answer, options and explanations.
/// Not its id, topic, tags or page quote.
pub fn item_texts(item: &Item) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    match &item.body {
        Body::Card { front, back } => out.extend([front.as_str(), back.as_str()]),
        Body::Cloze { text, extra } => {
            out.push(text);
            out.extend(extra.as_deref());
        }
        Body::Mcq { stem, options, why, .. } => {
            out.push(stem);
            out.extend(options.iter().chain(why).map(String::as_str));
        }
        Body::Enumerate { prompt, points, hook, .. } => {
            out.push(prompt);
            out.extend(points.iter().map(String::as_str));
            out.extend(hook.as_deref());
        }
        Body::Essay { prompt, outline } => {
            out.push(prompt);
            for entry in outline {
                match entry {
                    OutlineEntry::Heading(h) => out.push(h),
                    OutlineEntry::Section(s) => {
                        for (head, lines) in s {
                            out.push(head);
                            out.extend(lines.iter().map(String::as_str));
                        }
                    }
                }
            }
        }
        Body::Case { stem, steps } => {
            out.push(stem);
            for s in steps {
                out.extend(s.reveal.as_deref());
                out.push(&s.question);
                out.extend(s.options.iter().map(String::as_str));
                out.push(&s.why);
            }
        }
        Body::Table { title, columns, rows, .. } => {
            out.push(title);
            out.extend(columns.iter().chain(rows.iter().flatten()).map(String::as_str));
        }
        Body::Order { prompt, steps } => {
            out.push(prompt);
            out.extend(steps.iter().map(String::as_str));
        }
    }
    if let Some(u) = &item.update {
        out.push(&u.text);
    }
    out
}

/// The words a lesson file shows: each lesson's recap and every step's
/// shown text, as the «اختصارات» chip reads them.
pub fn lesson_texts(file: &LessonFile) -> Vec<String> {
    let mut out = Vec::new();
    for l in &file.lessons {
        out.extend(l.recap.iter().cloned());
        out.extend(l.steps.iter().flat_map(|s| s.shown_text()));
    }
    out
}

/// The abbreviations in `texts` that `own` (a course's glossary file) does
/// not explain, though `all` (every glossary file as one) does. A word the
/// course lists as `plain:` counts as settled, and so does a spelling its
/// file covers under another entry (`BCL-XL` for `XL`).
pub fn outside_own_file<'a, S: AsRef<str>>(all: &'a Glossary, own: &Glossary, texts: &[S]) -> Vec<&'a str> {
    let spelled: HashSet<&str> = own.entries.iter().flat_map(Entry::spellings).collect();
    let plain: HashSet<&str> = own.plain.iter().map(String::as_str).collect();
    let mut out: Vec<&str> = all
        .abbreviations_in(texts)
        .into_iter()
        .filter(|e| !e.spellings().any(|s| spelled.contains(s) || plain.contains(s)))
        .map(|e| e.term.as_str())
        .collect();
    out.sort_unstable();
    out
}

/// The shared entries that nothing in `texts` uses: abbreviations by exact
/// spelling, drug names in any case.
pub fn unused_shared<'a, S: AsRef<str>>(shared: &'a Glossary, texts: &[S]) -> Vec<&'a str> {
    let used: Vec<&Entry> = shared.abbreviations_in(texts);
    let words: HashSet<String> = texts
        .iter()
        .flat_map(|t| t.as_ref().split(|c: char| !(c.is_alphanumeric() || c == '-')))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    shared
        .entries
        .iter()
        .filter(|e| !used.iter().any(|u| std::ptr::eq(*u, *e)))
        .filter(|e| !(e.kind == Kind::Drug && e.spellings().any(|s| words.contains(&s.to_lowercase()))))
        .map(|e| e.term.as_str())
        .collect()
}

/// Reads one glossary file, or `None` when it is missing or broken (the
/// main glossary check reports those).
fn read(path: &Path) -> Option<Glossary> {
    serde_yaml_ng::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Every text the app can show outside the glossary itself: the content
/// files as written (so titles, goals, page quotes, lab names and video
/// titles count as uses) and the quoted strings of the UI text.
fn everything(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let mut paths: Vec<_> =
            std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "glossary") {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|e| e == "yaml")
                && p.file_name().is_some_and(|n| n != "glossary.yaml")
            {
                out.extend(std::fs::read_to_string(&p));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("content"), &mut out);
    for f in std::fs::read_dir(root.join("crates/app/src/i18n")).into_iter().flatten().flatten() {
        if let Ok(text) = std::fs::read_to_string(f.path()) {
            // The string literals: every second piece between double quotes.
            out.extend(text.split('"').skip(1).step_by(2).map(String::from));
        }
    }
    out
}

/// Checks every course against its own glossary file and the shared file
/// against the content. Returns the line `validate` prints and the problems.
pub fn check(root: &Path, all: &Glossary, lectures: &[Lecture]) -> (String, Vec<String>) {
    let content = root.join("content");
    let mut problems = Vec::new();
    // What each course shows in its lessons and items.
    let mut shown: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for lec in lectures {
        let course = steps_core::program::course_of(&lec.lecture).to_string();
        let texts = shown.entry(course).or_default();
        for item in &lec.items {
            texts.extend(item_texts(item).into_iter().map(String::from));
        }
    }
    for (_, file, _) in crate::lessons::load(root).unwrap_or_default() {
        let course = steps_core::program::course_of(&file.lecture).to_string();
        shown.entry(course).or_default().extend(lesson_texts(&file));
    }
    for (course, texts) in &shown {
        let Some(own) = read(&content.join(format!("glossary/{course}.yaml"))) else {
            continue;
        };
        let outside = outside_own_file(all, &own, texts);
        if !outside.is_empty() {
            problems.push(format!(
                "content/glossary/{course}.yaml: {course} lessons or items show {}, explained only in another glossary file; add each here with the meaning {course} intends (or under plain: if it isn't an abbreviation)",
                outside.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    let report = match read(&content.join("glossary.yaml")) {
        Some(shared) => {
            let unused = unused_shared(&shared, &everything(root));
            let n = shared.entries.len();
            match unused.len() {
                0 => format!("glossary: {n} shared terms, each used by some content or screen text"),
                k if k <= NAMED => format!(
                    "glossary: {n} shared terms, {k} that no content or screen text uses ({})",
                    unused.join(", ")
                ),
                k => format!(
                    "glossary: {n} shared terms, {k} that no content or screen text uses ({}, …)",
                    unused[..NAMED].join(", ")
                ),
            }
        }
        None => "glossary: no shared file".to_string(),
    };
    (report, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glossary(yaml: &str) -> Glossary {
        serde_yaml_ng::from_str(&format!("schema: steps.glossary/v1\n{yaml}")).unwrap()
    }

    fn all() -> Glossary {
        glossary(
            "entries:
  - { term: LA, kind: abbr, text: Lupus anticoagulant, status: unverified }
  - { term: AG, kind: abbr, text: Anion gap, status: unverified }
  - { term: XL, kind: abbr, text: Extra-large, status: unverified }
  - { term: BCL-XL, kind: abbr, text: B-cell lymphoma-extra large, status: unverified }
  - { term: PPI, aliases: [PPIs], kind: abbr, text: Proton pump inhibitor, status: unverified }
  - { term: octreotide, kind: drug, text: Somatostatin analog, status: unverified }
  - { term: KDOQI, kind: abbr, text: A kidney guideline group, status: unverified }",
        )
    }

    #[test]
    fn a_term_only_another_file_explains_is_named() {
        let own = glossary(
            "entries:\n  - { term: PPI, kind: abbr, text: Proton pump inhibitor, status: unverified }",
        );
        let texts = ["The LA fills, then PPIs."];
        assert_eq!(outside_own_file(&all(), &own, &texts), ["LA"]);
        // Once the course explains it, nothing is left.
        let own = glossary(
            "entries:
  - { term: PPI, kind: abbr, text: Proton pump inhibitor, status: unverified }
  - { term: LA, kind: abbr, text: Left atrium, status: unverified }",
        );
        assert!(outside_own_file(&all(), &own, &texts).is_empty());
    }

    #[test]
    fn plain_words_and_longer_own_spellings_count_as_explained() {
        let own = glossary(
            "entries:\n  - { term: BCL-XL, kind: abbr, text: B-cell lymphoma-extra large, status: unverified }\nplain: [AG]",
        );
        let texts = ["Introns end in AG.", "Opposed by BCL-XL."];
        assert!(outside_own_file(&all(), &own, &texts).is_empty());
        // Without them both words read as another file's abbreviation.
        let bare = glossary("entries: []");
        assert_eq!(outside_own_file(&all(), &bare, &texts), ["AG", "BCL-XL"]);
    }

    #[test]
    fn shared_entries_nothing_uses_are_listed() {
        let texts = ["LA and AG; Octreotide for varices; BCL-XL; XL sizes; PPIs."];
        assert_eq!(unused_shared(&all(), &texts), ["KDOQI"]);
        assert_eq!(unused_shared(&all(), &["LA"]).len(), 6);
    }
}
