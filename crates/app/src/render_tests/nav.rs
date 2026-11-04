//! Moving around: the phone's More sheet and the rail's tools, and the
//! system Back button (Android), driven through a test document that plays
//! the part of the WebView's back script.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use dioxus::document::{Document, Eval, EvalError, Evaluator};
use dioxus::prelude::*;
use dioxus::signals::{AnyStorage, Owner, UnsyncStorage};

use super::App;
use super::lecture::screen;
use crate::i18n::nav as tx;
use crate::{AppAt, Screen};

const BACK_JS: &str = include_str!("../../assets/system-back.js");

thread_local! {
    /// Messages the back script sends to Rust, oldest first.
    static INBOX: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
    static WAKER: RefCell<Option<Waker>> = const { RefCell::new(None) };
    /// Every script the app ran, in order.
    static SCRIPTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Keeps each script's evaluator alive.
    static OWNERS: RefCell<Vec<Owner<UnsyncStorage>>> = const { RefCell::new(Vec::new()) };
}

/// A document whose back script answers from [`INBOX`]; other scripts do nothing.
struct TestDocument;

struct Script {
    back: bool,
}

impl Evaluator for Script {
    fn send(&self, _: serde_json::Value) -> Result<(), EvalError> {
        Ok(())
    }

    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Result<serde_json::Value, EvalError>> {
        if !self.back {
            return Poll::Ready(Err(EvalError::Unsupported));
        }
        match INBOX.with_borrow_mut(VecDeque::pop_front) {
            Some(m) => Poll::Ready(Ok(serde_json::Value::String(m))),
            None => {
                WAKER.with_borrow_mut(|w| *w = Some(cx.waker().clone()));
                Poll::Pending
            }
        }
    }

    fn poll_join(&mut self, _: &mut Context<'_>) -> Poll<Result<serde_json::Value, EvalError>> {
        Poll::Ready(Ok(serde_json::Value::Null))
    }
}

impl Document for TestDocument {
    fn eval(&self, js: String) -> Eval {
        let back = js == BACK_JS;
        SCRIPTS.with_borrow_mut(|s| s.push(js));
        let owner = UnsyncStorage::owner();
        let ev = owner.insert(Box::new(Script { back }) as Box<dyn Evaluator>);
        OWNERS.with_borrow_mut(|o| o.push(owner));
        Eval::new(ev)
    }
}

fn root() -> Element {
    use_context_provider(|| Rc::new(TestDocument) as Rc<dyn Document>);
    rsx! { AppAt { start: Screen::Today } }
}

fn start() -> App {
    INBOX.with_borrow_mut(VecDeque::clear);
    SCRIPTS.with_borrow_mut(Vec::clear);
    super::learn::mount(root)
}

/// The back script reports a press of the system Back button.
fn press_back(app: &mut App) {
    INBOX.with_borrow_mut(|q| q.push_back("back".into()));
    if let Some(w) = WAKER.with_borrow_mut(Option::take) {
        w.wake();
    }
    app.settle();
}

/// What the app last told the back script: whether Back has somewhere to go.
fn told() -> Option<bool> {
    SCRIPTS.with_borrow(|s| {
        s.iter().rev().find_map(|js| {
            if js.contains("__stepsSetScreenBack(true)") {
                Some(true)
            } else if js.contains("__stepsSetScreenBack(false)") {
                Some(false)
            } else {
                None
            }
        })
    })
}

#[test]
fn system_back_walks_the_screens_visited_then_stops_at_today() {
    let mut app = start();
    assert_eq!(told(), Some(false), "nothing to go back to on Today");
    INBOX.with_borrow_mut(|q| q.push_back("ready".into()));
    app.tap(Screen::Learn.label());
    app.tap(Screen::Study.label());
    assert_eq!(told(), Some(true));
    press_back(&mut app);
    assert_eq!(screen(&app), "Learn");
    press_back(&mut app);
    assert_eq!(screen(&app), "Today");
    assert_eq!(told(), Some(false));
    // On Today, Back has nowhere to go in the app.
    press_back(&mut app);
    assert_eq!(screen(&app), "Today");
}

#[test]
fn more_holds_the_screens_off_the_phone_bar_and_the_tools() {
    for (row, check) in
        [(Screen::Cases.label(), "Cases"), (Screen::Oral.label(), "Oral"), (Screen::Stats.label(), "Stats")]
    {
        let mut app = start();
        app.tap("nav-more");
        app.assert_has(tx::MORE_PRACTICE);
        app.assert_has(tx::MORE_TOOLS);
        super::sheets::tap_row(&mut app, "more-row", row);
        assert_eq!(screen(&app), check);
        app.assert_lacks(tx::MORE_PRACTICE);
    }
    // Tools open over the screen.
    let mut app = start();
    app.tap("nav-more");
    super::sheets::tap_row(&mut app, "more-row", crate::i18n::refs::TITLE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Today)).as_deref(), Some(crate::lecture::REFERENCES));
    let mut app = start();
    app.tap("nav-more");
    super::sheets::tap_row(&mut app, "more-row", crate::i18n::abbr::DICT_TITLE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Today)).as_deref(), Some(crate::abbrdict::PAGE));
    let mut app = start();
    app.tap("nav-more");
    super::sheets::tap_row(&mut app, "more-row", crate::i18n::courses::TITLE);
    assert_eq!(app.run(|| crate::lecture::shown(Screen::Today)).as_deref(), Some(crate::courses::MAP));
    let mut app = start();
    app.tap("nav-more");
    super::sheets::tap_row(&mut app, "more-row", crate::i18n::refs::ADD_CARD);
    app.assert_lacks(tx::MORE_TOOLS);
    let mut app = start();
    app.tap("nav-more");
    super::sheets::tap_row(&mut app, "more-row", Screen::Settings.label());
    app.assert_lacks(tx::MORE_TOOLS);
}

#[test]
fn more_closes_by_its_button_the_backdrop_escape_or_a_second_tap() {
    let mut app = start();
    app.tap("nav-more");
    app.tap("more-close");
    app.assert_lacks(tx::MORE_TOOLS);
    app.tap("nav-more");
    app.tap("more-backdrop");
    app.assert_lacks(tx::MORE_TOOLS);
    app.tap("nav-more");
    super::learn::press(&mut app, "more-sheet", "Escape");
    app.assert_lacks(tx::MORE_TOOLS);
    app.tap("nav-more");
    app.tap("nav-more");
    app.assert_lacks(tx::MORE_TOOLS);
}

#[test]
fn the_rail_tools_open_their_pages() {
    for (label, page) in [
        (crate::i18n::courses::TITLE, crate::courses::MAP),
        (crate::i18n::refs::TITLE, crate::lecture::REFERENCES),
        (crate::i18n::abbr::DICT_TITLE, crate::abbrdict::PAGE),
    ] {
        let mut app = start();
        super::sheets::tap_row(&mut app, "nav-tool", label);
        assert_eq!(app.run(|| crate::lecture::shown(Screen::Today)).as_deref(), Some(page));
    }
    let mut app = start();
    super::sheets::tap_row(&mut app, "nav-tool", crate::i18n::refs::ADD_CARD);
    assert_eq!(screen(&app), "Today");
}
