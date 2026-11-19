//! The paper's navigator: every question's number with its state (here,
//! answered, not yet, marked, and once its answer shows, right or wrong),
//! drawn with a colour and a symbol and spoken in words. From 840dp it is a
//! column beside the question; narrower, a sheet that «N / M» opens. A tap
//! jumps to that question.

use dioxus::prelude::*;

use crate::i18n::bank as tx;

/// What one question's cell shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    /// Not answered yet.
    Blank,
    /// Answered on a timed paper, its result not shown yet.
    Answered,
    Right,
    /// Answered wrong.
    Wrong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub state: CellState,
    pub marked: bool,
    /// Outside what a review goes through (the wrong ones, the marked
    /// ones, …): drawn faint, still a tap away (UX-19).
    pub out: bool,
}

/// "سؤال 3، متجاوب، متعلّم عليه، إنت هنا" for a navigator cell.
pub fn cell_label(i: usize, cell: Cell, here: bool) -> String {
    let mut parts = vec![format!("{} {}", tx::QUESTION, i + 1)];
    parts.push(
        match cell.state {
            CellState::Blank => tx::UNANSWERED,
            CellState::Answered => tx::ANSWERED,
            CellState::Right => tx::RIGHT,
            CellState::Wrong => tx::WRONG,
        }
        .to_string(),
    );
    if cell.marked {
        parts.push(tx::MARKED.to_string());
    }
    if cell.out {
        parts.push(tx::OUT_OF_REVIEW.to_string());
    }
    if here {
        parts.push(tx::NOW_HERE.to_string());
    }
    parts.join("، ")
}

fn state_name(s: CellState) -> &'static str {
    match s {
        CellState::Blank => "blank",
        CellState::Answered => "answered",
        CellState::Right => "right",
        CellState::Wrong => "wrong",
    }
}

/// Which states a navigator's legend names: a timed paper shows answered
/// ones, a tutor paper and a review right and wrong; a review's blanks were
/// never answered rather than not yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Legend {
    Timed,
    Tutor,
    Review,
}

/// The cells and their legend.
#[component]
pub fn Navigator(cells: Vec<Cell>, at: usize, legend: Legend, ongo: EventHandler<usize>) -> Element {
    let some_out = cells.iter().any(|c| c.out);
    rsx! {
        // A list of buttons (not buttons playing list items), so each cell
        // stays a button to a screen reader. The numbers run left to right,
        // as the question's own text does.
        ol { class: "navigator", dir: "ltr",
            for (i, cell) in cells.into_iter().enumerate() {
                li { key: "{i}",
                    button {
                        class: "nav-cell",
                        "data-state": state_name(cell.state),
                        "data-flag": if cell.marked { "yes" } else { "no" },
                        "data-out": if cell.out { "true" } else { "false" },
                        "aria-current": if i == at { "true" } else { "false" },
                        // The state is spoken too, not only drawn.
                        "aria-label": cell_label(i, cell, i == at),
                        onclick: move |_| ongo.call(i),
                        span { class: "nav-num", "{i + 1}" }
                        match cell.state {
                            CellState::Right => rsx! { span { class: "nav-mark", aria_hidden: "true", "✓" } },
                            CellState::Wrong => rsx! { span { class: "nav-mark", aria_hidden: "true", "✗" } },
                            _ => rsx! {},
                        }
                        if cell.marked {
                            span { class: "nav-flag", aria_hidden: "true", {crate::icons::flag()} }
                        }
                    }
                }
            }
        }
        ul { class: "nav-legend", aria_hidden: "true",
            if legend == Legend::Timed {
                li { span { class: "nav-swatch", "data-state": "answered" } {tx::LEGEND_ANSWERED} }
            } else {
                li { span { class: "nav-swatch", "data-state": "right", "✓" } {tx::LEGEND_RIGHT} }
                li { span { class: "nav-swatch", "data-state": "wrong", "✗" } {tx::LEGEND_WRONG} }
            }
            li {
                span { class: "nav-swatch", "data-state": "blank" }
                if legend == Legend::Review { {tx::TILE_BLANK} } else { {tx::LEGEND_BLANK} }
            }
            li { span { class: "nav-swatch nav-flag", {crate::icons::flag()} } {tx::MARKED} }
            if some_out {
                li { span { class: "nav-swatch", "data-out": "true" } {tx::LEGEND_OUT} }
            }
        }
    }
}

/// The navigator as a column beside the question (from 840dp; hidden
/// narrower, where [`NavSheet`] takes its place).
#[component]
pub fn NavColumn(cells: Vec<Cell>, at: usize, legend: Legend, ongo: EventHandler<usize>) -> Element {
    rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Aside, class: "run-nav", aria_label: tx::NAV_LABEL,
            h2 { class: "run-nav-title", {tx::NAV_TITLE} }
            Navigator { cells, at, legend, ongo }
        }
    }
}

/// The navigator under 840dp: a sheet over the paper; a tap on a number goes
/// there and closes it.
#[component]
pub fn NavSheet(
    cells: Vec<Cell>,
    at: usize,
    legend: Legend,
    ongo: EventHandler<usize>,
    onclose: EventHandler<()>,
) -> Element {
    // The question on screen takes the focus; closing gives it back to «N / M».
    super::run::use_sheet_focus(
        "nav",
        ".nav-sheet .nav-cell[aria-current=\"true\"]",
        ".bank-run .exam-count",
    );
    rsx! {
        div { class: "sheet-backdrop", onclick: move |_| onclose.call(()) }
        div {
            class: "sheet nav-sheet",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "nav-sheet-title",
            div { class: "sheet-head",
                h2 { id: "nav-sheet-title", {tx::NAV_TITLE} }
                button {
                    class: "btn btn-ghost bank-close",
                    "data-back": "true",
                    "aria-keyshortcuts": "Escape",
                    onclick: move |_| onclose.call(()),
                    {tx::CLOSE}
                }
            }
            Navigator {
                cells,
                at,
                legend,
                ongo: move |i| {
                    onclose.call(());
                    ongo.call(i);
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigator_cells_say_their_state() {
        let cell = |state, marked| Cell { state, marked, out: false };
        assert_eq!(cell_label(0, cell(CellState::Blank, false), false), "سؤال 1، من غير إجابة");
        assert_eq!(
            cell_label(2, cell(CellState::Answered, true), true),
            "سؤال 3، متجاوب، متعلّم عليه، إنت هنا"
        );
        assert_eq!(cell_label(4, cell(CellState::Right, false), false), "سؤال 5، صح");
        assert_eq!(cell_label(5, cell(CellState::Wrong, true), false), "سؤال 6، غلط، متعلّم عليه");
        let out = Cell { out: true, ..cell(CellState::Right, false) };
        assert_eq!(cell_label(0, out, false), "سؤال 1، صح، مش من اللي بتراجعه");
    }
}
