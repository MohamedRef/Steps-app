//! Verify-while-study: after the answer, the first time Mohamed sees an
//! item, [`VerifyBar`] asks ✓ صح / ✎ عدّل / ✗ غلط. The study player puts it
//! above the grade row; it renders nothing when the item needs no check.

mod export;

use dioxus::prelude::*;
use steps_core::content::Body;
use steps_core::review::verify::{self, Check, Edit, FieldValue, Verdict};

pub use export::ExportVerdicts;

use crate::i18n::verify as tx;
use crate::look::{Css, css};
use crate::store::use_review;

pub(crate) const CSS: Css = css!("src/verify/verify.css");

/// Rendered above the study player's grade row (`study::grade::GradeRow`).
#[component]
pub fn VerifyBar(item_id: String) -> Element {
    // Keyed, so a bar reused for the next item starts fresh.
    rsx! { Bar { key: "{item_id}", item_id: item_id.clone() } }
}

/// A verdict write, kept when it fails so "try again" repeats it.
#[derive(Clone, PartialEq)]
enum Attempt {
    Judge(Verdict, Edit, Option<String>),
    Undo,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Ask,
    Wrong,
    Edit,
}

#[component]
fn Bar(item_id: String) -> Element {
    let review = use_review();
    let mut mode = use_signal(|| Mode::Ask);
    // The verdict given in this bar, so it can say what happened and undo it.
    let mut done = use_signal(|| None::<Verdict>);
    let mut note = use_signal(String::new);
    // The last tap that wasn't saved; "try again" repeats exactly it.
    let mut failed = use_signal(|| None::<Attempt>);

    let check = review.backend.borrow().check(&item_id);
    if done().is_none() && check != Check::Ask {
        return rsx! {};
    }

    // Both writes are idempotent (a verdict replaces the last one), so a
    // retry may simply run the same attempt again.
    let attempt = {
        let id = item_id.clone();
        let review = review.clone();
        move |attempt: Attempt| {
            let saved = match &attempt {
                Attempt::Judge(verdict, edit, n) => {
                    review.backend.borrow_mut().judge(&id, *verdict, edit.clone(), n.clone())
                }
                Attempt::Undo => review.backend.borrow_mut().undo_judgement(&id),
            };
            if !saved {
                failed.set(Some(attempt));
                return;
            }
            failed.set(None);
            match attempt {
                Attempt::Judge(verdict, ..) => {
                    done.set(Some(verdict));
                    mode.set(Mode::Ask);
                }
                Attempt::Undo => done.set(None),
            }
        }
    };
    let judge = {
        let mut attempt = attempt.clone();
        move |verdict: Verdict, edit: Edit, n: Option<String>| attempt(Attempt::Judge(verdict, edit, n))
    };

    let body = match (done(), mode()) {
        (Some(v), _) => {
            let mut undo = attempt.clone();
            let said = match v {
                Verdict::Approved => tx::SAVED_APPROVED,
                Verdict::Edited => tx::SAVED_EDITED,
                Verdict::Wrong => tx::SAVED_WRONG,
            };
            rsx! {
                p { class: "verify-saved", role: "status", "data-verdict": v.as_str(), {said} }
                button {
                    class: "verify-undo",
                    onclick: move |_| undo(Attempt::Undo),
                    {tx::UNDO}
                }
            }
        }
        (None, Mode::Ask) => {
            let mut approve = judge.clone();
            rsx! {
                span { class: "verify-q", {tx::QUESTION} }
                div { class: "verify-actions",
                    button { class: "verify-btn ok", onclick: move |_| approve(Verdict::Approved, Edit::new(), None), {tx::RIGHT} }
                    button { class: "verify-btn edit", onclick: move |_| mode.set(Mode::Edit), {tx::EDIT} }
                    button { class: "verify-btn bad", onclick: move |_| mode.set(Mode::Wrong), {tx::WRONG} }
                }
            }
        }
        (None, Mode::Wrong) => {
            let mut wrong = judge.clone();
            rsx! {
                label { class: "verify-field",
                    span { {tx::WRONG_NOTE} }
                    textarea {
                        rows: 2,
                        dir: "auto",
                        placeholder: tx::WRONG_PLACEHOLDER,
                        value: "{note}",
                        oninput: move |e| note.set(e.value()),
                    }
                }
                div { class: "verify-actions",
                    button {
                        class: "verify-btn bad",
                        onclick: move |_| wrong(Verdict::Wrong, Edit::new(), Some(note())),
                        {tx::WRONG_CONFIRM}
                    }
                    button { class: "verify-btn plain", onclick: move |_| mode.set(Mode::Ask), {tx::CANCEL} }
                }
            }
        }
        (None, Mode::Edit) => {
            let entry = review.backend.borrow().entry(&item_id).cloned();
            let Some(entry) = entry else { return rsx! {} };
            let save = judge.clone();
            rsx! {
                Editor {
                    body: entry.item.body.clone(),
                    on_save: move |(edit, n): (Edit, String)| {
                        let mut save = save.clone();
                        save(Verdict::Edited, edit, Some(n));
                    },
                    on_cancel: move |_| mode.set(Mode::Ask),
                }
            }
        }
    };
    rsx! {
        style { {CSS.text()} }
        section { class: "verify", role: "group", aria_label: tx::GROUP,
            p { class: "verify-head",
                strong { {tx::GROUP} }
            }
            {body}
            if let Some(last) = failed() {
                crate::persistence::WriteFailed {
                    message: tx::NOT_SAVED,
                    on_retry: {
                        let mut attempt = attempt.clone();
                        move |_| attempt(last.clone())
                    },
                }
            }
        }
    }
}

/// The ✎ editor: the kind's text fields, each in its own box, plus a note.
#[component]
fn Editor(body: Body, on_save: EventHandler<(Edit, String)>, on_cancel: EventHandler<()>) -> Element {
    let fields = verify::editable_fields(&body);
    let mut drafts = use_signal(|| fields.iter().map(|(k, v)| (*k, draft_text(v))).collect::<Vec<_>>());
    let mut note = use_signal(String::new);
    let mut error = use_signal(|| None::<&'static str>);
    let options = match &body {
        Body::Mcq { options, .. } => options.clone(),
        _ => Vec::new(),
    };

    let save = {
        let body = body.clone();
        let fields = fields.clone();
        move |_| {
            let edited: Vec<(&str, FieldValue)> = drafts
                .read()
                .iter()
                .zip(&fields)
                .map(|((k, text), (_, before))| (*k, parse_draft(before, text)))
                .collect();
            let edit = verify::diff(&body, &edited);
            if edit.is_empty() && note.read().trim().is_empty() {
                error.set(Some(tx::NOTHING_CHANGED));
                return;
            }
            if let Some(problem) = verify::edit_problem(&body, &edit) {
                error.set(Some(problem));
                return;
            }
            on_save.call((edit, note()));
        }
    };

    rsx! {
        div { class: "verify-editor",
            for (i, (key, before)) in fields.iter().enumerate() {
                label { key: "{key}", class: "verify-field",
                    span { {field_label(key)} " " bdi { class: "muted small", "({key})" } }
                    match before {
                        FieldValue::Index(_) => rsx! {
                            select {
                                value: "{drafts.read()[i].1}",
                                onchange: move |e| {
                                    drafts.write()[i].1 = e.value();
                                    error.set(None);
                                },
                                for (n, o) in options.iter().enumerate() {
                                    option { key: "{n}", value: "{n}", "{letter(n)}. {o}" }
                                }
                            }
                        },
                        _ => rsx! {
                            textarea {
                                dir: "ltr",
                                rows: rows_for(&drafts.read()[i].1),
                                value: "{drafts.read()[i].1}",
                                oninput: move |e| {
                                    drafts.write()[i].1 = e.value();
                                    error.set(None);
                                },
                            }
                        },
                    }
                }
            }
            label { class: "verify-field",
                span { {tx::EDIT_NOTE} }
                textarea {
                    rows: 2,
                    dir: "auto",
                    value: "{note}",
                    oninput: move |e| {
                        note.set(e.value());
                        error.set(None);
                    },
                }
            }
            if let Some(e) = error() {
                p { class: "verify-error", role: "alert", {e} }
            }
            div { class: "verify-actions",
                button { class: "verify-btn ok", onclick: save, {tx::SAVE} }
                button { class: "verify-btn plain", onclick: move |_| on_cancel.call(()), {tx::CANCEL} }
            }
        }
    }
}

fn draft_text(v: &FieldValue) -> String {
    match v {
        FieldValue::Index(i) => i.to_string(),
        FieldValue::Text(t) => t.clone(),
        FieldValue::List(l) => l.join("\n"),
    }
}

/// The draft back in the field's own shape. Blank lines in a list are dropped.
fn parse_draft(before: &FieldValue, text: &str) -> FieldValue {
    match before {
        FieldValue::Index(i) => FieldValue::Index(text.trim().parse().unwrap_or(*i)),
        FieldValue::Text(_) => FieldValue::Text(text.trim().to_string()),
        FieldValue::List(_) => FieldValue::List(
            text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
        ),
    }
}

fn rows_for(text: &str) -> i64 {
    let lines = text.lines().count() + text.len() / 60;
    lines.clamp(2, 10) as i64
}

fn letter(n: usize) -> char {
    (b'A' + (n as u8).min(25)) as char
}

fn field_label(key: &str) -> &'static str {
    match key {
        "front" => tx::F_FRONT,
        "back" => tx::F_BACK,
        "text" => tx::F_TEXT,
        "extra" => tx::F_EXTRA,
        "stem" => tx::F_STEM,
        "options" => tx::F_OPTIONS,
        "answer" => tx::F_ANSWER,
        "why" => tx::F_WHY,
        "prompt" => tx::F_PROMPT,
        "points" => tx::F_POINTS,
        "steps" => tx::F_STEPS,
        "title" => tx::F_TITLE,
        "question" => tx::F_QUESTION,
        "expect" => tx::F_EXPECT,
        "interpret" => tx::F_INTERPRET,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_round_trip_each_shape() {
        for v in [
            FieldValue::Index(2),
            FieldValue::Text("Child-Pugh B".into()),
            FieldValue::List(vec!["a".into(), "b".into()]),
        ] {
            assert_eq!(parse_draft(&v, &draft_text(&v)), v);
        }
        let list = FieldValue::List(vec![]);
        assert_eq!(parse_draft(&list, " x \n\n y"), FieldValue::List(vec!["x".into(), "y".into()]));
        assert_eq!(parse_draft(&FieldValue::Index(1), "oops"), FieldValue::Index(1));
    }

    #[test]
    fn every_editable_field_has_a_label() {
        let lec: steps_core::content::Lecture =
            serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
        for item in &lec.items {
            for (key, _) in verify::editable_fields(&item.body) {
                assert!(!field_label(key).is_empty(), "{key}");
            }
        }
    }
}
