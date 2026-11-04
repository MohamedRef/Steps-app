//! The mistakes log: everything answered wrong in the last 14 days and not
//! yet recovered, grouped by topic, with a one-tap session made from it.

use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::stats;

use super::{SessionButton, StudyData};
use crate::i18n::stats as tx;

#[component]
pub fn Mistakes(data: Rc<StudyData>) -> Element {
    let mut ui = use_context::<crate::Ui>();
    // A session on exactly these items (one mistake, or one topic's).
    let mut session = move |ids: Vec<String>| {
        *crate::study::START.write() =
            Some(crate::study::Start { lectures: None, items: Some(ids), minutes: 10 });
        ui.screen.set(crate::Screen::Study);
    };
    let groups = stats::mistakes(&data.lectures, &data.events, &data.marks, data.clock.now_ms);
    let ids = stats::mistakes_session(&groups);
    let find = |id: &str| data.lectures.iter().flat_map(|l| &l.items).find(|i| i.id == id);
    rsx! {
        crate::ui::Card { class: "",
            h2 { {tx::MISTAKES_TITLE} }
            if groups.is_empty() {
                p { {tx::MISTAKES_EMPTY} }
            } else {
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
                                span { class: "chip bad", {missed(m.misses)} }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
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
