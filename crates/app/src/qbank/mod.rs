//! Boards and Beyond videos and UWorld question IDs per First Aid lecture
//! (docs/QBANK.md): on a lecture's page, each B&B video with its UWorld
//! Step 1 QIDs to paste into UWorld › Create Test › Custom Test, a mark for
//! the ones he has done, and a lookup from a QID he missed back to its
//! lecture. Numbers and video names only, never question text.
//!
//! Content: `content/qbank/uworld.yaml` (`steps_core::qbank`), refreshed by
//! `scripts/qbank-sync.sh`.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use dioxus::prelude::*;
use steps_core::qbank::{LectureSets, QBank, Set};

use crate::Ui;
use crate::content::Library;
use crate::i18n::qbank as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/qbank/qbank.css");
/// Synced, one key per video: `qbank.done.<video>` = true.
const DONE: &str = "qbank.done.";
/// The page id of the lookup on the lecture page stack.
pub const PAGE: &str = "uworld";

pub fn data() -> &'static QBank {
    static DATA: OnceLock<QBank> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_yaml_ng::from_str(include_str!("../../../../content/qbank/uworld.yaml"))
            .expect("content/qbank/uworld.yaml parses (steps-tools validate)")
    })
}

fn load_done() -> BTreeSet<String> {
    crate::store::shared()
        .settings_with_prefix::<bool>(DONE)
        .into_iter()
        .filter(|(_, on)| *on)
        .filter_map(|(k, _)| k.strip_prefix(DONE).map(str::to_string))
        .collect()
}

static DONE_SET: GlobalSignal<BTreeSet<String>> = Signal::global(load_done);

fn set_done(video: &str, on: bool) {
    crate::store::save_setting(&format!("{DONE}{video}"), &on);
    let mut done = DONE_SET.write();
    if on {
        done.insert(video.to_string());
    } else {
        done.remove(video);
    }
}

/// After a sync or restore.
pub fn reload() {
    let fresh = load_done();
    if *DONE_SET.peek() != fresh {
        *DONE_SET.write() = fresh;
    }
}

/// QIDs as UWorld's custom test takes them: comma separated.
pub fn qid_list<'a>(sets: impl IntoIterator<Item = &'a Set>) -> String {
    sets.into_iter().flat_map(|s| s.qids.iter()).map(u32::to_string).collect::<Vec<_>>().join(", ")
}

/// The B&B video name without its course: `Atherosclerosis`.
pub fn video_name(set: &Set) -> &str {
    set.video.split_once(" › ").map_or(set.video.as_str(), |(_, v)| v)
}

fn copy(text: &str) {
    document::eval(&format!(
        "navigator.clipboard && navigator.clipboard.writeText({})",
        serde_json::to_string(text).unwrap_or_default()
    ));
}

/// Opens the QID lookup over the current screen.
pub fn open(ui: Ui) {
    crate::pagesheet::close();
    crate::lecture::open(ui, PAGE);
}

/// A lecture page's section: its B&B videos and their UWorld QIDs.
#[component]
pub fn LectureSection(lecture: String) -> Element {
    let ui = use_context::<Ui>();
    let mut copied = use_signal(|| None::<String>);
    let Some(lec) = data().lecture(&lecture) else {
        return rsx! {};
    };
    let done = DONE_SET();
    let left: Vec<&Set> =
        lec.sets.iter().filter(|s| !s.qids.is_empty() && !done.contains(&s.video)).collect();
    let left_n: usize = left.iter().map(|s| s.qids.len()).sum();
    let all_left = qid_list(left.iter().copied());
    rsx! {
        style { {CSS.text()} }
        section { class: "lec-section qb", aria_labelledby: "qb-title",
            h2 { id: "qb-title",
                {tx::TITLE}
                span { class: "muted small", " · {tx::summary(lec.sets.len(), lec.count())}" }
            }
            p { class: "muted small qb-how", {tx::HOW} }
            if left_n > 0 {
                button {
                    class: "btn btn-tonal qb-all",
                    onclick: move |_| {
                        copy(&all_left);
                        copied.set(Some("*".into()));
                    },
                    if copied().as_deref() == Some("*") { {tx::COPIED} } else { {tx::copy_left(left_n)} }
                }
            }
            ul { class: "qb-sets",
                for set in lec.sets.iter() {
                    SetRow { key: "{set.video}", set: set.clone(), done: done.contains(&set.video), copied }
                }
            }
            button { class: "btn-link qb-find", onclick: move |_| open(ui), {tx::FIND} }
        }
    }
}

#[component]
fn SetRow(set: Set, done: bool, copied: Signal<Option<String>>) -> Element {
    let n = set.qids.len();
    let qids = qid_list([&set]);
    let video = set.video.clone();
    let key = set.video.clone();
    rsx! {
        li { class: "qb-set", "data-done": done,
            label { class: "qb-done",
                input {
                    r#type: "checkbox",
                    checked: done,
                    aria_label: tx::DONE,
                    onchange: move |e| set_done(&video, e.checked()),
                }
                span { class: "qb-video",
                    bdi { lang: "en", dir: "ltr", {video_name(&set)} }
                }
            }
            if n > 0 {
                button {
                    class: "chip tap-chip qb-copy",
                    title: "{qids}",
                    onclick: move |_| {
                        copy(&qids);
                        copied.set(Some(key.clone()));
                    },
                    if copied().as_deref() == Some(set.video.as_str()) { {tx::COPIED} } else { {tx::copy(n)} }
                }
            } else {
                span { class: "muted small qb-none", {tx::NONE} }
            }
        }
    }
}

/// Where a typed QID is filed: its lecture and video.
pub fn lookup(text: &str) -> Option<(&'static str, &'static Set)> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    data().find(digits.parse().ok()?)
}

/// More › UWorld: from a QID he missed back to its lecture.
#[component]
pub fn LookupPage() -> Element {
    let ui = use_context::<Ui>();
    let mut text = use_signal(String::new);
    let lib = Library::get();
    let typed = !text.read().trim().is_empty();
    let found = lookup(&text.read());
    let retired = text.read().trim().parse::<u32>().is_ok_and(|q| data().retired.contains(&q));
    let total: usize = data().lectures.iter().map(LectureSets::count).sum();
    rsx! {
        style { {crate::lecture::CSS.text()} }
        style { {CSS.text()} }
        div { class: "lec-page qb-page",
            div { class: "qb-head",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: crate::i18n::lecture::BACK,
                    title: crate::i18n::lecture::BACK,
                    onclick: move |_| crate::lecture::close(),
                    span { aria_hidden: "true", "→" }
                }
                header { class: "page-head",
                    h1 { {tx::PAGE_TITLE} }
                    p { class: "page-sub", {tx::page_sub(total)} }
                }
            }
            label { class: "search",
                {crate::icons::search()}
                input {
                    r#type: "search",
                    inputmode: "numeric",
                    dir: "ltr",
                    aria_label: tx::FIND,
                    placeholder: tx::PLACEHOLDER,
                    value: "{text}",
                    oninput: move |e| text.set(e.value()),
                }
            }
            if let Some((lecture, set)) = found {
                div { class: "qb-found",
                    p {
                        {tx::FOUND}
                        " "
                        bdi { lang: "en", dir: "ltr", {lib.crumb(lecture)} }
                    }
                    p { class: "muted small",
                        {tx::FROM_VIDEO}
                        " "
                        bdi { lang: "en", dir: "ltr", {video_name(set)} }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            crate::lecture::close();
                            crate::lecture::open(ui, lecture);
                        },
                        {tx::OPEN_LECTURE}
                    }
                }
            } else if retired {
                p { class: "note", {tx::RETIRED} }
            } else if typed {
                p { class: "note", {tx::NOT_FOUND} }
            }
            p { class: "qb-note muted small", {tx::SOURCE} }
        }
    }
}

#[cfg(test)]
mod tests;
