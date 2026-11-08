//! Adapted from batna-app main: crates/app/src/learn/highlight.rs (#770).
//!
//! Mohamed's own highlights in a lesson (2026-11-08): select words in the
//! open step, pick a colour, a fill or an underline, and a pill or rectangle
//! (`highlight.js` draws them and runs the bar). Each highlight is one synced
//! setting, `hl.<lesson>.<id>` (`steps_core::highlight`), so every device
//! shows it and two devices never overwrite each other's.

use dioxus::prelude::*;
use serde_json::json;
use steps_core::highlight::{self as hl, Highlight, Saved};

use crate::i18n::learn as tx;

const HL_JS: &str = include_str!("highlight.js");

/// This lesson's highlights, oldest first by key.
pub(super) fn load(lesson: &str) -> Vec<Highlight> {
    crate::store::shared()
        .settings_with_prefix::<Saved>(&hl::prefix(lesson))
        .into_iter()
        .filter_map(|(_, saved)| match saved {
            Saved::Live(h) => Some(h),
            Saved::Gone { .. } => None,
        })
        .collect()
}

/// What a message from the script asks for.
#[derive(Debug, PartialEq)]
enum Message {
    Put(Highlight),
    Remove(String),
}

fn parse(msg: &str) -> Option<Message> {
    if let Some(id) = msg.strip_prefix("del:") {
        let ok = !id.is_empty()
            && id.len() <= 40
            && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        return ok.then(|| Message::Remove(id.to_owned()));
    }
    let body = msg.strip_prefix("add:").or_else(|| msg.strip_prefix("put:"))?;
    serde_json::from_str::<Highlight>(body).ok()?.cleaned().map(Message::Put)
}

fn apply(lesson: &str, msg: Message) {
    match msg {
        Message::Put(h) => {
            crate::store::save_setting(&hl::key(lesson, &h.id), &Saved::Live(h));
        }
        Message::Remove(id) => {
            crate::store::save_setting(&hl::key(lesson, &id), &Saved::Gone { gone: true });
        }
    }
}

fn init(lesson: &str) -> serde_json::Value {
    json!({
        "lesson": lesson,
        "list": load(lesson),
        "labels": {
            "bar": tx::HL_BAR,
            "colours": {
                "yellow": tx::HL_YELLOW,
                "green": tx::HL_GREEN,
                "blue": tx::HL_BLUE,
                "pink": tx::HL_PINK,
                "orange": tx::HL_ORANGE,
            },
            "fill": tx::HL_FILL,
            "underline": tx::HL_UNDERLINE,
            "pill": tx::HL_PILL,
            "rect": tx::HL_RECT,
            "remove": tx::HL_REMOVE,
        },
    })
}

/// Turns highlighting on for the open lesson, and off when it closes.
pub(super) fn use_highlights(lesson: &str) {
    let id = lesson.to_owned();
    use_hook(move || {
        spawn(async move {
            let script = format!(
                "{HL_JS}\nwindow.__stepsHl.start({}, (m) => dioxus.send(m));\nawait new Promise(() => {{}});",
                init(&id)
            );
            let mut eval = document::eval(&script);
            while let Ok(msg) = eval.recv::<String>().await {
                if let Some(msg) = parse(&msg) {
                    apply(&id, msg);
                }
            }
        });
    });
    use_drop(|| {
        document::eval("window.__stepsHl && window.__stepsHl.stop();");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use steps_core::highlight::{Colour, Shape, Style};

    fn sample(id: &str) -> Highlight {
        Highlight {
            id: id.into(),
            step: 1,
            quote: "delta wave".into(),
            prefix: "ECG: ".into(),
            suffix: "، short PR".into(),
            at: 40,
            colour: Colour::Pink,
            style: Style::Underline,
            shape: Shape::Rect,
        }
    }

    #[test]
    fn messages_parse_and_bad_ones_are_dropped() {
        let h = sample("k3x9");
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(parse(&format!("add:{json}")), Some(Message::Put(h.clone())));
        assert_eq!(parse(&format!("put:{json}")), Some(Message::Put(h)));
        assert_eq!(parse("del:k3x9"), Some(Message::Remove("k3x9".into())));
        assert_eq!(parse("del:../etc"), None);
        assert_eq!(parse("add:{not json"), None);
        assert_eq!(parse(r#"add:{"id":"a","step":0,"quote":"  "}"#), None);
        assert_eq!(parse("hello"), None);
    }

    #[test]
    fn saved_highlights_load_and_removed_ones_stay_gone() {
        let store = crate::store::shared();
        let lesson = "cvs.L01.lesson2";
        store.put_setting(&hl::key(lesson, "a"), &Saved::Live(sample("a"))).unwrap();
        store.put_setting(&hl::key(lesson, "b"), &Saved::Live(sample("b"))).unwrap();
        store.put_setting(&hl::key("cvs.L01.lesson3", "c"), &Saved::Live(sample("c"))).unwrap();
        store.put_setting(&hl::key(lesson, "b"), &Saved::Gone { gone: true }).unwrap();
        let ids: Vec<String> = load(lesson).into_iter().map(|h| h.id).collect();
        assert_eq!(ids, ["a"]);
        let init = init(lesson);
        assert_eq!(init["list"][0]["colour"], "pink");
        assert_eq!(init["labels"]["underline"], tx::HL_UNDERLINE);
    }
}
