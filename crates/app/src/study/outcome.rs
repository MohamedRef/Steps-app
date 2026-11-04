//! Study completion and due-time labels.

use super::*;

#[component]
pub(super) fn SummaryView(
    items: usize,
    right: usize,
    fresh: usize,
    shaky: Vec<(String, String)>,
    next: Option<String>,
    can_undo: bool,
    on_undo: EventHandler<()>,
    undo_failed: bool,
    on_again: EventHandler<()>,
) -> Element {
    rsx! {
        crate::ui::Card { class: "st-summary",
            // The last grade button is gone: land focus on the result.
            h1 {
                tabindex: "-1",
                onmounted: move |e: MountedEvent| async move {
                    let _ = e.set_focus(true).await;
                },
                {if items == 0 { tx::SUMMARY_EMPTY } else { tx::SUMMARY_TITLE }}
            }
            if items > 0 {
                dl { class: "st-stats",
                    div { dt { {tx::SUM_ITEMS} } dd { "{items}" } }
                    div { dt { {tx::SUM_RIGHT} } dd { "{right}" } }
                    div { dt { {tx::SUM_NEW} } dd { "{fresh}" } }
                }
                if shaky.is_empty() {
                    p { {tx::SUM_ALL_SOLID} }
                } else {
                    h2 { {tx::SUM_BACK_SOON} }
                    ul { class: "st-shaky", lang: "en", dir: "ltr",
                        for (id , text) in shaky {
                            li { key: "{id}", "{text}" }
                        }
                    }
                }
            }
            p { class: "st-next",
                match next {
                    Some(when) => rsx! { "{tx::NEXT_SESSION} " strong { "{when}" } },
                    None => rsx! { {tx::NEXT_TOMORROW} },
                }
            }
            if undo_failed {
                p { class: "note st-undo-note", role: "status", "data-failed": "yes", {tx::UNDO_FAILED} }
            }
            div { class: "st-actions",
                button { class: "btn btn-primary btn-lg", onclick: move |_| on_again.call(()), {tx::ANOTHER} }
                // He tapped one grade too many: bring the last item back.
                if can_undo {
                    button { class: "btn btn-secondary btn-lg", onclick: move |_| on_undo.call(()), {tx::UNDO_LAST} }
                }
            }
        }
    }
}
/// "بعد 20 دقيقة", "بعد ساعتين", "بكرة", "بعد 3 أيام". Days count study
/// days (from 4 am), the days due dates land on, not 24-hour spans (#245).
pub(super) fn when_label(now: i64, due: i64, zone: &steps_core::zone::Zone) -> String {
    let days = zone.study_day(due).0 - zone.study_day(now).0;
    let mins = ((due - now).max(0) as f64 / 60_000.0).round() as i64;
    match (days, mins) {
        (..=0, 0..=1) => tx::NOW.to_string(),
        (..=0, 2..=59) => format!("{} {}", tx::AFTER, count(mins, &MINUTE)),
        (..=0, _) => format!("{} {}", tx::AFTER, count((mins as f64 / 60.0).round() as i64, &HOUR)),
        (1, _) => tx::TOMORROW.to_string(),
        _ => format!("{} {}", tx::AFTER, count(days, &DAY)),
    }
}
