//! Which bar tab a session sits under (UX-36). A session Today started stays
//! under Today while it runs and on its summary, so the bar doesn't jump to
//! «المراجعة» under him; once he leaves the session, or starts another from
//! the setup, it is the Review tab's again. Any other way in is the Review
//! tab's from the start.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::{Screen, Ui};

/// The screen before the one on show, and the one on show.
#[derive(Clone)]
pub(super) struct Trail(Rc<Cell<[Option<Screen>; 2]>>);

/// The tab the session on screen belongs to, when it isn't Review's.
static TAB: GlobalSignal<Option<Screen>> = Signal::global(|| None);

/// Kept by the app shell: notes each change of screen, and hands the
/// session back to the Review tab once he leaves it.
pub(crate) fn use_screen_trail() {
    let ui = try_use_context::<Ui>();
    let trail = use_context_provider(|| Trail(Rc::new(Cell::new([None, None]))));
    // The shell renders on every change of screen, before the new screen's
    // own effects run, so Study knows where its START came from.
    if let Some(ui) = ui {
        let now = *ui.screen.peek();
        let [_, last] = trail.0.get();
        if last != Some(now) {
            trail.0.set([last, Some(now)]);
        }
    }
    use_effect(move || {
        let Some(ui) = ui else { return };
        if *ui.screen.read() != Screen::Study && TAB.peek().is_some() {
            *TAB.write() = None;
        }
    });
}

/// Study's view of the trail (none in the browser fixtures' shell).
pub(super) fn use_trail() -> Option<Trail> {
    try_use_context::<Trail>()
}

/// A START was just taken: when it came with the move from Today (the
/// Study screen had only just opened), the session stays under Today.
pub(super) fn started(trail: Option<&Trail>, just_opened: bool) {
    let from_today =
        just_opened && trail.is_some_and(|t| t.0.get() == [Some(Screen::Today), Some(Screen::Study)]);
    let tab = from_today.then_some(Screen::Today);
    if *TAB.peek() != tab {
        *TAB.write() = tab;
    }
}

/// Back to the setup: that is the Review tab's.
pub(super) fn release() {
    if TAB.peek().is_some() {
        *TAB.write() = None;
    }
}

/// The bar tab to mark for `screen` (main.rs `NavButton`).
pub fn shown(screen: Screen) -> Screen {
    match screen {
        Screen::Study => TAB.read().unwrap_or(screen),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_tests::App;
    use crate::study::{START, Start};

    /// The bar tabs marked as the current page.
    fn marked(app: &App) -> Vec<String> {
        app.html()
            .split("<button")
            .skip(1)
            .filter_map(|tag| tag.split('>').next())
            .filter(|tag| tag.contains("class=\"nav-item") && tag.contains("aria-current=\"page\""))
            .filter_map(|tag| tag.split("aria-label=\"").nth(1)?.split('"').next().map(str::to_string))
            .collect()
    }

    /// Today's "or the cards" button: a session started from Today.
    fn from_today() -> App {
        let mut app = App::open(Screen::Today);
        assert_eq!(marked(&app), [Screen::Today.label()]);
        let minutes = app.run(crate::onboarding::session_minutes);
        app.tap(&crate::i18n::today::cards_anyway(minutes));
        app.assert_has(crate::i18n::study::END);
        app
    }

    #[test]
    fn a_session_from_today_stays_under_today_and_its_summary_too() {
        let mut app = from_today();
        assert_eq!(marked(&app), [Screen::Today.label()], "the session Today started");
        app.tap(crate::i18n::study::END);
        assert_eq!(marked(&app), [Screen::Today.label()], "its summary");
        // Another session from the setup is the Review tab's.
        app.tap(crate::i18n::study::ANOTHER);
        assert_eq!(marked(&app), [Screen::Study.label()]);
    }

    #[test]
    fn leaving_the_session_hands_it_to_the_review_tab() {
        let mut app = from_today();
        app.tap(Screen::Learn.label());
        app.tap(Screen::Study.label());
        app.assert_has(crate::i18n::study::END);
        assert_eq!(marked(&app), [Screen::Study.label()]);
    }

    #[test]
    fn a_session_asked_for_on_the_study_screen_is_the_review_tabs() {
        let mut app = from_today();
        app.tap(crate::i18n::study::END);
        // "Study these" from a sheet over the summary.
        let ids = crate::render_tests::kit::ids_of("card").into_iter().take(1).collect::<Vec<_>>();
        app.run(move || *START.write() = Some(Start { lectures: None, items: Some(ids), minutes: 5 }));
        app.assert_has(crate::i18n::study::END);
        assert_eq!(marked(&app), [Screen::Study.label()]);
    }

    #[test]
    fn a_session_from_the_setup_is_the_review_tabs() {
        let mut app = App::open(Screen::Study);
        app.tap(crate::i18n::study::START);
        app.assert_has(crate::i18n::study::END);
        assert_eq!(marked(&app), [Screen::Study.label()]);
    }
}
