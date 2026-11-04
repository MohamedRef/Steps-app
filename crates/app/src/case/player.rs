//! Clinical case player stages and completion recap.

use super::*;

/// Brings a step of the case into view (the stepper's buttons).
pub(super) fn scroll_to(id: &str) {
    document::eval(&format!(
        r#"document.getElementById("{id}")?.scrollIntoView({{behavior: "smooth", block: "start"}});"#
    ));
}
#[component]
pub fn CasePlayer(item_id: String, on_back: EventHandler) -> Element {
    let store = use_case_store();
    let mut save_failed = use_signal(|| None::<String>);
    if RUN.peek().as_ref().is_none_or(|r| r.item_id != item_id) {
        *RUN.write() = Some(CaseRun::new(&item_id));
    }
    let Some(run) = RUN.read().clone() else { return rsx! {} };

    let Some((lecture, item)) = find_item(&item_id) else { return rsx! {} };
    let item = effective(store.as_ref(), item);
    let Body::Case { stem, steps } = &item.body else { return rsx! {} };
    let total = steps.len();
    let k = run.step;
    let done = k >= total;
    let answered = run.results.len() > k;
    let page = item.src.page;
    let record_id = item.id.clone();
    let step_answer = steps.get(k).and_then(|s| s.answer);
    // A fresh order on every attempt, steady while the step is open.
    let order = steps.get(k).map_or_else(Vec::new, |s| option_order(&s.options, run.started as u64, k));
    let selected = run.picked.or(run.draft);
    let edit = |f: &dyn Fn(&mut CaseRun)| {
        if let Some(r) = RUN.write().as_mut() {
            f(r);
        }
    };
    let leave = move |_| {
        // A finished case starts fresh next time; one half-way is kept.
        if RUN.peek().as_ref().is_some_and(|r| r.step >= total) {
            *RUN.write() = None;
        }
        on_back.call(());
    };

    rsx! {
        article { class: "player case",
            ItemMeta { lecture: lecture.to_owned(), item_id: item.id.clone(), on_back: leave }
            if save_failed.read().as_ref().is_some_and(|id| id == &item_id) {
                p { class: "verdict bad", role: "alert", {tx::SAVE_FAILED} }
            }
            // Where he is in the case: each step done (✓/✗) opens its place
            // in the data, the current one its question.
            ol { class: "case-stepper", "aria-label": tx::STEPS_LABEL,
                for i in 0..total {
                    li { key: "{i}",
                        if i < k || (i == k && !done) {
                            button {
                                class: "case-stepper-step",
                                "data-state": match run.results.get(i) {
                                    _ if i == k => "now",
                                    Some((true, _)) => "ok",
                                    _ => "miss",
                                },
                                "aria-current": if i == k { "step" } else { "false" },
                                onclick: move |_| scroll_to(&if i == k { "case-q".to_string() } else { format!("case-step-{i}") }),
                                span { class: "num-ltr", "{i + 1}" }
                                match run.results.get(i) {
                                    Some((true, _)) if i < k => rsx! { span { aria_hidden: "true", "✓" } },
                                    Some(_) if i < k => rsx! { span { aria_hidden: "true", "✗" } },
                                    _ => rsx! { span { {tx::NOW} } },
                                }
                            }
                        } else {
                            span { class: "case-stepper-later", "data-state": if done { "ok" } else { "later" },
                                span { class: "num-ltr", "{i + 1}" }
                            }
                        }
                    }
                }
            }
            div { class: "case-grid",
                // What is known so far: the stem, every reveal up to this
                // step, and each finished step's question and answer.
                crate::ui::Card { class: "case-data",
                    h2 { class: "label", {tx::KNOWN} }
                    bdi { class: "case-stem", lang: "en", "{stem}" }
                    for (i, s) in steps.iter().enumerate().take(k + 1) {
                        div { key: "{i}", id: "case-step-{i}", class: "case-history",
                            if let Some(reveal) = &s.reveal {
                                div { class: if i == k && !done { "case-reveal new" } else { "case-reveal" },
                                    span { class: "small muted", "{tx::STEP} " span { class: "num-ltr", "{i + 1}" } if i == k && !done { " · {tx::NEW_DATA}" } }
                                    bdi { lang: "en", "{reveal}" }
                                }
                            }
                            if let Some((ok, pick)) = run.results.get(i).copied().filter(|_| i < k) {
                                div { class: if ok { "case-done ok" } else { "case-done miss" },
                                    bdi { class: "small case-done-q", dir: "ltr", "{s.question}" }
                                    match pick.and_then(|p| s.options.get(p)) {
                                        Some(o) => rsx! {
                                            p { class: "small", dir: "ltr",
                                                span { aria_hidden: "true", if ok { "✓ " } else { "✗ " } }
                                                "{o}"
                                            }
                                        },
                                        None => rsx! {
                                            p { class: "small",
                                                span { aria_hidden: "true", if ok { "✓ " } else { "✗ " } }
                                                if ok { {tx::GOT_IT} } else { {tx::MISSED} }
                                            }
                                        },
                                    }
                                    if let Some(right) = s.answer.filter(|_| !ok).and_then(|a| s.options.get(a)) {
                                        p { class: "small", "{tx::CORRECT_WAS} " bdi { "{right}" } }
                                    }
                                }
                            }
                        }
                    }
                }
                if !done {
                    crate::ui::Card { class: "case-step", key: "{k}", id: "case-q",
                        p { class: "small muted", "{tx::STEP} " span { class: "num-ltr", "{k + 1} / {total}" } }
                        bdi { class: "case-question", lang: "en", "{steps[k].question}" }
                        if !steps[k].options.is_empty() {
                            div { class: "options", role: "list",
                                for (pos, i) in order.iter().copied().enumerate() {
                                    button {
                                        key: "{i}",
                                        role: "listitem",
                                        dir: "auto",
                                        "aria-keyshortcuts": option_key(pos),
                                        "aria-pressed": if selected == Some(i) { "true" } else { "false" },
                                        class: if option_state(i, run.picked, run.draft, steps[k].answer) == "dim" { "answer case-option dim" } else { "answer case-option" },
                                        "data-state": option_state(i, run.picked, run.draft, steps[k].answer),
                                        disabled: run.picked.is_some(),
                                        onclick: move |_| edit(&|r| r.select(i)),
                                        span { class: "answer-key", aria_hidden: "true", {option_key(pos)} }
                                        bdi { class: "answer-text", "{steps[k].options[i]}" }
                                        match option_state(i, run.picked, run.draft, steps[k].answer) {
                                            "correct" | "missed" => rsx! { span { class: "answer-mark", "✓ {tx::MARK_RIGHT}" } },
                                            "wrong" => rsx! { span { class: "answer-mark", "✗ {tx::MARK_YOURS}" } },
                                            _ => rsx! {},
                                        }
                                    }
                                }
                            }
                            if run.picked.is_none() {
                                if run.draft.is_none() {
                                    p { class: "muted small case-hint", {tx::PICK_FIRST} }
                                }
                                button {
                                    class: "btn btn-primary btn-lg case-check",
                                    disabled: run.draft.is_none(),
                                    "aria-keyshortcuts": "Enter",
                                    onclick: move |_| edit(&|r| { r.check(step_answer); }),
                                    {tx::CHECK}
                                }
                            }
                        } else if !run.shown {
                            button { class: "btn btn-primary", "aria-keyshortcuts": "Space", onclick: move |_| edit(&|r| r.shown = true), {tx::SHOW_ANSWER} }
                        }
                        if run.picked.is_some() || run.shown {
                            // Brought into view on the phone, where it would
                            // otherwise open below the fold.
                            div {
                                class: "case-feedback",
                                onmounted: move |e: MountedEvent| async move {
                                    let _ = e
                                        .scroll_to_with_options(ScrollToOptions {
                                            behavior: ScrollBehavior::Smooth,
                                            vertical: ScrollLogicalPosition::Nearest,
                                            horizontal: ScrollLogicalPosition::Nearest,
                                        })
                                        .await;
                                },
                                if let Some(p) = run.picked {
                                    p { class: if steps[k].answer == Some(p) { "verdict good" } else { "verdict bad" },
                                        if steps[k].answer == Some(p) { {tx::RIGHT} } else { {tx::WRONG} }
                                    }
                                }
                                div { class: "case-why",
                                    bdi { lang: "en", "{steps[k].why}" }
                                    BookPage { page, quote: item.src.quote.clone(), of: item.id.clone() }
                                }
                                if run.shown && !answered {
                                    div { class: "row",
                                        button { class: "btn btn-good", "aria-keyshortcuts": "3", onclick: move |_| edit(&|r| r.grade_self(true)), {tx::GOT_IT} }
                                        button { class: "btn btn-secondary", "aria-keyshortcuts": "1", onclick: move |_| edit(&|r| r.grade_self(false)), {tx::MISSED} }
                                    }
                                }
                                if answered {
                                    button {
                                        class: "btn btn-primary btn-lg",
                                        "aria-keyshortcuts": "Enter",
                                        onclick: {
                                            let record_id = record_id.clone();
                                            move |_| {
                                                let Some(mut r) = RUN.peek().clone() else { return };
                                                match r.next_logged(total, store.as_ref()) {
                                                    Ok(()) => {
                                                        save_failed.set(None);
                                                        *RUN.write() = Some(r);
                                                    }
                                                    Err(_) => save_failed.set(Some(record_id.clone())),
                                                }
                                            }
                                        },
                                        if k + 1 == total { {tx::FINISH} } else { {tx::NEXT} }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    Summary {
                        recap: steps
                            .iter()
                            .zip(&run.results)
                            .map(|(s, &(ok, pick))| Recap {
                                ok,
                                question: s.question.clone(),
                                yours: pick.and_then(|p| s.options.get(p)).cloned(),
                                right: s.answer.filter(|_| !ok).and_then(|a| s.options.get(a)).cloned(),
                                why: s.why.clone(),
                            })
                            .collect::<Vec<_>>(),
                        update: item.update.as_ref().map(|u| format!("{} ({})", u.text, u.reference)),
                        on_again: {
                            let id = item_id.clone();
                            move |_| *RUN.write() = Some(CaseRun::new(&id))
                        },
                        on_back: move |_| leave(()),
                    }
                }
            }
        }
    }
}
/// One finished step in the recap.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Recap {
    pub(super) ok: bool,
    pub(super) question: String,
    /// The option he picked; `None` when he graded himself.
    pub(super) yours: Option<String>,
    /// The right option, when his was not.
    pub(super) right: Option<String>,
    pub(super) why: String,
}
// Option ordering and answer states live in `choices.rs`.
#[component]
pub(super) fn Summary(
    recap: Vec<Recap>,
    update: Option<String>,
    on_again: EventHandler,
    on_back: EventHandler,
) -> Element {
    let right = recap.iter().filter(|r| r.ok).count();
    let total = recap.len();
    let message = match grade_for(right as u32, total as u32) {
        Grade::Good | Grade::Easy => tx::ALL_RIGHT,
        Grade::Hard => tx::SOME_RIGHT,
        Grade::Again => tx::FEW_RIGHT,
    };
    rsx! {
        crate::ui::Card { class: "case-summary",
            p { class: "case-score", bdi { class: "num-ltr", "{right} / {total}" } " " {tx::SCORE} }
            p { {message} }
            // The reasoning, step by step: the question, his answer, the
            // right one when his wasn't, and why.
            h2 { class: "label", {tx::RECAP} }
            ol { class: "case-steps-done",
                for (i, r) in recap.into_iter().enumerate() {
                    li { key: "{i}", class: if r.ok { "ok" } else { "miss" },
                        p { class: "case-done-q", dir: "ltr", lang: "en",
                            span { aria_hidden: "true", if r.ok { "✓ " } else { "✗ " } }
                            "{r.question}"
                        }
                        p { class: "small",
                            "{tx::YOURS} "
                            match r.yours {
                                Some(y) => rsx! { bdi { lang: "en", "{y}" } },
                                None => rsx! { if r.ok { {tx::GOT_IT} } else { {tx::MISSED} } },
                            }
                        }
                        if let Some(right) = r.right {
                            p { class: "small", "{tx::CORRECT_WAS} " bdi { lang: "en", "{right}" } }
                        }
                        p { class: "small muted", dir: "ltr", lang: "en", "{r.why}" }
                    }
                }
            }
            if let Some(u) = update { p { class: "small update", "{tx::UPDATE}: " bdi { "{u}" } } }
            div { class: "row",
                button { class: "btn btn-secondary", "aria-keyshortcuts": "Space", onclick: move |_| on_again.call(()), {tx::AGAIN} }
                button { class: "btn btn-primary", "aria-keyshortcuts": "Enter", onclick: move |_| on_back.call(()), {tx::BACK} }
            }
        }
    }
}
