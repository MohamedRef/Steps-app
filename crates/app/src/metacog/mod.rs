//! Two optional taps around an answer, shared by the study player and the
//! question bank: how sure Mohamed was before the answer came out
//! ([`ConfidencePicker`]), and why he missed it ([`ReasonPicker`]). Neither
//! ever blocks moving on; both go on the review event
//! (`steps_core::review::{Confidence, ErrorReason}`) and feed the mistakes log.

use dioxus::prelude::*;
use steps_core::review::{Confidence, ErrorReason};

use crate::i18n::metacog as tx;
use crate::look::{Css, css};

pub const METACOG_CSS: Css = css!("src/metacog/metacog.css");

pub fn confidence_label(c: Confidence) -> &'static str {
    match c {
        Confidence::Sure => tx::SURE,
        Confidence::Unsure => tx::UNSURE,
        Confidence::Guess => tx::GUESS,
    }
}

pub fn reason_label(r: ErrorReason) -> &'static str {
    match r {
        ErrorReason::Knowledge => tx::KNOWLEDGE,
        ErrorReason::Misread => tx::MISREAD,
        ErrorReason::SecondGuessed => tx::SECOND_GUESSED,
        ErrorReason::Reasoning => tx::REASONING,
        ErrorReason::Timing => tx::TIMING,
    }
}

/// A second tap on the chosen chip takes the choice back.
pub fn toggled<T: PartialEq>(current: Option<T>, picked: T) -> Option<T> {
    if current.as_ref() == Some(&picked) { None } else { Some(picked) }
}

/// "متأكد قد إيه؟ (اختياري)" and three chips. `on_pick` gets the new value:
/// `None` when he taps the chosen one again.
#[component]
pub fn ConfidencePicker(value: Option<Confidence>, on_pick: EventHandler<Option<Confidence>>) -> Element {
    rsx! {
        style { {METACOG_CSS.text()} }
        div { class: "mc mc-confidence", role: "group", aria_labelledby: "mc-confidence-label",
            p { class: "mc-head small",
                strong { id: "mc-confidence-label", {tx::CONFIDENCE_LABEL} }
                span { class: "muted", {tx::OPTIONAL} }
            }
            div { class: "mc-chips",
                for c in Confidence::ALL {
                    button {
                        key: "{c.as_str()}",
                        class: "mc-chip",
                        "data-confidence": c.as_str(),
                        aria_pressed: if value == Some(c) { "true" } else { "false" },
                        onclick: move |_| on_pick.call(toggled(value, c)),
                        {confidence_label(c)}
                    }
                }
            }
        }
    }
}

/// "غلطت ليه؟" and one chip per reason. `on_pick` gets the reason tapped.
#[component]
pub fn ReasonPicker(
    value: Option<ErrorReason>,
    on_pick: EventHandler<ErrorReason>,
    #[props(default = tx::REASON_LABEL)] label: &'static str,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        style { {METACOG_CSS.text()} }
        div { class: "mc mc-reason", role: "group", aria_labelledby: "mc-reason-label",
            p { class: "mc-head small",
                strong { id: "mc-reason-label", {label} }
                span { class: "muted", {tx::OPTIONAL} }
            }
            div { class: "mc-chips",
                for r in ErrorReason::ALL {
                    button {
                        key: "{r.as_str()}",
                        class: "mc-chip",
                        "data-reason": r.as_str(),
                        disabled,
                        aria_pressed: if value == Some(r) { "true" } else { "false" },
                        onclick: move |_| on_pick.call(r),
                        {reason_label(r)}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_tap_takes_the_choice_back() {
        assert_eq!(toggled(None, Confidence::Sure), Some(Confidence::Sure));
        assert_eq!(toggled(Some(Confidence::Sure), Confidence::Sure), None);
        assert_eq!(toggled(Some(Confidence::Sure), Confidence::Guess), Some(Confidence::Guess));
    }

    #[test]
    fn every_choice_has_its_own_words() {
        let mut labels: Vec<&str> = Confidence::ALL.into_iter().map(confidence_label).collect();
        labels.extend(ErrorReason::ALL.into_iter().map(reason_label));
        let n = labels.len();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), n);
    }
}
