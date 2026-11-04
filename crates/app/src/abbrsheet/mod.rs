//! The "اختصارات" chip beside a lesson step's "ص N" chip, and the sheet it
//! opens: every abbreviation the step shows (its text, options, feedback,
//! table cells and the words in its figure), each with its expansion from
//! `content/glossary.yaml` (Mohamed, 2026-11-01). The chip only shows when
//! the step has one; `steps-tools validate` fails on a lesson abbreviation
//! the glossary doesn't explain, so none is left out.

use dioxus::prelude::*;
use steps_core::content::Status;
use steps_core::glossary::Entry;

use crate::hover::glossary;
use crate::i18n::abbr as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/abbrsheet/abbrsheet.css");

/// The terms of the open sheet, in the order the step shows them.
pub(crate) static OPEN: GlobalSignal<Option<Vec<String>>> = Signal::global(|| None);

/// The abbreviations in a step's texts, from the bundled glossary.
pub fn entries(texts: &[String]) -> Vec<&'static Entry> {
    glossary().abbreviations_in(texts)
}

pub fn open(terms: Vec<String>) {
    document::eval("window.__stepsHover && window.__stepsHover.hide();");
    crate::pagesheet::close();
    crate::labs::close();
    *OPEN.write() = Some(terms);
}

pub fn close() {
    *OPEN.write() = None;
}

/// The chip; nothing when the step has no abbreviation.
#[component]
pub fn AbbrChip(texts: Vec<String>) -> Element {
    let terms: Vec<String> = entries(&texts).iter().map(|e| e.term.clone()).collect();
    if terms.is_empty() {
        return rsx! {};
    }
    let n = terms.len();
    rsx! {
        button {
            class: "chip abbr tap-chip",
            aria_label: "{tx::CHIP_HINT} ({n})",
            aria_haspopup: "dialog",
            onclick: move |e| {
                e.stop_propagation();
                open(terms.clone());
            },
            {tx::CHIP}
            span { class: "abbr-n", "{n}" }
        }
    }
}

/// The sheet, drawn once by the shell; empty while none is open.
#[component]
pub fn AbbrSheet() -> Element {
    let ui = use_context::<crate::Ui>();
    let Some(terms) = OPEN.read().clone() else {
        return rsx! {};
    };
    let g = glossary();
    let shown: Vec<&Entry> = terms.iter().filter_map(|t| g.entries.iter().find(|e| &e.term == t)).collect();
    let unverified = shown.iter().any(|e| e.status != Status::Approved);
    rsx! {
        style { {CSS.text()} }
        crate::sheet::Sheet {
            label_id: "ab-title",
            on_close: move |_| close(),
            head: rsx! {
                h2 { id: "ab-title", class: "ab-title", {tx::TITLE} }
            },
            dl { class: "ab-list",
                for e in shown {
                    // English rows read left to right inside the Arabic sheet.
                    div { class: "ab-row", key: "{e.term}", lang: "en", dir: "ltr",
                        dt { bdi { "{e.term}" } }
                        dd { "{e.text}" }
                    }
                }
            }
            if unverified {
                p { class: "ab-note muted small", {tx::UNVERIFIED} }
            }
            button {
                class: "btn btn-tonal ab-dict",
                onclick: move |_| {
                    let course = crate::learn::OPEN.peek().as_deref().map(|id| steps_core::program::course_of(id).to_string());
                    crate::abbrdict::open(ui, course);
                },
                {tx::DICT_OPEN}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_step_lists_its_abbreviations_once_in_reading_order() {
        // SPEC §24-style step: Arabic prose around English terms.
        let found = entries(&texts(&[
            "الـ HRS-AKI بيحصل في الـ cirrhosis مع ascites.",
            "قيس الـ SBP والـ MAP، وراجع الـ HRS-AKI تاني.",
        ]));
        let terms: Vec<&str> = found.iter().map(|e| e.term.as_str()).collect();
        assert_eq!(terms, ["HRS-AKI", "SBP", "MAP"]);
    }

    #[test]
    fn plain_text_has_no_chip() {
        assert!(entries(&texts(&["Ascites is fluid in the peritoneal cavity."])).is_empty());
    }
}
