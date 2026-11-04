//! What the platform layer does where Steps has no native bridge (the
//! desktop build that the tests and `cargo run` use): every call is a safe
//! no-op or a clear "not here" answer, so the screens above it carry on.
//! The Android and Mac bridges themselves need a device (the Mac workflow
//! and the Android preview build run them).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;

use super::documents::{self, DocState};
use super::haptics::{self, Haptic};
use super::tts::{self, SpeakError};
use super::{background, network, reminders, system_bars, time, update_notice, webview_guard};
use crate::Screen;
use crate::render_tests::App;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

#[test]
fn time_sleep_waits_and_timeout_tells_done_from_late() {
    rt().block_on(async {
        let start = std::time::Instant::now();
        time::sleep(Duration::from_millis(5)).await;
        assert!(start.elapsed() >= Duration::from_millis(5));

        let done = time::timeout(Duration::from_secs(5), async { 7 }).await;
        assert_eq!(done, Ok(7));
        let late = time::timeout(Duration::from_millis(5), time::sleep(Duration::from_secs(5))).await;
        assert_eq!(late, Err(time::Elapsed));
    });
}

#[test]
fn time_blocking_runs_off_the_ui_thread_and_reports_a_panic() {
    rt().block_on(async {
        let ui = std::thread::current().id();
        let worker = time::blocking(|| std::thread::current().id()).await.unwrap();
        assert_ne!(worker, ui, "blocking work leaves the UI thread");
        let failed = time::blocking(|| -> u8 { panic!("a lecture file that does not parse") }).await;
        assert!(failed.is_err_and(|e| e.contains("panic")));
    });
}

#[test]
fn speech_bounds_grow_with_the_question() {
    assert_eq!(tts::longest(""), Duration::from_millis(4_000));
    // A Step 1 vignette stem of 40 words: 4 s start-up plus 0.4 s a word.
    let stem = "word ".repeat(40);
    assert_eq!(tts::longest(&stem), Duration::from_millis(20_000));
    assert!(tts::RATES.windows(2).all(|w| w[0] < w[1]), "rates go slow to fast");
    assert!(tts::RATES.contains(&0.95));
}

#[test]
fn speech_with_nothing_to_say_is_done_at_once() {
    rt().block_on(async {
        assert_eq!(tts::say("", 1.0).await, Ok(()));
        assert_eq!(tts::say("  \n\t", 1.0).await, Ok(()));
    });
}

#[test]
fn speech_without_a_webview_fails_quietly_and_the_session_goes_on() {
    // The screen tests have no WebView: `eval` cannot run, so `say` reports
    // a failure (logged, not shown) and oral mode shows the text instead.
    let mut app = App::open(Screen::Today);
    let out: Rc<RefCell<Option<Result<(), SpeakError>>>> = Rc::default();
    let slot = out.clone();
    app.run(move || {
        spawn(async move {
            let said = tts::say("Which enzyme is deficient in Lesch-Nyhan syndrome?", 1.0).await;
            *slot.borrow_mut() = Some(said);
        });
    });
    app.wait(50);
    let said = out.borrow_mut().take().expect("say finished");
    assert!(matches!(said, Err(SpeakError::Failed(_))), "{said:?}");
    app.run(tts::stop);
}

#[test]
fn documents_are_android_only() {
    assert_eq!(documents::save("/tmp/steps-export.json", "steps-export.json"), Err("not on Android".into()));
    assert_eq!(documents::open(), Err("not on Android".into()));
    assert_eq!(documents::state(), DocState::Failed);
    assert_eq!(rt().block_on(documents::finished()), DocState::Failed);
    assert!(documents::POLL < Duration::from_secs(1));
}

#[test]
fn haptics_are_off_and_silent_here() {
    assert!(!haptics::available());
    for kind in [Haptic::Right, Haptic::Wrong, Haptic::SessionEnd] {
        haptics::vibrate(kind);
    }
}

#[test]
fn haptic_codes_match_the_android_bridge() {
    // app.steps.Haptics.play reads 0 right, 1 wrong, 2 session end.
    let codes = [Haptic::Right, Haptic::Wrong, Haptic::SessionEnd].map(|h| h.code());
    assert_eq!(codes, [0, 1, 2]);
}

#[test]
fn the_desktop_connection_is_never_metered() {
    assert!(!network::metered());
}

#[test]
fn background_oral_mode_is_a_no_op_off_android() {
    let labels = background::Labels {
        show: "show",
        again: "again",
        good: "good",
        end: "end",
        channel: "oral",
        channel_hint: "oral mode",
    };
    background::start(&labels);
    background::update("cvs.L01", "Which embryonic structure gives rise to the SVC?", false);
    background::update("cvs.L01", "Right common cardinal vein", true);
    assert_eq!(background::take(), None);
    background::stop();
    assert_eq!(background::take(), None);
}

#[test]
fn reminders_are_not_local_here_and_every_call_is_safe() {
    assert!(!reminders::local(), "Settings hides the reminder rows");
    reminders::schedule(r#"[{"date":"2026-11-05","time":"20:00","title":"Steps","text":"cvs.L01"}]"#);
    reminders::schedule("[]");
    reminders::repeat("{}");
    assert_eq!(reminders::enabled(), None);
    assert_eq!(reminders::battery_unrestricted(), None);
    reminders::request_permission();
    reminders::open_notification_settings();
    reminders::open_app_settings();
}

#[test]
fn update_notifications_are_android_only() {
    let n = crate::update::Notice {
        key: "1.0.0-alpha.2+12:ready".into(),
        title: "Steps · 1.0.0-alpha.2".into(),
        body: "cvs.L02".into(),
        action: "install",
        button: "install",
        channel: "updates",
        channel_description: "new versions",
    };
    assert!(!update_notice::post(&n));
    update_notice::schedule(true);
    update_notice::schedule(false);
    update_notice::clear();
    assert_eq!(update_notice::take_tap(), None);
}

#[test]
fn the_webview_guard_installs_quietly_off_android() {
    webview_guard::install();
}

#[test]
fn system_bars_take_the_theme_as_opaque_argb() {
    assert_eq!(system_bars::argb("#000000"), 0xff00_0000_u32 as i32);
    assert_eq!(system_bars::argb("#ffffff"), -1);
    assert_eq!(system_bars::argb("1a2b3c"), 0xff1a_2b3c_u32 as i32);
    assert_eq!(system_bars::argb("not a colour"), 0xff00_0000_u32 as i32, "a bad colour is black");
    system_bars::apply("#fafafa", "#000000");
    assert_eq!(system_bars::device_dark(), None, "the page's prefers-color-scheme decides");
}
