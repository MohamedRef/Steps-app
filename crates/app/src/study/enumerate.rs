//! "Mention / Enumerate": recall as many points as you can (typed, or said
//! to yourself), then tick the ones you got. Typed recall pre-ticks matches.

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::review::{Grade, enumerate_grade};
use steps_core::session::recalled;

use super::grade::GradeRow;
use crate::i18n::study as tx;

/// The study browser fixture opens the points, so the تحشيشة box is checked too.
#[cfg(test)]
pub(crate) static OPEN_FOR_FIXTURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn opened_for_fixture(_total: usize) -> Option<Vec<bool>> {
    #[cfg(test)]
    if OPEN_FOR_FIXTURE.load(std::sync::atomic::Ordering::Relaxed) {
        return Some(vec![false; _total]);
    }
    None
}

#[component]
pub fn EnumerateItem(item: Item, on_grade: EventHandler<Grade>) -> Element {
    let Body::Enumerate { prompt, points, pass, hook } = &item.body else { return rsx! {} };
    let (total, pass) = (points.len(), *pass);
    let mut typed = use_signal(String::new);
    let mut ticks = use_signal(|| opened_for_fixture(total));
    let pts = points.clone();
    let open = move |_| ticks.set(Some(recalled(&pts, &typed.read())));

    let got = ticks.read().as_ref().map(|t| t.iter().filter(|x| **x).count());
    let wrote = typed.read().trim().to_string();
    // Tablet and Mac: the prompt and the recall on one side, the points on the
    // other. Once open, the points take the wide column; grading sits under both.
    rsx! {
        crate::desk::Panes {
            wide_side: got.is_some(),
            primary: rsx! {
                p { class: "st-prompt", lang: "en", dir: "ltr", "{prompt}" }
                p { class: "muted small", {tx::enum_goal(total, pass)} }
                if got.is_none() {
                    textarea {
                        class: "st-recall",
                        lang: "en",
                        dir: "ltr",
                        rows: "5",
                        placeholder: tx::ENUM_PLACEHOLDER,
                        value: "{typed}",
                        oninput: move |e| typed.set(e.value()),
                    }
                    div { class: "st-actions",
                        button { class: "btn btn-primary btn-lg", "aria-keyshortcuts": "Enter", onclick: open, {tx::SHOW_POINTS} }
                    }
                } else if !wrote.is_empty() {
                    section { class: "st-mine-col",
                        h3 { {tx::WHAT_I_WROTE} }
                        p { lang: "en", dir: "ltr", class: "st-pre", "{wrote}" }
                    }
                }
            },
            secondary: got.map(|n| rsx! {
                p { class: "muted small",
                    {tx::TICK_WHAT_YOU_GOT}
                    if !wrote.is_empty() {
                        " {tx::PRE_TICKED}"
                    }
                }
                ul { class: "st-points", lang: "en", dir: "ltr",
                    for (i , point) in points.iter().enumerate() {
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
                                span { "{point}" }
                            }
                        }
                    }
                }
                p {
                    class: "st-verdict",
                    "data-match": if n >= pass { "right" } else { "wrong" },
                    "{n} {tx::OF} {total} · "
                    if n >= pass { {tx::ENUM_PASS} } else { "{tx::ENUM_SHORT_A} {pass - n} {tx::ENUM_SHORT_B}" }
                }
                if let Some(h) = hook {
                    div { class: "st-hook",
                        p { class: "st-hook-label", span { aria_hidden: "true", "😄 " } {tx::ENUM_HOOK} }
                        p { "{h}" }
                    }
                }
            }),
            footer: got.map(|n| rsx! {
                GradeRow { item_id: item.id.clone(), suggested: Some(enumerate_grade(n, total, pass)), on_grade }
            }),
        }
    }
}
