//! Settings › Review: desired retention and new items a day. The export of
//! his content checks and the backup rows live in Settings › Your data.

use dioxus::prelude::*;
use steps_core::srs::{MIN_REVIEWS_TO_OPTIMIZE, NEW_PER_DAY_CHOICES, Retention, ReviewSettings};

use crate::i18n::study_settings as tx;
use crate::store::use_review;

#[component]
pub fn StudySection() -> Element {
    let review = use_review();
    let settings = review.backend.borrow().settings.clone();
    let reviews = review.backend.borrow().reviews_logged();
    let personal = settings.fsrs.as_ref().map(|p| p.reviews);
    // With an exam day the plan sets the new items, so only its number shows (#99).
    let plan_new = crate::plan::target_now(&*review.backend.borrow()).map(|t| t.new_goal());

    rsx! {
        style { {super::CSS.text()} }
        crate::ui::Card { class: "",
            h2 { {tx::RETENTION} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::RETENTION,
                for r in Retention::ALL {
                    button {
                        key: "{r.percent()}",
                        role: "radio",
                        "aria-checked": settings.retention == r,
                        onclick: {
                            let review = review.clone();
                            move |_| save_choice(&review, ReviewChoice::Retention(r))
                        },
                        "{r.percent()}%"
                        span { class: "seg-hint", {retention_hint(r)} }
                    }
                }
            }
            p { class: "muted small",
                match personal {
                    Some(n) => rsx! { {tx::personal(n)} },
                    None => rsx! { {tx::default_params(MIN_REVIEWS_TO_OPTIMIZE, reviews)} },
                }
            }
        }
        crate::ui::Card { class: "",
            h2 { {tx::NEW_PER_DAY} }
            if let Some(n) = plan_new {
                p { "{tx::NEW_FROM_PLAN_1} {n} {tx::NEW_FROM_PLAN_2}" }
            } else {
                div { class: "segmented", role: "radiogroup", aria_label: tx::NEW_PER_DAY,
                    for n in NEW_PER_DAY_CHOICES {
                        button {
                            key: "{n}",
                            role: "radio",
                            "aria-checked": settings.new_per_day == n,
                            onclick: {
                                let review = review.clone();
                                move |_| save_choice(&review, ReviewChoice::NewPerDay(n))
                            },
                            "{n}"
                        }
                    }
                }
            }
        }
    }
}

fn retention_hint(r: Retention) -> &'static str {
    match r {
        Retention::R85 => tx::R85,
        Retention::R90 => tx::R90,
        Retention::R95 => tx::R95,
    }
}

/// Commands change one field in current settings, including after sync or refit.
#[derive(Clone, Copy)]
enum ReviewChoice {
    Retention(Retention),
    NewPerDay(u32),
}

impl ReviewChoice {
    fn key(self) -> &'static str {
        match self {
            Self::Retention(_) => "review.retention",
            Self::NewPerDay(_) => "review.new_per_day",
        }
    }

    fn apply(self, settings: &mut ReviewSettings) {
        match self {
            Self::Retention(value) => settings.retention = value,
            Self::NewPerDay(value) => settings.new_per_day = value,
        }
    }
}

fn save_choice(review: &crate::store::Review, choice: ReviewChoice) {
    let backend = review.backend.clone();
    crate::persistence::save(choice.key(), move || {
        let mut backend = backend.borrow_mut();
        let mut settings = backend.settings.clone();
        choice.apply(&mut settings);
        if backend.set_settings(settings) {
            Ok(())
        } else {
            Err("Review preferences could not be saved".into())
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retry_changes_only_its_field_in_the_current_settings() {
        let mut current = ReviewSettings { new_per_day: 60, ..Default::default() };
        ReviewChoice::Retention(Retention::R95).apply(&mut current);
        assert_eq!(current.new_per_day, 60);
        ReviewChoice::NewPerDay(15).apply(&mut current);
        assert_eq!(current.retention, Retention::R95);
    }
}
