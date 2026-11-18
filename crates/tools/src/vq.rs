//! `steps-tools vq`: Mohamed's VQ question bank, sealed in git
//! (`steps_core::sealed`, docs/VQ.md).
//!
//! - `vq keygen` prints a new key and its key id. Nothing is written.
//! - `vq open DIR` (with `STEPS_VQ_KEY`) writes every sealed lecture as plain
//!   YAML to `DIR/<course>/LNN.yaml` and the figures to
//!   `DIR/images/VQ-NNNN.webp`. DIR must be outside the repo, and new or empty.
//! - `vq seal DIR` (with `STEPS_VQ_KEY`) is the reverse: it writes
//!   `content/<course>/vq/LNN.yaml` for every lecture in DIR and removes the
//!   sealed file of a lecture DIR no longer has questions for. Opening, then
//!   sealing, changes nothing.
//!
//! Also home of the sealed files' part of `validate` ([`check`]) and of
//! reading them for packs and ids ([`sealed_lectures`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use steps_core::content::{Index, Item, Lecture};
use steps_core::sealed::{self, Key, Opened, SealedLecture};
use steps_core::validate::{self, Context};

type Res<T> = Result<T, Vec<String>>;

pub const KEY_VAR: &str = "STEPS_VQ_KEY";
pub const REQUIRED_VAR: &str = "STEPS_VQ_REQUIRED";

/// The folder of each course that holds its sealed files.
const FOLDER: &str = "vq";
/// Where `vq open` puts the figures, by VQ number.
const IMAGES: &str = "images";
/// `scripts/check-tracked-files.sh` refuses a tracked file over this.
const MAX_FILE_BYTES: usize = 5 * 1024 * 1024;

/// The key and the CI switch, as the environment gives them.
#[derive(Debug, Default)]
pub struct Env {
    pub key: Option<Key>,
    /// `STEPS_VQ_REQUIRED=1` (CI): sealed questions validate can't open fail it.
    pub required: bool,
}

impl Env {
    /// Reads `STEPS_VQ_KEY` (empty = not set) and `STEPS_VQ_REQUIRED`. A key
    /// that doesn't parse is an error that never repeats the key.
    pub fn from_env() -> Result<Env, String> {
        let required = std::env::var(REQUIRED_VAR).is_ok_and(|v| v.trim() == "1");
        let key = match std::env::var(KEY_VAR) {
            Ok(text) if !text.trim().is_empty() => {
                Some(Key::parse(&text).map_err(|e| format!("{KEY_VAR}: {e}"))?)
            }
            _ => None,
        };
        Ok(Env { key, required })
    }

    fn key(&self) -> Res<&Key> {
        self.key.as_ref().ok_or_else(|| {
            vec![format!("{KEY_VAR} is not set: the VQ key is in Mohamed's password manager (docs/VQ.md)")]
        })
    }
}

pub fn run(root: &Path, args: &[String]) -> Res<String> {
    let dir = args.get(1).filter(|a| !a.starts_with("--")).map(PathBuf::from);
    match (args.first().map(String::as_str), dir) {
        (Some("keygen"), _) => keygen(),
        (Some("open"), Some(dir)) => open(root, &dir, &Env::from_env().map_err(|e| vec![e])?),
        (Some("seal"), Some(dir)) => seal(root, &dir, &Env::from_env().map_err(|e| vec![e])?),
        _ => Err(vec![
            "usage: steps-tools vq <keygen | open DIR | seal DIR> (STEPS_VQ_KEY for open and seal)".into(),
        ]),
    }
}

fn keygen() -> Res<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| vec![format!("no randomness: {e}")])?;
    let key = Key::from_bytes(bytes);
    Ok(format!(
        "{KEY_VAR}={}\nkey id: {}\nKeep the key in your password manager and in the GitHub secret {KEY_VAR}, never in git. Nothing was written to disk.",
        key.text(),
        key.id()
    ))
}

/// A sealed file and where it was read (`content/cvs/vq/L04.yaml`).
pub struct File {
    pub rel: String,
    pub sealed: SealedLecture,
}

/// `cvs.L04` → `content/cvs/vq/L04.yaml`, relative to the repo.
pub fn sealed_rel(lecture: &str) -> String {
    let (course, num) = lecture.split_once('.').unwrap_or((lecture, ""));
    format!("content/{course}/{FOLDER}/{num}.yaml")
}

/// Every `content/<course>/vq/*.yaml`, sorted by path, and the problems met
/// reading them. Nothing but sealed files belongs in a `vq/` folder: a plain
/// question or figure there would be in git in clear.
pub fn load(root: &Path) -> (Vec<File>, Vec<String>) {
    let content = root.join("content");
    let (mut files, mut problems) = (Vec::new(), Vec::new());
    let mut courses: Vec<PathBuf> = std::fs::read_dir(&content)
        .into_iter()
        .flatten()
        .flatten()
        .map(|d| d.path())
        .filter(|p| {
            p.is_dir() && !["lessons", "glossary", "qbank", "book-refs"].iter().any(|n| p.ends_with(n))
        })
        .collect();
    courses.sort();
    for course in courses {
        let folder = course.join(FOLDER);
        let mut entries: Vec<PathBuf> =
            std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            let rel = crate::rel(root, &path);
            if path.is_dir() || path.extension().is_none_or(|e| e != "yaml") {
                problems.push(format!("{rel}: only sealed files belong in a vq folder (docs/VQ.md)"));
                continue;
            }
            match crate::read_yaml::<SealedLecture>(&path) {
                Ok(sealed) => files.push(File { rel, sealed }),
                Err(_) => problems.push(format!("{rel}: not a sealed file ({})", sealed::SCHEMA)),
            }
        }
    }
    (files, problems)
}

/// The sealed files for a pack, as they are; any that can't be read fail.
pub fn sealed_lectures(root: &Path) -> Res<Vec<SealedLecture>> {
    let (files, problems) = load(root);
    if problems.is_empty() { Ok(files.into_iter().map(|f| f.sealed).collect()) } else { Err(problems) }
}

/// `validate`'s part (docs/VQ.md): a report line, and every problem.
///
/// Always: each file's own shape (`validate::sealed::file`), its folder, a
/// listed and written lecture, ids no other question or item has, each VQ
/// number once, and one key id everywhere. With the key: every question
/// opened and checked inside its lecture (`validate::sealed::opened`).
pub fn check(
    root: &Path,
    index: &Index,
    lectures: &[Lecture],
    approved: &BTreeSet<String>,
    qbank: &BTreeSet<String>,
    env: &Env,
) -> (String, Vec<String>) {
    let (files, mut problems) = load(root);
    let listed: BTreeSet<&str> = index.lectures.iter().map(|e| e.id.as_str()).collect();
    let written: BTreeMap<&str, &Lecture> = lectures.iter().map(|l| (l.lecture.as_str(), l)).collect();
    let open_ids: BTreeSet<&str> =
        lectures.iter().flat_map(|l| l.items.iter().map(|i| i.id.as_str())).collect();
    let mut ids = BTreeSet::new();
    let mut numbers: BTreeMap<u32, &str> = BTreeMap::new();
    let mut keys: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for f in &files {
        let (rel, lid) = (f.rel.as_str(), f.sealed.lecture.as_str());
        problems.extend(validate::sealed::file(&f.sealed).into_iter().map(|p| format!("{rel}: {p}")));
        if rel != sealed_rel(lid) {
            problems.push(format!("{rel}: holds lecture {lid}, which belongs in {}", sealed_rel(lid)));
        }
        if !listed.contains(lid) {
            problems.push(format!("{rel}: lecture {lid} is not in content/index.yaml"));
        } else if !written.contains_key(lid) {
            problems.push(format!(
                "{rel}: lecture {lid} has no {}, so its questions would never show",
                crate::rel(root, &crate::lecture_path(&root.join("content"), lid))
            ));
        }
        keys.entry(f.sealed.key.as_str()).or_default().push(rel);
        for item in &f.sealed.items {
            let at = item.id.as_str();
            if !ids.insert(at) {
                problems.push(format!("{rel}: {at}: id is used twice"));
            }
            if open_ids.contains(at) {
                problems.push(format!("{rel}: {at}: an open item has this id"));
            }
            if let Some(n) = sealed::number(at)
                && let Some(first) = numbers.insert(n, at).filter(|first| *first != at)
            {
                problems.push(format!("{rel}: {at}: {} is filed twice (also {first})", vq_label(n)));
            }
        }
    }
    if keys.len() > 1 {
        let named: Vec<String> = keys.iter().map(|(k, f)| format!("{k} ({})", f.join(", "))).collect();
        problems.push(format!(
            "sealed files name different keys: {}; seal them all with one key (steps-tools vq seal)",
            named.join("; ")
        ));
    }
    let count: usize = files.iter().map(|f| f.sealed.items.len()).sum();
    if count > 0 && !qbank.contains(sealed::SOURCE) {
        problems.push(format!(
            "content/sources.yaml: the {} source (kind qbank) is missing, and {count} sealed questions cite it",
            sealed::SOURCE
        ));
    }
    let Some(key) = &env.key else {
        let line = format!("VQ: {count} sealed questions not opened ({KEY_VAR} is not set)");
        if env.required && count > 0 {
            problems.push(format!("{line}, and {REQUIRED_VAR}=1 needs them checked"));
        }
        return (line, problems);
    };
    if let Some((named, _)) = keys.iter().find(|(k, _)| **k != key.id()) {
        problems.push(format!("VQ: {KEY_VAR} is key {}, but the sealed files name key {named}", key.id()));
        return (format!("VQ: {count} sealed questions not opened (another key)"), problems);
    }
    let mut figures = 0;
    for f in &files {
        let Some(lec) = written.get(f.sealed.lecture.as_str()) else { continue };
        for item in &f.sealed.items {
            let opened = sealed::open(key, &item.id, &item.sealed).and_then(|plain| {
                let o = sealed::read_opened(item, &plain)?;
                Ok((o, raw_item_keys(&plain)))
            });
            let (o, raw) = match opened {
                Ok(pair) => pair,
                Err(e) => {
                    problems.push(format!("{}: {}: {e}", f.rel, item.id));
                    continue;
                }
            };
            figures += usize::from(o.image.is_some());
            let cx = Context {
                raw_keys: [(item.id.clone(), raw)].into(),
                approved: Some(approved),
                qbank: Some(qbank),
            };
            problems.extend(
                validate::sealed::opened(lec, &o, &cx).into_iter().map(|p| format!("{}: {p}", f.rel)),
            );
        }
    }
    (format!("VQ: {count} sealed questions opened and checked, {figures} with a figure"), problems)
}

/// The keys written on the opened item, so a typo is caught as in a lecture file.
fn raw_item_keys(plain: &[u8]) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_slice(plain).unwrap_or_default();
    value.get("item").and_then(|i| i.as_object()).map(|o| o.keys().cloned().collect()).unwrap_or_default()
}

/// `VQ-0001`.
fn vq_label(n: u32) -> String {
    sealed::label("VQ", n)
}

/// `DIR/<course>/LNN.yaml`: one lecture's questions in clear.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plain {
    schema: String,
    lecture: String,
    items: Vec<PlainItem>,
}

/// A question in clear: the item, with its educational objective on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PlainItem {
    #[serde(flatten)]
    item: Item,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    objective: Option<String>,
}

/// The one key a plain question has beyond an item's.
const OBJECTIVE: &str = "objective";

/// DIR as an absolute path, refused when it is inside the repo: plain
/// questions must never be where git could take them.
fn outside(root: &Path, dir: &Path) -> Res<PathBuf> {
    let fail = |e: std::io::Error| vec![format!("{}: {e}", dir.display())];
    let root = root.canonicalize().map_err(fail)?;
    let mut real = std::path::absolute(dir).map_err(fail)?;
    // The nearest folder that exists, with its links resolved, then the rest.
    let mut rest = Vec::new();
    while !real.exists() {
        rest.push(real.file_name().map(|n| n.to_owned()).unwrap_or_default());
        if !real.pop() {
            break;
        }
    }
    let mut real = real.canonicalize().map_err(fail)?;
    real.extend(rest.iter().rev());
    if real.starts_with(&root) {
        return Err(vec![format!(
            "{} is inside the repo; open the questions outside it (for example ../vq-open), so they never reach git",
            dir.display()
        )]);
    }
    Ok(real)
}

fn write(path: &Path, bytes: &[u8]) -> Res<()> {
    let fail = |e: std::io::Error| vec![format!("{}: {e}", path.display())];
    std::fs::create_dir_all(path.parent().expect("a file in a folder")).map_err(fail)?;
    std::fs::write(path, bytes).map_err(fail)
}

/// `vq open DIR`.
fn open(root: &Path, dir: &Path, env: &Env) -> Res<String> {
    let key = env.key()?;
    let dir = outside(root, dir)?;
    if std::fs::read_dir(&dir).is_ok_and(|mut d| d.next().is_some()) {
        return Err(vec![format!(
            "{} is not empty; vq open writes into a new or empty folder, so nothing old mixes in",
            dir.display()
        )]);
    }
    let (files, mut problems) = load(root);
    let mut plain = Vec::new();
    for f in &files {
        if f.sealed.key != key.id() {
            problems.push(format!(
                "{}: sealed with key {}, but {KEY_VAR} is key {}",
                f.rel,
                f.sealed.key,
                key.id()
            ));
            continue;
        }
        let mut items = Vec::new();
        for item in &f.sealed.items {
            match sealed::open_item(key, item) {
                Ok(o) => items.push(o),
                Err(e) => problems.push(format!("{}: {}: {e}", f.rel, item.id)),
            }
        }
        plain.push((f.sealed.lecture.clone(), items));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let (mut questions, mut figures) = (0, 0);
    for (lecture, items) in plain {
        for o in &items {
            if let (Some(bytes), Some(n)) = (o.image_bytes(), sealed::number(&o.item.id)) {
                write(&dir.join(IMAGES).join(format!("{}.webp", vq_label(n))), &bytes)?;
                figures += 1;
            }
        }
        questions += items.len();
        let file = Plain {
            schema: steps_core::content::SCHEMA.into(),
            lecture: lecture.clone(),
            items: items.into_iter().map(|o| PlainItem { item: o.item, objective: o.objective }).collect(),
        };
        let yaml = serde_yaml_ng::to_string(&file).map_err(|e| vec![e.to_string()])?;
        let head = format!(
            "# VQ questions filed under {lecture}, opened from {} (docs/VQ.md).\n# Plain text: never commit it. `steps-tools vq seal` seals it back.\n",
            sealed_rel(&lecture)
        );
        let (course, num) = lecture.split_once('.').unwrap_or((&lecture, ""));
        write(&dir.join(course).join(format!("{num}.yaml")), format!("{head}{yaml}").as_bytes())?;
    }
    Ok(format!(
        "opened {questions} questions and {figures} figures into {}. Edit them there, then run `steps-tools vq seal {}`",
        dir.display(),
        dir.display()
    ))
}

/// `vq seal DIR`.
fn seal(root: &Path, dir: &Path, env: &Env) -> Res<String> {
    let key = env.key()?;
    let dir = outside(root, dir)?;
    if !dir.is_dir() {
        return Err(vec![format!("{}: no such folder (vq open writes it)", dir.display())]);
    }
    let index: Index = crate::read_yaml(&root.join("content/index.yaml")).map_err(|e| vec![e])?;
    let listed: BTreeSet<&str> = index.lectures.iter().map(|e| e.id.as_str()).collect();
    let (existing, mut problems) = load(root);
    if let Some(f) = existing.iter().find(|f| f.sealed.key != key.id()) {
        problems.push(format!(
            "{} was sealed with key {}, but {KEY_VAR} is key {}; seal every file with one key",
            f.rel,
            f.sealed.key,
            key.id()
        ));
    }
    let (lectures, images, read_problems) = read_plain(&dir, &listed);
    problems.extend(read_problems);
    // Each figure belongs to a question.
    let numbers: BTreeSet<u32> =
        lectures.values().flat_map(|items| items.iter().filter_map(|i| sealed::number(&i.item.id))).collect();
    for n in images.keys().filter(|n| !numbers.contains(n)) {
        problems.push(format!("{IMAGES}/{}.webp: no question {} in the folder", vq_label(*n), vq_label(*n)));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    // Every file first, so a problem leaves the repo as it was.
    let mut texts = Vec::new();
    for (lecture, items) in lectures.iter().filter(|(_, items)| !items.is_empty()) {
        let sealed_items = items
            .iter()
            .map(|p| {
                let figure = sealed::number(&p.item.id).and_then(|n| images.get(&n)).map(Vec::as_slice);
                let opened = Opened::with_image(p.item.clone(), figure).with_objective(p.objective.clone());
                sealed::seal_item(key, &opened)
            })
            .collect();
        let text = sealed_yaml(&SealedLecture::new(lecture, key, sealed_items));
        let rel = sealed_rel(lecture);
        if text.len() > MAX_FILE_BYTES {
            problems.push(format!(
                "{rel} would be {:.1} MB, over the 5 MB a tracked file may be; save its figures smaller",
                text.len() as f64 / (1024.0 * 1024.0)
            ));
        }
        texts.push((rel, text, items.len()));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let (mut changed, mut questions) = (Vec::new(), 0);
    let keep: BTreeSet<String> = texts.iter().map(|(rel, ..)| rel.clone()).collect();
    for (rel, text, n) in texts {
        questions += n;
        let path = root.join(&rel);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            write(&path, text.as_bytes())?;
            changed.push(rel);
        }
    }
    let mut removed = Vec::new();
    for f in existing.iter().filter(|f| !keep.contains(&f.rel)) {
        std::fs::remove_file(root.join(&f.rel)).map_err(|e| vec![format!("{}: {e}", f.rel)])?;
        removed.push(f.rel.clone());
    }
    let mut out = format!("sealed {questions} questions into {} files with key {}", keep.len(), key.id());
    for rel in &changed {
        out.push_str(&format!("\nwrote {rel}"));
    }
    for rel in &removed {
        out.push_str(&format!("\nremoved {rel} (no questions left)"));
    }
    if changed.is_empty() && removed.is_empty() {
        out.push_str("\nnothing changed");
    }
    Ok(out)
}

/// The plain lectures in DIR (lecture → questions, sorted by id) and its
/// figures by VQ number, with every problem in them.
type PlainDir = (BTreeMap<String, Vec<PlainItem>>, BTreeMap<u32, Vec<u8>>, Vec<String>);

fn read_plain(dir: &Path, listed: &BTreeSet<&str>) -> PlainDir {
    let (mut lectures, mut images, mut problems) = (BTreeMap::new(), BTreeMap::new(), Vec::new());
    let mut seen: BTreeMap<u32, String> = BTreeMap::new();
    let mut folders: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        let name = folder.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut entries: Vec<PathBuf> =
            std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            let file = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let shown = format!("{name}/{file}");
            if name == IMAGES {
                let n = file
                    .strip_suffix(".webp")
                    .and_then(|s| s.strip_prefix("VQ-"))
                    .filter(|d| d.len() == 4 && d.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|d| d.parse::<u32>().ok());
                match (n, std::fs::read(&path)) {
                    (Some(n), Ok(bytes)) => {
                        images.insert(n, bytes);
                    }
                    (None, _) => problems.push(format!("{shown}: figures are named VQ-NNNN.webp")),
                    (_, Err(e)) => problems.push(format!("{shown}: {e}")),
                }
                continue;
            }
            let Some(num) = file.strip_suffix(".yaml") else {
                problems.push(format!("{shown}: not a lecture file (LNN.yaml)"));
                continue;
            };
            let lecture = format!("{name}.{num}");
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    problems.push(format!("{shown}: {e}"));
                    continue;
                }
            };
            let plain: Plain = match serde_yaml_ng::from_str(&text) {
                Ok(p) => p,
                Err(e) => {
                    problems.push(format!("{shown}: {e}"));
                    continue;
                }
            };
            if plain.lecture != lecture {
                problems.push(format!("{shown}: lecture is {}, the file name says {lecture}", plain.lecture));
            }
            if !listed.contains(lecture.as_str()) {
                problems.push(format!("{shown}: lecture {lecture} is not in content/index.yaml"));
            }
            if plain.schema != steps_core::content::SCHEMA {
                problems.push(format!("{shown}: schema must be {}", steps_core::content::SCHEMA));
            }
            for (id, keys) in crate::raw_keys(&text) {
                let item = plain.items.iter().find(|i| i.item.id == id);
                let own =
                    item.and_then(|i| steps_core::content::allowed_keys(i.item.body.kind())).unwrap_or(&[]);
                for k in keys.iter().filter(|k| {
                    !own.contains(&k.as_str())
                        && !steps_core::content::SHARED_KEYS.contains(&k.as_str())
                        && k.as_str() != OBJECTIVE
                }) {
                    problems.push(format!("{shown}: {id}: unknown key `{k}`"));
                }
            }
            let mut items = plain.items;
            items.sort_by(|a, b| a.item.id.cmp(&b.item.id));
            for PlainItem { item, .. } in &items {
                match sealed::under(&item.id, &lecture) {
                    None => problems.push(format!(
                        "{shown}: {}: id must be {lecture}.vqNNNN (to move a question, change its id)",
                        item.id
                    )),
                    Some(n) => {
                        if let Some(other) = seen.insert(n, item.id.clone()) {
                            problems.push(format!(
                                "{shown}: {}: {} is filed twice (also {other})",
                                item.id,
                                vq_label(n)
                            ));
                        }
                    }
                }
            }
            lectures.insert(lecture, items);
        }
    }
    (lectures, images, problems)
}

/// The sealed file's text: written here, byte for byte the same for the
/// same questions and key, so an untouched question never shows in a diff.
fn sealed_yaml(f: &SealedLecture) -> String {
    let mut out = format!(
        "# VQ bank questions filed under this lecture, sealed (docs/VQ.md).\nschema: {}\nlecture: {}\nkey: \"{}\"  # key id: which key sealed them (never the key)\nitems:\n",
        f.schema, f.lecture, f.key
    );
    for item in &f.items {
        out.push_str(&format!(
            "  - {{ id: {}, kind: {}, sealed: \"{}\" }}\n",
            item.id, item.kind, item.sealed
        ));
    }
    out
}

/// With the key, every sealed question opened, with its lecture, for the
/// commands that read items (`verdicts`); without it, none.
pub fn opened_items(root: &Path) -> Res<Vec<(String, Item)>> {
    let files = sealed_lectures(root)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let Some(key) = Env::from_env().map_err(|e| vec![e])?.key else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for file in files {
        for item in &file.items {
            let o = sealed::open_item(&key, item).map_err(|e| vec![format!("{}: {e}", item.id)])?;
            out.push((file.lecture.clone(), o.item));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
