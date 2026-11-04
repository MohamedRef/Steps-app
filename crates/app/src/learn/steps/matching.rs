//! Lesson matching interaction and response restoration.

use super::*;

/// Match pairs: each pair is checked as he makes it, and wrong taps count.
#[component]
pub(super) fn Match(
    prompt: String,
    pairs: Vec<[String; 2]>,
    why: Option<String>,
    seed: String,
    on_answer: EventHandler<bool>,
) -> Element {
    let n = pairs.len();
    let rights = use_hook(|| shuffled(n, &format!("{seed}/r")));
    let (saved, put) = use_draft();
    let (done0, misses0) = match saved {
        Draft::Match { done, misses } => (done, misses as usize),
        _ => (Vec::new(), 0),
    };
    let mut done = use_signal(|| done0);
    let mut left = use_signal(|| None::<usize>);
    let mut flash = use_signal(|| None::<usize>);
    let mut misses = use_signal(|| misses0);
    let finished = done.read().len() == n;
    let mut tap_right = move |r: usize| {
        let Some(l) = left() else { return };
        if l == r {
            done.write().push(l);
            left.set(None);
            flash.set(None);
        } else {
            *misses.write() += 1;
            flash.set(Some(r));
        }
        put.call(Draft::Match { done: done.peek().clone(), misses: *misses.peek() as u32 });
        if l == r && done.peek().len() == n {
            on_answer.call(*misses.peek() == 0);
            crate::sounds::play(crate::sounds::Cue::Right);
        }
    };
    rsx! {
        div { class: "ls-match",
            p { class: "ls-kicker", {tx::MATCH} }
            div { class: "ls-q", Rich { text: prompt } }
            if !finished { p { class: "muted small", {tx::MATCH_HINT} } }
            div { class: "ls-match-cols", dir: "ltr", lang: "en",
                div { class: "ls-match-col",
                    for (i , p) in pairs.iter().enumerate() {
                        button {
                            key: "l{i}",
                            class: "ls-tile",
                            "data-state": if done.read().contains(&i) { "right" } else if left() == Some(i) { "picked" } else { "open" },
                            "aria-pressed": if left() == Some(i) { "true" } else { "false" },
                            disabled: done.read().contains(&i),
                            onclick: move |_| {
                                left.set(Some(i));
                                flash.set(None);
                            },
                            bdi { "{p[0]}" }
                        }
                    }
                }
                div { class: "ls-match-col",
                    for &r in rights.iter() {
                        button {
                            key: "r{r}",
                            class: "ls-tile",
                            "data-state": if done.read().contains(&r) { "right" } else if flash() == Some(r) { "wrong" } else { "open" },
                            disabled: done.read().contains(&r) || left().is_none(),
                            onclick: move |_| tap_right(r),
                            bdi { "{pairs[r][1]}" }
                        }
                    }
                }
            }
            if finished {
                p { class: if misses() == 0 { "ls-verdict good" } else { "ls-verdict" }, role: "status",
                    {if misses() == 0 { tx::RIGHT.to_string() } else { tx::misses(misses()) }}
                }
                if let Some(w) = why { div { class: "ls-why-box", Rich { text: w } } }
            }
        }
    }
}
