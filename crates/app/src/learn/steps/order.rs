//! Lesson order interaction and response restoration.

use super::*;

/// Put the steps in order: numbered slots fill as he taps the pool. Before
/// Check any placed step can come out (tap it, or Undo the last); after
/// Check the order is read-only and every slot shows right or wrong.
#[component]
pub(super) fn Order(
    prompt: String,
    steps: Vec<String>,
    why: Option<String>,
    seed: String,
    on_answer: EventHandler<bool>,
) -> Element {
    let n = steps.len();
    let pool = use_hook(|| shuffled(n, &seed));
    let (saved, put) = use_draft();
    let (placed0, checked0) = match saved {
        Draft::Order { placed, checked } => {
            let full = placed.len() == n;
            (placed, checked && full)
        }
        _ => (Vec::new(), false),
    };
    let mut placed = use_signal(|| placed0);
    let mut checked = use_signal(|| checked0);
    let mut set = move |v: Vec<usize>| {
        put.call(Draft::Order { placed: v.clone(), checked: false });
        placed.set(v);
    };
    use_check_bar(
        move || placed.read().len() == n && !checked(),
        move |_| {
            let v = placed.peek().clone();
            checked.set(true);
            let right = misplaced(&v).is_empty();
            put.call(Draft::Order { placed: v, checked: true });
            on_answer.call(right);
            cue(right);
        },
    );
    let wrong = if checked() { misplaced(&placed.read()) } else { Vec::new() };
    let k = placed.read().len();
    rsx! {
        div { class: "ls-order",
            p { class: "ls-kicker", {tx::ORDER} }
            div { class: "ls-q", Rich { text: prompt } }
            if !checked() {
                div { class: "ls-order-bar",
                    span { class: "ls-order-count small", role: "status", {tx::placed_of(k, n)} }
                    button {
                        class: "btn-link ls-undo",
                        disabled: k == 0,
                        onclick: move |_| {
                            let mut v = placed.peek().clone();
                            v.pop();
                            set(v);
                        },
                        span { aria_hidden: "true", "↶ " }
                        {tx::UNDO}
                    }
                }
            }
            ol { class: "ls-slots", dir: "ltr", lang: "en",
                for pos in 0..n {
                    match placed.read().get(pos).copied() {
                        Some(s) if !checked() => rsx! {
                            li { key: "{pos}", "data-state": "filled",
                                // The whole slot takes the step back out.
                                button {
                                    class: "ls-slot",
                                    aria_label: "{tx::REMOVE}: {steps[s]}",
                                    title: tx::REMOVE,
                                    onclick: move |_| {
                                        let mut v = placed.peek().clone();
                                        if pos < v.len() {
                                            v.remove(pos);
                                        }
                                        set(v);
                                    },
                                    span { class: "ls-num", "{pos + 1}" }
                                    bdi { class: "ls-slot-text", "{steps[s]}" }
                                    span { class: "ls-x", aria_hidden: "true", "×" }
                                }
                            }
                        },
                        Some(s) => rsx! {
                            li { key: "{pos}", "data-state": if wrong.contains(&pos) { "wrong" } else { "right" },
                                div { class: "ls-slot",
                                    span { class: "ls-num", "{pos + 1}" }
                                    bdi { class: "ls-slot-text", "{steps[s]}" }
                                    span { class: "ls-mark", aria_hidden: "true", if wrong.contains(&pos) { "✗" } else { "✓" } }
                                }
                                if wrong.contains(&pos) {
                                    span { class: "ls-should small", dir: "rtl", lang: "ar", {tx::should_be(s + 1)} }
                                }
                            }
                        },
                        None => rsx! {
                            li { key: "{pos}", "data-state": "empty",
                                div { class: "ls-slot",
                                    span { class: "ls-num", "{pos + 1}" }
                                    span { class: "ls-slot-text muted small", dir: "rtl", lang: "ar", {tx::slot(pos + 1)} }
                                }
                            }
                        },
                    }
                }
            }
            if k < n && !checked() {
                div { class: "ls-pool", dir: "ltr", lang: "en",
                    for &s in pool.iter().filter(|s| !placed.read().contains(s)) {
                        button {
                            key: "{s}",
                            class: "ls-tile",
                            onclick: move |_| {
                                let mut v = placed.peek().clone();
                                if !v.contains(&s) && v.len() < n {
                                    v.push(s);
                                }
                                set(v);
                            },
                            bdi { "{steps[s]}" }
                        }
                    }
                }
            }
            if checked() {
                p { class: if wrong.is_empty() { "ls-verdict good" } else { "ls-verdict bad" }, role: "status",
                    {if wrong.is_empty() { tx::RIGHT.to_string() } else { tx::in_place(n - wrong.len(), n) }}
                }
                if !wrong.is_empty() {
                    ol { class: "ls-key small", dir: "ltr", lang: "en",
                        for (i , s) in steps.iter().enumerate() { li { key: "{i}", "{s}" } }
                    }
                }
                if let Some(w) = why { div { class: "ls-why-box", Rich { text: w } } }
            }
        }
    }
}
