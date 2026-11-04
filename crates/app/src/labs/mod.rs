//! The lab values sheet: First Aid's "Selected USMLE Laboratory Values"
//! (`content/labs.yaml`, printed pages xviii–xix), open from any question
//! the way Step 1 lets him open its lab values beside every item. A search
//! narrows the rows; SI units are a tap away. Drawn once by the shell; the
//! [`LabButton`] beside a question opens it.

use std::sync::OnceLock;

use dioxus::prelude::*;
use serde::Deserialize;

use crate::i18n::labs as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/labs/labs.css");
const LABS_YAML: &str = include_str!("../../../../content/labs.yaml");

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Labs {
    pub schema: String,
    pub src: Src,
    pub status: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Src {
    pub book: String,
    pub pages: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub test: String,
    /// One line, or one per sex ("Male: …", "Female: …"). Empty for a
    /// heading row whose values sit in the `sub` rows under it.
    #[serde(default)]
    pub range: Vec<String>,
    #[serde(default)]
    pub si: Vec<String>,
    /// The book's asterisk: part of the Biochemical Profile (SMA-12).
    #[serde(default)]
    pub sma12: bool,
    /// Indented under the row above it.
    #[serde(default)]
    pub sub: bool,
}

pub fn labs() -> &'static Labs {
    static L: OnceLock<Labs> = OnceLock::new();
    L.get_or_init(|| {
        serde_yaml_ng::from_str(LABS_YAML).unwrap_or(Labs {
            schema: String::new(),
            src: Src { book: String::new(), pages: String::new() },
            status: String::new(),
            sections: Vec::new(),
        })
    })
}

/// Rows of a section that match `query` (case-insensitive, in the test's
/// name or the section's title), with each matching sub row's heading kept
/// above it so "Total" still reads as bilirubin's.
pub fn matching<'a>(section: &'a Section, query: &str) -> Vec<&'a Row> {
    let q = query.trim().to_lowercase();
    if q.is_empty() || section.title.to_lowercase().contains(&q) {
        return section.rows.iter().collect();
    }
    let mut out: Vec<&Row> = Vec::new();
    let mut heading: Option<&Row> = None;
    let mut heading_hit = false;
    for row in &section.rows {
        if !row.sub {
            heading = Some(row);
            heading_hit = row.test.to_lowercase().contains(&q);
            if heading_hit {
                out.push(row);
            }
        } else if heading_hit || row.test.to_lowercase().contains(&q) {
            if let Some(h) = heading.filter(|h| !out.iter().any(|r| std::ptr::eq(*r, *h))) {
                out.push(h);
            }
            out.push(row);
        }
    }
    out
}

static OPEN: GlobalSignal<bool> = Signal::global(|| std::env::var_os("STEPS_LABS").is_some());
static QUERY: GlobalSignal<String> = Signal::global(String::new);
/// Show SI units instead of the book's conventional ones (this device).
static SI: GlobalSignal<bool> = Signal::global(|| false);

pub fn open() {
    document::eval("window.__stepsHover && window.__stepsHover.hide();");
    crate::pagesheet::close();
    crate::abbrsheet::close();
    crate::gopage::close();
    *OPEN.write() = true;
}

pub fn close() {
    if *OPEN.peek() {
        *OPEN.write() = false;
    }
}

/// "Lab values" beside a question, as on the exam screen.
#[component]
pub fn LabButton(#[props(default)] class: String) -> Element {
    rsx! {
        button {
            class: "btn btn-ghost lab-open {class}",
            aria_haspopup: "dialog",
            title: tx::OPEN_HINT,
            onclick: move |e| {
                e.stop_propagation();
                open();
            },
            span { class: "lab-open-icon", aria_hidden: "true", "⚗" }
            {tx::OPEN}
        }
    }
}

/// The sheet, drawn once by the shell; empty while closed.
#[component]
pub fn LabSheet() -> Element {
    if !OPEN() {
        return rsx! {};
    }
    let l = labs();
    let query = QUERY();
    let si = SI();
    let shown: Vec<(&Section, Vec<&Row>)> =
        l.sections.iter().map(|s| (s, matching(s, &query))).filter(|(_, rows)| !rows.is_empty()).collect();
    rsx! {
        style { {CSS.text()} }
        crate::sheet::Sheet {
            label_id: "lab-title",
            on_close: move |_| close(),
            head: rsx! {
                h2 { id: "lab-title", class: "lab-title", {tx::TITLE} }
            },
            div { class: "lab-tools",
                input {
                    class: "lab-search",
                    r#type: "search",
                    lang: "en",
                    dir: "auto",
                    placeholder: tx::SEARCH,
                    aria_label: tx::SEARCH,
                    value: "{query}",
                    oninput: move |e| *QUERY.write() = e.value(),
                }
                div { class: "segmented lab-units", role: "radiogroup", aria_label: tx::UNITS,
                    button { role: "radio", "aria-checked": !si, onclick: move |_| *SI.write() = false, {tx::CONVENTIONAL} }
                    button { role: "radio", "aria-checked": si, onclick: move |_| *SI.write() = true, "SI" }
                }
            }
            if shown.is_empty() {
                p { class: "muted small lab-empty", {tx::NO_MATCH} }
            }
            for (section, rows) in shown {
                section { key: "{section.title}", class: "lab-section", lang: "en", dir: "ltr",
                    h3 { class: "lab-section-title", "{section.title}" }
                    table { class: "lab-table",
                        tbody {
                            for (i, row) in rows.into_iter().enumerate() {
                                tr { key: "{i}", "data-sub": row.sub,
                                    th { scope: "row",
                                        "{row.test}"
                                        if row.sma12 {
                                            span { class: "lab-star", title: tx::SMA12, "*" }
                                        }
                                    }
                                    td {
                                        for (j, v) in (if si { &row.si } else { &row.range }).iter().enumerate() {
                                            span { key: "{j}", class: "lab-val", "{v}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "muted small lab-foot",
                {tx::SMA12_NOTE}
                " · "
                bdi { "First Aid 2026, p. {l.src.pages}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheet_holds_every_section_of_the_book() {
        let l = labs();
        assert_eq!(l.schema, "steps.labs/v1");
        assert_eq!(l.status, "unverified");
        assert_eq!(l.src.book, "fa2026");
        let titles: Vec<&str> = l.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["Blood, Plasma, Serum", "Cerebrospinal Fluid", "Hematologic", "Urine", "Other"]);
        for s in &l.sections {
            assert!(!s.rows.first().unwrap().sub, "{} starts with a sub row", s.title);
            for r in &s.rows {
                // A row with values has both units, line for line.
                assert_eq!(r.range.len(), r.si.len(), "{}", r.test);
            }
        }
        let na = l.sections[0].rows.iter().find(|r| r.test.starts_with("Sodium")).unwrap();
        assert_eq!(na.range, ["136–146 mEq/L"]);
        let sma: usize = l.sections.iter().flat_map(|s| &s.rows).filter(|r| r.sma12).count();
        assert_eq!(sma, 12, "the SMA-12 is twelve tests");
    }

    #[test]
    fn a_search_keeps_a_sub_rows_heading() {
        let blood = &labs().sections[0];
        let rows: Vec<&str> = matching(blood, "direct").iter().map(|r| r.test.as_str()).collect();
        assert_eq!(rows, ["Bilirubin, serum (adult)", "Direct"]);
        let rows: Vec<&str> = matching(blood, "bilirubin").iter().map(|r| r.test.as_str()).collect();
        assert_eq!(rows, ["Bilirubin, serum (adult)", "Total", "Direct"]);
        assert_eq!(matching(blood, "  ").len(), blood.rows.len());
        assert!(matching(blood, "zzz").is_empty());
        let csf = &labs().sections[1];
        assert_eq!(matching(csf, "cerebrospinal").len(), csf.rows.len());
    }

    #[test]
    fn the_open_sheet_renders_its_rows_and_source() {
        fn app() -> Element {
            *OPEN.write() = true;
            rsx! { LabSheet {} }
        }
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for needle in [tx::TITLE, "Cerebrospinal Fluid", "136–146 mEq/L", "xviii–xix"] {
            assert!(html.contains(needle), "missing {needle}");
        }
    }
}
