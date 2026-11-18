//! The paper's top bar, as a Step 1 question bank's test interface has one,
//! in Steps' look: the paper's kind and «سؤال N من M», a VQ question's
//! number, the mark, the moves, the tools (lab values, a note), the clock
//! and «إنهاء البلوك». Under 840dp «N / M» opens the navigator; under 600dp
//! the bar is compact: «N / M», the number, the clock, the mark and a tools
//! menu, with the moves at the foot of the screen (`player.rs`).

use dioxus::prelude::*;
use steps_core::note::Anchor;

use super::clock::{Clock, ClockChip};
use crate::i18n::bank as tx;

/// How the bar ends: a running paper's «إنهاء البلوك», or a review's way
/// back to the result (its way out, so Back takes it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Paper,
    Review,
}

#[component]
pub fn ExamBar(
    kind: String,
    at: usize,
    n: usize,
    item_id: String,
    /// The question's First Aid page, for the note.
    page: Option<u32>,
    marked: bool,
    onmark: EventHandler<()>,
    prev: Option<usize>,
    next: Option<usize>,
    ongo: EventHandler<usize>,
    clock: Clock,
    onnav: EventHandler<()>,
    end: End,
    onend: EventHandler<()>,
    /// The running paper's ✕: leave it (a timed one waits, its clock on).
    onleave: Option<EventHandler<()>>,
    #[props(default)] leave_hint: String,
) -> Element {
    let mut tools = use_signal(|| false);
    // A VQ question's number; a First Aid question has none to show.
    let label = crate::vq::name(&item_id);
    let end_label = match end {
        End::Paper => tx::END,
        End::Review => tx::BACK_TO_RESULT,
    };
    let review = end == End::Review;
    rsx! {
        header { class: "exam-bar",
            if let Some(leave) = onleave {
                button {
                    class: "icon-button run-leave",
                    // Android Back takes it while the app's bar is hidden.
                    "data-back": "true",
                    // Esc leaves too, as in study, lessons and cases (UX #296).
                    "aria-keyshortcuts": "Escape",
                    "aria-label": tx::LEAVE,
                    title: if leave_hint.is_empty() { tx::LEAVE.to_string() } else { leave_hint.clone() },
                    onclick: move |_| leave.call(()),
                    span { aria_hidden: "true", "✕" }
                }
            }
            div { class: "exam-where",
                span { class: "chip exam-kind", "data-tone": "accent", "{kind}" }
                // Under 840dp the count opens the navigator.
                button {
                    class: "btn btn-secondary exam-count",
                    aria_haspopup: "dialog",
                    "aria-label": "{tx::NAV_OPEN}: {tx::question_of(at + 1, n)}",
                    title: tx::NAV_OPEN,
                    onclick: move |_| onnav.call(()),
                    span { class: "exam-count-icon", aria_hidden: "true", {crate::icons::grid()} }
                    bdi { class: "num-ltr", "{at + 1} / {n}" }
                }
                strong { class: "exam-of", {tx::question_of(at + 1, n)} }
                if let Some(label) = label {
                    bdi { class: "exam-qid", dir: "ltr", lang: "en", "{label}" }
                }
            }
            button {
                class: "btn exam-mark",
                "aria-pressed": if marked { "true" } else { "false" },
                "aria-keyshortcuts": "F",
                "aria-label": if marked { tx::MARKED } else { tx::MARK },
                onclick: move |_| onmark.call(()),
                span { class: "exam-mark-icon", aria_hidden: "true", {crate::icons::flag()} }
                span { class: "exam-mark-word", aria_hidden: "true", if marked { {tx::MARKED} } else { {tx::MARK} } }
            }
            div { class: "exam-moves",
                // The app reads right to left: → goes back, ← goes on.
                Move { to: prev, next: false, ongo }
                Move { to: next, next: true, ongo }
            }
            span { class: "exam-grow" }
            div { class: "exam-tools",
                crate::labs::LabButton { class: "exam-tool" }
                crate::notes::NoteChip { anchor: Anchor::Item(item_id.clone()), page, show_text: false }
            }
            ClockChip { clock }
            button {
                class: if review { "btn btn-secondary exam-end" } else { "btn btn-danger exam-end" },
                "data-exit": if review { "true" } else { "false" },
                "data-back": if review { "true" } else { "false" },
                "aria-keyshortcuts": if review { "Escape" } else { "" },
                onclick: move |_| onend.call(()),
                {end_label}
            }
            button {
                class: "icon-button exam-more",
                aria_haspopup: "dialog",
                "aria-label": tx::TOOLS,
                title: tx::TOOLS_TITLE,
                onclick: move |_| tools.set(true),
                {crate::icons::more()}
            }
        }
        if tools() {
            ToolsSheet {
                item_id: item_id.clone(),
                page,
                end: if review { None } else { Some(onend) },
                onclose: move |_| tools.set(false),
            }
        }
    }
}

/// The previous or next question; disabled at either end.
#[component]
pub fn Move(to: Option<usize>, next: bool, ongo: EventHandler<usize>) -> Element {
    rsx! {
        button {
            class: "icon-button exam-move",
            disabled: to.is_none(),
            "aria-label": if next { tx::NEXT } else { tx::PREV },
            title: if next { tx::NEXT } else { tx::PREV },
            "aria-keyshortcuts": if next { "ArrowLeft" } else { "ArrowRight" },
            onclick: move |_| {
                if let Some(to) = to {
                    ongo.call(to);
                }
            },
            {crate::icons::chevron(next)}
        }
    }
}

/// The compact bar's tools menu: lab values, a note, and the end of the
/// paper. Each closes the menu as it opens what it names.
#[component]
fn ToolsSheet(
    item_id: String,
    page: Option<u32>,
    end: Option<EventHandler<()>>,
    onclose: EventHandler<()>,
) -> Element {
    let review = crate::store::use_review();
    let anchor = Anchor::Item(item_id.clone());
    let has_note = review.backend.borrow().note(&anchor).is_some();
    rsx! {
        div { class: "sheet-backdrop", onclick: move |_| onclose.call(()) }
        div {
            class: "sheet exam-tools-sheet",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "exam-tools-title",
            div { class: "sheet-head",
                h2 { id: "exam-tools-title", {tx::TOOLS_TITLE} }
                button {
                    class: "btn btn-ghost bank-close",
                    "data-back": "true",
                    "aria-keyshortcuts": "Escape",
                    onclick: move |_| onclose.call(()),
                    {tx::CLOSE}
                }
            }
            ul { class: "tap-list",
                li {
                    button {
                        class: "tap tap-row",
                        onclick: move |_| {
                            onclose.call(());
                            crate::labs::open();
                        },
                        span { class: "row-icon", aria_hidden: "true", "⚗" }
                        span { class: "tap-row-main", {crate::i18n::labs::OPEN} }
                    }
                }
                li {
                    button {
                        class: "tap tap-row",
                        aria_haspopup: "dialog",
                        onclick: move |_| {
                            onclose.call(());
                            crate::notes::edit(anchor.clone(), page);
                        },
                        span { class: "row-icon", aria_hidden: "true", {crate::icons::notes()} }
                        span { class: "tap-row-main",
                            if has_note { {crate::i18n::notes::CHIP_HAS} } else { {crate::i18n::notes::CHIP_ADD} }
                        }
                    }
                }
                if let Some(end) = end {
                    li {
                        button {
                            class: "tap tap-row exam-tools-end",
                            onclick: move |_| {
                                onclose.call(());
                                end.call(());
                            },
                            span { class: "row-icon", aria_hidden: "true", "■" }
                            span { class: "tap-row-main", {tx::END} }
                        }
                    }
                }
            }
        }
    }
}
