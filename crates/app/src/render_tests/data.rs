//! Settings › Your data: a backup out and back in, a backup from another
//! device merged, and the Anki deck.

use std::time::{Duration, Instant};

use steps_core::review::{Grade, Origin, ReviewEvent};

use super::App;
use crate::Screen;
use crate::i18n::{anki as atx, data as tx, settings as stx};
use crate::store::{Store, export_dir, shared};

fn open_data() -> App {
    let mut app = App::open(Screen::Today);
    app.tap(Screen::Settings.label());
    app.tap(stx::DATA);
    app
}

/// Closes Settings and opens Your data again (it reads the folder anew).
fn reopen(app: &mut App) {
    app.tap("settings-backdrop");
    app.tap(Screen::Settings.label());
    app.tap(stx::DATA);
}

#[track_caller]
fn wait_for(app: &mut App, words: &str) {
    let end = Instant::now() + Duration::from_secs(10);
    while !app.has(words) {
        assert!(Instant::now() < end, "never said {words:?}:\n{}", app.text());
        app.wait(20);
    }
}

/// A backup from the tablet: two answers on the first cardiovascular item
/// and a setting.
fn tablet_backup() -> String {
    let tab = Store::open_in_memory().unwrap();
    let item =
        crate::content::Library::get().lectures.values().flat_map(|l| &l.items).next().unwrap().id.clone();
    for grade in [Grade::Good, Grade::Again] {
        let at_ms = crate::store::now_ms();
        tab.append_review(ReviewEvent {
            confidence: None,
            reason: None,
            item_id: item.clone(),
            grade,
            origin: Origin::Study,
            at_ms,
            took_ms: 4000,
        })
        .unwrap();
    }
    tab.put_setting("plan", &serde_json::json!({ "exam": "2027-06-01" })).unwrap();
    serde_json::to_string(&tab.export("1.0.0", crate::store::now_ms()).unwrap().document).unwrap()
}

#[test]
fn a_backup_is_saved_found_and_says_it_holds_nothing_new() {
    let mut app = open_data();
    app.assert_has(tx::EXPORT_TITLE);
    app.assert_has(tx::IMPORT_PICK_FIRST);
    app.tap(tx::EXPORT_BUTTON);
    app.assert_has(tx::EXPORT_SAVED_1);
    let dir = export_dir().unwrap();
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("steps-export-"))
        .expect("the backup file")
        .file_name()
        .to_string_lossy()
        .into_owned();
    app.assert_has(&file);
    // Opened again, the section finds it.
    reopen(&mut app);
    app.assert_has(tx::IMPORT_FOUND);
    app.assert_has(tx::IMPORT_PICK);
    app.tap(&file);
    wait_for(&mut app, tx::IMPORT_NOTHING);
    app.assert_has(&shared().device());
    app.assert_has(tx::COL_IN_FILE);
    app.assert_has(tx::T_SETTINGS);
    app.tap_nth(tx::IMPORT_BACK, 1); // #0 is the sheet's own Back
    app.assert_has(tx::IMPORT_FOUND);
}

#[test]
fn a_pasted_backup_from_another_device_is_merged() {
    let mut app = open_data();
    let text = tablet_backup();
    app.type_into("export-text", &text);
    app.tap(tx::IMPORT_READ);
    app.assert_has(tx::IMPORT_PASTED);
    app.assert_has(tx::IMPORT_SAFE);
    app.assert_has(tx::T_REVIEWS);
    // Cancel first: nothing changes.
    app.tap(tx::IMPORT_CANCEL);
    assert!(shared().reviews().unwrap().is_empty());
    app.type_into("export-text", &text);
    app.tap(tx::IMPORT_READ);
    app.tap(&tx::import_apply(3));
    app.assert_has(&tx::import_done(3));
    assert_eq!(shared().reviews().unwrap().len(), 2);
    assert_eq!(shared().setting::<serde_json::Value>("plan").unwrap()["exam"], "2027-06-01");
    app.tap(tx::IMPORT_AGAIN);
    app.assert_has(tx::IMPORT_PASTE);
}

#[test]
fn a_file_that_is_not_a_backup_is_refused() {
    let mut app = open_data();
    app.type_into("export-text", "{\"not\": \"a steps export\"}");
    app.tap(tx::IMPORT_READ);
    app.assert_has(tx::IMPORT_FAILED);
    app.tap_nth(tx::IMPORT_BACK, 1); // #0 is the sheet's own Back
    app.assert_has(tx::IMPORT_READ);

    // One that is far too big isn't even read.
    let dir = export_dir().unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    let big = dir.join("steps-export-2026-11-01.json");
    std::fs::File::create(&big).unwrap().set_len(60 * 1024 * 1024).unwrap();
    std::fs::write(dir.join("steps-export-2026-10-31.json"), "not json").unwrap();
    reopen(&mut app);
    app.tap("steps-export-2026-11-01.json");
    wait_for(&mut app, tx::IMPORT_TOO_BIG);
    app.tap_nth(tx::IMPORT_BACK, 1); // #0 is the sheet's own Back
    app.tap("steps-export-2026-10-31.json");
    wait_for(&mut app, tx::IMPORT_FAILED);
}

#[test]
fn a_failed_backup_says_why_and_tries_again() {
    let mut app = open_data();
    // Something sits where the backup is written first: it can't be saved.
    let date = crate::data_export::local_date();
    let blocker = export_dir().unwrap().join(format!("steps-export-{date}.json.part"));
    std::fs::create_dir_all(blocker.join("inside")).unwrap();
    app.tap(tx::EXPORT_BUTTON);
    app.assert_has(tx::EXPORT_FAILED);
    std::fs::remove_dir_all(&blocker).unwrap();
    app.tap(crate::i18n::saving::TRY_AGAIN);
    app.assert_has(tx::EXPORT_SAVED_1);
}

#[test]
fn the_anki_deck_is_built_from_the_app_and_saved() {
    let mut app = open_data();
    let lib = crate::content::Library::get();
    let items: usize = lib.lectures.values().map(|l| l.items.len()).sum();
    app.assert_has(&atx::contents(lib.lectures.len(), items));
    app.tap(atx::BUTTON);
    wait_for(&mut app, atx::SAVED_IN);
    let path = crate::prefs::test_dir().join("steps.apkg");
    assert!(std::fs::read(&path).unwrap().starts_with(b"PK"), "an .apkg is a zip");
    app.assert_has(&path.display().to_string());
}

/// Picks `path` in the "Choose file" input, as the WebView does.
fn choose_file(app: &mut App, path: &std::path::Path) {
    use dioxus_html::{EventData, SerializedFileData, SerializedFormData, SerializedFormObject};
    let file = SerializedFileData { path: path.to_path_buf(), ..SerializedFileData::empty() };
    let data = SerializedFormData::new(
        String::new(),
        vec![SerializedFormObject { key: "file".into(), text: None, file: Some(file) }],
    );
    let n = app.target("change", "", 0);
    app.fire("change", EventData::Form(data), n);
}

#[test]
fn a_backup_chosen_from_disk_is_read_and_an_unreadable_one_says_why() {
    let mut app = open_data();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tablet.json");
    std::fs::write(&path, tablet_backup()).unwrap();
    choose_file(&mut app, &path);
    wait_for(&mut app, tx::IMPORT_SAFE);
    app.assert_has("tablet.json");
    app.tap(tx::IMPORT_CANCEL);
    // The dialog closed with nothing picked: nothing happens.
    let n = app.target("change", "", 0);
    app.fire(
        "change",
        dioxus_html::EventData::Form(dioxus_html::SerializedFormData::new(String::new(), Vec::new())),
        n,
    );
    app.assert_has(tx::IMPORT_READ);
    app.assert_lacks(tx::IMPORT_FAILED);
    // Gone before it could be read.
    choose_file(&mut app, &dir.path().join("gone.json"));
    wait_for(&mut app, tx::IMPORT_FAILED);
}
