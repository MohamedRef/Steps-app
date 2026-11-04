//! The Again / Hard / Good / Easy row every item shows once its answer is
//! out ("افتكرتها قد إيه؟"), with the real FSRS interval under each grade.
//! Shared by all renderers, so the content check above it and the intervals
//! look the same for every kind.

use dioxus::prelude::*;
use steps_core::review::Grade;

use super::backend::{now_ms, use_store};
use crate::i18n::study as tx;

const GRADES: [Grade; 4] = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy];

#[component]
pub fn GradeRow(item_id: String, suggested: Option<Grade>, on_grade: EventHandler<Grade>) -> Element {
    let store = use_store();
    let intervals = store.0.borrow().intervals(&item_id, now_ms());
    // Pressed at this count of failed saves: a failed save re-enables the row.
    let mut done = use_signal(|| None::<u32>);
    let pressed = move || done() == Some(*super::SAVE_FAILS.read());
    // The answer is out: let the frame show the book quote.
    let revealed = try_use_context::<super::item::Revealed>();
    // In the study player: a reason chip rates it "تاني" and says why.
    let tagging = try_use_context::<super::item::Tagging>().is_some_and(|t| t.0);
    use_effect(move || {
        if let Some(super::item::Revealed(mut r)) = revealed {
            r.set(true);
        }
    });
    rsx! {
        // Two separate judgements: whether the content is right (its own
        // box, only while the item is unchecked), then how well he
        // remembered it (the rating row, always in the same place).
        crate::verify::VerifyBar { item_id: item_id.clone() }
        div { class: "st-grades", role: "group", aria_labelledby: "st-grade-label",
            p { class: "st-grade-head",
                strong { id: "st-grade-label", {tx::GRADE_LABEL} }
                span { class: "st-suggest muted small",
                    {if suggested.is_some() { tx::SUGGESTED } else { tx::GRADE_HINT }}
                }
            }
            div { class: "rating-row",
                for (i , grade) in GRADES.into_iter().enumerate() {
                    button {
                        key: "{i}",
                        class: "rating",
                        "aria-keyshortcuts": "{i + 1}",
                        "data-grade": grade_name(grade),
                        "data-suggested": if suggested == Some(grade) { "yes" } else { "no" },
                        disabled: pressed(),
                        onclick: move |_| {
                            if !pressed() {
                                done.set(Some(*super::SAVE_FAILS.peek()));
                                on_grade.call(grade);
                            }
                        },
                        span { {grade_label(grade)} }
                        if let Some(label) = suggestion_label(suggested, grade) {
                            span { class: "rating-suggested", "{label}" }
                        }
                        if let Some(days) = intervals.map(|v| v[i]) {
                            span { class: "rating-when", {interval_label(days)} }
                        }
                    }
                }
            }
            if tagging {
                crate::metacog::ReasonPicker {
                    value: None,
                    label: crate::i18n::metacog::REASON_LABEL_AGAIN,
                    disabled: pressed(),
                    on_pick: move |r| {
                        if !pressed() {
                            done.set(Some(*super::SAVE_FAILS.peek()));
                            super::TAGS.write().reason = Some(r);
                            on_grade.call(Grade::Again);
                        }
                    },
                }
            }
        }
    }
}

/// Mark an optional recommendation in words, not just with a CSS ring.
fn suggestion_label(suggested: Option<Grade>, grade: Grade) -> Option<&'static str> {
    (suggested == Some(grade)).then_some(tx::SUGGESTED_SHORT)
}

fn grade_name(g: Grade) -> &'static str {
    match g {
        Grade::Again => "again",
        Grade::Hard => "hard",
        Grade::Good => "good",
        Grade::Easy => "easy",
    }
}

pub fn grade_label(g: Grade) -> &'static str {
    match g {
        Grade::Again => tx::AGAIN,
        Grade::Hard => tx::HARD,
        Grade::Good => tx::GOOD,
        Grade::Easy => tx::EASY,
    }
}

/// "10 دقايق", "5 ساعات", "يومين", "3 شهور": when this grade brings it back.
fn interval_label(days: f32) -> String {
    use crate::i18n::count::{DAY, HOUR, MINUTE, MONTH, count};
    let n = |x: f32| x.round().max(1.0) as u64;
    if days < 1.0 / 24.0 {
        count(n(days * 24.0 * 60.0), &MINUTE)
    } else if days < 1.0 {
        count(n(days * 24.0), &HOUR)
    } else if days < 60.0 {
        count(n(days), &DAY)
    } else {
        count(n(days / 30.0), &MONTH)
    }
}

#[cfg(test)]
mod suggestion_tests {
    use super::*;

    #[test]
    fn only_the_recommended_grade_has_a_text_label() {
        for grade in GRADES {
            assert_eq!(
                suggestion_label(Some(Grade::Hard), grade),
                (grade == Grade::Hard).then_some(tx::SUGGESTED_SHORT),
            );
        }
        assert!(GRADES.into_iter().all(|grade| suggestion_label(None, grade).is_none()));
    }
}
