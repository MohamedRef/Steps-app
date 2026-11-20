//! Files shared to Steps (#836): which card reads them, and that card opening
//! on them.

use super::*;
use crate::Screen;
use crate::i18n::{data as dtx, ownq as otx, settings as stx};
use crate::ownq::import::{TEMPLATE, TEMPLATE_NAME};
use crate::render_tests::App;
use crate::store::Store;

/// A backup from another device, holding one setting.
fn backup(version: Option<u64>) -> Vec<u8> {
    let tab = Store::open_in_memory().unwrap();
    tab.put_setting("plan", &serde_json::json!({ "exam": "2027-06-01" })).unwrap();
    let mut doc = tab.export("1.0.0", crate::store::now_ms()).unwrap().document;
    if let Some(v) = version {
        doc["format_version"] = v.into();
    }
    serde_json::to_vec(&doc).unwrap()
}

#[test]
fn a_backup_goes_to_restore_and_anything_else_to_questions() {
    assert_eq!(route("steps-export-2026-11-20.json", &backup(None)), For::Backup);
    assert_eq!(route("FROM-TABLET.JSON", &backup(None)), For::Backup);
    // One from a newer Steps still goes to restore, which says so.
    assert_eq!(route("newer.json", &backup(Some(99))), For::Backup);
    assert_eq!(route("q.json", br#"{"questions": []}"#), For::Questions);
    assert_eq!(route("q.json", b"{nope"), For::Questions);
    assert_eq!(route(TEMPLATE_NAME, TEMPLATE), For::Questions);
    assert_eq!(route("q.yaml", b"questions: []"), For::Questions);
    // A backup's text under another name is not taken for one.
    assert_eq!(route("backup.yaml", &backup(None)), For::Questions);
}

#[test]
fn the_waiting_file_is_read_once_and_its_copy_deleted() {
    let dir = std::env::temp_dir().join(format!("steps-inbox-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("shared-file");
    std::fs::write(&copy, b"questions: []").unwrap();
    let json = serde_json::json!({ "name": "q.yaml", "path": copy.to_string_lossy() }).to_string();
    let arrived = open_pending(parse_pending(&json).unwrap());
    assert_eq!(arrived, Arrived { name: "q.yaml".into(), bytes: Some(b"questions: []".to_vec()) });
    assert!(!copy.exists());
    // Android couldn't copy it: an empty path, and the name to say so with.
    let arrived = open_pending(parse_pending(r#"{"name": "big.zip", "path": ""}"#).unwrap());
    assert_eq!(arrived, Arrived { name: "big.zip".into(), bytes: None });
    assert!(parse_pending("not json").is_none());
    let _ = std::fs::remove_dir_all(dir);
}

/// What the manifest offers Steps for is what the questions picker offers
/// (plain text is the text share's own filter).
#[test]
fn the_manifest_takes_every_kind_of_file_the_questions_picker_does() {
    let manifest = include_str!("../../../android/src/main/AndroidManifest.xml");
    let share =
        manifest.split("app.steps.ShareActivity").nth(1).unwrap().split("</activity>").next().unwrap();
    for action in ["android.intent.action.SEND", "android.intent.action.VIEW"] {
        let filter = share
            .split("<intent-filter>")
            .filter(|f| f.contains(action))
            .find(|f| f.contains("application/zip"))
            .unwrap_or_else(|| panic!("no file filter for {action}"));
        for t in crate::platform::documents::QUESTIONS.iter().filter(|t| **t != "text/plain") {
            assert!(filter.contains(&format!("android:mimeType=\"{t}\"")), "{action}: {t}");
        }
    }
}

fn share(name: &str, bytes: Option<Vec<u8>>) {
    let settings = dioxus::prelude::consume_context::<SettingsNav>();
    receive(Arrived { name: name.into(), bytes }, settings);
}

#[test]
fn a_shared_file_of_questions_opens_your_data_with_it_read() {
    let mut app = App::with(Screen::Today, || share(TEMPLATE_NAME, Some(TEMPLATE.to_vec())));
    app.assert_has(stx::DATA);
    app.assert_has(otx::TITLE);
    app.assert_has(TEMPLATE_NAME);
    // The sheet alone: its second question's figure would come in a zip.
    app.assert_has(&otx::plan(1, 0, 0));
    assert!(app.run(|| QUESTIONS.peek().is_none()), "taken once");
    // One Android couldn't copy says so (Settings is open on the card already).
    app.run(|| *QUESTIONS.write() = Some(Arrived { name: "big.zip".into(), bytes: None }));
    app.assert_has(otx::COPY_FAILED);
}

#[test]
fn a_shared_backup_opens_restore_with_it_read() {
    let mut app = App::with(Screen::Today, || share("steps-export-tablet.json", Some(backup(None))));
    app.assert_has(dtx::IMPORT_TITLE);
    app.assert_has("steps-export-tablet.json");
    app.assert_has(dtx::IMPORT_SAFE);
    app.assert_has(dtx::T_SETTINGS);
    assert!(app.run(|| BACKUP.peek().is_none()), "taken once");
    // The questions card wasn't handed it.
    app.assert_has(otx::INTRO);
}
