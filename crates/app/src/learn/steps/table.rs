//! Lesson table interaction and response restoration.

use super::*;

#[component]
pub(super) fn Table(
    title: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    hide: Vec<[usize; 2]>,
) -> Element {
    let (saved, put) = use_draft();
    let mut open = use_signal(|| match saved {
        Draft::Table { open } => open,
        _ => Vec::new(),
    });
    let mut show = move |v: Vec<[usize; 2]>| {
        put.call(Draft::Table { open: v.clone() });
        open.set(v);
    };
    let hidden_left = hide.iter().filter(|c| !open.read().contains(c)).count();
    rsx! {
        div { class: "ls-table",
            h2 { class: "ls-h", Line { text: title } }
            if hidden_left > 0 { p { class: "muted small", {tx::TABLE_HINT} } }
            div { class: "ls-table-scroll", dir: "ltr", lang: "en",
                table {
                    thead {
                        tr {
                            for (c , h) in columns.iter().enumerate() {
                                th { key: "{c}", "data-short": if is_value(h) { "yes" } else { "no" }, "{h}" }
                            }
                        }
                    }
                    tbody {
                        for (r , row) in rows.iter().enumerate() {
                            tr { key: "{r}",
                                for (c , cell) in row.iter().enumerate() {
                                    if hide.contains(&[r, c]) && !open.read().contains(&[r, c]) {
                                        td { key: "{c}",
                                            button { class: "ls-cell-hidden", onclick: move |_| {
                                                let mut v = open.peek().clone();
                                                v.push([r, c]);
                                                show(v);
                                            }, aria_label: tx::REVEAL, "؟" }
                                        }
                                    } else if c == 0 {
                                        th { key: "{c}", scope: "row", "{cell}" }
                                    } else {
                                        td {
                                            key: "{c}",
                                            "data-opened": if hide.contains(&[r, c]) { "yes" } else { "no" },
                                            "data-short": if is_value(cell) { "yes" } else { "no" },
                                            "{cell}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if hidden_left > 0 {
                button { class: "btn-link", onclick: move |_| show(hide.clone()), {tx::SHOW_ALL} }
            }
        }
    }
}
