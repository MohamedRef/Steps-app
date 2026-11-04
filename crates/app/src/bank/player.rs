//! Answering a paper. Tutor mode reveals after each pick; timed papers and
//! mocks run against a calm clock with a question navigator and flags.

use dioxus::prelude::*;
use steps_core::exam::{self, Mode};

use super::question::QuestionView;
use super::{
    PHASE, Phase, check, choose, finish, go, leave, minutes, now_ms, set_confidence, set_reason, toggle_flag,
};
use crate::i18n::bank as tx;
use crate::i18n::count::{MINUTE, count};

#[component]
pub fn Player() -> Element {
    // A running paper gets the whole phone screen; ✕ is the way out.
    crate::parts::use_focus_mode(true);
    let Phase::Running(p) = PHASE.read().clone() else {
        return rsx! {};
    };
    let n = p.questions.len();
    let at = p.at;
    let q = p.questions[at].clone();
    let a = p.answers[at].clone();
    let timed = p.mode.timed();
    // Tutor mode: select, then check, then the explanation.
    let checked = !timed && a.chosen.is_some();
    let selected = if timed { a.chosen } else { a.chosen.or(p.draft) };
    let answered = p.answers.iter().filter(|a| a.chosen.is_some()).count();
    let right = p.questions.iter().zip(&p.answers).filter(|(q, a)| a.correct(q)).count();
    let left = minutes(exam::remaining_ms(n, p.started_ms, now_ms()));
    let last = at + 1 == n;
    let cells: Vec<(usize, bool, bool)> =
        p.answers.iter().enumerate().map(|(i, a)| (i, a.chosen.is_some(), a.flagged)).collect();
    let flagged = p.answers.iter().filter(|a| a.flagged).count();
    let first_blank = p.answers.iter().position(|a| a.chosen.is_none());
    // The right option's letter as shown, for the spoken verdict.
    let right_letter =
        q.order.iter().position(|&o| o == q.answer).map_or('?', |i| char::from(b'A' + i as u8));
    let mode_name = match p.mode {
        Mode::Tutor => tx::MODE_TUTOR_NAME,
        Mode::Timed => tx::MODE_TIMED_NAME,
        Mode::Mock => tx::MODE_MOCK_NAME,
    };
    // Done questions over all, for the bar: answered on a timed paper,
    // checked in tutor mode.
    let progress = 100 * (if timed { answered } else { at + usize::from(checked) }) / n.max(1);
    // A timed paper with blanks or flags asks once before it goes in.
    let mut confirming = use_signal(|| false);
    let submit = move |_| {
        if answered < n || flagged > 0 {
            let mut confirming = confirming;
            confirming.set(true);
        } else {
            finish();
        }
    };

    rsx! {
        div { class: "bank-run", "data-mode": if timed { "timed" } else { "tutor" },
            div { class: "run-main",
                // The count, time left and clock stay in view while he
                // scrolls a long question (#234).
                div { class: "run-sticky",
                    div { class: "run-head",
                        button {
                            class: "icon-button run-leave",
                            // Android Back takes it while the bar is hidden.
                            "data-back": "true",
                            "aria-label": tx::LEAVE,
                            title: if timed { tx::LEAVE_TIMED_HINT } else { tx::LEAVE },
                            onclick: move |_| leave(),
                            span { aria_hidden: "true", "✕" }
                        }
                        div {
                            class: "progress run-progress",
                            role: "progressbar",
                            "aria-valuemin": "0",
                            "aria-valuemax": "100",
                            "aria-valuenow": "{progress}",
                            div { class: "progress-fill", style: "inline-size: {progress}%;" }
                        }
                        span { class: "run-count num-ltr", "aria-label": "{tx::QUESTION} {at + 1} {tx::OF} {n}", "{at + 1} / {n}" }
                    }
                    div { class: "run-sub",
                        span { class: "chip", "data-tone": "accent", {mode_name} }
                        if timed {
                            span { class: "muted small", "{tx::LEFT_ABOUT} " {count(left, &MINUTE)} }
                        } else if answered > 0 {
                            span { class: "muted small", "{tx::RIGHT} " bdi { class: "num-ltr", "{right} / {answered}" } }
                        }
                    }
                    if timed {
                        Timer { key: "{p.started_ms}", n, started_ms: p.started_ms }
                    }
                }
                // The Check button goes away once he checks, so the verdict
                // is read out here as well as written on the options.
                p { class: "bank-sr", role: "status", "aria-live": "polite",
                    if checked { {tx::checked(a.correct(&q), right_letter)} }
                }
                QuestionView {
                    key: "{q.id}-{at}",
                    q: q.clone(),
                    chosen: selected,
                    reveal: checked,
                    onchoose: if checked { None } else { Some(EventHandler::new(choose)) },
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
                // The one next step stays at the foot of the screen.
                div { class: "run-foot",
                    if timed {
                        div { class: "run-actions",
                            // The app reads right to left: → goes back, ← goes on.
                            button { class: "btn btn-ghost", disabled: at == 0, "aria-keyshortcuts": "ArrowRight", onclick: move |_| go(at.saturating_sub(1)), {tx::PREV} }
                            button {
                                class: "btn btn-ghost",
                                "aria-pressed": if a.flagged { "true" } else { "false" },
                                "aria-keyshortcuts": "F",
                                onclick: move |_| toggle_flag(),
                                if a.flagged { "⚑ {tx::FLAGGED}" } else { "⚐ {tx::FLAG}" }
                            }
                            if last {
                                // A whole timed paper never goes in on a stray Enter.
                                button { class: "btn btn-primary", onclick: submit, {tx::SUBMIT} }
                            } else {
                                button { class: "btn btn-primary", "aria-keyshortcuts": "Enter ArrowLeft", onclick: move |_| go(at + 1), {tx::NEXT} }
                            }
                        }
                    } else if checked {
                        button {
                            class: "btn btn-primary btn-lg run-main-action",
                            "aria-keyshortcuts": if last { "Enter" } else { "Enter ArrowLeft" },
                            // Check is gone: keep the keyboard on the next step.
                            onmounted: move |e: MountedEvent| async move {
                                let _ = e.set_focus(true).await;
                            },
                            onclick: move |_| if last { finish() } else { go(at + 1) },
                            if last { {tx::FINISH} } else { {tx::NEXT} }
                        }
                    } else {
                        if selected.is_none() {
                            p { class: "muted small run-hint", {tx::PICK_FIRST} }
                        }
                        button {
                            class: "btn btn-primary btn-lg run-main-action",
                            disabled: selected.is_none(),
                            "aria-keyshortcuts": "Enter",
                            onclick: move |_| check(),
                            {tx::CHECK}
                        }
                    }
                }
            }
            if timed {
                crate::ui::Card { element: crate::ui::CardElement::Aside, class: "run-side",
                    div { class: "navigator", role: "list",
                        for (i, done, flagged) in cells {
                            button {
                                key: "{i}",
                                role: "listitem",
                                class: "nav-cell",
                                "data-done": if done { "yes" } else { "no" },
                                "data-flag": if flagged { "yes" } else { "no" },
                                "aria-current": if i == at { "true" } else { "false" },
                                // Answered and flagged are spoken too, not only drawn.
                                "aria-label": cell_label(i, done, flagged, i == at),
                                onclick: move |_| go(i),
                                "{i + 1}"
                                if flagged {
                                    span { class: "nav-flag", aria_hidden: "true", "⚑" }
                                }
                            }
                        }
                    }
                    p { class: "muted small", bdi { "{n - answered}" } " {tx::UNANSWERED}" }
                    // The foot holds the one primary step; this is the early way in.
                    button { class: "btn btn-secondary", onclick: submit, {tx::SUBMIT} }
                }
            }
            if timed && confirming() {
                ConfirmSubmit {
                    blank: n - answered,
                    flagged,
                    left,
                    first_blank,
                    onclose: move |_| confirming.set(false),
                }
            }
        }
    }
}

/// "سؤال 3، متجاوب، متعلّم" for a navigator cell.
fn cell_label(i: usize, done: bool, flagged: bool, here: bool) -> String {
    let mut parts = vec![format!("{} {}", tx::QUESTION, i + 1)];
    parts.push(if done { tx::ANSWERED } else { tx::UNANSWERED }.to_string());
    if flagged {
        parts.push(tx::FLAGGED.to_string());
    }
    if here {
        parts.push(tx::NOW_HERE.to_string());
    }
    parts.join("، ")
}

/// "Hand it in?" for a timed paper with blanks or flags. The safe choice,
/// going back to the first blank, comes first and is what Escape does.
#[component]
fn ConfirmSubmit(
    blank: usize,
    flagged: usize,
    left: u64,
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
                if blank > 0 {
                    p { {tx::CONFIRM_BLANK_A} " " bdi { "{blank}" } " " {tx::CONFIRM_BLANK_B} }
                }
                if flagged > 0 {
                    p { bdi { "{flagged}" } " " {tx::CONFIRM_FLAGGED} }
                }
                p { class: "muted small", "{tx::LEFT_ABOUT} " {count(left, &MINUTE)} }
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
                        {tx::CONFIRM_SUBMIT}
                    }
                }
            }
    }
}

/// The clock: a bar that drains over the paper's time. It is pure CSS, so
/// nothing ticks; its delay is fixed when it mounts, and the end of the
/// animation submits the paper. Choosing or moving also checks the time, for
/// systems with animations turned off.
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
    use super::cell_label;

    #[test]
    fn navigator_cells_say_their_state() {
        assert_eq!(cell_label(0, false, false, false), "سؤال 1، من غير إجابة");
        assert_eq!(cell_label(2, true, true, true), "سؤال 3، متجاوب، متعلّم، إنت هنا");
    }

    #[test]
    fn player_and_result_keep_48dp_targets() {
        let css = include_str!("bank.css");
        let rule = |sel: &str| -> String {
            let at = css.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("no rule for {sel}"));
            css[at..].split('}').next().unwrap().to_string()
        };
        for sel in
            [".chip.toggle", ".review-item", ".score-row", ".bank-close", ".q-meta .chip.tap", ".report"]
        {
            assert!(rule(sel).contains("min-block-size: var(--target);"), "{sel}");
        }
        assert!(rule(".nav-cell").contains("inline-size: var(--target);"));
        assert!(rule(".nav-cell").contains("block-size: var(--target);"));
        assert!(rule(".run-leave").contains("inline-size: var(--target);"));
        for small in ["min-height: 40px", "inline-size: 40px", "block-size: 40px"] {
            assert!(!css.contains(small), "{small}");
        }
    }
}
