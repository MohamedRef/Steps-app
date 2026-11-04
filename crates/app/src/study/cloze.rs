//! A cloze: type each blank (or reveal it), then see what matched. Numbers
//! are checked by `steps_core::session::check_typed`, which ignores units
//! and spacing.

use dioxus::prelude::*;
use steps_core::content::{Body, Item};
use steps_core::review::Grade;
use steps_core::session::{Match, check_typed, suggest_cloze};

use super::grade::GradeRow;
use crate::i18n::study as tx;

#[derive(Debug, Clone, PartialEq)]
enum Seg {
    Text(String),
    Blank { answer: String, hint: Option<String> },
}

/// Splits `{{c1::answer::hint}}` text into plain runs and blanks. Malformed
/// braces stay as text (the validator rejects them anyway).
fn segments(text: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{c") {
        let Some(len) = rest[start..].find("}}") else { break };
        let inner = &rest[start + 2..start + len];
        let mut parts = inner.splitn(3, "::");
        let (_, Some(answer)) = (parts.next(), parts.next()) else { break };
        if start > 0 {
            out.push(Seg::Text(rest[..start].to_string()));
        }
        out.push(Seg::Blank {
            answer: answer.trim().to_string(),
            hint: parts.next().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()),
        });
        rest = &rest[start + len + 2..];
    }
    if !rest.is_empty() {
        out.push(Seg::Text(rest.to_string()));
    }
    out
}

#[component]
pub fn ClozeItem(item: Item, on_grade: EventHandler<Grade>) -> Element {
    let Body::Cloze { text, extra } = &item.body else { return rsx! {} };
    let segs = segments(text);
    let answers: Vec<String> = segs
        .iter()
        .filter_map(|s| if let Seg::Blank { answer, .. } = s { Some(answer.clone()) } else { None })
        .collect();
    let any_numeric = answers.iter().any(|a| a.chars().any(|c| c.is_ascii_digit()));
    let mut typed = use_signal(|| vec![String::new(); answers.len()]);
    // None while answering; Some(results) once checked or revealed.
    let mut results = use_signal(|| None::<Vec<Match>>);

    let check = {
        let answers = answers.clone();
        move |reveal: bool| {
            let got = typed.read().clone();
            let r = answers
                .iter()
                .zip(&got)
                .map(|(a, t)| if reveal && t.trim().is_empty() { Match::Wrong } else { check_typed(a, t) })
                .collect();
            results.set(Some(r));
        }
    };
    let mut check_typed_now = check.clone();
    let mut reveal_now = check;

    let done = results.read().clone();
    let mut blank = 0usize;
    rsx! {
        p { class: "st-prompt st-cloze", lang: "en", dir: "ltr",
            for (i , seg) in segs.into_iter().enumerate() {
                match seg {
                    Seg::Text(t) => rsx! { span { key: "{i}", "{t}" } },
                    Seg::Blank { answer, hint } => {
                        let b = blank;
                        blank += 1;
                        let numeric = answer.chars().any(|c| c.is_ascii_digit());
                        match &done {
                            None => rsx! {
                                input {
                                    key: "{i}",
                                    class: "st-blank",
                                    r#type: "text",
                                    inputmode: if numeric { "decimal" } else { "text" },
                                    autocomplete: "off",
                                    autocapitalize: "off",
                                    spellcheck: false,
                                    autofocus: b == 0,
                                    size: "{answer.chars().count().clamp(3, 18)}",
                                    placeholder: hint.clone().unwrap_or_else(|| "…".into()),
                                    aria_label: "{tx::BLANK} {b + 1}",
                                    value: "{typed.read()[b]}",
                                    oninput: move |e| typed.write()[b] = e.value(),
                                    onkeydown: {
                                        let mut go = check_typed_now.clone();
                                        move |e: KeyboardEvent| {
                                            if e.key() == Key::Enter {
                                                go(false);
                                            }
                                        }
                                    },
                                }
                            },
                            Some(r) => {
                                let m = r[b];
                                let got = typed.read()[b].trim().to_string();
                                rsx! {
                                    span { key: "{i}", class: "st-filled", "data-match": match_name(m),
                                        if m != Match::Right && !got.is_empty() {
                                            s { class: "st-typed", "{got}" }
                                            " "
                                        }
                                        strong { "{answer}" }
                                        span { class: "st-mark", aria_hidden: "true", {match_mark(m)} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        match done {
            None => rsx! {
                div { class: "st-actions",
                    button { class: "btn btn-primary btn-lg", "aria-keyshortcuts": "Enter", onclick: move |_| check_typed_now(false), {tx::CHECK} }
                    button { class: "btn btn-secondary btn-lg", "aria-keyshortcuts": "Space", onclick: move |_| reveal_now(true), {tx::REVEAL_BLANKS} }
                }
                if any_numeric {
                    p { class: "muted small", {tx::CLOZE_TIP} }
                }
            },
            Some(r) => rsx! {
                p { class: "st-verdict", "data-match": match_name(overall(&r)), {verdict(&r)} }
                if let Some(extra) = extra {
                    div { class: "st-answer", lang: "en", dir: "ltr", "{extra}" }
                }
                GradeRow { item_id: item.id.clone(), suggested: Some(suggest_cloze(&r)), on_grade }
            },
        }
    }
}

fn overall(r: &[Match]) -> Match {
    if r.contains(&Match::Wrong) {
        Match::Wrong
    } else if r.contains(&Match::Close) {
        Match::Close
    } else {
        Match::Right
    }
}

fn verdict(r: &[Match]) -> &'static str {
    match overall(r) {
        Match::Right => tx::CLOZE_RIGHT,
        Match::Close => tx::CLOZE_CLOSE,
        Match::Wrong => tx::CLOZE_WRONG,
    }
}

fn match_name(m: Match) -> &'static str {
    match m {
        Match::Right => "right",
        Match::Close => "close",
        Match::Wrong => "wrong",
    }
}

fn match_mark(m: Match) -> &'static str {
    match m {
        Match::Right => " ✓",
        Match::Close => " ≈",
        Match::Wrong => " ✗",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_keep_text_and_hints() {
        let s = segments("PMN {{c1::≥250::count}} in {{c2::SBP}}.");
        assert_eq!(
            s,
            [
                Seg::Text("PMN ".into()),
                Seg::Blank { answer: "≥250".into(), hint: Some("count".into()) },
                Seg::Text(" in ".into()),
                Seg::Blank { answer: "SBP".into(), hint: None },
                Seg::Text(".".into()),
            ]
        );
        assert_eq!(segments("plain"), [Seg::Text("plain".into())]);
    }
}
