//! Weak-fact marks (research brief A4): an item whose last answer was a
//! miss (an MCQ answered wrong, a card rated Again) marks its book page.
//! A lesson step citing that page shows a quiet "راجعها تاني" under it,
//! and the page sheet lists the facts to go over again. The next right
//! answer clears the mark.
//!
//! Marks are computed from the review log, so they agree on every device
//! with nothing new to sync. Showing them is a synced setting
//! (`weak_marks`), on by default.
//! Inspired by AMBOSS's underline that stays until the concept is answered
//! correctly.

use std::collections::{BTreeSet, HashMap};

use dioxus::prelude::*;
use steps_core::content::Item;
use steps_core::review::{Grade, LoggedReview};

use crate::content::Library;
use crate::i18n::weak as tx;
use crate::look::{Css, css};

pub(crate) const CSS: Css = css!("src/weak/weak.css");
const KEY: &str = "weak_marks";

/// Items whose last logged answer was a miss. `log` comes oldest first.
pub fn weak_ids<'a>(log: impl IntoIterator<Item = &'a LoggedReview>) -> BTreeSet<String> {
    let mut last = HashMap::new();
    for l in log {
        last.insert(l.event.item_id.as_str(), l.event.grade == Grade::Again);
    }
    last.into_iter().filter(|&(_, missed)| missed).map(|(id, _)| id.to_string()).collect()
}

/// The weak items of `lecture` cited on `page`, in book order.
pub fn on_page<'a>(lib: &'a Library, weak: &BTreeSet<String>, lecture: &str, page: u32) -> Vec<&'a Item> {
    lib.lectures
        .get(lecture)
        .map(|l| l.items.iter().filter(|i| i.src.page == page && weak.contains(&i.id)).collect())
        .unwrap_or_default()
}

fn load() -> bool {
    crate::store::shared().setting(KEY).unwrap_or(true)
}

/// Whether the marks show.
pub static SHOW: GlobalSignal<bool> = Signal::global(load);

pub fn set_show(on: bool) {
    crate::store::save_setting(KEY, &on);
    *SHOW.write() = on;
}

/// After a sync or restore.
pub fn reload() {
    let fresh = load();
    if *SHOW.peek() != fresh {
        *SHOW.write() = fresh;
    }
}

/// The weak set, when the review state is in context and the marks show.
/// Subscribes the caller to answers.
pub fn use_weak() -> Option<BTreeSet<String>> {
    let review = try_use_context::<crate::store::Review>()?;
    let _ = (review.revision)();
    if !SHOW() {
        return None;
    }
    Some(review.backend.borrow().weak())
}

/// Under a lesson step: a quiet mark when a fact from its page was missed.
/// A tap opens the page's sheet, which lists those facts.
#[component]
pub fn StepMark(of: String, page: u32) -> Element {
    let Some(weak) = use_weak() else {
        return rsx! {};
    };
    let lecture = steps_core::program::lecture_of(&of).to_string();
    if on_page(Library::get(), &weak, &lecture, page).is_empty() {
        return rsx! {};
    }
    rsx! {
        style { {CSS.text()} }
        button {
            class: "weak-mark",
            title: tx::WHY,
            onclick: move |_| crate::pagesheet::open(page, &of, None, false, None),
            {tx::MARK}
        }
    }
}

/// Settings › Study: show or hide the marks.
#[component]
pub fn WeakSection() -> Element {
    let on = SHOW();
    rsx! {
        style { {CSS.text()} }
        crate::ui::Card {
            h2 { {tx::SETTING_TITLE} }
            label { class: "weak-toggle",
                input { r#type: "checkbox", checked: on, onchange: move |e| set_show(e.checked()) }
                span { {tx::SETTING_LINE} }
            }
        }
    }
}

#[cfg(test)]
mod tests;
