//! A short-essay question: write the headings you would use, then compare
//! them with the model outline and rate yourself.

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::review::{Grade, enumerate_grade};
use steps_core::session::{outline_sections, recalled};

use super::grade::GradeRow;
use crate::i18n::study as tx;

#[component]
pub fn EssayItem(item: Item, on_grade: EventHandler<Grade>) -> Element {
    let Body::Essay { prompt, outline } = &item.body else { return rsx! {} };
    let sections = outline_sections(outline);
    let headings: Vec<String> = sections.iter().map(|(h, _)| h.clone()).collect();
    let total = headings.len();
    let mut typed = use_signal(String::new);
    let mut ticks = use_signal(|| None::<Vec<bool>>);
    let compare = move |_| ticks.set(Some(recalled(&headings, &typed.read())));

    let got = ticks.read().as_ref().map(|t| t.iter().filter(|x| **x).count());
    rsx! {
        p { class: "st-prompt", lang: "en", dir: "ltr", "{prompt}" }
        match got {
            None => rsx! {
                p { class: "muted small", {tx::ESSAY_HOW} }
                textarea {
                    class: "st-recall",
                    lang: "en",
                    dir: "ltr",
                    rows: "6",
                    placeholder: tx::ESSAY_PLACEHOLDER,
                    value: "{typed}",
                    oninput: move |e| typed.set(e.value()),
                }
                div { class: "st-actions",
                    button { class: "btn btn-primary btn-lg", "aria-keyshortcuts": "Enter", onclick: compare, {tx::COMPARE} }
                }
            },
            Some(n) => rsx! {
                div { class: "st-compare",
                    if !typed.read().trim().is_empty() {
                        section { class: "st-mine-col",
                            h3 { {tx::WHAT_I_WROTE} }
                            p { lang: "en", dir: "ltr", class: "st-pre", "{typed}" }
                        }
                    }
                    section { class: "st-outline-col",
                        h3 { {tx::MODEL_OUTLINE} }
                        p { class: "muted small", {tx::TICK_HEADINGS} }
                        ol { class: "st-outline", lang: "en", dir: "ltr",
                            for (i , (heading , bullets)) in sections.iter().enumerate() {
                                li { key: "{i}",
                                    label { class: "st-point",
                                        input {
                                            r#type: "checkbox",
                                            checked: ticks.read().as_ref().is_some_and(|t| t[i]),
                                            onchange: move |e| {
                                                if let Some(t) = ticks.write().as_mut() {
                                                    t[i] = e.checked();
                                                }
                                            },
                                        }
                                        strong { "{heading}" }
                                    }
                                    if !bullets.is_empty() {
                                        ul {
                                            for (j , b) in bullets.iter().enumerate() {
                                                li { key: "{j}", "{b}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "st-verdict", "data-match": "close", "{tx::HEADINGS_GOT} {n} {tx::OF} {total}" }
                GradeRow {
                    item_id: item.id.clone(),
                    suggested: Some(enumerate_grade(n, total, total.div_ceil(2) + total / 4)),
                    on_grade,
                }
            },
        }
    }
}
