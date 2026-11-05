//! `steps-tools anki [--lecture cvs.L04] [--verdicts FILE] [--own EXPORT]
//! [--drafts DIR] [--merged-only] [--out DIR]`: the Anki file, so Mohamed can study before
//! the app is done. The deck itself is built by the `steps-anki` crate, which
//! the app's «export to Anki» button shares; see docs/ANKI.md.
//!
//! By default it writes one file, `dist/anki/steps.apkg`, with a `Steps`
//! deck, a level per course and one subdeck per lecture (`Steps::Cardiovascular::01 Embryology`).
//! `--lecture` writes just that lecture to `steps-cvs-L01.apkg`.
//!
//! So the file never shrinks while content threads work, a lecture that
//! isn't merged yet comes from the project's shared folder, like
//! make_apkg.py does: `<medstudy>/content-wip/<course>/LNN.yaml`. `<medstudy>` is
//! `--drafts DIR`, or `/mnt/project-files/steps` when it exists;
//! `--merged-only` turns this off. Those notes get a `draft` tag.
//!
//! `--verdicts FILE` reads the app's ✓/✎/✗ export: ✗ items are left out, ✎
//! items use his edited text, ✓ items are tagged `approved`. A verdict on an
//! item whose text changed since he judged it is ignored (#18), and a ✓
//! without a fingerprint doesn't approve.
//!
//! `--own steps-export-….json` adds Mohamed's own cards (the app's Settings ›
//! Your data export) as one more subdeck, `Steps::My own cards`, tagged
//! `own` and never `unverified`, and puts his notes on items in each
//! note's `Notes` field.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use steps_anki::{Options, build, to_apkg};
use steps_core::content::{Item, Lecture};

use serde_json::Value;

use crate::verdicts::{Verdict, apply_edit, current};

pub fn run(root: &Path, args: &[String]) -> Result<String, Vec<String>> {
    let out = crate::flag(args, "--out").map(PathBuf::from).unwrap_or_else(|| root.join("dist/anki"));
    let only = crate::flag(args, "--lecture");
    let verdicts = match crate::flag(args, "--verdicts") {
        Some(f) => Some(crate::verdicts::read(Path::new(f)).map_err(|e| vec![e])?),
        None => None,
    };
    let (index, mut lectures) = crate::packs::load(root)?;
    let mut warnings = Vec::new();
    let mut drafts = BTreeSet::new();
    let medstudy = match crate::flag(args, "--drafts") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => Some(PathBuf::from(MEDSTUDY)).filter(|d| d.is_dir()),
    };
    if let Some(dir) = medstudy.filter(|_| !args.iter().any(|a| a == "--merged-only")) {
        for entry in &index.lectures {
            if lectures.iter().any(|l| l.lecture == entry.id) {
                continue;
            }
            match draft_for(&dir, &entry.id) {
                Some(Ok(lec)) => {
                    drafts.insert(lec.lecture.clone());
                    lectures.push(lec);
                }
                Some(Err(e)) => warnings.push(format!("{e} (left out)")),
                None => {}
            }
        }
        let order = |id: &str| index.lectures.iter().position(|e| e.id == id).unwrap_or(usize::MAX);
        lectures.sort_by_key(|l| order(&l.lecture));
    }
    if let Some(id) = only {
        lectures.retain(|l| l.lecture == id);
        if lectures.is_empty() {
            return Err(vec![format!("{id}: no such lecture written yet")]);
        }
    }
    let mut mine = BTreeMap::new();
    if let Some(file) = crate::flag(args, "--own") {
        mine = item_notes(Path::new(file)).map_err(|e| vec![e])?;
        let cards = own_cards(Path::new(file)).map_err(|e| vec![e])?;
        if cards.is_empty() {
            warnings.push(format!("{file}: no own cards in this export"));
        } else if only.is_none() {
            lectures.push(steps_core::own::lecture(&cards));
        }
    }
    // Older checkouts have no program: then every deck sits under Steps.
    let program: Option<steps_core::program::Program> =
        crate::read_yaml(&root.join("content/program.yaml")).ok();
    let mut approved = crate::read_approved(&root.join("content/approved.txt"));
    let mut wrong = BTreeSet::new();
    let mut edits = BTreeMap::new();
    if let Some(v) = &verdicts {
        let items: BTreeMap<&str, &Item> =
            lectures.iter().flat_map(|l| &l.items).map(|i| (i.id.as_str(), i)).collect();
        for (id, entry) in crate::verdicts::latest(v) {
            // A verdict on text that changed since he judged it no longer applies (#18).
            let now = items.get(id).and_then(|item| current(entry, item));
            if now == Some(false) {
                continue;
            }
            match entry.verdict {
                // Only when the export shows he saw this exact text.
                Verdict::Approved if now != Some(true) => {}
                Verdict::Approved => {
                    approved.insert(id.to_string());
                }
                Verdict::Wrong => {
                    wrong.insert(id.to_string());
                }
                Verdict::Edited => match items.get(id).map(|item| apply_edit(item, &entry.edit)) {
                    Some(Ok(fixed)) => {
                        edits.insert(id.to_string(), fixed);
                    }
                    Some(Err(e)) => warnings.push(format!("{e} (kept the book version)")),
                    None => {}
                },
            }
        }
    }
    let opts = Options {
        approved: Some(&approved),
        wrong: Some(&wrong),
        edits: Some(&edits),
        drafts: Some(&drafts),
        program: program.as_ref(),
        notes: Some(&mine),
    };
    let deck = build(&lectures, &opts);

    let name = match only {
        Some(id) => format!("steps-{}.apkg", id.replace('.', "-")),
        None => "steps.apkg".into(),
    };
    let path = out.join(name);
    std::fs::create_dir_all(&out).map_err(|e| vec![format!("{}: {e}", out.display())])?;
    let bytes = to_apkg(&deck, now_ms()).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    std::fs::write(&path, bytes).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    let mut report = format!("{}: {} notes, {} cards", path.display(), deck.notes.len(), deck.card_count());
    for (lecture, n) in &deck.per_lecture {
        let from = if drafts.contains(lecture) { " (draft, not merged yet)" } else { "" };
        report.push_str(&format!("\n  {lecture}: {n} notes{from}"));
    }
    if deck.skipped > 0 {
        report.push_str(&format!("\n  left out: {} item(s) marked ✗ or rejected", deck.skipped));
    }
    for w in &warnings {
        report.push_str(&format!("\n  warning: {w}"));
    }
    Ok(report)
}

/// His live own cards from an app export (`tables.own_cards`).
pub fn own_cards(path: &Path) -> Result<Vec<steps_core::own::OwnCard>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if doc["format"] != "steps-export" {
        return Err(format!("{}: not a Steps export (Settings › Your data)", path.display()));
    }
    let rows = doc["tables"]["own_cards"]["rows"].as_array().cloned().unwrap_or_default();
    Ok(rows
        .iter()
        .filter(|r| r["deleted"].as_i64() == Some(0))
        .filter_map(|r| r["value"].as_str().and_then(|v| serde_json::from_str(v).ok()))
        .collect())
}

/// His live notes on items from an app export (`tables.notes`), by item id.
pub fn item_notes(path: &Path) -> Result<BTreeMap<String, String>, String> {
    use steps_core::note::{Anchor, Note};
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let rows = doc["tables"]["notes"]["rows"].as_array().cloned().unwrap_or_default();
    Ok(rows
        .iter()
        .filter(|r| r["deleted"].as_i64() == Some(0))
        .filter_map(|r| r["value"].as_str().and_then(|v| serde_json::from_str::<Note>(v).ok()))
        .filter_map(|n| match n.anchor {
            Anchor::Item(id) => Some((id, n.text)),
            _ => None,
        })
        .collect())
}

/// The project's shared folder, where unmerged lectures wait.
const MEDSTUDY: &str = "/mnt/project-files/steps";

/// The newest unmerged version of a lecture, if there is one.
fn draft_for(medstudy: &Path, id: &str) -> Option<Result<Lecture, String>> {
    let (course, num) = id.split_once('.')?;
    let path = medstudy.join("content-wip").join(course).join(format!("{num}.yaml"));
    if !path.exists() {
        return None;
    }
    Some(crate::read_yaml::<Lecture>(&path).and_then(|lec| {
        if lec.lecture == id {
            Ok(lec)
        } else {
            Err(format!("{}: says it is {}", path.display(), lec.lecture))
        }
    }))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
