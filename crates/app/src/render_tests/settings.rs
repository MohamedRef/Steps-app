//! The Settings sheet: the list, search, each section and what its rows
//! save. Sync and Updates have their own files (`cloud.rs`, `update.rs`).

use super::App;
use crate::Screen;
use crate::i18n::settings as tx;
use crate::store::shared;

fn open_settings() -> App {
    let mut app = App::open(Screen::Today);
    app.tap(Screen::Settings.label());
    app
}

fn open_section(title: &str) -> App {
    let mut app = open_settings();
    app.tap(title);
    app
}

#[test]
fn the_list_groups_the_sections_and_search_narrows_it() {
    let mut app = open_settings();
    for line in [tx::GROUP_STUDY, tx::GROUP_DEVICE, tx::GROUP_DATA, tx::LOOK, tx::DATA, tx::ABOUT] {
        app.assert_has(line);
    }
    app.assert_has(&format!("{} {}", tx::VERSION, env!("CARGO_PKG_VERSION")));
    // The Mac's section, never the tablet's, on a desktop.
    app.assert_has(crate::i18n::desk::MAC);
    app.assert_lacks(crate::i18n::desk::TABLET);
    app.type_into(tx::SEARCH, "font");
    app.assert_has(tx::LOOK_SUB);
    app.assert_lacks(tx::DATA_SUB);
    app.type_into(tx::SEARCH, "zzzz");
    app.assert_has(tx::NO_RESULTS);
    app.tap(tx::CLEAR_SEARCH);
    app.assert_lacks(tx::NO_RESULTS);
    app.assert_has(tx::DATA_SUB);
}

#[test]
fn the_sheet_closes_by_its_button_or_beside_it_and_back_returns_to_the_list() {
    let mut app = open_section(tx::ABOUT);
    assert!(app.html().contains(r#"data-drilled="true""#));
    app.tap(tx::BACK);
    assert!(app.html().contains(r#"data-drilled="false""#));
    app.tap("icon-button settings-back");
    app.tap_nth(tx::CLOSE, 0);
    assert!(!app.can_tap("settings-backdrop"));
    app.tap(Screen::Settings.label());
    assert!(app.can_tap("settings-backdrop"));
    app.tap("settings-backdrop");
    assert!(!app.can_tap("settings-backdrop"));
}

#[test]
fn about_says_what_this_build_carries() {
    let app = open_section(tx::ABOUT);
    let lib = crate::content::Library::get();
    app.assert_has(&format!("{} {}/{}", tx::LECTURES_WRITTEN, lib.lectures.len(), lib.index.lectures.len()));
    app.assert_has(tx::BOOK);
    app.assert_has(crate::i18n::app::TAGLINE);
}

#[test]
fn the_mac_section_saves_the_sidebar_and_hover_and_opens_the_keys() {
    use crate::i18n::desk as dtx;
    let mut app = open_section(dtx::MAC);
    app.tap(dtx::NAV_SLIM);
    assert_eq!(crate::prefs::DevicePrefs::load().nav_style, crate::prefs::NavStyle::Slim);
    app.tap(dtx::NAV_LABELS);
    assert_eq!(crate::prefs::DevicePrefs::load().nav_style, crate::prefs::NavStyle::Labels);
    let before = crate::prefs::DevicePrefs::load().mouse_hover;
    app.form("change", "", if before { "false" } else { "true" });
    app.assert_has(crate::i18n::hover::MOUSE);
    app.tap(crate::i18n::keys::OPEN);
    app.assert_has(crate::i18n::keys::NOTE);
    app.assert_has(crate::i18n::keys::REVEAL);
}

#[test]
fn sounds_are_switched_and_each_variation_is_picked() {
    use crate::i18n::sound as stx;
    let mut app = open_section(crate::i18n::study_settings::SECTION);
    app.assert_has(stx::TITLE);
    app.tap(&format!("{} 2", stx::VARIATION));
    app.tap_nth(&format!("{} 3", stx::VARIATION), 1);
    let saved: crate::sounds::SoundPrefs = shared().setting("sound").unwrap();
    assert_eq!((saved.chime, saved.calm), (2, 3));
    app.tap(stx::OFF);
    assert!(!shared().setting::<crate::sounds::SoundPrefs>("sound").unwrap().on);
    app.assert_lacks(stx::RIGHT);
    app.tap(stx::ON);
    app.assert_has(stx::RIGHT);
    // A sync brings another device's choice; reload picks it up.
    shared()
        .put_setting("sound", &crate::sounds::SoundPrefs { chime: 1, ..crate::sounds::SoundPrefs::default() })
        .unwrap();
    app.run(crate::sounds::reload);
    assert_eq!(app.run(|| *crate::sounds::PREFS.peek()).chime, 1);
    // Playing a cue with sounds on does nothing harmful here.
    app.run(|| {
        crate::sounds::play(crate::sounds::Cue::Right);
        crate::sounds::play(crate::sounds::Cue::Wrong);
        crate::sounds::play(crate::sounds::Cue::SessionEnd);
    });
}

use dioxus::prelude::ReadableExt as _;

fn on_a_phone(allowed: Option<bool>, unrestricted: Option<bool>) {
    crate::reminders::TEST_PHONE.with(|p| p.set(Some((allowed, unrestricted))));
}

#[test]
fn reminders_live_on_the_phone_only() {
    use crate::i18n::remind as rtx;
    let app = open_section(rtx::SECTION);
    app.assert_has(rtx::PHONE_ONLY);
    assert!(!app.can_tap(rtx::ADD));
}

#[test]
fn reminder_times_and_the_ringing_device_are_saved() {
    use crate::i18n::remind as rtx;
    on_a_phone(Some(true), Some(true));
    let mut app = open_section(rtx::SECTION);
    app.assert_has(rtx::DAILY);
    app.assert_has("19:00");
    app.tap(rtx::ADD);
    app.assert_has("21:00");
    app.tap(rtx::ADD);
    assert!(!app.can_tap(rtx::ADD), "three at most");
    app.form("change", rtx::TIME, "06:15");
    app.form("change", rtx::TIME, "nonsense");
    let saved: crate::reminders::RemindPrefs = shared().setting("remind").unwrap();
    assert_eq!(saved.times, [6 * 60 + 15, 21 * 60, 23 * 60]);
    app.tap_nth(rtx::REMOVE, 0);
    let saved: crate::reminders::RemindPrefs = shared().setting("remind").unwrap();
    assert_eq!(saved.times, [21 * 60, 23 * 60]);
    // This device rings, or not (device-only, never synced).
    app.tap_nth(rtx::OFF, 1);
    assert_eq!(shared().local_setting::<bool>("remind_here"), Some(false));
    app.tap_nth(rtx::ON, 1);
    assert_eq!(shared().local_setting::<bool>("remind_here"), Some(true));
    // Off: the times go.
    app.tap_nth(rtx::OFF, 0);
    app.assert_lacks(rtx::TIMES);
    assert!(!shared().setting::<crate::reminders::RemindPrefs>("remind").unwrap().on);
    app.tap_nth(rtx::ON, 0);
    app.assert_has(rtx::TIMES);
    // Another device changed them: reload follows.
    shared().put_setting("remind", &crate::reminders::RemindPrefs { on: true, times: vec![8 * 60] }).unwrap();
    app.run(crate::reminders::reload);
    app.assert_has("08:00");
}

#[test]
fn blocked_notifications_and_a_sleeping_battery_are_explained() {
    use crate::i18n::remind as rtx;
    on_a_phone(Some(false), Some(false));
    let mut app = open_section(rtx::SECTION);
    app.assert_lacks(rtx::BLOCKED);
    // Only once this device is the one that rings.
    app.tap_nth(rtx::ON, 1);
    app.assert_has(rtx::BLOCKED);
    app.assert_has(rtx::BATTERY);
    app.tap(rtx::ALLOW);
    app.tap(rtx::OPEN_NOTIFICATIONS);
    app.tap(rtx::OPEN_APP);
    // Answering every day keeps the planner arming them.
    let review = shared().reviews().unwrap().len();
    assert_eq!(review, 0);
}

#[test]
fn the_data_section_says_where_data_lives() {
    let app = open_section(tx::DATA);
    app.assert_has(tx::DATA_FOLDER);
    app.assert_has(&crate::prefs::test_dir().display().to_string());
    app.assert_has(crate::i18n::study_settings::STORE_MEMORY);
    app.assert_has(tx::DIAG);
}
