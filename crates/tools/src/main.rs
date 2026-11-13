//! steps-tools: content checks for Steps.
//!
//! ```text
//! steps-tools validate [--root DIR]   check content/ against the rules (docs/CONTENT.md)
//! steps-tools anki [--lecture cvs.L04] [--verdicts FILE] [--out DIR]   Anki deck (docs/ANKI.md)
//! steps-tools packs build [--out DIR] [--check]   the offline content pack
//! steps-tools verdicts FILE [--approve]   fix list from the app's ✎/✗ export
//! steps-tools update <keygen | manifest | sign | verify>   update feeds (docs/RELEASE.md §5)
//! steps-tools refs [--lecture cvs.L04] [--write]   page references against the book (refs.rs)
//! steps-tools qbank sync --from EXPORT.txt [--check]   UWorld QIDs per lecture (docs/QBANK.md)
//! steps-tools counts [--write | --check]   content numbers in README.md and docs/HANDBOOK.md (counts.rs)
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steps_core::content::{Index, Lecture, SCHEMA};
use steps_core::validate::{self, Context};

mod anki;
mod changes;
mod content;
mod counts;
mod ids;
mod lessons;
mod packs;
mod qbank;
mod refs;
#[cfg(test)]
mod testkit;
mod update;
mod verdicts;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = match args.iter().position(|a| a == "--root") {
        Some(i) => match args.get(i + 1) {
            Some(r) => PathBuf::from(r),
            None => return usage(),
        },
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
    };
    match args.first().map(String::as_str) {
        Some("validate") => match validate_all(&root) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(problems) => {
                for p in &problems {
                    eprintln!("{p}");
                }
                eprintln!("{} problem(s)", problems.len());
                ExitCode::FAILURE
            }
        },
        Some("anki") => report(anki::run(&root, &args[1..])),
        Some("packs") => report(packs::run(&root, &args[1..])),
        Some("verdicts") => report(verdicts::run(&root, &args[1..])),
        Some("update") => report(update::run(&root, &args[1..])),
        Some("refs") => report(refs::run(&root, &args[1..])),
        Some("ids") => report(ids::run(&root, &args[1..])),
        Some("content") => report(content::run(&root, &args[1..])),
        Some("qbank") => report(qbank::run(&root, &args[1..])),
        Some("counts") => report(counts::run(&root, &args[1..])),
        _ => usage(),
    }
}

fn report(result: Result<String, Vec<String>>) -> ExitCode {
    match result {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for p in &problems {
                eprintln!("{p}");
            }
            ExitCode::FAILURE
        }
    }
}

/// The value after `name`, if given.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: steps-tools <validate | anki | packs build | verdicts FILE | update … | refs | ids <list | check --base REF> | content <next-id | new-item | diff | preflight> | qbank sync --from FILE | counts [--write | --check]> [--root DIR] (see the top of crates/tools/src/main.rs)"
    );
    ExitCode::from(2)
}

/// Checks `content/index.yaml` and every `content/<chapter>/LNN.yaml`.
/// Returns a one-line summary, or every problem found.
pub fn validate_all(root: &Path) -> Result<String, Vec<String>> {
    let mut problems = Vec::new();
    let content = root.join("content");
    let index: Index = match read_yaml(&content.join("index.yaml")) {
        Ok(i) => i,
        Err(e) => return Err(vec![e]),
    };
    if index.schema != SCHEMA {
        problems.push(format!("content/index.yaml: schema must be {SCHEMA}"));
    }
    // The program's courses: every lecture belongs to one.
    let mut courses = 0;
    match read_yaml::<steps_core::program::Program>(&content.join("program.yaml")) {
        Ok(p) => {
            courses = p.courses.len();
            problems.extend(p.problems(&index).into_iter().map(|e| format!("content/program.yaml: {e}")));
        }
        Err(e) => problems.push(e),
    }
    let approved = read_approved(&content.join("approved.txt"));
    // The sources items cite (`src.book`); First Aid is the primary one.
    let mut known_sources = BTreeSet::new();
    match read_yaml::<steps_core::content::Sources>(&content.join("sources.yaml")) {
        Ok(s) => {
            if s.schema != steps_core::content::SOURCES_SCHEMA {
                problems.push(format!(
                    "content/sources.yaml: schema must be {}",
                    steps_core::content::SOURCES_SCHEMA
                ));
            }
            for b in &s.sources {
                if !known_sources.insert(b.id.clone()) {
                    problems.push(format!("content/sources.yaml: {} appears twice", b.id));
                }
            }
            if !known_sources.contains(steps_core::content::PRIMARY_SOURCE) {
                problems.push(format!(
                    "content/sources.yaml: the primary source {} is missing",
                    steps_core::content::PRIMARY_SOURCE
                ));
            }
        }
        Err(e) => problems.push(e),
    }

    let mut items = 0;
    let mut files = 0;
    let mut listed = BTreeSet::new();
    for entry in &index.lectures {
        listed.insert(entry.id.clone());
        let path = lecture_path(&content, &entry.id);
        if !path.exists() {
            continue; // not written yet
        }
        files += 1;
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let lec: Lecture = match serde_yaml_ng::from_str(&text) {
            Ok(l) => l,
            Err(e) => {
                problems.push(format!("{}: {e}", rel(root, &path)));
                continue;
            }
        };
        if lec.lecture != entry.id {
            problems.push(format!(
                "{}: lecture is {}, the file name says {}",
                rel(root, &path),
                lec.lecture,
                entry.id
            ));
        }
        if lec.book.pages != entry.pages {
            problems.push(format!(
                "{}: book pages {:?} differ from content/index.yaml {:?}",
                rel(root, &path),
                lec.book.pages,
                entry.pages
            ));
        }
        let cx = Context { raw_keys: raw_keys(&text), approved: Some(&approved) };
        for p in validate::lecture(&lec, &cx) {
            problems.push(format!("{}: {p}", rel(root, &path)));
        }
        for item in &lec.items {
            if !known_sources.contains(item.src.book_id()) {
                problems.push(format!(
                    "{}: {}: src.book `{}` is not in content/sources.yaml",
                    rel(root, &path),
                    item.id,
                    item.src.book_id()
                ));
            }
        }
        items += lec.items.len();
    }
    // Lecture files that the index doesn't list.
    if let Ok(dirs) = std::fs::read_dir(&content) {
        for dir in dirs.flatten().filter(|d| {
            d.path().is_dir() && !["lessons", "glossary", "qbank"].iter().any(|n| d.file_name() == *n)
        }) {
            let chapter = dir.file_name().to_string_lossy().to_string();
            for f in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if let Some(stem) = name.strip_suffix(".yaml") {
                    let id = format!("{chapter}.{stem}");
                    if !listed.contains(&id) {
                        problems.push(format!("content/{chapter}/{name}: not listed in content/index.yaml"));
                    }
                }
            }
        }
    }
    // The hover glossary (PARITY.md §6.3), when there is one: the shared
    // file plus `content/glossary/<course>.yaml`, checked as one.
    let mut terms = 0;
    let mut glossary = None;
    match read_glossary(&content) {
        Ok(Some(g)) => {
            terms = g.entries.len();
            for p in validate::glossary(&g, Some(&approved)) {
                problems.push(format!("content/glossary: {p}"));
            }
            glossary = Some(g);
        }
        Ok(None) => {}
        Err(e) => problems.extend(e),
    }
    // Lessons (docs/LESSONS.md), with every abbreviation in the glossary.
    let (_, lessons) = lessons::validate_all(root, &approved, glossary.as_ref(), &mut problems);
    // UWorld QIDs per lecture (docs/QBANK.md).
    let lecture_ids: BTreeSet<&str> = index.lectures.iter().map(|l| l.id.as_str()).collect();
    problems.extend(qbank::problems(root, &lecture_ids, &known_sources));
    // Nothing left from a template (`steps-tools content new-item`).
    problems.extend(content::unfinished(root));
    // Page references checked against the book (refs.rs).
    problems.extend(refs::check_locks(root, &listed.iter().cloned().collect::<Vec<_>>()));
    if problems.is_empty() {
        Ok(format!(
            "content OK: {courses} courses, {files} of {} lectures written, {items} items, {lessons} lessons, {terms} glossary terms",
            index.lectures.len()
        ))
    } else {
        Err(problems)
    }
}

/// `content/glossary.yaml` and every `content/glossary/<course>.yaml`, as
/// one glossary. A course file must be named after a course in
/// `content/program.yaml` and carry the same schema line.
fn read_glossary(content: &Path) -> Result<Option<steps_core::glossary::Glossary>, Vec<String>> {
    let base = content.join("glossary.yaml");
    if !base.exists() {
        return Ok(None);
    }
    let mut g = read_yaml::<steps_core::glossary::Glossary>(&base).map_err(|e| vec![e])?;
    let courses: Vec<String> = read_yaml::<steps_core::program::Program>(&content.join("program.yaml"))
        .map(|p| p.courses.into_iter().map(|c| c.id).collect())
        .unwrap_or_default();
    let mut parts: Vec<_> = std::fs::read_dir(content.join("glossary"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    parts.sort();
    let mut problems = Vec::new();
    for p in parts {
        let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        if !courses.contains(&stem) {
            problems.push(format!("content/glossary/{stem}.yaml: not a course in content/program.yaml"));
        }
        match read_yaml::<steps_core::glossary::Glossary>(&p) {
            Ok(part) if part.schema != g.schema => problems.push(format!(
                "content/glossary/{stem}.yaml: schema is {:?}, expected {:?}",
                part.schema, g.schema
            )),
            Ok(part) => g.absorb(part),
            Err(e) => problems.push(e),
        }
    }
    if problems.is_empty() { Ok(Some(g)) } else { Err(problems) }
}

/// `cvs.L04` → `content/cvs/L04.yaml`.
fn lecture_path(content: &Path, id: &str) -> PathBuf {
    let (chapter, num) = id.split_once('.').unwrap_or((id, ""));
    content.join(chapter).join(format!("{num}.yaml"))
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// One item id per line; `#` starts a comment. Missing file = nothing approved.
fn read_approved(path: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// The keys written on each item, by id, so typos like `anwser:` are caught.
fn raw_keys(text: &str) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out = std::collections::BTreeMap::new();
    let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return out;
    };
    let Some(items) = doc.get("items").and_then(|i| i.as_sequence()) else {
        return out;
    };
    for item in items {
        let Some(map) = item.as_mapping() else {
            continue;
        };
        let Some(id) = map.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let keys = map.keys().filter_map(|k| k.as_str().map(String::from)).collect();
        out.insert(id.to_string(), keys);
    }
    out
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).display().to_string()
}

#[cfg(test)]
mod tests;
