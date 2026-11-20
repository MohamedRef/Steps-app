//! Settings › بياناتك › «ضيف أسئلة من ملف»: pick a file, see what it would
//! add and change and what is wrong with which row, then add the good ones.
//! Below: his categories, each with how many questions, to delete one.

use std::path::PathBuf;

use dioxus::prelude::*;
use steps_core::ownq::{self, OwnQuestion, Places, Plan, RowProblem};

use super::file::{self, Read, Shape};
use super::report::{self, Report, ReportView};
use crate::content::Library;
use crate::i18n::ownq as tx;
use crate::platform::documents::{self, DocState};

/// The template, `steps-questions-template.xlsx` beside this file (docs/OWN-QUESTIONS.md).
pub const TEMPLATE: &[u8] = include_bytes!("steps-questions-template.xlsx");
pub const TEMPLATE_NAME: &str = "steps-questions-template.xlsx";
/// The last file's report, kept while Steps is open: a question opened from
/// it closes Settings, and the report is still there on the way back, until
/// another file.
static LAST: GlobalSignal<Option<(String, Report)>> = Signal::global(|| None);

#[derive(Clone, PartialEq)]
enum Step {
    Pick,
    Reading,
    Preview {
        name: String,
        read: Box<Checked>,
    },
    /// What the file did, question by question.
    Done {
        name: String,
        report: Box<Report>,
    },
    Failed(String),
}

/// A file read and checked against the library and the store.
#[derive(Clone, PartialEq)]
pub struct Checked {
    pub questions: Vec<OwnQuestion>,
    pub plan: Plan,
    /// Each problem in words, with where it is.
    pub problems: Vec<String>,
    /// What it would do, question by question.
    pub report: Report,
    figures: Vec<(ownq::Image, Vec<u8>)>,
}

/// Reads and checks a file; what a tap on «ضيفهم» would store.
pub fn check(name: &str, bytes: &[u8]) -> Checked {
    let read: Read = file::read(name, bytes);
    let lib = Library::get();
    let places = Places { index: &lib.index, lectures: &lib.lectures };
    let (questions, row_problems) = ownq::read(&read.rows, &places, &read.images);
    let sheet = read.shape != Some(Shape::List);
    let mut problems: Vec<String> = read.problems.iter().map(tx::file_problem).collect();
    problems.extend(
        row_problems.iter().map(|RowProblem { line, problem }| {
            format!("{}: {}", tx::at(*line, sheet), tx::problem(problem))
        }),
    );
    let existing = crate::store::shared().own_questions();
    let plan = ownq::plan(&existing, &questions);
    let report = report::report(&questions, &existing, &plan, problems.clone());
    let figures = read.figures(&questions);
    Checked { questions, plan, problems, report, figures }
}

/// Stores the new and changed questions and opens the library again with them.
pub fn apply(checked: &Checked, review: &crate::store::Review) -> Result<(usize, usize), String> {
    let wanted: std::collections::BTreeSet<&String> =
        checked.plan.new.iter().chain(&checked.plan.changed).collect();
    let questions: Vec<OwnQuestion> =
        checked.questions.iter().filter(|q| wanted.contains(&q.key())).cloned().collect();
    crate::store::shared().put_own_questions(&questions, &checked.figures).map_err(|e| e.to_string())?;
    reopen(review);
    Ok((checked.plan.new.len(), checked.plan.changed.len()))
}

/// The library again, with the questions as the store has them now.
pub fn reopen(review: &crate::store::Review) {
    Library::reload();
    review.backend.borrow_mut().library_replaced();
    *crate::content::REV.write() += 1;
}

#[component]
pub fn OwnQuestionsCard() -> Element {
    let review = crate::store::use_review();
    let ui = use_context::<crate::Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    let mut step = use_signal(|| match LAST.peek().as_ref() {
        Some((name, report)) => Step::Done { name: name.clone(), report: Box::new(report.clone()) },
        None => Step::Pick,
    });
    // Back to picking a file: the last report goes.
    let mut restart = move || {
        *LAST.write() = None;
        step.set(Step::Pick);
    };
    // A category deleted below takes the report of its questions with it.
    use_effect(move || {
        if LAST.read().is_none() && matches!(*step.peek(), Step::Done { .. }) {
            step.set(Step::Pick);
        }
    });
    let mut picking = use_signal(|| false);
    let mut template = use_signal(|| None::<Result<String, String>>);
    let _ = crate::content::REV.read();

    let mut load = move |name: String, bytes: Vec<u8>| {
        *LAST.write() = None;
        step.set(Step::Reading);
        let read = check(&name, &bytes);
        step.set(Step::Preview { name, read: Box::new(read) });
    };
    // A file shared to Steps (`crate::inbox`): read as if picked here.
    use_effect(move || {
        if crate::inbox::QUESTIONS.read().is_some() {
            spawn(async move {
                let Some(arrived) = crate::inbox::QUESTIONS.write().take() else { return };
                match arrived.bytes {
                    Some(bytes) => load(arrived.name, bytes),
                    None => step.set(Step::Failed(tx::COPY_FAILED.into())),
                }
                crate::inbox::scroll_to(crate::inbox::QUESTIONS_CARD);
            });
        }
    });
    let mut load_path = move |name: String, path: PathBuf| match std::fs::read(&path) {
        Ok(bytes) => {
            let _ = std::fs::remove_file(&path);
            load(name, bytes);
        }
        Err(_) => step.set(Step::Failed(tx::COPY_FAILED.into())),
    };

    let body = match step() {
        Step::Pick | Step::Reading => rsx! {
            p { class: "small", {tx::INTRO} }
            p { class: "muted small", {tx::UPDATE_HINT} }
            div { class: "row",
                if cfg!(target_os = "android") {
                    button {
                        class: "btn btn-primary",
                        disabled: picking(),
                        onclick: move |_| {
                            if documents::open(documents::QUESTIONS).is_ok() {
                                picking.set(true);
                                spawn(async move {
                                    let done = documents::finished().await;
                                    picking.set(false);
                                    match done {
                                        DocState::Opened { name, path } => load_path(name, PathBuf::from(path)),
                                        DocState::Failed => step.set(Step::Failed(tx::COPY_FAILED.into())),
                                        _ => {}
                                    }
                                });
                            } else {
                                step.set(Step::Failed(tx::PICK_FAILED.into()));
                            }
                        },
                        if picking() { {tx::PICKING} } else { {tx::PICK} }
                    }
                } else {
                    label { class: "btn btn-primary file-pick",
                        span { {tx::PICK} }
                        input {
                            r#type: "file",
                            accept: ".zip,.xlsx,.json,.yaml,.yml",
                            onchange: move |e| async move {
                                if let Some(f) = e.files().into_iter().next() {
                                    match f.read_bytes().await {
                                        Ok(bytes) => load(f.name(), bytes.to_vec()),
                                        Err(err) => step.set(Step::Failed(err.to_string())),
                                    }
                                }
                            },
                        }
                    }
                }
                button { class: "btn btn-secondary", onclick: move |_| template.set(Some(save_template())), {tx::TEMPLATE} }
            }
            if step() == Step::Reading {
                p { class: "muted small", role: "status", {tx::READING} }
            }
            match template() {
                Some(Ok(path)) => rsx! {
                    div { role: "status",
                        p { class: "small", {tx::TEMPLATE_SAVED} }
                        p { class: "small", bdi { dir: "ltr", "{path}" } }
                        if cfg!(target_os = "android") {
                            crate::data_export::SaveTo { path }
                        }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "verify-error", role: "alert", "{tx::TEMPLATE_FAILED} " bdi { dir: "ltr", "{e}" } } },
                None => rsx! {},
            }
        },
        Step::Preview { name, read } => {
            let adds = read.plan.new.len() + read.plan.changed.len();
            let review = review.clone();
            rsx! {
                p { class: "small", "{tx::FILE} " bdi { dir: "ltr", "{name}" } }
                if read.questions.is_empty() {
                    p { strong { {tx::NONE_GOOD} } }
                } else if adds == 0 {
                    p { {tx::NOTHING_NEW} }
                } else {
                    p { role: "status",
                        strong { {tx::will(read.report.new.len(), read.report.changed.len(), &names(&read.report))} }
                    }
                }
                ReportView { report: read.report.clone(), done: false }
                if !read.problems.is_empty() && adds > 0 {
                    p { class: "muted small", {tx::SKIPPED} }
                }
                div { class: "row",
                    if adds > 0 {
                        button {
                            class: "btn btn-primary",
                            onclick: {
                                let read = read.clone();
                                let name = name.clone();
                                move |_| {
                                    step.set(match apply(&read, &review) {
                                        Ok(_) => {
                                            *LAST.write() = Some((name.clone(), read.report.clone()));
                                            Step::Done { name: name.clone(), report: Box::new(read.report.clone()) }
                                        }
                                        Err(e) => Step::Failed(e),
                                    });
                                }
                            },
                            {tx::APPLY}
                        }
                    }
                    button { class: "btn btn-secondary", onclick: move |_| restart(),
                        if adds > 0 { {tx::CANCEL} } else { {tx::BACK} }
                    }
                }
            }
        }
        Step::Done { name, report } => {
            rsx! {
                p { class: "small", "{tx::FILE} " bdi { dir: "ltr", "{name}" } }
                p { role: "status", strong { {tx::done(report.new.len(), report.changed.len(), &names(&report))} } }
                ReportView { report: (*report).clone(), done: true }
                div { class: "row",
                    for (i, (set, c)) in report.sets().into_iter().enumerate() {
                        button {
                            key: "{set}",
                            class: if i == 0 { "btn btn-primary" } else { "btn btn-tonal" },
                            onclick: move |_| report::open_category(ui, settings, &c),
                            {tx::open_set(&set)}
                        }
                    }
                    button { class: "btn btn-secondary", onclick: move |_| restart(), {tx::AGAIN} }
                }
            }
        }
        Step::Failed(e) => rsx! {
            p { class: "verify-error", role: "alert", "{tx::FAILED} " bdi { dir: "ltr", "{e}" } }
            button { class: "btn btn-secondary", onclick: move |_| restart(), {tx::BACK} }
        },
    };

    rsx! {
        style { {super::CSS.text()} }
        crate::ui::Card { class: "ownq-card", id: crate::inbox::QUESTIONS_CARD,
            h2 { {tx::TITLE} }
            {body}
            Categories {}
        }
    }
}

/// The banks a report adds to or changes, by name.
fn names(report: &Report) -> Vec<String> {
    report.sets().into_iter().map(|(name, _)| name).collect()
}

/// His categories, each with how many questions, and a way to delete one.
#[component]
fn Categories() -> Element {
    let review = crate::store::use_review();
    let ui = use_context::<crate::Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    let _ = crate::content::REV.read();
    let mut asking = use_signal(|| None::<String>);
    let mut said = use_signal(|| None::<String>);
    let cats = ownq::categories(&crate::store::shared().own_questions());
    if cats.is_empty() && said().is_none() {
        return rsx! {};
    }
    rsx! {
        h3 { class: "label", {tx::YOURS} }
        ul { class: "tap-list ownq-cats",
            for (c, n) in cats {
                li { key: "{c}",
                    div { class: "ownq-cat",
                        // The category opens in the bank.
                        button {
                            class: "tap tap-plain ownq-cat-open",
                            title: tx::open_set(&report::set_name(&c)),
                            onclick: { let c = c.clone(); move |_| report::open_category(ui, settings, &c) },
                            span { class: "tap-row-main",
                                bdi { "{tx::set(&c)}" }
                                span { class: "muted small", {crate::i18n::bank::questions(n)} }
                            }
                            span { class: "chev", aria_hidden: "true" }
                        }
                        if asking() == Some(c.clone()) {
                            button {
                                class: "btn btn-danger",
                                onclick: {
                                    let c = c.clone();
                                    let review = review.clone();
                                    move |_| {
                                        asking.set(None);
                                        match crate::store::shared().delete_own_category(&c) {
                                            Ok(n) => {
                                                // A report may name questions that are gone.
                                                *LAST.write() = None;
                                                reopen(&review);
                                                said.set(Some(tx::deleted(n)));
                                            }
                                            Err(e) => said.set(Some(format!("{} {e}", tx::FAILED))),
                                        }
                                    }
                                },
                                {tx::DELETE_YES}
                            }
                        } else {
                            button {
                                class: "btn btn-ghost",
                                onclick: { let c = c.clone(); move |_| asking.set(Some(c.clone())) },
                                {tx::DELETE}
                            }
                        }
                    }
                }
            }
        }
        if asking().is_some() {
            p { class: "small", role: "alert", {tx::DELETE_SURE} }
        }
        if let Some(s) = said() {
            p { class: "small", role: "status", "{s}" }
        }
    }
}

/// Puts the template where he can open it: a download on the web, the
/// export folder elsewhere (with "Save to" on Android).
fn save_template() -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    return crate::platform::web::download(
        TEMPLATE_NAME,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        TEMPLATE,
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = crate::store::export_dir()?;
        let path = dir.join(TEMPLATE_NAME);
        std::fs::write(&path, TEMPLATE).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path.display().to_string())
    }
}
