//! The mistakes log: everything answered wrong in the last 14 days and not
//! yet recovered, grouped by topic, with a one-tap session made from it.
//! Misses he was sure of come first and say so; when he gave reasons, a row
//! of chips counts them and narrows the log to one.

use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::review::ErrorReason;
use steps_core::stats;

use super::{SessionButton, StudyData};
use crate::i18n::metacog as mc;
use crate::i18n::stats as tx;
use crate::metacog::{METACOG_CSS, reason_label};

#[component]
pub fn Mistakes(data: Rc<StudyData>) -> Element {
    let mut ui = use_context::<crate::Ui>();
    // A session on exactly these items (one mistake, or one topic's).
    let mut session = move |ids: Vec<String>| {
        *crate::study::START.write() =
            Some(crate::study::Start { lectures: None, items: Some(ids), minutes: 10 });
        ui.screen.set(crate::Screen::Study);
    };
    let all = stats::mistakes(&data.lectures, &data.events, &data.marks, data.clock.now_ms);
    let (reasons, _) = stats::mistake_reasons(&all);
    let mut only = use_signal(|| None::<ErrorReason>);
    // A filter whose mistakes are all recovered falls back to the whole log.
    let shown = only().filter(|r| reasons.iter().any(|(x, _)| x == r));
    let groups = match shown {
        Some(r) => stats::mistakes_with_reason(&all, r),
        None => all.clone(),
    };
    let ids = stats::mistakes_session(&groups);
    let retest = stats::retest_due(&data.lectures, &data.events, &data.marks, data.clock);
    let find = |id: &str| data.lectures.iter().flat_map(|l| &l.items).find(|i| i.id == id);
    rsx! {
        style { {METACOG_CSS.text()} }
        // Misses he was sure of, 2 to 7 days on: one more try, so they stick.
        if !retest.is_empty() {
            RetestCard { ids: retest.clone(), data: data.clone() }
        }
        crate::ui::Card { class: "",
            h2 { {tx::MISTAKES_TITLE} }
            if groups.is_empty() {
                p { {tx::MISTAKES_EMPTY} }
            } else {
                // Only once he has given a reason: before that the row says nothing.
                if !reasons.is_empty() {
                    div { class: "mc mistake-reasons", role: "group", aria_labelledby: "mistake-reasons-label",
                        p { class: "mc-head small", strong { id: "mistake-reasons-label", {mc::BY_REASON} } }
                        div { class: "mc-chips",
                            button {
                                class: "mc-chip",
                                aria_pressed: if shown.is_none() { "true" } else { "false" },
                                onclick: move |_| only.set(None),
                                {mc::ALL_REASONS}
                                span { class: "mc-count num-ltr", "{all.iter().map(|g| g.items.len()).sum::<usize>()}" }
                            }
                            for (r , n) in reasons.iter().copied() {
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
                    }
                }
                SessionButton { label: tx::MISTAKES_START, ids }
            }
        }
        for g in groups.iter() {
            crate::ui::Card { class: "mistake-group", key: "{g.lecture}-{g.title}",
                h2 {
                    button {
                        class: "tap tap-plain mistake-topic",
                        title: tx::MISTAKES_TOPIC_HINT,
                        onclick: {
                            let ids: Vec<String> = g.items.iter().map(|m| m.item_id.clone()).collect();
                            move |_| session(ids.clone())
                        },
                        bdi { {crate::content::Library::get().crumb(&g.lecture)} }
                        span { class: "chev", aria_hidden: "true" }
                    }
                }
                ul { class: "mistakes tap-list",
                    for m in g.items.iter() {
                        li { key: "{m.item_id}",
                            button {
                                class: "tap tap-row",
                                title: tx::MISTAKE_HINT,
                                onclick: { let id = m.item_id.clone(); move |_| session(vec![id.clone()]) },
                                span { class: "mistake-text tap-row-main",
                                    bdi { dir: "ltr", {find(&m.item_id).map(preview).unwrap_or_default()} }
                                }
                                if m.sure {
                                    span { class: "chip mc-sure", title: mc::WAS_SURE_HINT, {mc::WAS_SURE} }
                                }
                                if let Some(r) = m.reason {
                                    span { class: "chip", "data-reason": r.as_str(), {reason_label(r)} }
                                }
                                span { class: "chip bad", {missed(m.misses)} }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }
        }
        // UWorld questions he missed, logged by ID (uworld.rs).
        super::uworld::UworldMistakes {}
    }
}

/// Misses he was sure of, back for one more try: cards and clozes as a study
/// session, MCQs as a tutor set in the bank.
#[component]
fn RetestCard(ids: Vec<String>, data: Rc<StudyData>) -> Element {
    let mut ui = use_context::<crate::Ui>();
    let pool: Vec<steps_core::exam::Mcq> = steps_core::exam::mcqs(&data.lectures)
        .into_iter()
        .filter(|q| ids.iter().any(|id| id == q.id))
        .collect();
    let questions = steps_core::exam::practice(&pool, pool.len(), data.clock.now_ms as u64);
    let n = questions.len();
    let cards = ids.iter().any(|id| crate::study::is_playable(id));
    rsx! {
        crate::ui::Card { class: "mistake-retest",
            h2 { {mc::RETEST_TITLE} }
            p { class: "muted small", {mc::RETEST_BODY} }
            if cards {
                SessionButton { label: mc::RETEST_START, ids }
            }
            if n > 0 {
                div { class: "session-start",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| {
                            crate::bank::start(steps_core::exam::Mode::Tutor, questions.clone());
                            ui.screen.set(crate::Screen::Bank);
                        },
                        {mc::RETEST_MCQS}
                    }
                    span { class: "muted small", {crate::i18n::count::count(n, &crate::i18n::count::QUESTION)} }
                }
            }
        }
    }
}

/// "غلطت مرة / مرتين / 3 مرات".
fn missed(n: u32) -> String {
    format!("{} {}", tx::MISSED, crate::i18n::count::count(n.max(1), &crate::i18n::count::TIME))
}

/// The item's question, short enough for one line or two.
pub fn preview(item: &Item) -> String {
    let text = match &item.body {
        Body::Card { front, .. } => front.clone(),
        Body::Cloze { text, .. } => hide_clozes(text),
        Body::Mcq { stem, .. } | Body::Case { stem, .. } => stem.clone(),
        Body::Enumerate { prompt, .. } | Body::Essay { prompt, .. } | Body::Order { prompt, .. } => {
            prompt.clone()
        }
        Body::Table { title, .. } => title.clone(),
    };
    let mut chars = text.chars();
    let short: String = chars.by_ref().take(140).collect();
    if chars.next().is_some() { format!("{short}…") } else { short }
}

/// `a {{c1::b}} c` → `a [...] c`, so the log doesn't give the answer away.
fn hide_clozes(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        match rest[start..].find("}}") {
            Some(end) => {
                out.push_str("[…]");
                rest = &rest[start + end + 2..];
            }
            None => {
                rest = &rest[start..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clozes_are_hidden() {
        assert_eq!(
            hide_clozes("Class A = {{c1::5-6}} points, C = {{c2::10-15}}."),
            "Class A = […] points, C = […]."
        );
        assert_eq!(hide_clozes("no blanks"), "no blanks");
    }
}
