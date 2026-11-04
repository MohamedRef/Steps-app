//! Helpers for the study, bank, oral, case and today screen tests: items of
//! a kind from the bundled First Aid content, what the review store saved,
//! form events on the nth field, and one part drawn inside the whole app.

use std::cell::RefCell;

use dioxus::prelude::*;
use dioxus_html::{EventData, SerializedFormData};
use steps_core::content::Item;
use steps_core::review::LoggedReview;

use super::{App, RootProps, dom::Dom};
use crate::{AppAt, Screen};

/// Ids of written items of `kind`, in id order (`cvs.L01.001` first).
pub fn ids_of(kind: &str) -> Vec<String> {
    crate::content::Library::get()
        .lectures
        .values()
        .flat_map(|l| l.items.iter())
        .filter(|i| i.body.kind() == kind)
        .map(|i| i.id.clone())
        .collect()
}

/// The first written item of `kind` (the content always has cards, clozes,
/// enumerations, tables, orders and MCQs once `cvs.L01` is in).
#[track_caller]
pub fn first_of(kind: &str) -> Item {
    let id = ids_of(kind).into_iter().next().unwrap_or_else(|| panic!("no {kind} in the content"));
    item(&id)
}

#[track_caller]
pub fn item(id: &str) -> Item {
    crate::content::Library::get()
        .lectures
        .values()
        .flat_map(|l| l.items.iter())
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("no item {id}"))
        .clone()
}

/// Every answer the review store kept, in order (tombstoned ones left out).
pub fn reviews() -> Vec<LoggedReview> {
    crate::store::shared().reviews().expect("the test store reads")
}

impl App {
    /// A form event on the `nth` field (innermost first) carrying `label`.
    #[track_caller]
    pub fn form_nth(&mut self, event: &str, label: &str, nth: usize, value: &str) -> &mut Self {
        let n = self.target(event, label, nth);
        let data = SerializedFormData::new(value.to_string(), Vec::new());
        self.fire(event, EventData::Form(data), n);
        self
    }

    /// How many things carrying `label` listen for `event`.
    pub fn count(&self, event: &str, label: &str) -> usize {
        self.targets(event, label).len()
    }

    /// The app on `start`, with `part` drawn inside its frame (every context
    /// in reach), for a component no bundled content leads to yet.
    pub fn with_part(start: Screen, part: fn() -> Element) -> App {
        PART.with(|p| *p.borrow_mut() = Some(part));
        App::start_root(start, part_root)
    }

    fn start_root(start: Screen, root: fn(RootProps) -> Element) -> App {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let dir = crate::prefs::test_dir();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::onboarding::mark_done_for_tests();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut vdom = VirtualDom::new_with_props(root, RootProps { start, setup: || {} });
        let mut page = Dom::default();
        {
            let _rt = rt.enter();
            vdom.rebuild(&mut page);
        }
        let mut app = App { vdom, page, rt };
        app.settle();
        app
    }
}

thread_local! {
    static PART: RefCell<Option<fn() -> Element>> = const { RefCell::new(None) };
}

fn part_root(p: RootProps) -> Element {
    rsx! { AppAt { start: p.start, Part {} } }
}

#[allow(non_snake_case)]
fn Part() -> Element {
    let draw = PART.with(|p| p.borrow().expect("a part to draw"));
    draw()
}

impl App {
    /// Presses a key on the field listening for keydown that carries
    /// `label` (the event as the webview sends it, `which` included).
    #[track_caller]
    pub fn press(&mut self, label: &str, key: &str) -> &mut Self {
        let n = self.target("keydown", label, 0);
        let json = serde_json::json!({
            "element": 0, "name": "keydown", "bubbles": true,
            "data": {
                "char_code": 0, "key": key, "code": "Unidentified", "key_code": 0, "location": 0,
                "alt_key": false, "ctrl_key": false, "meta_key": false, "shift_key": false,
                "repeat": false, "is_composing": false, "which": 0,
            }
        });
        let ev: dioxus_html::HtmlEvent = serde_json::from_value(json).expect("a keydown event");
        self.fire("keydown", ev.data, n);
        self
    }
}

impl App {
    /// Sends a drag event (`dragstart`, `dragover`, `drop`, `dragend`) to the
    /// `nth` node carrying `label`.
    #[track_caller]
    pub fn drag(&mut self, event: &str, label: &str, nth: usize) -> &mut Self {
        let n = self.target(event, label, nth);
        let json = serde_json::json!({
            "mouse": dioxus_html::point_interaction::SerializedPointInteraction::default(),
            "data_transfer": { "items": [], "files": [], "effect_allowed": "", "drop_effect": "" },
        });
        let data: dioxus_html::SerializedDragData = serde_json::from_value(json).expect("a drag event");
        self.fire(event, EventData::Drag(data), n);
        self
    }

    /// The end of a CSS animation on the node carrying `label`.
    #[track_caller]
    pub fn animation_end(&mut self, label: &str) -> &mut Self {
        let n = self.target("animationend", label, 0);
        let json =
            serde_json::json!({ "animation_name": "drain", "pseudo_element": "", "elapsed_time": 1.0 });
        let data: dioxus_html::SerializedAnimationData =
            serde_json::from_value(json).expect("an animation event");
        self.fire("animationend", EventData::Animation(data), n);
        self
    }
}

impl App {
    /// Tells every node that asked to know when it mounted (focus, scroll
    /// into view) that it did, as the webview does. Returns how many there were.
    pub fn mount_events(&mut self) -> usize {
        let nodes: Vec<usize> = self
            .page
            .walk()
            .into_iter()
            .filter(|&n| self.page.nodes[n].listeners.contains(&"mounted"))
            .collect();
        for &n in &nodes {
            if self.page.attached(n) {
                self.fire("mounted", EventData::Mounted, n);
            }
        }
        nodes.len()
    }
}

/// A First Aid course with no lecture written yet, if one is left. Tests
/// that need an empty course skip once every course has content.
pub fn empty_course() -> Option<String> {
    let lib = crate::content::Library::get();
    lib.program
        .courses
        .iter()
        .find(|c| !lib.lectures.keys().any(|id| id.starts_with(&format!("{}.", c.id))))
        .map(|c| c.id.clone())
}
