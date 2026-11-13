//! `steps-tools counts`: the content numbers quoted in `README.md` and
//! `docs/HANDBOOK.md` §12, worked out from `content/` so they never have to
//! be counted by hand.
//!
//! - `counts` prints them.
//! - `counts --write` rewrites the text between `<!-- counts -->` and
//!   `<!-- /counts -->` in both files.
//! - `counts --check` fails when a number there is more than
//!   [`TOLERANCE_PCT`] % away from the content. The numbers are rounded
//!   ("about"), so a few new items don't make them wrong; content pull
//!   requests stay inside their course folders, and the Nightly checks
//!   catch the drift instead (docs/RUN.md § Counts).

use std::collections::BTreeMap;
use std::path::Path;

use steps_core::content::{Index, Lecture};

type Res<T> = Result<T, Vec<String>>;

/// The files that quote the numbers, each with its own wording.
pub const FILES: [(&str, Style); 2] =
    [("README.md", Style::ReadmeTable), ("docs/HANDBOOK.md", Style::HandbookLine)];

pub const START: &str = "<!-- counts -->";
pub const END: &str = "<!-- /counts -->";

/// How far a quoted number may drift from the content before `--check` fails.
pub const TOLERANCE_PCT: u64 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    ReadmeTable,
    HandbookLine,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Items by kind (`card`, `mcq`, …).
    pub kinds: BTreeMap<&'static str, u64>,
    pub items: u64,
    pub lessons: u64,
    pub terms: u64,
}

impl Counts {
    fn kind(&self, k: &str) -> u64 {
        self.kinds.get(k).copied().unwrap_or(0)
    }
}

pub fn run(root: &Path, args: &[String]) -> Res<String> {
    let counts = count(root)?;
    match args.first().map(String::as_str) {
        None => Ok(summary(&counts)),
        Some("--write") => {
            let mut changed = Vec::new();
            for (file, style) in FILES {
                let path = root.join(file);
                let text = std::fs::read_to_string(&path).map_err(|e| vec![format!("{file}: {e}")])?;
                let new = replace_block(&text, &render(&counts, style))
                    .map_err(|e| vec![format!("{file}: {e}")])?;
                if new != text {
                    std::fs::write(&path, new).map_err(|e| vec![format!("{file}: {e}")])?;
                    changed.push(file);
                }
            }
            Ok(if changed.is_empty() {
                format!("counts already current: {}", summary(&counts))
            } else {
                format!("updated {}: {}", changed.join(", "), summary(&counts))
            })
        }
        Some("--check") => {
            let mut problems = Vec::new();
            for (file, style) in FILES {
                let text =
                    std::fs::read_to_string(root.join(file)).map_err(|e| vec![format!("{file}: {e}")])?;
                if let Err(e) = check_block(&text, &render(&counts, style)) {
                    problems.push(format!("{file}: {e}; run `just counts` (steps-tools counts --write)"));
                }
            }
            if problems.is_empty() { Ok(format!("counts OK: {}", summary(&counts))) } else { Err(problems) }
        }
        Some(_) => Err(vec!["usage: steps-tools counts [--write | --check]".into()]),
    }
}

/// Items by kind in every written lecture, lessons and glossary terms.
pub fn count(root: &Path) -> Res<Counts> {
    let content = root.join("content");
    let index: Index = crate::read_yaml(&content.join("index.yaml")).map_err(|e| vec![e])?;
    let mut c = Counts::default();
    for entry in &index.lectures {
        let path = crate::lecture_path(&content, &entry.id);
        if !path.exists() {
            continue;
        }
        let lec: Lecture = crate::read_yaml(&path).map_err(|e| vec![e])?;
        for item in &lec.items {
            *c.kinds.entry(item.body.kind()).or_default() += 1;
            c.items += 1;
        }
    }
    c.lessons = crate::lessons::load(root)?.iter().map(|(_, f, _)| f.lessons.len() as u64).sum();
    c.terms = crate::read_glossary(&content)?.map_or(0, |g| g.entries.len() as u64);
    Ok(c)
}

fn summary(c: &Counts) -> String {
    let kinds: Vec<String> = c.kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    format!("{} items ({}), {} lessons, {} glossary terms", c.items, kinds.join(", "), c.lessons, c.terms)
}

/// Rounded for prose: exact under 100, else to the nearest ten, with
/// thousands separated (`10,030`).
pub fn about(n: u64) -> String {
    let n = if n < 100 { n } else { (n + 5) / 10 * 10 };
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The text that goes between the markers.
pub fn render(c: &Counts, style: Style) -> String {
    let a = about;
    match style {
        Style::ReadmeTable => format!(
            "\n| | |\n|---|---|\n\
             | Cards | about {} |\n\
             | Step 1 style questions (single best answer, A–E) | about {} |\n\
             | Clozes | about {} |\n\
             | Tables and lists to recall | about {} |\n\
             | Steps to put in order, and cases | about {} |\n\
             | Lessons in Egyptian Arabic, with drawings and try-it steps | about {} |\n\
             | Glossary terms explained on hover | about {} |\n\
             | **All study items** | **about {}** |\n",
            a(c.kind("card")),
            a(c.kind("mcq")),
            a(c.kind("cloze")),
            a(c.kind("table") + c.kind("enumerate")),
            a(c.kind("order") + c.kind("case") + c.kind("essay")),
            a(c.lessons),
            a(c.terms),
            a(c.items),
        ),
        Style::HandbookLine => format!(
            "about {} items ({} cards, {} MCQs, {} clozes, {} tables, {} lists, {} orders, {} cases), {} lessons and {} glossary terms",
            a(c.items),
            a(c.kind("card")),
            a(c.kind("mcq")),
            a(c.kind("cloze")),
            a(c.kind("table")),
            a(c.kind("enumerate")),
            a(c.kind("order")),
            a(c.kind("case")),
            a(c.lessons),
            a(c.terms),
        ),
    }
}

/// Where the block's text sits in `text`.
fn block(text: &str) -> Result<std::ops::Range<usize>, String> {
    let start = text.find(START).ok_or_else(|| format!("no `{START}` marker"))? + START.len();
    let end = text[start..].find(END).ok_or_else(|| format!("no `{END}` after `{START}`"))? + start;
    if text[end + END.len()..].contains(START) {
        return Err(format!("more than one `{START}` block"));
    }
    Ok(start..end)
}

pub fn replace_block(text: &str, body: &str) -> Result<String, String> {
    let r = block(text)?;
    Ok(format!("{}{body}{}", &text[..r.start], &text[r.end..]))
}

/// Every number in the block within the tolerance of the fresh one, in
/// the same order (a reworded block must be rewritten with `--write`).
pub fn check_block(text: &str, fresh: &str) -> Result<(), String> {
    let r = block(text)?;
    let old = numbers(&text[r]);
    let new = numbers(fresh);
    if old.len() != new.len() {
        return Err("the counts block is not in the generated form".into());
    }
    let off: Vec<String> = old
        .iter()
        .zip(&new)
        .filter(|(o, n)| o.abs_diff(**n) * 100 > **n * TOLERANCE_PCT)
        .map(|(o, n)| format!("{o} (now {n})"))
        .collect();
    if off.is_empty() {
        Ok(())
    } else {
        Err(format!("out of date by more than {TOLERANCE_PCT}%: {}", off.join(", ")))
    }
}

/// The numbers in prose, `10,030` read as one (the `1` of `Step 1` too:
/// it is the same in both, so it lines up).
fn numbers(s: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut cur: Option<u64> = None;
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if let Some(d) = ch.to_digit(10) {
            cur = Some(cur.unwrap_or(0) * 10 + u64::from(d));
        } else if ch == ',' && cur.is_some() && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
            continue;
        } else if let Some(n) = cur.take() {
            out.push(n);
        }
    }
    out.extend(cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{args, content_tree, write};

    fn sample() -> Counts {
        let mut c = Counts { items: 10_025, lessons: 359, terms: 2_867, ..Counts::default() };
        for (k, n) in [
            ("card", 5_441),
            ("mcq", 1_708),
            ("cloze", 1_167),
            ("table", 979),
            ("enumerate", 581),
            ("order", 143),
            ("case", 6),
        ] {
            c.kinds.insert(k, n);
        }
        c
    }

    #[test]
    fn rounds_for_prose() {
        assert_eq!(about(6), "6");
        assert_eq!(about(99), "99");
        assert_eq!(about(143), "140");
        assert_eq!(about(359), "360");
        assert_eq!(about(10_025), "10,030");
        assert_eq!(about(1_234_567), "1,234,570");
        assert_eq!(numbers("about 10,030 items, 5,440 cards, Step 1, A, 7"), vec![10_030, 5_440, 1, 7]);
    }

    #[test]
    fn handbook_line_reads_well() {
        assert_eq!(
            render(&sample(), Style::HandbookLine),
            "about 10,030 items (5,440 cards, 1,710 MCQs, 1,170 clozes, 980 tables, 580 lists, 140 orders, 6 cases), 360 lessons and 2,870 glossary terms"
        );
    }

    #[test]
    fn check_allows_small_drift_and_catches_large() {
        let doc = |body: &str| format!("intro\n{START}{body}{END}\nrest\n");
        let c = sample();
        let current = doc(&render(&c, Style::HandbookLine));
        assert_eq!(check_block(&current, &render(&c, Style::HandbookLine)), Ok(()));
        // A handful of new MCQs: still "about" right.
        let mut more = sample();
        *more.kinds.get_mut("mcq").unwrap() += 40;
        assert_eq!(check_block(&current, &render(&more, Style::HandbookLine)), Ok(()));
        // A tenth more: out of date, and the message says which.
        *more.kinds.get_mut("mcq").unwrap() += 200;
        let err = check_block(&current, &render(&more, Style::HandbookLine)).unwrap_err();
        assert!(err.contains("1710 (now 1950)"), "{err}");
        // Missing or doubled markers.
        assert!(check_block("no block", "x").is_err());
        assert!(block(&format!("{}{}", doc("a"), doc("b"))).is_err());
        assert!(check_block(&doc("about 3 items"), &render(&c, Style::HandbookLine)).is_err());
        // Writing keeps everything around the block.
        let written = replace_block(&doc("old"), "new").unwrap();
        assert_eq!(written, format!("intro\n{START}new{END}\nrest\n"));
    }

    #[test]
    fn counts_a_content_tree_and_writes_both_files() {
        let dir = content_tree();
        let root = dir.path();
        let c = count(root).unwrap();
        assert!(c.items > 0);
        assert_eq!(c.items, c.kinds.values().sum::<u64>());
        assert_eq!((c.lessons, c.terms), (0, 0));
        for (file, _) in FILES {
            write(root, file, &format!("# {file}\n{START}stale 1{END}\n"));
        }
        assert!(run(root, &args(&["--check"])).is_err());
        assert!(run(root, &args(&["--write"])).unwrap().starts_with("updated"));
        assert!(run(root, &args(&["--check"])).unwrap().starts_with("counts OK"));
        assert!(run(root, &args(&["--write"])).unwrap().starts_with("counts already current"));
        assert!(run(root, &args(&[])).unwrap().contains("items"));
        assert!(run(root, &args(&["--bogus"])).is_err());
        let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("| **All study items** |"), "{readme}");
    }

    #[test]
    fn repo_docs_quote_current_counts() {
        // Both files carry a counts block in the generated form. How far
        // the numbers may drift is the Nightly checks' job (`--check`),
        // so this does not depend on the size of the content.
        let root = crate::testkit::repo_root();
        for (file, style) in FILES {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let r = block(&text).unwrap();
            assert_eq!(numbers(&text[r]).len(), numbers(&render(&sample(), style)).len(), "{file}");
        }
    }
}
