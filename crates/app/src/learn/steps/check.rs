//! Lesson check interaction and response restoration.

use super::*;

/// Single best answer: a tap picks (and can change), the footer's "صحّح"
/// checks, then every option explains itself.
#[component]
pub(super) fn Check(
    question: String,
    options: Vec<String>,
    answer: usize,
    why: Vec<String>,
    seed: String,
    on_answer: EventHandler<bool>,
) -> Element {
    let order = use_hook(|| shuffled(options.len(), &seed));
    let (saved, put) = use_draft();
    let (pick0, checked0) = match saved {
        Draft::Check { pick, checked } => (pick, checked && pick.is_some()),
        _ => (None, false),
    };
    let mut picked = use_signal(|| pick0);
    let mut checked = use_signal(|| checked0);
    use_check_bar(
        move || picked().is_some() && !checked(),
        move |_| {
            let Some(p) = *picked.peek() else { return };
            checked.set(true);
            put.call(Draft::Check { pick: Some(p), checked: true });
            on_answer.call(p == answer);
            cue(p == answer);
        },
    );
    let ltr = options.iter().all(|o| is_latin(o));
    rsx! {
        div { class: "ls-check",
            p { class: "ls-kicker", {tx::CHECK} }
            div { class: "ls-q", dir: if is_latin(&question) { "ltr" } else { "rtl" }, Rich { text: question.clone() } }
            div { class: "ls-options", dir: if ltr { "ltr" } else { "rtl" }, role: if checked() { "list" } else { "radiogroup" },
                for (pos , &i) in order.iter().enumerate() {
                    {
                        let (state, mark) = answer_state(checked(), picked(), i, answer);
                        rsx! {
                            div { key: "{i}", class: "ls-option-wrap",
                                button {
                                    class: "answer",
                                    role: if !checked() { "radio" } else { "listitem" },
                                    "aria-checked": if !checked() { if picked() == Some(i) { "true" } else { "false" } } else { "" },
                                    "data-state": state,
                                    "aria-keyshortcuts": LETTERS.get(pos).copied().unwrap_or(""),
                                    disabled: checked(),
                                    onclick: move |_| {
                                        if !checked() {
                                            picked.set(Some(i));
                                            put.call(Draft::Check { pick: Some(i), checked: false });
                                        }
                                    },
                                    span { class: "answer-key", "{LETTERS.get(pos).copied().unwrap_or(\"\")}" }
                                    span { class: "answer-text", Line { text: options[i].clone() } }
                                    if let Some(m) = mark {
                                        span { class: "answer-mark", dir: "rtl", lang: "ar", {m} }
                                    }
                                }
                                if checked() && (i == answer || picked() == Some(i)) {
                                    p { class: "ls-why small ls-pop", Line { text: why.get(i).cloned().unwrap_or_default() } }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(p) = picked().filter(|_| checked()) {
                p { class: if p == answer { "ls-verdict good" } else { "ls-verdict bad" }, role: "status",
                    {if p == answer { tx::RIGHT } else { tx::NOT_QUITE }}
                }
            }
        }
    }
}
