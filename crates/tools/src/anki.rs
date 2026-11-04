//! `steps-tools anki [--lecture hb.L01] [--verdicts FILE] [--own EXPORT]
//! [--drafts DIR] [--merged-only] [--out DIR]`: the Anki file, so Mohamed can study before
//! the app is done. The deck itself is built by the `steps-anki` crate, which
//! the app's «export to Anki» button shares; see docs/ANKI.md.
//!
//! By default it writes one file, `dist/anki/steps.apkg`, with a `Steps`
//! deck, a level per course and one subdeck per lecture (`Steps::Cardiovascular::01 Embryology`).
//! `--lecture` writes just that lecture to `steps-hb-L01.apkg`.
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
//! `own` and never `unverified`.

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
    if let Some(file) = crate::flag(args, "--own") {
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
mod tests {
    use super::*;

    #[test]
    fn unmerged_lectures_come_from_the_shared_folder() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        std::fs::create_dir_all(d.join("content/hb")).unwrap();
        std::fs::create_dir_all(d.join("steps/content-wip/hb")).unwrap();
        std::fs::write(
            d.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: hb.L01, title: Liver Cirrhosis, pages: [1, 22], chunk: 10 }\n  - { id: hb.L02, title: Two, pages: [23, 36], chunk: 10 }\n  - { id: hb.L03, title: Three, pages: [37, 64], chunk: 11 }\n",
        )
        .unwrap();
        let sample = include_str!("../../../fixtures/content/sample.yaml");
        std::fs::write(d.join("content/hb/L01.yaml"), sample).unwrap();
        // Merged wins over wip; a broken wip file is left out.
        std::fs::write(d.join("steps/content-wip/hb/L01.yaml"), "not: read").unwrap();
        std::fs::write(d.join("steps/content-wip/hb/L02.yaml"), sample.replace("hb.L01", "hb.L02")).unwrap();
        std::fs::write(d.join("steps/content-wip/hb/L03.yaml"), "lecture: [").unwrap();
        let args = |extra: &[&str]| {
            let mut a: Vec<String> =
                ["--drafts", "steps", "--out", "out"].iter().map(|s| s.to_string()).collect();
            a[1] = d.join("steps").display().to_string();
            a[3] = d.join("out").display().to_string();
            a.extend(extra.iter().map(|s| s.to_string()));
            a
        };
        let report = run(d, &args(&[])).unwrap();
        assert!(report.contains("steps.apkg: 22 notes"), "{report}");
        assert!(report.contains("hb.L02: 11 notes (draft, not merged yet)"), "{report}");
        assert!(report.contains("content-wip/hb/L03.yaml") && report.contains("(left out)"), "{report}");
        let merged = run(d, &args(&["--merged-only"])).unwrap();
        assert!(merged.contains("steps.apkg: 11 notes"), "{merged}");
    }

    #[test]
    fn verdicts_drop_wrong_items_and_use_edits() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        std::fs::create_dir_all(d.join("content/hb")).unwrap();
        std::fs::write(
            d.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: hb.L01, title: Liver Cirrhosis, pages: [1, 22], chunk: 10 }\n",
        )
        .unwrap();
        std::fs::write(d.join("content/hb/L01.yaml"), include_str!("../../../fixtures/content/sample.yaml"))
            .unwrap();
        std::fs::write(
            d.join("export.yaml"),
            "schema: steps.verdicts/v1\nexported: x\nverdicts:\n  - { id: hb.L01.003, verdict: wrong, at: a }\n  - { id: hb.L01.001, verdict: edited, at: a, edit: { back: Fibrosis plus nodules. } }\n  - { id: hb.L01.002, verdict: edited, at: a, edit: { nope: x } }\n  - { id: hb.L01.004, verdict: wrong, at: b, fingerprint: \"0000000000000000\" }\n",
        )
        .unwrap();
        let args: Vec<String> =
            ["--merged-only", "--verdicts", "e", "--out", "o"].iter().map(|s| s.to_string()).collect();
        let mut args = args;
        args[2] = d.join("export.yaml").display().to_string();
        args[4] = d.join("out").display().to_string();
        let report = run(d, &args).unwrap();
        assert!(report.contains("steps.apkg: 10 notes"), "{report}");
        assert!(report.contains("left out: 1 item(s) marked ✗ or rejected"), "{report}");
        assert!(
            report.contains("hb.L01.002: a cloze has no field `nope` (kept the book version)"),
            "{report}"
        );
    }

    #[test]
    fn own_cards_from_an_export_get_their_own_subdeck() {
        let card = |id: &str, deleted: i64| {
            let value = format!(
                r#"{{"id":"{id}","front":"SAAG?","back":"≥ 1.1","lecture":"hb.L09","created_ms":1}}"#
            );
            serde_json::json!({"id": id, "value": value, "deleted": deleted})
        };
        let doc = serde_json::json!({"format": "steps-export", "tables": {"own_cards": {"rows": [card("own.a", 0), card("own.b", 1)]}}});
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.json");
        std::fs::write(&path, doc.to_string()).unwrap();
        let cards = own_cards(&path).unwrap();
        assert_eq!(cards.len(), 1, "deleted cards stay out");

        let lec = steps_core::own::lecture(&cards);
        let deck = build(std::slice::from_ref(&lec), &Options::default());
        assert_eq!(deck.decks[1].1, "Steps::My own cards");
        let note = &deck.notes[0];
        assert_eq!(note.tags, ["own::cards", "own", "hb.L09"]);
        assert_eq!(note.fields[2], "My own cards");
    }
}
