//! The "اختصارات" chip beside a lesson step's "ص N" chip, and the sheet it
//! opens: every abbreviation the step shows (its text, options, feedback,
//! table cells and the words in its figure), each with its expansion from
//! `content/glossary.yaml` (Mohamed, 2026-11-01). The chip only shows when
//! the step has one; `steps-tools validate` fails on a lesson abbreviation
//! the glossary doesn't explain, so none is left out.

use dioxus::prelude::*;
use steps_core::content::Status;
use steps_core::glossary::Entry;

use crate::hover::glossary_for;
use crate::i18n::abbr as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/abbrsheet/abbrsheet.css");

/// The open sheet: the lesson's course, and its terms in the order the
/// step shows them.
pub(crate) static OPEN: GlobalSignal<Option<(String, Vec<String>)>> = Signal::global(|| None);

/// The abbreviations in a step's texts, with the meanings `course`'s lessons
/// use ([`crate::hover::glossary_for`]).
pub fn entries(course: &str, texts: &[String]) -> Vec<&'static Entry> {
    glossary_for(course).abbreviations_in(texts)
}

pub fn open(course: String, terms: Vec<String>) {
    document::eval("window.__stepsHover && window.__stepsHover.hide();");
    crate::pagesheet::close();
    crate::labs::close();
    *OPEN.write() = Some((course, terms));
}

pub fn close() {
    *OPEN.write() = None;
}

/// The chip; nothing when the step has no abbreviation.
#[component]
pub fn AbbrChip(course: String, texts: Vec<String>) -> Element {
    let terms: Vec<String> = entries(&course, &texts).iter().map(|e| e.term.clone()).collect();
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
                open(course.clone(), terms.clone());
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
    let Some((course, terms)) = OPEN.read().clone() else {
        return rsx! {};
    };
    let g = glossary_for(&course);
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
        let found = entries(
            "cvs",
            &texts(&[
                "الـ PFO بيفضل مفتوح لما الـ foramen ovale ما يقفلش.",
                "قيس الـ MAP، وفرّق بين الـ ASD والـ PFO تاني.",
            ]),
        );
        let terms: Vec<&str> = found.iter().map(|e| e.term.as_str()).collect();
        assert_eq!(terms, ["PFO", "MAP", "ASD"]);
    }

    #[test]
    fn plain_text_has_no_chip() {
        assert!(entries("cvs", &texts(&["Tamponade is fluid around the heart."])).is_empty());
    }
}
