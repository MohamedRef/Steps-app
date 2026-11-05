//! Settings › Sync in every state the engine reports, and the commands its
//! buttons send.

use std::cell::RefCell;

use dioxus::prelude::*;
use steps_protocol::cloud::{DeviceSummary, Platform};
use tokio::sync::mpsc::UnboundedReceiver;

use super::App;
use crate::Screen;
use crate::cloud::{Cloud, CloudStatus, Command, Problem, SyncState};
use crate::i18n::cloud as tx;

thread_local! {
    static CLOUD: RefCell<Option<(Cloud, UnboundedReceiver<Command>)>> = const { RefCell::new(None) };
    static REVISION: std::cell::Cell<Option<Signal<u64>>> = const { std::cell::Cell::new(None) };
}

fn listen() {
    let cloud = consume_context::<Cloud>();
    let rx = cloud.listen_for_tests();
    REVISION.with(|r| r.set(Some(consume_context::<crate::store::Review>().revision)));
    CLOUD.with(|c| *c.borrow_mut() = Some((cloud, rx)));
}

fn cloud() -> Cloud {
    CLOUD.with(|c| c.borrow().as_ref().unwrap().0)
}

/// The commands sent since the last look.
fn sent() -> Vec<String> {
    CLOUD.with(|c| {
        let mut c = c.borrow_mut();
        let rx = &mut c.as_mut().unwrap().1;
        let mut out = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            out.push(format!("{cmd:?}"));
        }
        out
    })
}

fn open_sync() -> App {
    let mut app = App::with(Screen::Today, listen);
    app.tap(Screen::Settings.label());
    app.tap(tx::SECTION);
    sent();
    app
}

fn status(sync: SyncState) -> CloudStatus {
    CloudStatus {
        configured: true,
        email: None,
        device_id: "d-mac".into(),
        sync,
        pending: 0,
        last_sync_ms: None,
        problem: None,
        finishing_sign_in: false,
        temporary: false,
    }
}

fn set(app: &mut App, s: CloudStatus) {
    app.run(move || cloud().status.set(s));
}

fn device(
    id: &str,
    name: &str,
    platform: Platform,
    current: bool,
    revoked: bool,
    last: Option<u64>,
) -> DeviceSummary {
    DeviceSummary {
        device_id: id.into(),
        name: name.into(),
        model: None,
        platform,
        signed_in_at_ms: 1,
        last_seen_ms: 1,
        last_sync_ms: last,
        current,
        revoked,
    }
}

#[test]
fn a_build_without_steps_cloud_says_so() {
    let app = open_sync();
    app.assert_has(tx::TITLE);
    app.assert_has(tx::NOT_CONFIGURED);
    assert!(!app.can_tap(tx::SIGN_IN));
}

#[test]
fn signing_in_shows_each_step_and_its_problems() {
    let mut app = open_sync();
    set(&mut app, status(SyncState::SignedOut));
    app.tap(tx::SIGN_IN);
    assert_eq!(sent(), ["SignInGoogle"]);
    app.run(|| cloud().signing_in.set(true));
    app.assert_has(tx::SIGNING_IN);
    app.run(|| cloud().signing_in.set(false));
    set(
        &mut app,
        CloudStatus {
            finishing_sign_in: true,
            problem: Some(Problem::SignInWaiting),
            ..status(SyncState::SignedOut)
        },
    );
    app.assert_has(tx::FINISHING);
    app.assert_has(tx::P_SIGN_IN_WAITING);
    app.tap(tx::TRY_NOW);
    assert_eq!(sent(), ["Kick"]);
    for (p, line) in [
        (Problem::NotConfigured, tx::P_NOT_CONFIGURED),
        (Problem::NotAllowed, tx::P_NOT_ALLOWED),
        (Problem::SignInCancelled, tx::P_CANCELLED),
        (Problem::SignInFailed, tx::P_FAILED),
        (Problem::SignInInterrupted, tx::P_INTERRUPTED),
        (Problem::SignedOutRemotely, tx::P_REMOTE),
        (Problem::Network, tx::P_NETWORK),
        (Problem::Server, tx::P_SERVER),
        (Problem::Outdated, tx::P_OUTDATED),
        (Problem::ClockOff, tx::P_CLOCK),
    ] {
        set(&mut app, CloudStatus { problem: Some(p), ..status(SyncState::SignedOut) });
        app.assert_has(line);
    }
}

#[test]
fn signed_in_it_shows_the_account_the_state_and_the_devices() {
    let mut app = open_sync();
    let now = crate::store::now_ms() as u64;
    set(
        &mut app,
        CloudStatus {
            email: Some("mohamed@example.com".into()),
            pending: 3,
            last_sync_ms: Some(now - 5 * 60_000),
            ..status(SyncState::Connecting)
        },
    );
    app.assert_has("mohamed@example.com");
    app.assert_has(tx::SIGNED_IN_AS);
    app.assert_has(tx::CONNECTING);
    app.assert_has(&tx::pending(3));
    app.assert_has(&format!("{}: {}", tx::LAST, crate::cloud::ago(now - 5 * 60_000, now)));
    app.assert_has(tx::LOADING);
    for (state, line) in [
        (SyncState::CatchingUp, tx::CATCHING_UP),
        (SyncState::Live, tx::LIVE),
        (SyncState::Offline, tx::OFFLINE),
        (SyncState::Stopped, tx::STOPPED),
    ] {
        set(&mut app, CloudStatus { email: Some("mohamed@example.com".into()), ..status(state) });
        app.assert_has(line);
        // Unreachable, the cards waiting on the server say so instead of
        // «بيحمّل…» forever, and Refresh has nothing to ask (UX #581).
        let away = matches!(state, SyncState::Offline | SyncState::Stopped);
        assert_eq!(app.html().contains(tx::WAITING_SERVER), away, "{state:?}");
        assert_eq!(app.can_tap(tx::REFRESH), !away, "{state:?}");
    }
    set(&mut app, CloudStatus { email: Some("mohamed@example.com".into()), ..status(SyncState::Live) });
    // Live or catching up: the Devices list is asked for.
    assert!(sent().iter().filter(|c| *c == "LoadDevices").count() >= 1);

    let list = vec![
        device("d-mac", "Mohamed’s MacBook Air", Platform::Macos, true, false, Some(now)),
        device("d-tab", "SM-X730", Platform::Android, false, false, Some(now - 2 * 3_600_000)),
        device("d-s24", "SM-S928B", Platform::Android, false, false, None),
        device("d-web", "Safari", Platform::Web, false, true, None),
        device("d-pi", "Raspberry", Platform::Other, false, false, None),
    ];
    app.run(move || cloud().devices.set(Some(Ok(list))));
    app.assert_has(tx::DEVICES);
    app.assert_has(tx::THIS_DEVICE);
    app.assert_has("SM-X730");
    app.assert_has(tx::NEVER_SYNCED);
    app.assert_has(tx::DEVICE_OUT);
    assert!(app.html().contains("device-row revoked"));
    // Only other, signed-in devices can be signed out from here.
    app.tap_nth(tx::SIGN_OUT_DEVICE, 0);
    assert_eq!(sent(), [r#"RevokeDevice("d-tab")"#]);
    assert!(app.can_tap(tx::SIGN_OUT_DEVICE));
    app.tap(tx::REFRESH);
    assert_eq!(sent(), ["LoadDevices"]);
    app.run(|| cloud().devices.set(Some(Err(Problem::Server))));
    app.assert_has(tx::P_SERVER);
    app.tap(tx::SIGN_OUT);
    assert_eq!(sent(), ["SignOut"]);
}

#[test]
fn a_new_answer_kicks_the_outbox() {
    let mut app = open_sync();
    let mut revision = REVISION.with(std::cell::Cell::get).unwrap();
    app.run(move || *revision.write() += 1);
    assert!(sent().contains(&"Kick".to_owned()));
}

thread_local! {
    static ENGINE: RefCell<Option<(UnboundedReceiver<Command>, tokio::sync::mpsc::UnboundedSender<crate::cloud::Event>)>> =
        const { RefCell::new(None) };
    static SHOWN: std::cell::Cell<Option<Cloud>> = const { std::cell::Cell::new(None) };
}

fn stand_in_engine() {
    let cloud = consume_context::<Cloud>();
    SHOWN.with(|c| c.set(Some(cloud)));
    let pair = cloud.engine_for_tests();
    ENGINE.with(|e| *e.borrow_mut() = Some(pair));
}

/// The engine says `event`; the screen hears it on its next turn.
fn engine_says(app: &mut App, event: crate::cloud::Event) {
    ENGINE.with(|e| e.borrow().as_ref().unwrap().1.send(event).unwrap());
    app.wait(30);
}

fn engine_got() -> Vec<String> {
    ENGINE.with(|e| {
        let mut e = e.borrow_mut();
        let rx = &mut e.as_mut().unwrap().0;
        std::iter::from_fn(|| rx.try_recv().ok()).map(|c| format!("{c:?}")).collect()
    })
}

#[test]
fn what_the_sync_engine_reports_reaches_the_screen() {
    use crate::cloud::Event;
    let mut app = App::with(Screen::Today, stand_in_engine);
    app.tap(Screen::Settings.label());
    app.tap(tx::SECTION);
    engine_got();
    let live = CloudStatus { email: Some("mohamed@example.com".into()), ..status(SyncState::Live) };
    engine_says(&mut app, Event::Status(live.clone()));
    app.assert_has(tx::LIVE);
    app.assert_has("mohamed@example.com");
    // The same status again changes nothing.
    engine_says(&mut app, Event::Status(live));
    app.assert_has(tx::LIVE);
    let signing_in = |app: &mut App| app.run(|| *SHOWN.with(std::cell::Cell::get).unwrap().signing_in.peek());
    engine_says(&mut app, Event::SigningIn(true));
    assert!(signing_in(&mut app));
    engine_says(&mut app, Event::SigningIn(false));
    assert!(!signing_in(&mut app));
    engine_says(
        &mut app,
        Event::Devices(Ok(vec![
            device("d-mac", "Mohamed’s MacBook Air", Platform::Macos, true, false, None),
            device("d-tab", "SM-X730", Platform::Android, false, false, None),
        ])),
    );
    app.assert_has("SM-X730");
    // The screen's buttons reach the engine.
    app.tap(tx::REFRESH);
    assert!(engine_got().contains(&"LoadDevices".to_owned()));
    // The tablet switched sounds off: a sync brings it, and the screens follow.
    crate::store::shared()
        .put_setting(
            "sound",
            &crate::sounds::SoundPrefs { on: false, ..crate::sounds::SoundPrefs::default() },
        )
        .unwrap();
    engine_says(&mut app, Event::Changed(["settings".to_owned()].into()));
    assert!(!app.run(|| crate::sounds::PREFS.peek().on));
    // The reload counts as a change: the outbox is kicked.
    assert!(engine_got().contains(&"Kick".to_owned()));
}
