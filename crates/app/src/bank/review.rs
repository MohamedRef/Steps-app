//! A finished paper's review: the paper's own screen (top bar, navigator,
//! question), every question revealed with his pick, his struck options and
//! highlights, the verdict and the explanation. It goes through the
//! questions the result asked for (the wrong ones, the marked ones, …),
//! counted among themselves; the navigator still jumps to any question,
//! the others drawn faint.

use dioxus::prelude::*;
use steps_core::exam::{Answer, Graded, Question};

use super::bar::{ExamBar, Move};
use super::clock::Clock;
use super::navigator::{Cell, CellState, Legend, NavColumn, NavSheet};
use super::question::QuestionView;
use super::{PHASE, Phase, lookup, toggle_review_mark};
use crate::i18n::bank as tx;

/// Which questions a review goes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Show {
    All,
    Right,
    /// Answered wrong.
    Wrong,
    Blank,
    Marked,
    /// Not right: wrong or blank («راجع الأسئلة الغلط»).
    Missed,
}

impl Show {
    pub fn takes(self, q: &Question, a: &Answer) -> bool {
        let graded = a.graded(q);
        match self {
            Show::All => true,
            Show::Right => graded == Graded::Right,
            Show::Wrong => graded == Graded::Wrong,
            Show::Blank => graded == Graded::Blank,
            Show::Marked => a.flagged,
            Show::Missed => graded != Graded::Right,
        }
    }

    /// What the review goes through, in a word or two. «راجع الأسئلة
    /// الغلط» takes the blanks too, and then says so (UX-19).
    fn label(self, wrong: bool, blank: bool) -> &'static str {
        match self {
            Show::All => tx::SHOW_ALL,
            Show::Right => tx::SHOW_RIGHT,
            Show::Missed if blank && !wrong => tx::SHOW_BLANK,
            Show::Missed if blank => tx::SHOW_MISSED,
            Show::Wrong | Show::Missed => tx::SHOW_WRONG,
            Show::Blank => tx::SHOW_BLANK,
            Show::Marked => tx::SHOW_MARKED,
        }
    }
}

/// The result's way into the missed ones, named by what they are.
pub fn review_label(wrong: u32, blank: u32) -> &'static str {
    match (wrong > 0, blank > 0) {
        (true, true) => tx::REVIEW_MISSED,
        (false, true) => tx::REVIEW_BLANK,
        _ => tx::REVIEW_WRONG,
    }
}

#[component]
pub fn Review(at: usize, show: Show, ongo: EventHandler<usize>, onclose: EventHandler<()>) -> Element {
    // Like the paper, it gets the whole phone screen; «رجوع للنتيجة» is the way out.
    crate::parts::use_focus_mode(true);
    super::stem::use_highlights();
    use_drop(super::run::unwatch);
    let mut nav_open = use_signal(|| false);
    let Phase::Done { paper, .. } = PHASE.read().clone() else {
        return rsx! {};
    };
    let n = paper.questions.len();
    let at = at.min(n.saturating_sub(1));
    let (Some(q), Some(a)) = (paper.questions.get(at).cloned(), paper.answers.get(at).cloned()) else {
        return rsx! {};
    };
    let taken: Vec<usize> = (0..n).filter(|&i| show.takes(&paper.questions[i], &paper.answers[i])).collect();
    let prev = taken.iter().rev().find(|&&i| i < at).copied();
    let next = taken.iter().find(|&&i| i > at).copied();
    // Counted inside what it goes through: «1 من 8», not «2 / 10» (UX-19).
    let walk = (taken.len() < n)
        .then(|| taken.iter().position(|&i| i == at).map(|k| (k + 1, taken.len())))
        .flatten();
    let graded = |g: Graded| taken.iter().any(|&i| paper.answers[i].graded(&paper.questions[i]) == g);
    let what = show.label(graded(Graded::Wrong), graded(Graded::Blank));
    let cells: Vec<Cell> = paper
        .questions
        .iter()
        .zip(&paper.answers)
        .enumerate()
        .map(|(i, (q, a))| Cell {
            state: match a.graded(q) {
                Graded::Right => CellState::Right,
                Graded::Wrong => CellState::Wrong,
                Graded::Blank => CellState::Blank,
            },
            marked: a.flagged,
            out: !taken.contains(&i),
        })
        .collect();
    let page = lookup(&q.id).and_then(|(_, item)| item.src.first_aid_page());
    rsx! {
        div { class: "bank-run", "data-mode": "review", "data-reveal": "true",
            div { class: "run-sticky",
                onmounted: move |_| super::run::watch(),
                ExamBar {
                    kind: format!("{} · {what}", tx::REVIEWING),
                    at,
                    n,
                    walk,
                    item_id: q.id.clone(),
                    page,
                    marked: a.flagged,
                    onmark: move |_| toggle_review_mark(at),
                    prev,
                    next,
                    ongo,
                    clock: Clock::On { took_ms: u64::from(a.took_ms), since_ms: None },
                    onnav: move |_| nav_open.set(true),
                    // Its way out is back to the result; it has no end.
                    onout: onclose,
                    out_label: tx::BACK_TO_RESULT.to_string(),
                    onend: None,
                }
            }
            div { class: "run-body",
                NavColumn { cells: cells.clone(), at, legend: Legend::Review, ongo }
                div { class: "run-main",
                    QuestionView { key: "{q.id}-{at}", q: q.clone(), a: a.clone(), chosen: a.chosen, reveal: true, at, onchoose: None }
                    div { class: "run-foot",
                        Move { to: prev, next: false, ongo }
                        div { class: "run-foot-main",
                            match next {
                                Some(i) => rsx! {
                                    button { class: "btn btn-primary btn-lg run-main-action", "aria-keyshortcuts": "Enter", onclick: move |_| ongo.call(i), {tx::NEXT} }
                                },
                                None => rsx! {
                                    button { class: "btn btn-primary btn-lg run-main-action", "aria-keyshortcuts": "Enter", onclick: move |_| onclose.call(()), {tx::BACK_TO_RESULT} }
                                },
                            }
                        }
                        Move { to: next, next: true, ongo }
                    }
                }
            }
            if nav_open() {
                NavSheet { cells, at, legend: Legend::Review, ongo, onclose: move |_| nav_open.set(false) }
            }
        }
    }
}
