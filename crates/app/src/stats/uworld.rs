//! The UWorld mistake log (research item: "qbank mistake log by Q ID"), on
//! the mistakes tab under Steps' own misses: Mohamed types the ID of a
//! UWorld question he missed and the reason he missed it, and reads the
//! patterns: the reason behind most misses, with what to
//! change, and the First Aid lectures the misses fall in, each with its QIDs
//! to copy back into a UWorld custom test. Core rules: `steps_core::uwmiss`.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use steps_core::review::ErrorReason;
use steps_core::stats::Clock;
use steps_core::uwmiss::{self as uw, Miss, PREFIX, Patterns, Problem};

use crate::Ui;
use crate::content::Library;
use crate::i18n::uwmiss as tx;
use crate::look::{Css, css};
use crate::metacog::{METACOG_CSS, reason_label};
use crate::qbank::data;

const CSS: Css = css!("src/stats/uworld.css");

fn load() -> Vec<Miss> {
    let all: Vec<Miss> =
        crate::store::shared().settings_with_prefix::<Miss>(PREFIX).into_iter().map(|(_, m)| m).collect();
    uw::newest_first(&all)
}

static MISSES: GlobalSignal<Vec<Miss>> = Signal::global(load);

fn copy(text: &str) {
    document::eval(&format!(
        "navigator.clipboard && navigator.clipboard.writeText({})",
        serde_json::to_string(text).unwrap_or_default()
    ));
}

/// After a sync or restore.
pub fn reload() {
    let fresh = load();
    if *MISSES.peek() != fresh {
        *MISSES.write() = fresh;
    }
}

fn put(m: Miss) {
    crate::store::save_setting(&m.key(), &m);
    let mut all = MISSES.write();
    all.retain(|x| x.qid != m.qid);
    all.push(m);
    *all = uw::newest_first(&all);
}

fn remove(qid: u32) {
    let key = uw::key(qid);
    crate::persistence::save(format!("synced.{key}"), move || {
        let result = crate::store::shared().delete_setting(&key).map_err(|e| e.to_string());
        match &result {
            Ok(()) => crate::store::write_recovered(&key),
            Err(e) => crate::store::write_failed(&key, e),
        }
        result
    });
    MISSES.write().retain(|x| x.qid != qid);
}

fn today() -> steps_core::plan::Day {
    let now_ms = crate::store::now_ms();
    Clock { now_ms, zone: crate::store::zone(), rollover_hour: u32::from(steps_core::srs::ROLLOVER_HOUR) }
        .today()
}

/// A QID's First Aid lecture, when the B&B mapping files it.
fn lecture_of(qid: u32) -> Option<String> {
    data().find(qid).map(|(l, _)| l.to_string())
}

pub fn patterns(misses: &[Miss]) -> Patterns {
    uw::patterns(misses, lecture_of)
}

/// Under a typed QID: log it as a miss, or change the reason it was logged
/// with. Keyed by QID, so another QID typed starts from its own entry.
#[component]
pub fn LogMiss(qid: u32) -> Element {
    let logged = MISSES.read().iter().find(|m| m.qid == qid).cloned();
    let mut reason = use_signal(|| logged.as_ref().map(|m| m.reason));
    let mut note = use_signal(|| logged.as_ref().and_then(|m| m.note.clone()).unwrap_or_default());
    let mut error = use_signal(|| None::<&'static str>);
    let mut saved = use_signal(|| false);
    let save = move |_| {
        let Some(r) = reason() else {
            error.set(Some(tx::PICK_REASON));
            return;
        };
        let created_ms =
            MISSES.peek().iter().find(|m| m.qid == qid).map_or_else(crate::store::now_ms, |m| m.created_ms);
        let m = Miss { qid, reason: r, note: Some(note()), day: today(), created_ms }.tidy();
        if let Some(Problem::NoteTooLong) = m.problem() {
            error.set(Some(tx::NOTE_TOO_LONG));
            return;
        }
        error.set(None);
        put(m);
        saved.set(true);
    };
    rsx! {
        style { {METACOG_CSS.text()} }
        section { class: "uwm-log", aria_labelledby: "uwm-log-title",
            h3 { id: "uwm-log-title", class: "uwm-sub", {tx::LOG_TITLE} }
            if let Some(m) = &logged {
                p { class: "muted small uwm-logged",
                    {tx::LOGGED_AS}
                    " "
                    strong { {reason_label(m.reason)} }
                }
            }
            div { class: "mc", role: "group", aria_labelledby: "uwm-reason-label",
                p { class: "mc-head small", strong { id: "uwm-reason-label", {tx::REASON} } }
                div { class: "mc-chips",
                    for r in ErrorReason::ALL {
                        button {
                            key: "{r.as_str()}",
                            class: "mc-chip",
                            "data-reason": r.as_str(),
                            aria_pressed: if reason() == Some(r) { "true" } else { "false" },
                            onclick: move |_| {
                                reason.set(Some(r));
                                saved.set(false);
                                error.set(None);
                            },
                            {reason_label(r)}
                        }
                    }
                }
            }
            label { class: "field uwm-note",
                span { class: "small", {tx::NOTE} }
                input {
                    r#type: "text",
                    maxlength: "{uw::MAX_NOTE}",
                    placeholder: tx::NOTE_HINT,
                    value: "{note}",
                    oninput: move |e| {
                        note.set(e.value());
                        saved.set(false);
                    },
                }
            }
            if let Some(e) = error() {
                p { class: "note uwm-error", role: "alert", {e} }
            }
            div { class: "uwm-actions",
                button { class: "btn btn-primary", onclick: save,
                    if logged.is_some() { {tx::UPDATE} } else { {tx::SAVE} }
                }
                if saved() {
                    span { class: "muted small", role: "status", {tx::SAVED} }
                }
            }
        }
    }
}

/// The mistakes tab's UWorld card: a QID field, the form for the typed QID,
/// then the log.
#[component]
pub fn UworldMistakes() -> Element {
    let ui = use_context::<Ui>();
    let mut text = use_signal(String::new);
    let qid = uw::parse_qid(&text.read());
    let lecture = qid.and_then(lecture_of);
    let lib = Library::get();
    rsx! {
        style { {CSS.text()} }
        crate::ui::Card { class: "uwm-card",
            h2 { {tx::CARD_TITLE} }
            p { class: "muted small", {tx::CARD_HOW} }
            label { class: "field uwm-qid",
                span { class: "small", {tx::QID} }
                input {
                    r#type: "text",
                    inputmode: "numeric",
                    dir: "ltr",
                    placeholder: tx::QID_HINT,
                    value: "{text}",
                    oninput: move |e| text.set(e.value()),
                }
            }
            if let Some(l) = lecture {
                p { class: "small uwm-from",
                    {tx::FROM}
                    " "
                    button {
                        class: "btn-link",
                        onclick: {
                            let l = l.clone();
                            move |_| crate::lecture::open(ui, &l)
                        },
                        bdi { lang: "en", dir: "ltr", {lib.crumb(&l)} }
                    }
                }
            }
            if let Some(qid) = qid {
                LogMiss { key: "{qid}", qid }
            }
            MissLog {}
        }
    }
}

/// The log: its patterns, then every miss, newest first.
#[component]
pub fn MissLog() -> Element {
    let misses = MISSES();
    let p = patterns(&misses);
    // A reason chip narrows the list; one no longer logged shows them all.
    let only = use_signal(|| None::<ErrorReason>);
    let shown = only().filter(|r| misses.iter().any(|m| m.reason == *r));
    let listed: Vec<Miss> = misses.iter().filter(|m| shown.is_none_or(|r| m.reason == r)).cloned().collect();
    rsx! {
        section { class: "uwm", aria_labelledby: "uwm-title",
            h3 { id: "uwm-title", class: "uwm-sub",
                {tx::TITLE}
                if !misses.is_empty() {
                    span { class: "muted small", " · {tx::total(misses.len())}" }
                }
            }
            if misses.is_empty() {
                p { class: "muted small", {tx::EMPTY} }
            } else {
                PatternsCard { p, only, shown }
                h4 { class: "uwm-sub", {tx::LIST} }
                ul { class: "uwm-list",
                    for m in listed {
                        MissRow { key: "{m.qid}", m: m.clone() }
                    }
                }
            }
            p { class: "muted small", {tx::PRIVACY} }
        }
    }
}

#[component]
fn PatternsCard(p: Patterns, mut only: Signal<Option<ErrorReason>>, shown: Option<ErrorReason>) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let mut copied = use_signal(|| None::<String>);
    let top = p.top_reason();
    rsx! {
        div { class: "uwm-patterns",
            h4 { class: "uwm-sub", {tx::PATTERNS} }
            if let Some((r, n)) = top {
                p { class: "uwm-top", strong { {tx::top_reason(reason_label(r), n, p.total, p.share(n))} } }
                p { class: "small", {tx::advice(r)} }
            } else if p.total < uw::PATTERN_MIN {
                p { class: "muted small", {tx::TOO_FEW} }
            } else {
                p { class: "muted small", {tx::TIE} }
            }
            div { class: "mc-chips uwm-reasons", role: "group", aria_label: tx::FILTER,
                for (r, n) in p.by_reason.iter().copied() {
                    button {
                        key: "{r.as_str()}",
                        class: "mc-chip",
                        "data-reason": r.as_str(),
                        aria_pressed: if shown == Some(r) { "true" } else { "false" },
                        onclick: move |_| only.set(crate::metacog::toggled(only(), r)),
                        {reason_label(r)}
                        span { class: "mc-count num-ltr", "{n}" }
                    }
                }
            }
            if !p.by_lecture.is_empty() {
                h4 { class: "uwm-sub", {tx::BY_LECTURE} }
                ul { class: "uwm-lectures",
                    for (lecture, qids) in p.by_lecture.iter().cloned() {
                        li { key: "{lecture}", class: "uwm-lecture",
                            span { class: "uwm-lec-name",
                                bdi { lang: "en", dir: "ltr", {lib.crumb(&lecture)} }
                                span { class: "muted small", " · {tx::total(qids.len())}" }
                            }
                            span { class: "uwm-lec-actions",
                                button {
                                    class: "chip tap-chip",
                                    title: "{list(&qids)}",
                                    onclick: {
                                        let (lecture, qids) = (lecture.clone(), qids.clone());
                                        move |_| {
                                            copy(&list(&qids));
                                            copied.set(Some(lecture.clone()));
                                        }
                                    },
                                    if copied().as_deref() == Some(lecture.as_str()) { {tx::COPIED} } else { {tx::COPY_QIDS} }
                                }
                                button {
                                    class: "btn-link",
                                    onclick: {
                                        let lecture = lecture.clone();
                                        move |_| crate::lecture::open(ui, &lecture)
                                    },
                                    {tx::OPEN}
                                }
                            }
                        }
                    }
                }
            }
            if p.unfiled > 0 {
                p { class: "muted small", "{tx::UNFILED} {p.unfiled}" }
            }
        }
    }
}

/// QIDs as UWorld's custom test takes them, each once.
fn list(qids: &[u32]) -> String {
    let unique: BTreeSet<u32> = qids.iter().copied().collect();
    unique.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
}

#[component]
fn MissRow(m: Miss) -> Element {
    let mut confirm = use_signal(|| false);
    let lib = Library::get();
    let lecture = lecture_of(m.qid);
    let qid = m.qid;
    rsx! {
        li { class: "uwm-row",
            div { class: "uwm-row-main",
                strong { class: "num-ltr", dir: "ltr", "#{m.qid}" }
                span { class: "uwm-row-reason", {reason_label(m.reason)} }
                span { class: "muted small", {crate::plan::short_date(m.day)} }
            }
            if let Some(l) = lecture {
                p { class: "muted small uwm-row-lec", bdi { lang: "en", dir: "ltr", {lib.crumb(&l)} } }
            }
            if let Some(n) = &m.note {
                p { class: "small uwm-row-note", bdi { {n.clone()} } }
            }
            div { class: "uwm-row-actions",
                if confirm() {
                    button { class: "btn btn-danger", onclick: move |_| remove(qid), {tx::CONFIRM_DELETE} }
                    button { class: "btn btn-ghost", onclick: move |_| confirm.set(false), {tx::CANCEL} }
                } else {
                    button { class: "btn btn-ghost", onclick: move |_| confirm.set(true), {tx::DELETE} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
