//! Screen tests: the whole app (`AppAt`) over the real bundled content, run
//! in a virtual DOM whose edits a small page model follows (`dom.rs`), so a
//! test opens a screen, taps buttons by their words, types into fields and
//! reads what the screen says. Each test thread has its own empty data
//! (`prefs::test_dir`, an in-memory store). Layout and real taps on a screen
//! are the browser checks' job (`*/browser.rs`).

#![allow(dead_code)] // helpers for the screen tests; not every test uses each one

mod crawl;
mod dom;
mod screens;

use std::time::Duration;

use dioxus::dioxus_core::{ElementId, Event};
use dioxus::prelude::*;
use dioxus_html::{EventData, SerializedFormData, SerializedMouseData};

use crate::{AppAt, Screen};
use dom::Dom;

/// The app on a screen, ready to be tapped.
pub struct App {
    vdom: VirtualDom,
    page: Dom,
    rt: tokio::runtime::Runtime,
}

#[derive(Props, Clone)]
struct RootProps {
    start: Screen,
    setup: fn(),
}
impl PartialEq for RootProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn root(p: RootProps) -> Element {
    rsx! { AppAt { start: p.start, Setup { run: p.setup } } }
}

#[derive(Props, Clone)]
struct SetupProps {
    run: fn(),
}
impl PartialEq for SetupProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[allow(non_snake_case)]
fn Setup(p: SetupProps) -> Element {
    use_hook(p.run);
    rsx! {}
}

impl App {
    /// A first launch on `start`, onboarding showing.
    pub fn first_launch(start: Screen) -> App {
        App::start(start, || {}, false)
    }

    /// The app on `start`, onboarding done.
    pub fn open(start: Screen) -> App {
        App::with(start, || {})
    }

    /// The app on `start` after `setup` ran inside it, with its contexts.
    pub fn with(start: Screen, setup: fn()) -> App {
        App::start(start, setup, true)
    }

    fn start(start: Screen, setup: fn(), onboarded: bool) -> App {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let dir = crate::prefs::test_dir();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if onboarded {
            crate::onboarding::mark_done_for_tests();
        }
        // Timers and background work in the screens need a runtime, as in the app.
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut vdom = VirtualDom::new_with_props(root, RootProps { start, setup });
        let mut page = Dom::default();
        {
            let _rt = rt.enter();
            vdom.rebuild(&mut page);
        }
        let mut app = App { vdom, page, rt };
        app.settle();
        app
    }

    /// Lets signals, effects and short tasks finish.
    pub fn settle(&mut self) {
        let App { vdom, page, rt } = self;
        rt.block_on(async {
            for _ in 0..12 {
                vdom.render_immediate(page);
                if tokio::time::timeout(Duration::from_millis(3), vdom.wait_for_work()).await.is_err() {
                    vdom.render_immediate(page);
                    break;
                }
            }
        });
    }

    /// Lets time pass for timers (a toast, a delayed save).
    pub fn wait(&mut self, ms: u64) {
        let App { vdom, page, rt } = self;
        rt.block_on(async {
            let end = tokio::time::Instant::now() + Duration::from_millis(ms);
            while tokio::time::Instant::now() < end {
                let _ = tokio::time::timeout_at(end, vdom.wait_for_work()).await;
                vdom.render_immediate(page);
            }
        });
        self.settle();
    }

    pub fn html(&self) -> String {
        dioxus::ssr::render(&self.vdom)
    }

    /// The page's words, without markup.
    pub fn text(&self) -> String {
        self.page.text(0)
    }

    pub fn has(&self, words: &str) -> bool {
        self.text().contains(words) || self.html().contains(words)
    }

    #[track_caller]
    pub fn assert_has(&self, words: &str) {
        assert!(self.has(words), "the page should say {words:?}:\n{}", self.text());
    }

    #[track_caller]
    pub fn assert_lacks(&self, words: &str) {
        assert!(!self.text().contains(words), "the page should not say {words:?}:\n{}", self.text());
    }

    /// The nodes that listen for `event` and carry `label` in their words,
    /// aria-label, title or class: the innermost first.
    fn targets(&self, event: &str, label: &str) -> Vec<usize> {
        let mut found: Vec<(usize, usize)> = self
            .page
            .walk()
            .into_iter()
            .filter(|&n| self.page.nodes[n].listeners.contains(&event))
            .filter(|&n| {
                let text = self.page.text(n);
                text.contains(label)
                    || ["aria-label", "title", "class", "placeholder", "name", "value", "id", "data-id"]
                        .iter()
                        .any(|a| self.page.attr(n, a).is_some_and(|v| v.contains(label)))
            })
            .map(|n| (self.page.text(n).len(), n))
            .collect();
        found.sort_by_key(|(len, _)| *len);
        found.into_iter().map(|(_, n)| n).collect()
    }

    /// Whether something tappable carries `label`.
    pub fn can_tap(&self, label: &str) -> bool {
        !self.targets("click", label).is_empty()
    }

    fn element(&self, n: usize) -> ElementId {
        ElementId(self.page.nodes[n].id.expect("a listening node has an id"))
    }

    fn fire(&mut self, name: &str, data: EventData, n: usize) {
        let id = self.element(n);
        {
            let _rt = self.rt.enter();
            self.vdom.runtime().handle_event(name, Event::new(data.into_any(), true), id);
        }
        self.settle();
    }

    #[track_caller]
    fn target(&self, event: &str, label: &str, nth: usize) -> usize {
        let found = self.targets(event, label);
        match found.get(nth) {
            Some(&n) => n,
            None => panic!("nothing listens for {event} with {label:?} (#{nth}):\n{}", self.listing(event)),
        }
    }

    /// What can be tapped (or typed into), for a failing test's message.
    pub fn listing(&self, event: &str) -> String {
        self.page
            .walk()
            .into_iter()
            .filter(|&n| self.page.nodes[n].listeners.contains(&event))
            .map(|n| {
                let aria = self.page.attr(n, "aria-label").unwrap_or("");
                let class = self.page.attr(n, "class").unwrap_or("");
                format!(
                    "  <{}> [{aria}] .{class} {:?}",
                    self.page.nodes[n].tag.unwrap_or("#"),
                    self.page.text(n)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Taps the innermost button carrying `label`.
    #[track_caller]
    pub fn tap(&mut self, label: &str) -> &mut Self {
        self.tap_nth(label, 0)
    }

    #[track_caller]
    pub fn tap_nth(&mut self, label: &str, nth: usize) -> &mut Self {
        let n = self.target("click", label, nth);
        self.fire("click", EventData::Mouse(SerializedMouseData::default()), n);
        self
    }

    /// Taps it if it is there; says whether it was.
    pub fn tap_if(&mut self, label: &str) -> bool {
        let Some(&n) = self.targets("click", label).first() else { return false };
        self.fire("click", EventData::Mouse(SerializedMouseData::default()), n);
        true
    }

    /// Sends any mouse-like event (`pointerdown`, `dblclick`) to the node.
    #[track_caller]
    pub fn mouse(&mut self, event: &str, label: &str) -> &mut Self {
        let n = self.target(event, label, 0);
        self.fire(event, EventData::Mouse(SerializedMouseData::default()), n);
        self
    }

    /// Types `value` into the field carrying `label` (its oninput).
    #[track_caller]
    pub fn type_into(&mut self, label: &str, value: &str) -> &mut Self {
        self.form("input", label, value)
    }

    /// Sends a form event (`input`, `change`, `submit`) with `value`.
    #[track_caller]
    pub fn form(&mut self, event: &str, label: &str, value: &str) -> &mut Self {
        let n = self.target(event, label, 0);
        let data = SerializedFormData::new(value.to_string(), Vec::new());
        self.fire(event, EventData::Form(data), n);
        self
    }

    /// Presses a key on the node listening for keydown that carries `label`.
    #[track_caller]
    pub fn key(&mut self, label: &str, key: &str) -> &mut Self {
        let n = self.target("keydown", label, 0);
        let json = serde_json::json!({
            "element": 0, "name": "keydown", "bubbles": true,
            "data": {
                "char_code": 0, "key": key, "code": "Unidentified", "key_code": 0, "location": 0,
                "alt_key": false, "ctrl_key": false, "meta_key": false, "shift_key": false,
                "repeat": false, "is_composing": false,
            }
        });
        let ev: dioxus_html::HtmlEvent = serde_json::from_value(json).expect("a keydown event");
        self.fire("keydown", ev.data, n);
        self
    }

    /// Runs `f` inside the app, with its contexts in reach.
    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let out = {
            let _rt = self.rt.enter();
            self.vdom.in_scope(dioxus::dioxus_core::ScopeId::ROOT, f)
        };
        self.settle();
        out
    }
}

/// The app drawn on `start` after `setup`, as HTML.
pub fn render_with(start: Screen, setup: fn()) -> String {
    App::with(start, setup).html()
}

pub fn render(start: Screen) -> String {
    render_with(start, || {})
}

impl App {
    /// Labels that tell apart what can be tapped now, nav excluded.
    pub fn tappables(&self) -> Vec<String> {
        let mut out = Vec::new();
        for n in self.page.walk() {
            if !self.page.nodes[n].listeners.contains(&"click") {
                continue;
            }
            let class = self.page.attr(n, "class").unwrap_or("");
            if class.contains("nav-item")
                || class.contains("icon-button")
                    && self.page.attr(n, "aria-label") == Some(Screen::Settings.label())
            {
                continue;
            }
            let sig = self.signature(n);
            if !out.contains(&sig) {
                out.push(sig);
            }
        }
        out
    }

    fn signature(&self, n: usize) -> String {
        format!(
            "{}|{}|{}",
            self.page.attr(n, "aria-label").unwrap_or(""),
            self.page.attr(n, "class").unwrap_or(""),
            self.page.text(n)
        )
    }

    /// Taps what `signature` names, if it is still there.
    pub fn tap_signature(&mut self, sig: &str) -> bool {
        let Some(n) = self
            .page
            .walk()
            .into_iter()
            .find(|&n| self.page.nodes[n].listeners.contains(&"click") && self.signature(n) == sig)
        else {
            return false;
        };
        self.fire("click", EventData::Mouse(SerializedMouseData::default()), n);
        true
    }
}
