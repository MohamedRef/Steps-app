//! The Again / Hard / Good / Easy row every item shows once its answer is
//! out ("افتكرتها قد إيه؟"), with the real FSRS interval under each grade.
//! Shared by all renderers, so the content check above it and the intervals
//! look the same for every kind.

use dioxus::prelude::*;
use steps_core::review::Grade;

use super::backend::{now_ms, use_store};
use crate::i18n::study as tx;

const GRADES: [Grade; 4] = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy];

/// How many rating rows have shown "قيّم اللي افتكرته…" on this device. It
/// is for the first few times only (UX-12): after that he knows it, and the
/// row keeps its height for the grades.
const HINT_KEY: &str = "study.grade_hint";
const HINT_TIMES: u32 = 5;

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
    // Counted once per item, and only when it is the line shown.
    let hint = use_hook(|| suggested.is_none() && take_hint());
    use_effect(move || {
        if let Some(super::item::Revealed(mut r)) = revealed {
            r.set(true);
        }
    });
    let filled = emphasised(suggested);
    rsx! {
        // Two separate judgements: whether the content is right (its own
        // box, only while the item is unchecked), then how well he
        // remembered it (the rating row, always in the same place).
        crate::verify::VerifyBar { item_id: item_id.clone() }
        p { class: "st-grade-head",
            strong { id: "st-grade-label", {tx::GRADE_LABEL} }
            if suggested.is_some() {
                span { class: "st-suggest muted small", {tx::SUGGESTED} }
            } else if hint {
                span { class: "st-suggest muted small", {tx::GRADE_HINT} }
            }
        }
        // Only the four grades stay in reach at the foot of a long item on
        // a phone or a tablet in portrait (study.css); the line above and
        // the reasons below scroll with the item.
        div { class: "st-grades", role: "group", aria_labelledby: "st-grade-label",
            div { class: "rating-row", "data-suggested": if suggested.is_some() { "yes" } else { "no" },
                for (i , grade) in GRADES.into_iter().enumerate() {
                    button {
                        key: "{i}",
                        class: "rating",
                        "aria-keyshortcuts": "{i + 1}",
                        "data-grade": grade_name(grade),
                        "data-suggested": if suggested == Some(grade) { "yes" } else { "no" },
                        // One filled grade: the suggested one, else Good (UX-24).
                        "data-emph": if filled == grade { "yes" } else { "no" },
                        disabled: pressed(),
                        onclick: move |_| {
                            if !pressed() {
                                done.set(Some(*super::SAVE_FAILS.peek()));
                                on_grade.call(grade);
                            }
                        },
                        span { {grade_label(grade)} }
                        span { class: "st-grade-key", aria_hidden: "true", "{i + 1}" }
                        if let Some(label) = suggestion_label(suggested, grade) {
                            span { class: "rating-suggested", "{label}" }
                        }
                        if let Some(days) = intervals.map(|v| v[i]) {
                            IntervalLabel { days }
                        }
                    }
                }
            }
        }
        // Why he forgot it: one optional line that opens the reasons, each
        // of which rates the item "تاني" (UX-12).
        if tagging {
            details { class: "st-why",
                summary { class: "tap-summary", {crate::i18n::metacog::REASON_LABEL_AGAIN} }
                // Its own head says the same: read to screen readers only.
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

/// Whether this rating row shows the "rate what you recalled" line, counted
/// as shown: the first [`HINT_TIMES`] rows on this device do.
fn take_hint() -> bool {
    let db = crate::store::shared();
    let seen = db.local_setting::<u32>(HINT_KEY).unwrap_or(0);
    if seen >= HINT_TIMES {
        return false;
    }
    if let Err(e) = db.put_local_setting(HINT_KEY, &(seen + 1)) {
        crate::diag::report!("counting the grade hint: {e}");
    }
    true
}

/// The grade drawn filled: the one a kind suggests, else Good. Never two.
fn emphasised(suggested: Option<Grade>) -> Grade {
    suggested.unwrap_or(Grade::Good)
}

/// Mark an optional recommendation in words, not just with a fill.
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

/// "5 ساعات" under a grade, read right to left: the number is its own
/// left-to-right island, the word follows it (UX-05).
#[component]
fn IntervalLabel(days: f32) -> Element {
    let (n, word) = interval_parts(days);
    rsx! {
        span { class: "rating-when",
            if let Some(n) = n {
                bdi { class: "num-ltr", "{n}" }
                " "
            }
            "{word}"
        }
    }
}

/// "10 دقايق", "5 ساعات", "يومين", "3 شهور": when this grade brings it back,
/// as the number (none for one or two) and its word.
fn interval_parts(days: f32) -> (Option<u64>, &'static str) {
    use crate::i18n::count::{DAY, HOUR, MINUTE, MONTH};
    let n = |x: f32| x.round().max(1.0) as u64;
    if days < 1.0 / 24.0 {
        counted(n(days * 24.0 * 60.0), &MINUTE)
    } else if days < 1.0 {
        counted(n(days * 24.0), &HOUR)
    } else if days < 60.0 {
        counted(n(days), &DAY)
    } else {
        counted(n(days / 30.0), &MONTH)
    }
}

/// `count::count` split into its number and its word.
fn counted(n: u64, noun: &crate::i18n::count::Noun) -> (Option<u64>, &'static str) {
    match n {
        1 => (None, noun.one),
        2 => (None, noun.two),
        _ if (3..=10).contains(&(n % 100)) => (Some(n), noun.few),
        _ => (Some(n), noun.many),
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

    #[test]
    fn one_grade_is_filled_the_suggested_one_or_good() {
        assert_eq!(emphasised(None), Grade::Good);
        for grade in GRADES {
            assert_eq!(emphasised(Some(grade)), grade);
        }
        let css = include_str!("../../assets/parts.css");
        assert!(
            css.contains(".rating[data-emph=\"yes\"] { background: var(--accent);"),
            "the fill follows data-emph"
        );
        assert!(
            !css.contains(".rating[data-grade=\"good\"] { background"),
            "Good is not filled beside a suggestion"
        );
        let study = include_str!("../../assets/study.css");
        assert!(study.contains(".st-grades .rating[data-emph=\"yes\"] .st-grade-key { color: inherit; border-color: currentColor; }"));
    }

    #[test]
    fn intervals_read_in_minutes_hours_days_and_months() {
        use crate::i18n::count::{DAY, HOUR, MINUTE, MONTH, count};
        let label = |days: f32| match interval_parts(days) {
            (Some(n), word) => format!("{n} {word}"),
            (None, word) => word.to_string(),
        };
        assert_eq!(label(10.0 / 1440.0), count(10, &MINUTE));
        assert_eq!(label(0.0), count(1, &MINUTE), "never zero");
        assert_eq!(label(5.0 / 24.0), count(5, &HOUR));
        assert_eq!(label(2.0), count(2, &DAY));
        assert_eq!(label(3.0), count(3, &DAY));
        assert_eq!(label(45.0), count(45, &DAY));
        assert_eq!(label(90.0), count(3, &MONTH));
        // The split agrees with count() for every number.
        for n in 0..=130u64 {
            let joined = match counted(n, &DAY) {
                (Some(k), word) => format!("{k} {word}"),
                (None, word) => word.to_string(),
            };
            assert_eq!(joined, count(n, &DAY), "{n}");
        }
        assert_eq!(GRADES.map(grade_name), ["again", "hard", "good", "easy"]);
    }

    #[test]
    fn the_number_under_a_grade_is_its_own_left_to_right_island() {
        fn app() -> Element {
            rsx! { IntervalLabel { days: 5.0 / 24.0 } }
        }
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(r#"<bdi class="num-ltr">5</bdi>"#), "{html}");
        assert!(html.contains("ساعات"), "{html}");
        // The row reads right to left: the label is never forced ltr.
        let css = include_str!("../../assets/parts.css");
        let rule = css.split(".rating-when {").nth(1).unwrap().split('}').next().unwrap();
        assert!(!rule.contains("direction: ltr"), "{rule}");
        assert!(rule.contains("unicode-bidi: isolate"), "{rule}");
    }

    #[test]
    fn the_rating_hint_shows_the_first_few_times_only() {
        let shown: Vec<bool> = (0..HINT_TIMES + 2).map(|_| take_hint()).collect();
        let mut want = vec![true; HINT_TIMES as usize];
        want.extend([false, false]);
        assert_eq!(shown, want);
    }
}
