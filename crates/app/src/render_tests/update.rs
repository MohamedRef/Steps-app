//! Settings › Updates, the "new version" banner, the content note on Today
//! and the "what's new" sheet, on a real updater over the signed fixture
//! feed served from 127.0.0.1 (`update::testing`).

use std::time::{Duration, Instant};

use steps_core::update::{
    BuildChanges, Change, ChangeKind, Channel, UpdateAsset, UpdateNotes, UpdatePlatform,
};

use super::App;
use crate::Screen;
use crate::i18n::update as tx;
use crate::update::testing::{dead_feed, install, install_off, serve_fixture_feed};
use crate::update::{ContentCheck, Failure, Mode, Off, Offer, Phase, Prefs, Status, Updater};

fn open_updates() -> App {
    let mut app = App::open(Screen::Today);
    app.tap(Screen::Settings.label());
    app.tap(tx::SECTION);
    app
}

/// Lets the updater's thread work until `pred` holds.
#[track_caller]
fn until(app: &mut App, u: &Updater, what: &str, pred: impl Fn(&Status) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !pred(&u.status()) {
        assert!(Instant::now() < end, "the updater never got to {what}: {:?}", u.status().phase);
        app.wait(20);
    }
    app.wait(20);
}

fn offer(build: u64, changes: Vec<BuildChanges>) -> Offer {
    Offer {
        version: "0.1.0".into(),
        build,
        commit: None,
        notes: UpdateNotes { en: String::new(), ar_eg: "أسئلة جديدة في الكارديو".into() },
        asset: UpdateAsset {
            platform: UpdatePlatform::AndroidArm64,
            file: "steps.apk".into(),
            size: 3 * 1024 * 1024,
            sha256: "00".repeat(32),
            version_code: None,
            build: None,
        },
        changes,
    }
}

fn change(kind: ChangeKind, ar: &str) -> Change {
    Change { kind, text: UpdateNotes { en: String::new(), ar_eg: ar.into() } }
}

fn release(build: u64, published_at: &str, changes: Vec<Change>, content_only: bool) -> BuildChanges {
    BuildChanges { version: "0.1.0".into(), build, published_at: published_at.into(), changes, content_only }
}

fn notify() -> Prefs {
    Prefs { mode: Mode::Notify, ..Prefs::default() }
}

#[test]
fn check_download_and_install_from_the_signed_feed() {
    let u = install(&serve_fixture_feed(), "0.1.0", 41, notify());
    let mut app = open_updates();
    app.assert_has(tx::GROUP_HAVE);
    app.assert_has(tx::ROW_APP);
    app.assert_has(tx::NEVER_CHECKED);
    app.assert_has(tx::PREVIEW_CHANNEL_NOTE);

    app.tap(tx::CHECK_NOW);
    until(&mut app, u, "an offer", |s| matches!(s.phase, Phase::Available(_)));
    app.assert_has(tx::AVAILABLE);
    app.assert_has("0.1.0 (42)");
    app.assert_has(tx::LAST_CHECK);
    // The fixture's content pack isn't a real pack: the content line says it
    // couldn't take it, and the app's half stands on its own.
    app.assert_has(tx::RESULT_CONTENT);
    // "Tell me first": the banner offers the download too, above the screen.
    assert_eq!(u.status().banner().map(Offer::label).as_deref(), Some("0.1.0 (42)"));

    app.tap(tx::DOWNLOAD);
    until(&mut app, u, "a download", |s| matches!(s.phase, Phase::Ready(_)));
    app.assert_has(tx::READY);
    app.tap(tx::INSTALL_NOW);
    // A test run has no installer (Linux), or isn't Steps.app (macOS): it
    // says so and offers another try.
    until(&mut app, u, "the install", |s| matches!(s.phase, Phase::Failed { .. }));
    app.assert_has(tx::FAILED_INSTALL);
    app.assert_has(if cfg!(target_os = "macos") {
        "not running from Steps.app"
    } else {
        "no installer on this system"
    });
    assert!(app.can_tap(tx::TRY_AGAIN));
}

#[test]
fn the_feed_out_of_reach_is_one_line() {
    let u = install(&dead_feed(), "0.1.0", 41, notify());
    let mut app = open_updates();
    app.tap(tx::CHECK_NOW);
    until(&mut app, u, "a failure", |s| matches!(s.phase, Phase::Failed { .. }));
    app.assert_has(tx::FAILED_NETWORK);
    app.assert_lacks(tx::CONTENT_FAILED);
    // The settings list says it too.
    app.tap(crate::i18n::settings::BACK);
    app.assert_has(tx::SUMMARY_FAILED);
}

#[test]
fn mode_notifications_and_channel_are_saved() {
    let u = install(&serve_fixture_feed(), "0.1.0", 42, Prefs::default());
    let mut app = open_updates();
    app.assert_has(tx::NOTIFY);
    app.tap(tx::MODE_NOTIFY_SUB);
    assert_eq!(u.status().prefs.mode, Mode::Notify);
    app.form("change", "", "false");
    assert!(!u.status().prefs.notify);
    app.tap(tx::MODE_OFF_SUB);
    assert_eq!(u.status().prefs.mode, Mode::Off);
    app.assert_lacks(tx::NOTIFY);
    assert!(!u.background_checks());
    // Another channel looks at the feed again at once.
    app.tap(tx::BETA);
    until(&mut app, u, "the beta channel", |s| s.channel == Channel::Beta && s.prefs.last_check_ms.is_some());
    app.assert_lacks(tx::PREVIEW_CHANNEL_NOTE);
    let saved = std::fs::read_to_string(u.dir_for_tests().join("prefs.json")).unwrap();
    assert!(saved.contains("\"beta\"") && saved.contains("\"off\""), "{saved}");
}

#[test]
fn every_phase_says_where_the_update_stands() {
    let u = install(&dead_feed(), "0.1.0", 41, notify());
    let mut app = open_updates();
    let o = offer(42, Vec::new());
    let cases: Vec<(Phase, &str)> = vec![
        (Phase::Checking, tx::CHECKING),
        (Phase::UpToDate, tx::UP_TO_DATE),
        (Phase::NotForThisDevice, tx::NOT_FOR_DEVICE),
        (Phase::Available(o.clone()), tx::AVAILABLE),
        (Phase::Downloading { offer: o.clone(), done: 1024 }, tx::DOWNLOADING),
        (Phase::Ready(o.clone()), tx::READY),
        (Phase::Installing(o.clone()), tx::INSTALLING),
        (Phase::NeedsPermission(o.clone()), tx::NEEDS_PERMISSION),
        (Phase::Installed(o.clone()), tx::INSTALLED),
        (
            Phase::Failed { offer: Some(o.clone()), why: Failure::Verify("sha256 mismatch".into()) },
            tx::FAILED_VERIFY,
        ),
        (Phase::Failed { offer: None, why: Failure::TooNew }, tx::TOO_NEW),
        (Phase::Failed { offer: Some(o.clone()), why: Failure::Network }, tx::FAILED_NETWORK),
    ];
    for (phase, line) in cases {
        u.set_status_for_tests(|s| s.phase = phase.clone());
        app.settle();
        app.assert_has(line);
    }
    // Available: the size and the feed's notes, with "What's new:".
    u.set_status_for_tests(|s| s.phase = Phase::Available(o.clone()));
    app.settle();
    app.assert_has("3.0 ميجا");
    app.assert_has("أسئلة جديدة في الكارديو");
    app.assert_has(tx::WHATS_NEW_ASK);
    // Downloading shows how far it got, and the busy buttons wait.
    u.set_status_for_tests(|s| s.phase = Phase::Downloading { offer: o.clone(), done: 1024 });
    app.settle();
    assert!(app.html().contains("<progress"), "a progress bar");
    // Installed: it runs from the next launch.
    u.set_status_for_tests(|s| s.phase = Phase::Installed(o.clone()));
    app.settle();
    app.assert_has(tx::NEXT_LAUNCH);
    app.tap(tx::RESTART_NOW);
    // A failed check says why in the book's own words.
    u.set_status_for_tests(|s| {
        s.phase = Phase::Failed { offer: Some(o.clone()), why: Failure::Verify("sha256 mismatch".into()) }
    });
    app.settle();
    app.assert_has("sha256 mismatch");
    app.assert_lacks("أسئلة جديدة في الكارديو");
    // Android's permission: a button opens it.
    u.set_status_for_tests(|s| s.phase = Phase::NeedsPermission(o.clone()));
    app.settle();
    app.tap(tx::OPEN_PERMISSION);
}

#[test]
fn a_renewed_preview_key_explains_the_one_reinstall() {
    let u = install(&dead_feed(), "0.1.0", 41, notify());
    let mut app = open_updates();
    u.set_status_for_tests(|s| s.phase = Phase::Failed { offer: None, why: Failure::KeyChanged });
    app.settle();
    app.assert_has(tx::KEY_CHANGED);
    for step in tx::KEY_CHANGED_STEPS {
        app.assert_has(step);
    }
    app.tap(tx::GET_PREVIEW);
    let opened = crate::cloud::OPENED.lock().unwrap().clone();
    assert!(opened.iter().any(|u| u.ends_with("/steps-preview.apk")), "{opened:?}");
}

#[test]
fn updates_that_cannot_run_here_say_why() {
    // No updater at all: a development build.
    let app = open_updates();
    app.assert_has(tx::OFF_DEV);
    app.assert_lacks(tx::MODE);
    for (why, line) in [
        (Off::DevBuild, tx::OFF_DEV),
        (Off::NoKey, tx::OFF_KEY),
        (Off::NoPlatform, tx::OFF_PLATFORM),
        (Off::NotInBundle, tx::OFF_BUNDLE),
    ] {
        install_off(why);
        let app = open_updates();
        app.assert_has(line);
        assert!(!app.can_tap(tx::CHECK_NOW), "{}", app.text());
    }
}

#[test]
fn the_content_rows_say_what_came() {
    let u = install(&dead_feed(), "0.1.0", 40, notify());
    let mut app = open_updates();
    for (check, waiting, line) in [
        (Some(ContentCheck::Current), false, tx::CONTENT_CURRENT),
        (Some(ContentCheck::New), false, tx::CONTENT_NEW),
        (Some(ContentCheck::Failed), false, tx::CONTENT_FAILED),
        (Some(ContentCheck::Current), true, tx::CONTENT_NEW),
    ] {
        u.set_status_for_tests(|s| {
            s.phase = Phase::UpToDate;
            s.content_check = check;
            s.content_waiting = waiting;
        });
        app.settle();
        app.assert_has(line);
    }
    // A content-only update on top of build 40: its build and its date, and
    // "new" until its notes are seen.
    u.set_status_for_tests(|s| {
        s.content_build = 45;
        s.content_at = Some(1_700_000_000);
        s.history =
            vec![release(45, "2026-11-03T08:00:00Z", vec![change(ChangeKind::New, "درس الأجنة")], true)];
        s.prefs.content_from = Some("0.1.0+40".into());
        s.prefs.last_content = Some("0.1.0+45".into());
    });
    app.settle();
    app.assert_has(tx::CONTENT_PUBLISHED);
    app.assert_has("45");
    app.assert_has(tx::BADGE_NEW);
    // The row opens what it brought.
    app.tap(tx::ROW_CONTENT);
    app.assert_has(tx::CONTENT_UPDATED_TITLE);
    app.assert_has("درس الأجنة");
    app.tap(tx::GOT_IT);
    app.assert_lacks(tx::CONTENT_UPDATED_TITLE);
    assert!(!u.status().news_pending(), "seen once closed");
}

#[test]
fn the_banner_offers_the_next_step_and_waits_after_later() {
    let u = install(&dead_feed(), "0.1.0", 41, notify());
    let o = offer(
        42,
        vec![release(42, "2026-11-04T08:00:00Z", vec![change(ChangeKind::New, "حالات جديدة")], false)],
    );
    u.set_status_for_tests(|s| s.phase = Phase::Available(o.clone()));
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::AVAILABLE);
    app.assert_has("حالات جديدة");
    assert!(app.can_tap(tx::DOWNLOAD));
    for (phase, line) in [
        (Phase::Ready(o.clone()), tx::READY),
        (Phase::Installed(o.clone()), tx::INSTALLED),
        (Phase::NeedsPermission(o.clone()), tx::NEEDS_PERMISSION),
    ] {
        u.set_status_for_tests(|s| s.phase = phase);
        app.settle();
        app.assert_has(line);
    }
    // "What's new?" opens the sheet on the offer.
    u.set_status_for_tests(|s| s.phase = Phase::Ready(o.clone()));
    app.settle();
    app.tap(tx::WHATS_NEW_ASK);
    assert!(app.can_tap("us-backdrop"));
    app.tap_nth(tx::CLOSE, 0);
    assert!(!app.can_tap("us-backdrop"));
    // Later: gone until a newer version.
    app.tap(tx::LATER);
    app.assert_lacks(tx::READY);
    assert_eq!(u.status().prefs.dismissed.as_deref(), Some("0.1.0+42"));
    // Nothing to do (up to date): no banner at all.
    u.set_status_for_tests(|s| s.phase = Phase::UpToDate);
    app.settle();
    app.assert_lacks(tx::LATER);
}

#[test]
fn a_content_only_update_leaves_a_quiet_note_on_today() {
    let u = install(&dead_feed(), "0.1.0", 40, notify());
    u.set_status_for_tests(|s| {
        s.content_build = 45;
        s.history =
            vec![release(45, "2026-11-04T08:00:00Z", vec![change(ChangeKind::New, "أسئلة الصمامات")], true)];
        s.prefs.content_from = Some("0.1.0+40".into());
        s.prefs.last_content = Some("0.1.0+45".into());
    });
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::CONTENT_NOTE);
    app.assert_has("أسئلة الصمامات");
    app.tap(tx::WHATS_NEW_ASK);
    app.assert_has(tx::CONTENT_UPDATED_TITLE);
    app.assert_has(&tx::content_updates_count(1));
    app.tap(tx::GOT_IT);
    app.assert_lacks(tx::CONTENT_NOTE);

    // The ✕ marks it seen too.
    let u = install(&dead_feed(), "0.1.0", 40, notify());
    u.set_status_for_tests(|s| {
        s.content_build = 46;
        s.history =
            vec![release(46, "2026-11-04T08:00:00Z", vec![change(ChangeKind::Fixed, "صورة القلب")], true)];
        s.prefs.content_from = Some("0.1.0+40".into());
        s.prefs.last_content = Some("0.1.0+46".into());
    });
    let mut app = App::open(Screen::Today);
    app.assert_has("صورة القلب");
    app.tap(tx::CLOSE);
    app.assert_lacks(tx::CONTENT_NOTE);
    assert!(!u.status().news_pending());
}

fn many(kind: ChangeKind, n: usize, what: &str) -> Vec<Change> {
    (1..=n).map(|i| change(kind, &format!("{what} {i}"))).collect()
}

#[test]
fn the_changelog_folds_long_lists_and_older_builds() {
    let u = install(&dead_feed(), "0.1.0", 44, notify());
    // Relative dates: the sheet counts days from the real clock, so fixed
    // dates would drift out of "yesterday" / "5 days ago" as days pass.
    let ago = |days: i64| {
        (chrono::Utc::now() - chrono::Duration::days(days)).format("%Y-%m-%dT08:00:00Z").to_string()
    };
    let newest = ago(0);
    let mut history = vec![release(
        50,
        &newest,
        [
            many(ChangeKind::New, 8, "محاضرة"),
            many(ChangeKind::Improved, 2, "تحسين"),
            many(ChangeKind::Fixed, 1, "تصليح"),
        ]
        .concat(),
        false,
    )];
    let dates =
        [ago(1), ago(2), ago(5), "2026-09-01T08:00:00Z".to_owned(), "2025-12-01T08:00:00Z".to_owned()];
    for (i, d) in dates.iter().enumerate() {
        history.push(release(
            49 - i as u64,
            d,
            vec![change(ChangeKind::Fixed, &format!("إصلاح {}", 49 - i))],
            false,
        ));
    }
    history.push(release(
        44,
        "2025-11-01T08:00:00Z",
        vec![change(ChangeKind::New, "اللي معاك دلوقتي")],
        false,
    ));
    u.set_status_for_tests(|s| s.history = history.clone());
    let mut app = open_updates();
    app.tap(tx::HISTORY);
    app.assert_has(tx::HISTORY);
    // The newest is open, with its counts, and a long kind folds at five.
    app.assert_has(tx::TODAY);
    app.assert_has("محاضرة 5");
    app.assert_lacks("محاضرة 6");
    app.assert_has(tx::BADGE_LATEST);
    app.tap(&tx::SHOW_ALL.replace("{n}", "8"));
    app.assert_has("محاضرة 8");
    app.tap(tx::SHOW_LESS);
    app.assert_lacks("محاضرة 8");
    // Earlier builds: four at a time, folded until tapped.
    app.assert_has(tx::EARLIER);
    assert!(app.html().contains(r#"class="us-divider" role="separator""#), "the divider is a separator");
    app.assert_has(tx::YESTERDAY);
    app.assert_has(tx::TWO_DAYS_AGO);
    app.assert_has(&tx::DAYS_AGO.replace("{n}", "5"));
    app.assert_has(&format!("1 {}", tx::MONTHS[8]));
    app.assert_lacks("إصلاح 49");
    app.tap("v0.1.0 (49)");
    app.assert_has("إصلاح 49");
    app.tap("v0.1.0 (49)");
    app.assert_lacks("إصلاح 49");
    app.assert_lacks(tx::BADGE_INSTALLED);
    app.tap(tx::SHOW_OLDER);
    app.assert_has("2025");
    app.assert_has(tx::BADGE_INSTALLED);
    // The foot: GitHub opens the channel's releases page.
    app.tap("GitHub");
    let opened = crate::cloud::OPENED.lock().unwrap().clone();
    assert!(opened.iter().any(|u| u.contains("/releases/tag/")), "{opened:?}");
    // Tapping beside it closes it.
    app.tap("us-backdrop");
    assert!(!app.can_tap("us-backdrop"));
}

#[test]
fn an_empty_changelog_says_so() {
    install(&dead_feed(), "0.1.0", 41, notify());
    let mut app = open_updates();
    app.tap(tx::HISTORY);
    app.assert_has(tx::HISTORY_EMPTY);
}

#[test]
fn whats_new_opens_by_itself_once_after_an_update() {
    let u = install(&dead_feed(), "0.1.0", 42, notify());
    u.set_status_for_tests(|s| {
        s.history = vec![
            release(42, "2026-11-04T08:00:00Z", vec![change(ChangeKind::New, "بنك أسئلة الكارديو")], false),
            release(40, "2026-11-01T08:00:00Z", vec![change(ChangeKind::Improved, "لون أهدى")], false),
        ];
        s.prefs.whats_new_from = Some("0.1.0+39".into());
    });
    let mut app = App::open(Screen::Today);
    app.assert_has(tx::UPDATED_TITLE);
    app.assert_has("بنك أسئلة الكارديو");
    app.assert_has("#39");
    app.assert_has("#42");
    app.assert_has(&tx::builds_count(2));
    app.tap(tx::GOT_IT);
    assert!(!app.can_tap("us-backdrop"));
    assert!(u.status().whats_new().is_empty(), "seen stays seen");
}

#[test]
fn the_offer_sheet_lists_what_is_coming_and_moves_it_forward() {
    let u = install(&dead_feed(), "0.1.0", 40, notify());
    let coming = vec![
        release(42, "2026-11-04T08:00:00Z", vec![change(ChangeKind::New, "دروس الصمامات")], false),
        release(41, "2026-11-03T08:00:00Z", vec![change(ChangeKind::Fixed, "زرار المراجعة")], false),
    ];
    let o = offer(42, coming.clone());
    u.set_status_for_tests(|s| {
        s.history = [
            coming.clone(),
            vec![release(40, "2026-11-01T08:00:00Z", vec![change(ChangeKind::New, "اللي معاك")], false)],
        ]
        .concat();
        s.phase = Phase::Downloading { offer: o.clone(), done: 1024 * 1024 };
    });
    let mut app = open_updates();
    app.tap(tx::WHATS_NEW_ASK);
    app.assert_has(tx::DOWNLOADING);
    app.assert_has("#40");
    app.assert_has("#42");
    app.assert_has("1.0 ميجا / 3.0 ميجا");
    app.assert_has("دروس الصمامات");
    app.assert_has(&format!("1 {}", tx::KIND_FIXED));
    app.assert_has(tx::BADGE_NEW);
    // Ready: the sheet's own button installs.
    u.set_status_for_tests(|s| s.phase = Phase::Available(o.clone()));
    app.settle();
    assert!(app.html().contains("us-go"));
    app.tap_nth(tx::DOWNLOAD, 0);
    // The feed is out of reach: the download it started fails.
    until(&mut app, u, "the failed download", |s| matches!(s.phase, Phase::Failed { .. }));
    // An offer without changelog lines shows the feed's notes instead.
    u.set_status_for_tests(|s| s.phase = Phase::Installing(offer(42, Vec::new())));
    app.settle();
    app.assert_has("أسئلة جديدة في الكارديو");
    app.assert_has(tx::INSTALLING);
    app.tap_nth(tx::CLOSE, 0);
}

thread_local! {
    static NAV: std::cell::Cell<Option<crate::update::sheet::UpdateSheetNav>> = const { std::cell::Cell::new(None) };
}

fn keep_sheet_nav() {
    NAV.with(|n| n.set(Some(dioxus::prelude::consume_context())));
}

#[test]
fn a_notification_tap_opens_the_offer_or_the_changelog() {
    let u = install(&dead_feed(), "0.1.0", 41, notify());
    let mut app = App::with(Screen::Today, keep_sheet_nav);
    let nav = NAV.with(std::cell::Cell::get).expect("the sheet's nav");
    app.run(move || nav.open_offer());
    app.assert_has(tx::HISTORY_EMPTY);
    app.tap_nth(tx::CLOSE, 0);
    u.set_status_for_tests(|s| s.phase = Phase::Ready(offer(42, Vec::new())));
    app.settle();
    app.run(move || nav.open_offer());
    assert!(app.html().contains("us-go"), "the offer, with its button");
}

#[test]
fn content_that_came_while_open_is_switched_to_between_sessions() {
    let u = install(&dead_feed(), "0.1.0", 40, notify());
    let mut app = App::open(Screen::Study);
    u.pack_waiting_for_tests(43);
    let rev = app.run(|| *dioxus::prelude::ReadableExt::peek(&crate::content::REV));
    // Mid-session it waits.
    app.wait(2300);
    assert!(u.status().content_waiting, "never mid-session");
    // Between sessions it switches, and the screens are drawn again.
    app.tap(Screen::Today.label());
    let end = Instant::now() + Duration::from_secs(10);
    while u.status().content_waiting {
        assert!(Instant::now() < end, "never switched");
        app.wait(50);
    }
    assert_eq!(u.status().content_build, 43);
    assert!(app.run(|| *dioxus::prelude::ReadableExt::peek(&crate::content::REV)) > rev);
}
