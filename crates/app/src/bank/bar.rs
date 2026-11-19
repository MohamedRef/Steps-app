//! The paper's top bar, as a Step 1 question bank's test interface has one,
//! in Steps' look: the way out, the paper's kind and «سؤال N من M», a VQ
//! question's number, the mark, the moves, the tools (lab values, a note),
//! the clock and «إنهاء البلوك». Under 840dp «N / M» opens the navigator;
//! under 600dp the bar is compact: «N / M», the number, the clock, the mark
//! and a tools menu, with the moves at the foot of the screen (`player.rs`).
//! Which parts show at which width is bank.css's job; the markup is the
//! same at every width.

use dioxus::prelude::*;
use steps_core::note::Anchor;

use super::clock::{Clock, ClockChip};
use crate::i18n::bank as tx;

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
    /// The ✕ at the bar's start: leave the paper (it waits, a timed one with
    /// its clock on), or close a review. Android Back takes it; Escape only
    /// closes the paper's sheets and dialogs, never the paper (UX-03).
    onout: EventHandler<()>,
    out_label: String,
    #[props(default)] out_hint: String,
    /// «إنهاء البلوك», on a running paper; a review has none.
    onend: Option<EventHandler<()>>,
) -> Element {
    let mut tools = use_signal(|| false);
    // A VQ question's number; a First Aid question has none to show.
    let label = crate::vq::name(&item_id);
    let hint = if out_hint.is_empty() { out_label.clone() } else { out_hint.clone() };
    rsx! {
        header { class: "exam-bar",
            button {
                class: "icon-button run-leave",
                // Android Back takes it while the app's bar is hidden. Esc
                // does not: one stray press must never leave a block.
                "data-back": "true",
                "aria-label": "{out_label}",
                title: "{hint}",
                onclick: move |_| onout.call(()),
                span { aria_hidden: "true", "✕" }
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
            // The tools, the clock, the end and ⋮ keep together: when the
            // bar wraps, they go to its second line as one.
            div { class: "exam-side",
                div { class: "exam-tools",
                    crate::labs::LabButton { class: "exam-tool" }
                    crate::notes::NoteChip { anchor: Anchor::Item(item_id.clone()), page, show_text: false }
                }
                ClockChip { clock }
                if let Some(end) = onend {
                    button { class: "btn btn-danger exam-end", onclick: move |_| end.call(()), {tx::END} }
                }
                // Where the tools don't fit in the bar, they are here.
                button {
                    class: "icon-button exam-more",
                    aria_haspopup: "dialog",
                    "aria-label": if onend.is_some() { tx::TOOLS } else { tx::TOOLS_REVIEW },
                    title: tx::TOOLS_TITLE,
                    onclick: move |_| tools.set(true),
                    {crate::icons::more()}
                }
            }
        }
        if tools() {
            ToolsSheet { item_id: item_id.clone(), page, end: onend, onclose: move |_| tools.set(false) }
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
                    autofocus: true,
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
                        aria_haspopup: "dialog",
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
                            if has_note { {crate::i18n::notes::CHIP_HAS} } else { {crate::i18n::notes::ADD_HINT} }
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
