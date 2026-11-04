//! A recall card: read the question, answer in your head, reveal, grade.

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::review::Grade;

use super::grade::GradeRow;
use crate::i18n::study as tx;

#[component]
pub fn CardItem(item: Item, on_grade: EventHandler<Grade>) -> Element {
    let Body::Card { front, back } = &item.body else { return rsx! {} };
    let reopened = try_use_context::<super::item::Reopened>().is_some_and(|r| r.0);
    let mut shown = use_signal(|| reopened);
    // Tablet and Mac: the question on one side, the answer in the wide column;
    // grading sits under both.
    rsx! {
        crate::desk::Panes {
            wide_side: shown(),
            primary: rsx! {
                p { class: "st-prompt", lang: "en", dir: "ltr", "{front}" }
                if !shown() {
                    p { class: "muted small", {tx::THINK_FIRST} }
                    button {
                        class: "btn btn-primary btn-lg",
                        autofocus: true,
                        "aria-keyshortcuts": "Space",
                        onclick: move |_| shown.set(true),
                        {tx::REVEAL}
                    }
                }
            },
            secondary: shown().then(|| rsx! {
                div { class: "st-answer", lang: "en", dir: "ltr", "{back}" }
            }),
            footer: shown().then(|| rsx! {
                GradeRow { item_id: item.id.clone(), suggested: None, on_grade }
            }),
        }
    }
}
