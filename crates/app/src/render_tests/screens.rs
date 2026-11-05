//! Every screen opens and draws its own content.

use super::App;
use crate::Screen;

#[test]
fn every_screen_draws_with_its_name() {
    for screen in Screen::NAV {
        let app = App::open(screen);
        let html = app.html();
        assert!(html.contains(&format!("data-screen=\"{screen:?}\"")), "{screen:?}");
        assert!(html.contains(screen.label()), "{screen:?} has its label");
    }
}

#[test]
fn the_nav_moves_between_screens() {
    let mut app = App::open(Screen::Today);
    for screen in Screen::NAV {
        app.tap(screen.label());
        assert!(
            app.html().contains(&format!("data-screen=\"{screen:?}\"")),
            "{screen:?}:\n{}",
            app.listing("click")
        );
    }
}

#[test]
#[ignore]
fn explore() {
    let which = std::env::var("SCREEN").unwrap_or_default();
    for screen in Screen::NAV.into_iter().chain([Screen::Settings]) {
        if !which.is_empty() && !format!("{screen:?}").eq_ignore_ascii_case(&which) {
            continue;
        }
        let mut app = App::open(Screen::Today);
        if screen == Screen::Settings {
            app.tap_nth(Screen::Settings.label(), 0);
        } else {
            app.tap(screen.label());
        }
        for t in std::env::var("TAPS").unwrap_or_default().split('|').filter(|t| !t.is_empty()) {
            app.tap(t);
        }
        let mut out = format!("==== {screen:?}\n{}\n---- text\n{}\n", app.listing("click"), app.text());
        for ev in ["input", "change", "keydown", "pointerdown"] {
            out += &format!("---- {ev}\n{}\n", app.listing(ev));
        }
        std::fs::write(std::env::temp_dir().join(format!("explore-{screen:?}.txt")), out).unwrap();
    }
}
