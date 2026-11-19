//! Answering a paper, as on a Step 1 question bank's test screen: the top
//! bar (`bar.rs`), the navigator (`navigator.rs`), the question with its
//! struck options and highlights (`question.rs`), and at the foot the one
//! next step. Tutor mode checks each pick and shows the explanation; timed
//! papers and blocks run against the clock and show nothing until they go in.

use dioxus::prelude::*;
use steps_core::exam::{self, Mode};

use super::bar::{ExamBar, Move};
use super::clock::Clock;
use super::navigator::{Cell, CellState, Legend, NavColumn, NavSheet};
use super::question::{QuestionView, letters};
use super::{
    PHASE, Phase, check, choose, confirm, finish, go, leave, lookup, minutes, now_ms, set_confidence,
    set_reason, strike, toggle_mark,
};
use crate::i18n::bank as tx;
use crate::i18n::count::{MINUTE, count};

#[component]
pub fn Player() -> Element {
    // A running paper gets the whole phone screen; ✕ is the way out.
    crate::parts::use_focus_mode(true);
    super::stem::use_highlights();
    let mut confirming = use_signal(|| false);
    let mut nav_open = use_signal(|| false);
    let Phase::Running(p) = PHASE.read().clone() else {
        return rsx! {};
    };
    let n = p.questions.len();
    let at = p.at;
    let q = p.questions[at].clone();
    let a = p.answers[at].clone();
    let timed = p.mode.timed();
    // Tutor mode: select, then check, then the explanation.
    let checked = p.revealed(at);
    let selected = if timed { a.chosen } else { a.chosen.or(p.draft) };
    let answered = p.answers.iter().filter(|a| a.chosen.is_some()).count();
    let marked = p.answers.iter().filter(|a| a.flagged).count();
    let first_blank = p.answers.iter().position(|a| a.chosen.is_none());
    let left = minutes(exam::remaining_ms(n, p.started_ms, now_ms()));
    let last = at + 1 == n;
    let (right_letter, _) = letters(&q, a.chosen);
    let cells: Vec<Cell> = p
        .questions
        .iter()
        .zip(&p.answers)
        .map(|(q, a)| Cell {
            state: match (a.chosen, timed) {
                (None, _) => CellState::Blank,
                (Some(_), true) => CellState::Answered,
                (Some(_), false) if a.correct(q) => CellState::Right,
                (Some(_), false) => CellState::Wrong,
            },
            marked: a.flagged,
        })
        .collect();
    let clock = if timed {
        Clock::Left { n, started_ms: p.started_ms }
    } else {
        Clock::On { took_ms: u64::from(a.took_ms), since_ms: (!checked).then_some(p.shown_ms) }
    };
    let page = lookup(&q.id).and_then(|(_, item)| item.src.first_aid_page());
    let legend = if timed { Legend::Timed } else { Legend::Tutor };
    // Blanks left: he is asked once before the paper goes in.
    let mut end = move |_: ()| {
        if answered < n {
            confirming.set(true);
        } else {
            finish();
        }
    };
    let prev = at.checked_sub(1);
    let next = (!last).then_some(at + 1);

    rsx! {
        div {
            class: "bank-run",
            "data-mode": if timed { "timed" } else { "tutor" },
            // With the answer out, the explanation may take the navigator's
            // column (bank.css).
            "data-reveal": if checked { "true" } else { "false" },
            // The bar, and a timed paper's clock, stay in view while he
            // scrolls a long question (#234).
            div { class: "run-sticky",
                ExamBar {
                    kind: tx::kind(p.block, timed),
                    at,
                    n,
                    item_id: q.id.clone(),
                    page,
                    marked: a.flagged,
                    onmark: move |_| toggle_mark(),
                    prev,
                    next,
                    ongo: go,
                    clock,
                    onnav: move |_| nav_open.set(true),
                    onout: move |_| leave(),
                    out_label: tx::LEAVE.to_string(),
                    out_hint: (if timed { tx::LEAVE_TIMED_HINT } else { tx::LEAVE_TUTOR_HINT }).to_string(),
                    onend: Some(EventHandler::new(end)),
                }
                if timed {
                    Timer { key: "{p.started_ms}", n, started_ms: p.started_ms }
                }
            }
            // The Check button goes away once he checks, so the verdict
            // is read out here as well as written over the question.
            p { class: "bank-sr", role: "status", "aria-live": "polite",
                if checked { {tx::checked(a.correct(&q), right_letter)} }
            }
            div { class: "run-body",
                NavColumn { cells: cells.clone(), at, legend, ongo: go }
                div { class: "run-main",
                    QuestionView {
                        key: "{q.id}-{at}",
                        q: q.clone(),
                        a: a.clone(),
                        chosen: selected,
                        reveal: checked,
                        at,
                        onchoose: if checked { None } else { Some(EventHandler::new(choose)) },
                        onstrike: if checked { None } else { Some(EventHandler::new(strike)) },
                    }
                    // Optional taps (crate::metacog): how sure he is while the
                    // answer is still his, and why he missed it once it's out.
                    if !checked {
                        crate::metacog::ConfidencePicker {
                            value: a.confidence,
                            on_pick: set_confidence,
                        }
                    } else if !a.correct(&q) {
                        crate::metacog::ReasonPicker {
                            value: a.reason,
                            on_pick: set_reason,
                        }
                    }
                    // The one next step stays at the foot of the screen; on a
                    // phone the moves sit beside it.
                    div { class: "run-foot",
                        Move { to: prev, next: false, ongo: go }
                        div { class: "run-foot-main",
                            if checked && !last {
                                button {
                                    class: "btn btn-primary btn-lg run-main-action",
                                    "aria-keyshortcuts": "Enter",
                                    // Check is gone: keep the keyboard on the next step.
                                    onmounted: move |e: MountedEvent| async move {
                                        let _ = e.set_focus(true).await;
                                    },
                                    onclick: move |_| go(at + 1),
                                    {tx::NEXT}
                                }
                            } else if checked || (timed && last) {
                                // A whole timed paper never goes in on a stray Enter.
                                button {
                                    class: "btn btn-primary btn-lg run-main-action",
                                    "aria-keyshortcuts": if timed { "" } else { "Enter" },
                                    onmounted: move |e: MountedEvent| async move {
                                        if checked {
                                            let _ = e.set_focus(true).await;
                                        }
                                    },
                                    onclick: move |_| end(()),
                                    {tx::END}
                                }
                            } else {
                                button {
                                    class: "btn btn-primary btn-lg run-main-action",
                                    disabled: selected.is_none(),
                                    "aria-keyshortcuts": "Enter",
                                    onclick: move |_| if timed { confirm() } else { check() },
                                    {tx::CHECK}
                                }
                            }
                            // What a long press and a selection do, beside
                            // the step they come before (hidden on phones).
                            if !checked && !(timed && last) {
                                p { class: "muted small run-hint", {tx::TOOLS_HINT} }
                                if selected.is_none() {
                                    span { class: "bank-sr", {tx::PICK_FIRST} }
                                }
                            }
                        }
                        Move { to: next, next: true, ongo: go }
                    }
                }
            }
            if nav_open() {
                NavSheet {
                    cells,
                    at,
                    legend,
                    ongo: go,
                    onclose: move |_| nav_open.set(false),
                }
            }
            if confirming() {
                ConfirmEnd {
                    blank: n - answered,
                    marked,
                    left: timed.then_some(left),
                    first_blank,
                    onclose: move |_| confirming.set(false),
                }
            }
        }
    }
}

/// «إنهاء البلوك» with questions left blank: says how many before the paper
/// goes in. The safe choice, going back to the first blank, comes first and
/// is what Escape does.
#[component]
fn ConfirmEnd(
    blank: usize,
    marked: usize,
    left: Option<u64>,
    first_blank: Option<usize>,
    onclose: EventHandler<()>,
) -> Element {
    let back = move |_| {
        onclose.call(());
        if let Some(i) = first_blank {
            go(i);
        }
    };
    rsx! {
        div { class: "sheet-backdrop", onclick: move |_| onclose.call(()) }
        div {
                class: "sheet bank-confirm",
                role: "alertdialog",
                "aria-modal": "true",
                "aria-labelledby": "bank-confirm-title",
                h2 { id: "bank-confirm-title", {tx::CONFIRM_TITLE} }
                p { {tx::confirm_blank(blank)} }
                if marked > 0 {
                    p { {tx::confirm_marked(marked)} }
                }
                if let Some(left) = left {
                    p { class: "muted small", "{tx::LEFT_ABOUT} " {count(left, &MINUTE)} }
                }
                div { class: "start-row",
                    button {
                        class: "btn btn-primary",
                        autofocus: true,
                        "data-back": "true",
                        "aria-keyshortcuts": "Escape",
                        onclick: back,
                        if let Some(i) = first_blank {
                            {tx::CONFIRM_BACK_TO} " " bdi { "{i + 1}" }
                        } else {
                            {tx::CONFIRM_BACK}
                        }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| {
                            onclose.call(());
                            finish();
                        },
                        {tx::CONFIRM_END}
                    }
                }
            }
    }
}

/// The time a timed paper has: a bar that drains over it, under the top
/// bar. It is pure CSS, so nothing ticks; its delay is fixed when it
/// mounts, and the end of the animation hands the paper in. Choosing or
/// moving also checks the time, for systems with animations turned off.
#[component]
fn Timer(n: usize, started_ms: i64) -> Element {
    let elapsed_s = use_hook(|| (now_ms() - started_ms).max(0) as f64 / 1000.0);
    let limit_s = exam::time_limit_ms(n) as f64 / 1000.0;
    rsx! {
        div { class: "timer", aria_hidden: "true",
            div {
                class: "timer-fill",
                style: "animation-duration: {limit_s}s; animation-delay: -{elapsed_s}s;",
                onanimationend: move |_| {
                    if matches!(&*PHASE.read(), Phase::Running(p) if p.mode != Mode::Tutor) {
                        finish();
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn player_and_result_keep_48dp_targets() {
        let css = include_str!("bank.css");
        let rule = |sel: &str| -> String {
            let at = css.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("no rule for {sel}"));
            css[at..].split('}').next().unwrap().to_string()
        };
        for sel in [
            ".chip.toggle",
            ".bank-close",
            ".q-meta .chip.tap",
            ".q-foot button.tap-chip",
            ".q-behind > .btn",
            ".report",
            ".exam-bar :is(.btn, .chip.tap-chip)",
            ".sum-head .btn",
            ".sum-open",
            "button.chip.weak",
            ".sum .start-row .btn",
        ] {
            assert!(rule(sel).contains("min-block-size: var(--target);"), "{sel}");
        }
        // The foot's one step: 48dp, or the taller main-action size.
        assert!(rule(".run-main-action").contains("min-block-size: max(var(--target), var(--tap-lg));"));
        for sel in [".nav-cell", ".run-leave", ".exam-bar .icon-button", ".run-foot .exam-move"] {
            assert!(rule(sel).contains("inline-size: var(--target);"), "{sel}");
            assert!(rule(sel).contains("block-size: var(--target);"), "{sel}");
        }
        for small in ["min-height: 40px", "inline-size: 40px", "block-size: 40px"] {
            assert!(!css.contains(small), "{small}");
        }
    }
}
