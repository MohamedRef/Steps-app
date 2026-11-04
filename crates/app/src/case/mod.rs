//! Cases: key-feature clinical cases (Step 1 style multi-step vignettes).
//!
//! A case shows its stem, then each step reveals new data and asks one
//! decision, with feedback (`why` and the book page) right after. The data
//! revealed so far stays on screen; from 840dp it sits beside the question.
//! A finished case is one review: its grade comes from the steps right.

mod player;
pub use player::CasePlayer;

use std::sync::Mutex;

use dioxus::prelude::*;
use steps_core::content::{Body, Item, Status};
use steps_core::review::{Grade, MemorySink, Origin, ReviewEvent, ReviewSink, StudyStoreHandle};

use crate::content::Library;
use crate::hover::BookPage;
use crate::i18n::case as tx;
use crate::look::{Css, css};
use crate::parts::{Collection, Layout, Sort, ViewBar, sort_by};

const CSS: Css = css!("src/case/case.css");
mod choices;
use choices::{option_key, option_order, option_state};

// The open case, kept across screens so a case left half-way opens where
// it was. `open_due` clears it so Today lands on the list, due first.
static OPEN: GlobalSignal<Option<String>> = Signal::global(|| None);

/// Open the Cases screen on its due items (#242): due items sit first.
pub fn open_due(mut ui: crate::Ui) {
    *OPEN.write() = None;
    ui.screen.set(crate::Screen::Cases);
}

#[component]
pub fn CasesScreen() -> Element {
    let back = move |_| *OPEN.write() = None;
    rsx! {
        style { {CSS.text()} }
        if let Some(id) = OPEN.read().clone() {
            CasePlayer { key: "{id}", item_id: id, on_back: back }
        } else {
            CaseList { on_open: move |id| *OPEN.write() = Some(id) }
        }
    }
}

/// The review store from `App`'s context. `None` outside the app (tests,
/// previews): answers then stay in memory.
pub fn use_case_store() -> Option<StudyStoreHandle> {
    use_hook(try_consume_context::<StudyStoreHandle>)
}

/// Items of one kind across every written lecture, in book order, without
/// the ones Mohamed marked ✗ and with his ✎ edits applied.
fn items_of(kind: &str, store: Option<&StudyStoreHandle>) -> Vec<(&'static str, Item)> {
    items_in_library(kind)
        .into_iter()
        .filter(|(_, i)| !store.is_some_and(|s| s.0.borrow().is_hidden(&i.id)))
        .map(|(l, i)| (l, effective(store, i)))
        .collect()
}

/// The item as Mohamed should see it: with his ✎ edit, if he made one.
pub fn effective(store: Option<&StudyStoreHandle>, item: &Item) -> Item {
    store.map_or_else(|| item.clone(), |s| s.0.borrow().effective(item))
}

fn items_in_library(kind: &str) -> Vec<(&'static str, &'static Item)> {
    let lib = Library::get();
    lib.index
        .lectures
        .iter()
        .filter_map(|e| lib.lectures.get(&e.id))
        .flat_map(|l| l.items.iter().map(move |i| (lib.title(&l.lecture), i)))
        .filter(|(_, i)| i.body.kind() == kind && i.status != Status::Rejected)
        .collect()
}

/// The lecture title and the item, by item id.
pub fn find_item(id: &str) -> Option<(&'static str, &'static Item)> {
    let lib = Library::get();
    let lecture = id.rsplit_once('.').map(|(l, _)| l)?;
    let lec = lib.lectures.get(lecture)?;
    lec.items.iter().find(|i| i.id == id).map(|i| (lib.title(&lec.lecture), i))
}

/// The cases list: due first by default, cards by default
/// (each shows a few lines of its stem), and optionally grouped by lecture.
const CASES_VIEW: Collection =
    Collection { key: "cases", layout: Layout::DEFAULT, sorts: &[Sort::Due, Sort::Book], groups: true };

#[component]
fn CaseList(on_open: EventHandler<String>) -> Element {
    const KIND: &str = "case";
    let store = use_case_store();
    let view = CASES_VIEW;
    let pick = view.pick();
    let mut items = items_of(KIND, store.as_ref());
    // Due ones first (#242) unless he picked book order; the sort is
    // stable, so each part keeps book order.
    let due: std::collections::HashSet<String> =
        store.as_ref().map(|s| s.0.borrow().due(now_ms()).into_iter().collect()).unwrap_or_default();
    sort_by(&mut items, pick.sort, |_| false, |(_, i)| due.contains(&i.id), |(_, i)| first_line(i));
    // Grouped: one list per lecture, lectures in book order (each keeps the
    // picked order inside).
    let mut groups: Vec<(&'static str, Vec<Item>)> = Vec::new();
    if pick.grouped {
        let order: Vec<&'static str> = items_in_library(KIND).into_iter().map(|(l, _)| l).collect();
        for (lecture, item) in items {
            match groups.iter_mut().find(|(l, _)| *l == lecture) {
                Some((_, v)) => v.push(item),
                None => groups.push((lecture, vec![item])),
            }
        }
        groups.sort_by_key(|(l, _)| order.iter().position(|o| o == l));
    } else {
        groups.push(("", items.into_iter().map(|(_, i)| i).collect()));
    }
    let empty = tx::EMPTY;
    let lectures: std::collections::HashMap<String, &'static str> =
        items_in_library(KIND).into_iter().map(|(l, i)| (i.id.clone(), l)).collect();
    rsx! {
        if groups.iter().all(|(_, v)| v.is_empty()) {
            crate::ui::Card { element: crate::ui::CardElement::Paragraph, class: "muted", {empty} }
        } else {
            ViewBar { of: view }
        }
        for (lecture , list) in groups {
            div { key: "{lecture}", class: "case-group",
                if !lecture.is_empty() {
                    h3 { class: "coll-group", bdi { "{lecture}" } }
                }
                ul { class: "coll case-list", "data-layout": pick.layout.as_str(),
                    for item in list {
                        li { key: "{item.id}",
                            button { class: "coll-btn case-row", onclick: { let id = item.id.clone(); move |_| on_open.call(id.clone()) },
                                span { class: "coll-main",
                                    bdi { class: "coll-title case-row-stem", dir: "auto", {first_line(&item)} }
                                    span { class: "coll-sub", dir: "rtl",
                                        match &item.body {
                                            Body::Case { steps, .. } => rsx! { {tx::steps(steps.len())} },
                                            _ => rsx! {},
                                        }
                                        if !pick.grouped {
                                            " · "
                                            bdi { {lectures.get(&item.id).copied().unwrap_or_default()} }
                                        }
                                    }
                                }
                                if due.contains(&item.id) {
                                    span { class: "coll-end", span { class: "chip due case-due", {tx::DUE} } }
                                }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn first_line(item: &Item) -> String {
    let text = match &item.body {
        Body::Case { stem, .. } => stem.as_str(),
        _ => "",
    };
    let mut out: String = text.chars().take(120).collect();
    if text.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// The line above every player: back, lecture, and the review status.
#[component]
pub fn ItemMeta(lecture: String, item_id: String, on_back: EventHandler) -> Element {
    let unverified = find_item(&item_id).is_some_and(|(_, i)| i.status == Status::Unverified);
    rsx! {
        div { class: "player-meta",
            button { class: "btn-link", "aria-keyshortcuts": "Escape", onclick: move |_| on_back.call(()),
                span { aria_hidden: "true", "→ " }
                {tx::BACK}
            }
            bdi { class: "muted small", "{lecture}" }
            if unverified { span { class: "chip", {tx::UNVERIFIED} } }
        }
    }
}

/// A case in progress. Global, so leaving the screen half-way and coming
/// back finds the same step, selection and answers.
#[derive(Clone, Debug, PartialEq)]
struct CaseRun {
    item_id: String,
    /// Runs past the last index once the case is finished.
    step: usize,
    /// The option selected on this step but not yet checked (written index).
    draft: Option<usize>,
    /// The checked option on this step.
    picked: Option<usize>,
    /// A step without options: its answer is showing.
    shown: bool,
    /// One entry per finished step: right or not, and the option picked
    /// (written index; `None` when he graded himself).
    results: Vec<(bool, Option<usize>)>,
    started: i64,
}

impl CaseRun {
    fn new(item_id: &str) -> Self {
        CaseRun {
            item_id: item_id.to_owned(),
            step: 0,
            draft: None,
            picked: None,
            shown: false,
            results: Vec::new(),
            started: now_ms(),
        }
    }

    /// Select an option; a checked step is locked.
    fn select(&mut self, option: usize) {
        if self.picked.is_none() && self.results.len() == self.step {
            self.draft = Some(option);
        }
    }

    /// Check the selection against the step's answer, once.
    fn check(&mut self, answer: Option<usize>) -> bool {
        if self.picked.is_some() || self.results.len() != self.step {
            return false;
        }
        let Some(d) = self.draft.take() else { return false };
        self.picked = Some(d);
        self.results.push((answer == Some(d), Some(d)));
        true
    }

    /// A step without options, graded by himself.
    fn grade_self(&mut self, right: bool) {
        if self.results.len() == self.step {
            self.results.push((right, None));
        }
    }

    fn next(&mut self) {
        if self.results.len() > self.step {
            self.step += 1;
            self.draft = None;
            self.picked = None;
            self.shown = false;
        }
    }

    /// Persist a completed case before leaving its final answer. Failure
    /// retains the attempt so the learner can explicitly try Finish again.
    fn next_logged(&mut self, total: usize, store: Option<&StudyStoreHandle>) -> Result<(), String> {
        if self.step >= total || self.results.len() <= self.step {
            return Ok(());
        }
        if self.step + 1 == total && self.results.len() == total {
            let right = self.results.iter().filter(|answer| answer.0).count() as u32;
            record(store, &self.item_id, right, total as u32, self.started)?;
        }
        self.next();
        Ok(())
    }
}

static RUN: GlobalSignal<Option<CaseRun>> = Signal::global(|| None);

/// One review per case: every step right is Good, at least half is Hard,
/// fewer is Again. Easy is left to the self-rating screens.
pub fn grade_for(right: u32, total: u32) -> Grade {
    if total == 0 || right * 2 < total {
        Grade::Again
    } else if right < total {
        Grade::Hard
    } else {
        Grade::Good
    }
}

pub fn now_ms() -> i64 {
    use web_time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Answers given outside the app (no store in context) wait here.
static EVENTS: Mutex<MemorySink> = Mutex::new(MemorySink { events: Vec::new() });

/// One finished case as one review event in the store.
pub fn record(
    store: Option<&StudyStoreHandle>,
    item_id: &str,
    right: u32,
    total: u32,
    started_ms: i64,
) -> Result<(), String> {
    let at_ms = now_ms();
    let event = ReviewEvent {
        item_id: item_id.to_owned(),
        grade: grade_for(right, total),
        origin: Origin::Case,
        at_ms,
        took_ms: u32::try_from(at_ms.saturating_sub(started_ms)).unwrap_or(u32::MAX),
        confidence: None,
        reason: None,
    };
    match store {
        Some(s) => s.0.borrow_mut().record_logged(event).map(|_| ()),
        None => {
            let mut sink = EVENTS.lock().map_err(|e| e.to_string())?;
            sink.record(event);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
