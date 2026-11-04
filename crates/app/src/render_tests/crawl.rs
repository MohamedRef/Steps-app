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

fn crawl(screen: Screen) {
    let first = open(screen).tappables();
    for sig in first {
        let mut app = open(screen);
        let before = app.tappables();
        // A label that changes on its own (a clock, a random tip) is skipped.
        if !app.tap_signature(&sig) {
            continue;
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
    }
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
