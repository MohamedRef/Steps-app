//! Settings › Your data › Restore: pick a `steps-export` file, see what it
//! holds and what it would change, then merge it (`Store::apply_import`).
//! Nothing on this device is deleted; newer rows win, as in sync.

use std::path::PathBuf;

use dioxus::prelude::*;
use serde_json::Value;
use steps_core::review::verify::rfc3339;

use crate::i18n::data as tx;
use crate::platform::documents::{self, DocState};
use crate::store::import::{ImportPlan, parse};
use crate::store::{self, use_review};

/// Export files found where Steps writes them, newest name first.
fn found_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = store::export_dir()
        .ok()
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("steps-export-") && n.ends_with(".json"))
        })
        .collect();
    out.sort();
    out.reverse();
    out
}

/// A real export is a few MB; anything this big is the wrong file.
const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

/// Reads, parses and plans a backup file off the UI thread.
async fn read_plan(path: PathBuf) -> Result<(Value, ImportPlan), String> {
    crate::platform::time::blocking(move || {
        let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if len > MAX_IMPORT_BYTES {
            return Err(tx::IMPORT_TOO_BIG.to_string());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let doc = parse(&text).map_err(|e| e.to_string())?;
        let plan = store::shared().plan_import(&doc).map_err(|e| e.to_string())?;
        Ok((doc, plan))
    })
    .await?
}

#[derive(Clone, PartialEq)]
enum Step {
    Pick,
    Preview { name: String, doc: Value, plan: ImportPlan },
    Done(ImportPlan),
    Failed(String),
}

#[component]
pub fn ImportCard() -> Element {
    let review = use_review();
    let mut step = use_signal(|| Step::Pick);
    let mut pasted = use_signal(String::new);
    let mut picking = use_signal(|| false);

    let mut load_file = move |name: String, path: PathBuf, remove: bool| {
        picking.set(true);
        spawn(async move {
            let planned = read_plan(path.clone()).await;
            if remove {
                let _ = std::fs::remove_file(&path);
            }
            picking.set(false);
            step.set(match planned {
                Ok((doc, plan)) => Step::Preview { name, doc, plan },
                Err(e) => Step::Failed(e),
            });
        });
    };
    let mut load = move |name: String, text: String| {
        let s = store::shared();
        step.set(match parse(&text).and_then(|doc| s.plan_import(&doc).map(|plan| (doc, plan))) {
            Ok((doc, plan)) => Step::Preview { name, doc, plan },
            Err(e) => Step::Failed(e.to_string()),
        });
    };

    let body = match step() {
        Step::Pick => {
            let files = found_files();
            let none_found = files.is_empty();
            rsx! {
                if !files.is_empty() {
                    p { class: "small", {tx::IMPORT_FOUND} }
                    ul { class: "import-files",
                        for path in files {
                            li { key: "{path.display()}",
                                button {
                                    class: "btn-link",
                                    onclick: {
                                        let path = path.clone();
                                        move |_| {
                                            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                            load_file(name, path.clone(), false);
                                        }
                                    },
                                    bdi { dir: "ltr", {path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()} }
                                }
                            }
                        }
                    }
                }
                if cfg!(target_os = "android") {
                    // The WebView has no file chooser here: the system picker copies the file in.
                    button {
                        class: "verify-btn plain",
                        disabled: picking(),
                        onclick: move |_| {
                            if documents::open().is_ok() {
                                picking.set(true);
                                spawn(async move {
                                    let done = documents::finished().await;
                                    picking.set(false);
                                    match done {
                                        DocState::Opened { name, path } => load_file(name, PathBuf::from(path), true),
                                        DocState::Failed => step.set(Step::Failed(tx::IMPORT_COPY_FAILED.to_string())),
                                        _ => {}
                                    }
                                });
                            } else {
                                step.set(Step::Failed(tx::IMPORT_PICK_FAILED.to_string()));
                            }
                        },
                        if picking() { {tx::IMPORT_PICKING} } else if none_found { {tx::IMPORT_PICK_FIRST} } else { {tx::IMPORT_PICK} }
                    }
                } else {
                // The native "Choose File" control is English; the shared button
                // carries the Arabic label and the input stays hidden inside it.
                label { class: "btn btn-secondary file-pick",
                    span { if none_found { {tx::IMPORT_PICK_FIRST} } else { {tx::IMPORT_PICK} } }
                    input {
                        r#type: "file",
                        accept: ".json,application/json",
                        onchange: move |e| async move {
                            if let Some(file) = e.files().into_iter().next() {
                                match file.read_string().await {
                                    Ok(text) => load(file.name(), text),
                                    Err(err) => step.set(Step::Failed(err.to_string())),
                                }
                            }
                        },
                    }
                }
                }
                details {
                    summary { class: "small", {tx::IMPORT_PASTE} }
                    textarea {
                        class: "export-text",
                        dir: "ltr",
                        rows: 6,
                        value: "{pasted}",
                        oninput: move |e| pasted.set(e.value()),
                    }
                    button {
                        class: "verify-btn plain",
                        disabled: pasted.read().trim().is_empty(),
                        onclick: move |_| load(tx::IMPORT_PASTED.to_string(), pasted()),
                        {tx::IMPORT_READ}
                    }
                }
            }
        }
        Step::Preview { name, doc, plan } => {
            let changes = plan.changes();
            let review = review.clone();
            rsx! {
                p { class: "small", "{tx::IMPORT_FILE} " bdi { dir: "ltr", "{name}" } }
                p { class: "muted small",
                    "{tx::IMPORT_FROM} " bdi { dir: "ltr", "{plan.device_id}" }
                    " · {tx::IMPORT_AT} " bdi { dir: "ltr", "{rfc3339(plan.exported_at_ms)}" }
                }
                PlanTable { plan: plan.clone() }
                if changes == 0 {
                    p { {tx::IMPORT_NOTHING} }
                    button { class: "verify-btn plain", onclick: move |_| step.set(Step::Pick), {tx::IMPORT_BACK} }
                } else {
                    p { class: "muted small", {tx::IMPORT_SAFE} }
                    div { class: "verify-actions",
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                step.set(match store::shared().apply_import(&doc) {
                                    Ok(plan) => {
                                        review.backend.borrow_mut().reload_and_notify();
                                        Step::Done(plan)
                                    }
                                    Err(e) => Step::Failed(e.to_string()),
                                });
                            },
                            {tx::import_apply(changes)}
                        }
                        button { class: "verify-btn plain", onclick: move |_| step.set(Step::Pick), {tx::IMPORT_CANCEL} }
                    }
                }
            }
        }
        Step::Done(plan) => rsx! {
            p { role: "status", {tx::import_done(plan.changes())} }
            if plan.failed() > 0 {
                p { class: "verify-error", role: "alert", {tx::import_partial(plan.failed())} }
            }
            button { class: "verify-btn plain", onclick: move |_| step.set(Step::Pick), {tx::IMPORT_AGAIN} }
        },
        Step::Failed(e) => rsx! {
            p { class: "verify-error", role: "alert", "{tx::IMPORT_FAILED} " bdi { dir: "ltr", "{e}" } }
            button { class: "verify-btn plain", onclick: move |_| step.set(Step::Pick), {tx::IMPORT_BACK} }
        },
    };

    rsx! {
        crate::ui::Card { class: "",
            h2 { {tx::IMPORT_TITLE} }
            {body}
        }
    }
}

#[component]
fn PlanTable(plan: ImportPlan) -> Element {
    rsx! {
        table { class: "plan",
            thead {
                tr {
                    th { {tx::COL_WHAT} }
                    th { {tx::COL_IN_FILE} }
                    th { {tx::COL_NEW} }
                    th { {tx::COL_NEWER} }
                }
            }
            tbody {
                for t in plan.tables.iter() {
                    tr { key: "{t.name}",
                        td { {table_label(&t.name)} }
                        td { "{t.rows}" }
                        td { "{t.new}" }
                        td { "{t.newer}" }
                    }
                }
            }
        }
    }
}

fn table_label(name: &str) -> String {
    match name {
        "review_events" => tx::T_REVIEWS.to_string(),
        "item_status" => tx::T_CHECKS.to_string(),
        "settings" => tx::T_SETTINGS.to_string(),
        "own_cards" => tx::T_OWN.to_string(),
        "notes" => tx::T_NOTES.to_string(),
        other => other.replace('_', " "),
    }
}
