//! Every button on every screen, and what each one opens: none of them may
//! panic or leave the app without a screen.

use super::App;
use crate::Screen;

fn open(screen: Screen) -> App {
    let mut app = App::open(Screen::Today);
    if screen == Screen::Settings {
        app.tap(Screen::Settings.label());
    } else {
        app.tap(screen.label());
    }
    app
}

/// Runs `f` on a thread of its own: each test thread has its own data
/// (`prefs::test_dir`, the in-memory store), so `f` starts from an empty app
/// whatever earlier taps saved.
fn fresh<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(f)
        .expect("a crawl thread starts")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}

fn crawl(screen: Screen) {
    let first = fresh(move || open(screen).tappables());
    let mut missed = Vec::new();
    for sig in first {
        // Every button starts from a fresh app. Taps saved by an earlier
        // button (Today's session-length chips change the hero's label) would
        // otherwise rename the next one, and the crawl would skip it.
        let tapped = fresh({
            let sig = sig.clone();
            move || crawl_from(screen, &sig)
        });
        if !tapped {
            missed.push(sig);
        }
    }
    assert!(missed.is_empty(), "{screen:?}: a fresh app did not show {missed:#?}");
}

/// Taps `sig` on `screen`, then what it brought up, two levels deep.
/// False when a fresh app does not show `sig`.
fn crawl_from(screen: Screen, sig: &str) -> bool {
    let mut app = open(screen);
    let before = app.tappables();
    if !app.tap_signature(sig) {
        return false;
    }
    // Then everything the tap brought up, in turn, in the same app.
    let new: Vec<String> = app.tappables().into_iter().filter(|s| !before.contains(s)).take(25).collect();
    for next in new {
        let seen = app.tappables();
        if app.tap_signature(&next) {
            for third in app.tappables().into_iter().filter(|s| !seen.contains(s)).take(6) {
                app.tap_signature(&third);
            }
        }
    }
    assert!(app.html().contains("data-screen="), "{screen:?} after {sig}");
    true
}

#[test]
fn crawl_today() {
    crawl(Screen::Today);
}
#[test]
fn crawl_learn() {
    crawl(Screen::Learn);
}
#[test]
fn crawl_study() {
    crawl(Screen::Study);
}
#[test]
fn crawl_bank() {
    crawl(Screen::Bank);
}
#[test]
fn crawl_cases() {
    crawl(Screen::Cases);
}
#[test]
fn crawl_oral() {
    crawl(Screen::Oral);
}
#[test]
fn crawl_stats() {
    crawl(Screen::Stats);
}
#[test]
fn crawl_settings() {
    crawl(Screen::Settings);
}
