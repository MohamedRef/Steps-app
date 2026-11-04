//! `steps-tools content`: the authoring workflow (`docs/AUTHORING.md`).
//!
//! - `content next-id LECTURE` prints the next free item and lesson ids of
//!   a lecture (ids are never reused, so this skips retired ones too).
//! - `content new-item LECTURE KIND` prints a ready-to-paste item of that
//!   kind with the next free id, built from the worked example in
//!   `fixtures/content/sample.yaml`; every text in it starts with
//!   `TODO(author)`, which `validate` refuses until replaced.
//! - `content diff --base REF | --base-pack FILE` lists what a change adds,
//!   edits or removes, per lecture, as devices will see it.
//! - `content preflight [--base REF]` runs every check a content change has
//!   to pass and prints one table.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use steps_core::content::{Item, Lecture};
use steps_core::pack::Pack;

type Res<T> = Result<T, Vec<String>>;

/// Where the author still has to write something.
pub const TEMPLATE_MARK: &str = "TODO(author)";

pub fn run(root: &Path, args: &[String]) -> Res<String> {
    let arg = |i: usize| args.get(i).map(String::as_str);
    match arg(0) {
        Some("next-id") => {
            let lecture = arg(1).ok_or_else(|| vec!["usage: steps-tools content next-id cvs.L04".to_owned()])?;
            let (item, lesson) = next_ids(root, lecture)?;
            Ok(format!("next item id: {item}\nnext lesson id: {lesson}"))
        }
        Some("new-item") => match (arg(1), arg(2)) {
            (Some(lecture), Some(kind)) => new_item(root, lecture, kind),
            _ => Err(vec!["usage: steps-tools content new-item cvs.L04 <card|cloze|mcq|enumerate|essay|case|table|order>".into()]),
        },
        Some("diff") => {
            let base = base_pack(root, args)?;
            // `--head-pack FILE`: a built pack (images embedded, as devices
            // get it) instead of the working tree's files.
            let head = match crate::flag(args, "--head-pack") {
                Some(file) => {
                    let bytes = std::fs::read(file).map_err(|e| vec![format!("{file}: {e}")])?;
                    Pack::decode(&bytes).map_err(|e| vec![format!("{file}: {e}")])?
                }
                None => crate::ids::pack_at(root)?,
            };
            Ok(diff(&base, &head))
        }
        Some("preflight") => preflight(root, crate::flag(args, "--base").unwrap_or("origin/main")),
        _ => Err(vec!["usage: steps-tools content <next-id | new-item | diff | preflight> (docs/AUTHORING.md)".into()]),
    }
}

fn base_pack(root: &Path, args: &[String]) -> Res<Pack> {
    match (crate::flag(args, "--base"), crate::flag(args, "--base-pack")) {
        (Some(rev), None) => crate::ids::pack_at_rev(root, rev),
        (None, Some(file)) => {
            let bytes = std::fs::read(file).map_err(|e| vec![format!("{file}: {e}")])?;
            Pack::decode(&bytes).map_err(|e| vec![format!("{file}: {e}")])
        }
        _ => Err(vec!["give --base REF or --base-pack FILE".into()]),
    }
}

fn read_lecture(root: &Path, lecture: &str) -> Res<Lecture> {
    crate::read_yaml(&crate::lecture_path(&root.join("content"), lecture)).map_err(|e| vec![e])
}

/// The ids after the highest used (or retired) item and lesson number.
pub fn next_ids(root: &Path, lecture: &str) -> Res<(String, String)> {
    let lec = read_lecture(root, lecture)?;
    let retired = std::fs::read_to_string(root.join(crate::ids::RETIRED)).unwrap_or_default();
    let retired = steps_core::ids::parse_retired(&retired);
    let number = |id: &str, prefix: &str| -> Option<u32> {
        id.strip_prefix(prefix)?.trim_start_matches(|c: char| !c.is_ascii_digit()).parse().ok()
    };
    let item_prefix = format!("{lecture}.");
    let used_items = lec
        .items
        .iter()
        .map(|i| i.id.as_str())
        .chain(retired.iter().filter_map(|r| r.strip_prefix("item ")))
        .filter_map(|id| number(id, &item_prefix))
        .max()
        .unwrap_or(0);
    let lesson_prefix = format!("{lecture}.lesson");
    let lessons: Vec<String> = crate::lessons::load(root)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, f, _)| f.lecture == lecture)
        .flat_map(|(_, f, _)| f.lessons.into_iter().map(|l| l.id))
        .chain(retired.iter().filter_map(|r| r.strip_prefix("lesson ").map(str::to_owned)))
        .collect();
    let used_lessons = lessons.iter().filter_map(|id| number(id, &lesson_prefix)).max().unwrap_or(0);
    Ok((format!("{lecture}.{:03}", used_items + 1), format!("{lecture}.lesson{}", used_lessons + 1)))
}

/// A worked example of `kind` with the next free id, every text marked.
pub fn new_item(root: &Path, lecture: &str, kind: &str) -> Res<String> {
    let lec = read_lecture(root, lecture)?;
    let sample: Lecture =
        crate::read_yaml(&root.join("fixtures/content/sample.yaml")).map_err(|e| vec![e])?;
    let example = sample
        .items
        .iter()
        .find(|i| {
            serde_json::to_value(&i.body).ok().and_then(|v| v.get("kind").cloned()) == Some(kind.into())
        })
        .ok_or_else(|| vec![format!("no worked example of kind {kind:?} in fixtures/content/sample.yaml")])?;
    let (id, _) = next_ids(root, lecture)?;
    let mut value = serde_json::to_value(example).map_err(|e| vec![e.to_string()])?;
    mark(&mut value, "");
    value["id"] = id.clone().into();
    value["topic"] = lec.topics.first().map_or(serde_json::Value::Null, |t| t.id.clone().into());
    if let Some(src) = value.get_mut("src") {
        src["page"] = lec.book.pages[0].into();
        // Every page chip shows the quote, so it is required (docs/CONTENT.md).
        src["quote"] =
            format!("{TEMPLATE_MARK} the page's own words: the full passage or table that holds the fact")
                .into();
    }
    // Round-trip through the type so the snippet is a valid item.
    let item: Item = serde_json::from_value(value).map_err(|e| vec![format!("template: {e}")])?;
    let yaml = serde_yaml_ng::to_string(&vec![item]).map_err(|e| vec![e.to_string()])?;
    Ok(format!(
        "# Paste under `items:` in {}. Replace every {TEMPLATE_MARK} (validate refuses it),\n# set topic and src.page, then run `just content-preflight`.\n{yaml}",
        crate::rel(root, &crate::lecture_path(&root.join("content"), lecture))
    ))
}

/// Prefixes every string the author writes with the marker.
fn mark(v: &mut serde_json::Value, key: &str) {
    const KEEP: &[&str] = &["id", "kind", "topic", "status", "tags", "ref"];
    match v {
        serde_json::Value::String(s) if !KEEP.contains(&key) => *s = format!("{TEMPLATE_MARK} {s}"),
        serde_json::Value::Array(a) => a.iter_mut().for_each(|x| mark(x, key)),
        serde_json::Value::Object(o) => o.iter_mut().for_each(|(k, x)| mark(x, k)),
        _ => {}
    }
}

/// Every content YAML that still holds the template marker.
pub fn unfinished(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![root.join("content")];
    while let Some(dir) = dirs.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "yaml")
                && std::fs::read_to_string(&p).is_ok_and(|t| t.contains(TEMPLATE_MARK))
            {
                out.push(format!("{}: unfinished: replace every {TEMPLATE_MARK}", crate::rel(root, &p)));
            }
        }
    }
    out.sort();
    out
}

/// What changed between two packs, per lecture, as a Markdown table.
pub fn diff(base: &Pack, head: &Pack) -> String {
    let json = |x: &Item| serde_json::to_string(x).unwrap_or_default();
    let items = |p: &Pack| -> BTreeMap<String, BTreeMap<String, String>> {
        p.lectures
            .iter()
            .map(|l| (l.lecture.clone(), l.items.iter().map(|i| (i.id.clone(), json(i))).collect()))
            .collect()
    };
    let lessons = |p: &Pack| -> BTreeMap<String, BTreeMap<String, String>> {
        p.lessons
            .iter()
            .map(|f| {
                let rows =
                    f.lessons.iter().map(|l| (l.id.clone(), serde_json::to_string(l).unwrap_or_default()));
                (f.lecture.clone(), rows.collect())
            })
            .collect()
    };
    let count = |a: Option<&BTreeMap<String, String>>, b: Option<&BTreeMap<String, String>>| {
        let empty = BTreeMap::new();
        let (a, b) = (a.unwrap_or(&empty), b.unwrap_or(&empty));
        let added = b.keys().filter(|k| !a.contains_key(*k)).count();
        let removed = a.keys().filter(|k| !b.contains_key(*k)).count();
        let changed = b.iter().filter(|(k, v)| a.get(*k).is_some_and(|o| o != *v)).count();
        [added, changed, removed]
    };
    let (bi, hi, bl, hl) = (items(base), items(head), lessons(base), lessons(head));
    let lectures: BTreeSet<&String> = bi.keys().chain(hi.keys()).chain(bl.keys()).chain(hl.keys()).collect();
    let mut rows = Vec::new();
    for l in lectures {
        let i = count(bi.get(l), hi.get(l));
        let s = count(bl.get(l), hl.get(l));
        if i != [0; 3] || s != [0; 3] {
            rows.push(format!("| {l} | {} / {} / {} | {} / {} / {} |", i[0], i[1], i[2], s[0], s[1], s[2]));
        }
    }
    let mut out = String::new();
    if base.index != head.index {
        out.push_str("index: changed\n");
    }
    if base.program != head.program {
        out.push_str("program: changed\n");
    }
    if rows.is_empty() {
        out.push_str("no lecture or lesson differs");
        return out;
    }
    out.push_str(
        "| Lecture | Items added / changed / removed | Lessons added / changed / removed |\n|---|---|---|\n",
    );
    out.push_str(&rows.join("\n"));
    out
}

fn preflight(root: &Path, base: &str) -> Res<String> {
    let mut rows: Vec<(&str, Result<String, String>)> = Vec::new();
    let joined = |p: Vec<String>| {
        for x in &p {
            eprintln!("{x}");
        }
        format!("{} problem(s)", p.len())
    };
    rows.push(("content checks (validate)", crate::validate_all(root).map_err(joined)));
    rows.push((
        "pack devices take (packs build --check)",
        crate::packs::run(root, &["build".into(), "--check".into()]).map_err(joined),
    ));
    match (crate::ids::pack_at_rev(root, base), crate::ids::pack_at(root)) {
        (Ok(b), Ok(h)) => {
            let (rb, rh) = (steps_core::ids::registry(&b), steps_core::ids::registry(&h));
            rows.push((
                "content ids (records keep their targets)",
                crate::ids::check(root, &rb, &rh).map_err(joined),
            ));
            println!("Content compared with {base}:\n{}\n", diff(&b, &h));
        }
        (Err(e), _) | (_, Err(e)) => rows.push(("content ids (records keep their targets)", Err(joined(e)))),
    }
    let mut out = String::from("| Check | Result |\n|---|---|\n");
    let mut failed = 0;
    for (name, r) in &rows {
        match r {
            Ok(note) => {
                out.push_str(&format!("| {name} | ✓ {} |\n", note.lines().last().unwrap_or_default()))
            }
            Err(e) => {
                failed += 1;
                out.push_str(&format!("| {name} | ✗ {e} |\n"));
            }
        }
    }
    if failed > 0 {
        return Err(vec![out, format!("{failed} check(s) failed; see above")]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn next_ids_follow_the_highest_used_or_retired() {
        let (item, lesson) = next_ids(&root(), "cvs.L01").unwrap();
        let lec = read_lecture(&root(), "cvs.L01").unwrap();
        assert!(!lec.items.iter().any(|i| i.id == item), "{item} is free");
        assert!(item.starts_with("cvs.L01.") && lesson.starts_with("cvs.L01.lesson"));
    }

    #[test]
    fn a_new_item_is_valid_but_marked_unfinished() {
        for kind in ["card", "cloze", "mcq", "enumerate", "essay"] {
            let out = new_item(&root(), "cvs.L01", kind).unwrap();
            assert!(out.contains(TEMPLATE_MARK), "{kind}");
            let yaml: String =
                out.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{l}\n")).collect();
            let items: Vec<Item> = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(items[0].id, next_ids(&root(), "cvs.L01").unwrap().0, "{kind}");
        }
        assert!(new_item(&root(), "cvs.L01", "nonsense").is_err());
    }

    #[test]
    fn the_repo_has_nothing_unfinished_and_a_template_is_caught() {
        assert_eq!(unfinished(&root()), Vec::<String>::new());
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("content/cvs")).unwrap();
        std::fs::write(dir.path().join("content/cvs/L99.yaml"), format!("x: {TEMPLATE_MARK} y\n")).unwrap();
        assert_eq!(unfinished(dir.path()).len(), 1);
    }

    #[test]
    fn the_diff_counts_items_and_lessons_per_lecture() {
        let head = crate::ids::pack_at(&root()).unwrap();
        assert_eq!(diff(&head, &head), "no lecture or lesson differs");
        let mut base = head.clone();
        let first = base.lectures[0].lecture.clone();
        base.lectures[0].items.pop();
        base.lectures[0].items[0].status = steps_core::content::Status::Unverified;
        let out = diff(&base, &head);
        assert!(out.contains(&format!("| {first} | 1 / ")), "{out}");
    }
}
