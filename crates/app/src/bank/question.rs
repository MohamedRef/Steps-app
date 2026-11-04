//! One MCQ on screen: stem, options, and (once revealed) the right answer,
//! why each option is right or wrong, the book page and any guideline update.

use dioxus::prelude::*;
use steps_core::content::Body;
use steps_core::exam::Question;
use steps_core::review::verify::Check;

use super::{lookup, topic_title};
use crate::i18n::bank as tx;
use crate::store::use_review;

#[component]
pub fn QuestionView(
    q: Question,
    chosen: Option<usize>,
    reveal: bool,
    /// Called with the original option index; `None` makes the options read-only.
    onchoose: Option<EventHandler<usize>>,
) -> Element {
    let Some((lecture, item)) = lookup(&q.id) else {
        return rsx! { p { class: "note", {tx::NOT_FOUND} } };
    };
    let Body::Mcq { stem, options, why, .. } = &item.body else {
        return rsx! { p { class: "note", {tx::NOT_FOUND} } };
    };
    let topic = item.topic.as_deref().map(|t| topic_title(&lecture.lecture, t));
    let ui = use_context::<crate::Ui>();
    let rows = q.order.iter().enumerate().filter_map(|(shown, &orig)| {
        let text = options.get(orig)?.clone();
        let reason = why.get(orig).cloned().unwrap_or_default();
        let letter = char::from(b'A' + shown as u8);
        let state = answer_state(reveal, orig == q.answer, chosen == Some(orig));
        Some((orig, letter, text, reason, state))
    });
    let rows: Vec<_> = rows.collect();
    let locked = onchoose.is_none();
    let quote = item.src.quote.clone();
    let primary = rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Article, class: "q",
            // Once the answer shows, the lecture chip opens the lecture's
            // page. While a paper runs it stays a label, so a stray tap never
            // leaves the paper.
            div { class: "q-meta",
                if reveal {
                    button {
                        class: "chip tap",
                        title: tx::OPEN_LECTURE,
                        onclick: { let id = lecture.lecture.clone(); move |_| crate::lecture::open(ui, &id) },
                        bdi { "{crate::content::Library::get().crumb(&lecture.lecture)}" }
                    }
                } else {
                    span { class: "chip", bdi { "{crate::content::Library::get().crumb(&lecture.lecture)}" } }
                }
                if let Some(t) = topic {
                    span { class: "chip", bdi { "{t}" } }
                }
            }
            p { class: "q-stem", lang: "en", dir: "ltr", "{stem}" }
            ol { class: "q-options", lang: "en", dir: "ltr",
                for (orig, letter, text, reason, state) in rows.clone() {
                    li { key: "{orig}",
                        button {
                            class: "q-option answer",
                            "data-state": state,
                            disabled: locked,
                            "aria-pressed": if chosen == Some(orig) { "true" } else { "false" },
                            "aria-keyshortcuts": if locked { "" } else { letter.to_string() },
                            "data-peek": if reveal { reason.clone() } else { String::new() },
                            onclick: move |_| {
                                if let Some(h) = onchoose {
                                    h.call(orig);
                                }
                            },
                            span { class: "answer-key", "{letter}" }
                            span { class: "q-text answer-text", "{text}" }
                            // The state in words, so it never rests on colour alone.
                            if reveal && state == "correct" {
                                span { class: "answer-mark", lang: "ar", dir: "rtl", "✓ {tx::RIGHT_PICKED}" }
                            }
                            if reveal && state == "missed" {
                                span { class: "answer-mark", lang: "ar", dir: "rtl", "✓ {tx::RIGHT_ANSWER}" }
                            }
                            if reveal && state == "wrong" {
                                span { class: "answer-mark", lang: "ar", dir: "rtl", "✗ {tx::YOUR_ANSWER}" }
                            }
                        }
                        // Under each option on phones; the side pane has them on wide windows.
                        if reveal && !reason.is_empty() {
                            p { class: "q-why", "{reason}" }
                        }
                    }
                }
            }
        }
    };
    if !reveal {
        return primary;
    }
    // The explanation: why for every option (wide windows), the book page,
    // the ✓ / ✎ / ✗ check and any guideline update.
    let secondary = rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Div, class: "q-explain",
            ol { class: "q-why-list", lang: "en", dir: "ltr",
                for (orig, letter, _, reason, state) in rows {
                    li { key: "{orig}", "data-state": if matches!(state, "correct" | "missed") { "right" } else { state },
                        span { class: "q-letter", "{letter}" }
                        if matches!(state, "correct" | "missed") {
                            span { class: "q-why-mark", lang: "ar", dir: "rtl", "✓ {tx::RIGHT}" }
                        } else if state == "wrong" {
                            span { class: "q-why-mark", lang: "ar", dir: "rtl", "✗ {tx::YOUR_ANSWER}" }
                        }
                        span { "{reason}" }
                    }
                }
            }
            div { class: "q-foot",
                crate::hover::BookPage { page: item.src.page, quote, of: item.id.clone() }
            }
            Report { item_id: q.id.clone() }
            if let Some(u) = &item.update {
                aside { class: "q-update",
                    strong { {tx::UPDATE} }
                    p { lang: "en", dir: "ltr", "{u.text}" }
                    p { class: "muted small", bdi { "{u.reference}" } " · " {tx::UPDATE_NOTE} }
                }
            }
        }
    };
    rsx! {
        crate::desk::Panes { primary, secondary: Some(secondary) }
    }
}

/// The shared `button.answer` state (docs/DESIGN.md): a draft is
/// "selected"; once revealed the right option is "correct" when he picked
/// it and "missed" when he didn't, and his wrong pick is "wrong".
fn answer_state(reveal: bool, right: bool, picked: bool) -> &'static str {
    match (reveal, right, picked) {
        (true, true, true) => "correct",
        (true, true, false) => "missed",
        (true, false, true) => "wrong",
        (false, _, true) => "selected",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::answer_state;

    #[test]
    fn answer_states_follow_the_shared_part() {
        assert_eq!(answer_state(false, true, true), "selected");
        assert_eq!(answer_state(false, false, false), "");
        assert_eq!(answer_state(true, true, true), "correct");
        assert_eq!(answer_state(true, true, false), "missed");
        assert_eq!(answer_state(true, false, true), "wrong");
        assert_eq!(answer_state(true, false, false), "");
    }
}

/// ✓ / ✎ / ✗ the first time he sees the question (the review store's verify
/// bar), and afterwards a "report a problem" link that reopens that check.
#[component]
fn Report(item_id: String) -> Element {
    let review = use_review();
    let mut reopened = use_signal(|| false);
    let check = review.backend.borrow().check(&item_id);
    if check == Check::Ask || reopened() {
        return rsx! { crate::verify::VerifyBar { item_id } };
    }
    if check == Check::Hidden {
        return rsx! {};
    }
    rsx! {
        button {
            class: "btn-link small report",
            onclick: move |_| {
                review.backend.borrow_mut().undo_judgement(&item_id);
                reopened.set(true);
            },
            {tx::REPORT}
        }
    }
}
