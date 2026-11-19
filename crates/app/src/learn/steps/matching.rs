//! Lesson matching interaction and response restoration.

use super::*;

/// Match pairs: each pair is checked as he makes it, and wrong taps count.
/// While he plays, both tiles of a pair he made carry the same number (the
/// order he made it in); once every pair is made, the right column lines up
/// with the left, so each row reads as one pair (UX-04).
#[component]
pub(super) fn Match(
    prompt: String,
    pairs: Vec<[String; 2]>,
    why: Option<String>,
    seed: String,
    on_answer: EventHandler<bool>,
) -> Element {
    let n = pairs.len();
    let shuffled_rights = use_hook(|| shuffled(n, &format!("{seed}/r")));
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
    let rights = right_rows(&shuffled_rights, finished);
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
    // The number a made pair shows, while he is still matching.
    let badge = move |i: usize| if finished { None } else { pair_number(&done.read(), i) };
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
                            aria_label: tile_name(&p[0], &p[1], done.read().contains(&i)),
                            disabled: done.read().contains(&i),
                            onclick: move |_| {
                                left.set(Some(i));
                                flash.set(None);
                            },
                            if let Some(k) = badge(i) {
                                span { class: "ls-pair", aria_hidden: "true", "{k}" }
                            }
                            bdi { "{p[0]}" }
                        }
                    }
                }
                div { class: "ls-match-col",
                    for r in rights {
                        button {
                            key: "r{r}",
                            class: "ls-tile",
                            "data-state": if done.read().contains(&r) { "right" } else if flash() == Some(r) { "wrong" } else { "open" },
                            aria_label: tile_name(&pairs[r][1], &pairs[r][0], done.read().contains(&r)),
                            disabled: done.read().contains(&r) || left().is_none(),
                            onclick: move |_| tap_right(r),
                            if let Some(k) = badge(r) {
                                span { class: "ls-pair", aria_hidden: "true", "{k}" }
                            }
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

/// The right column, top to bottom: shuffled while he matches; once every
/// pair is made, in the left column's order, so row `i` is pair `i`.
pub(super) fn right_rows(shuffled: &[usize], finished: bool) -> Vec<usize> {
    if finished { (0..shuffled.len()).collect() } else { shuffled.to_vec() }
}

/// The number both tiles of pair `i` carry once he made it: the order he
/// made it in, from 1.
pub(super) fn pair_number(done: &[usize], i: usize) -> Option<usize> {
    done.iter().position(|&d| d == i).map(|k| k + 1)
}

/// A tile's name for screen readers: its words, and once paired, its pair.
fn tile_name(text: &str, other: &str, paired: bool) -> String {
    if paired { tx::paired_with(text, other) } else { text.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steps_core::response::sig;

    #[test]
    fn a_finished_match_reads_row_by_row() {
        let shuffled = [2, 0, 3, 1];
        assert_eq!(right_rows(&shuffled, false), shuffled, "while he plays, the shuffle stays");
        assert_eq!(right_rows(&shuffled, true), [0, 1, 2, 3], "then each row is a pair");
        // Pairs carry the order they were made in.
        let done = [2, 0];
        assert_eq!(pair_number(&done, 2), Some(1));
        assert_eq!(pair_number(&done, 0), Some(2));
        assert_eq!(pair_number(&done, 1), None);
        assert_eq!(tile_name("A", "B", false), "A");
        assert!(tile_name("A", "B", true).contains('A') && tile_name("A", "B", true).contains('B'));
    }

    thread_local! {
        static DONE: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn pairs() -> Vec<[String; 2]> {
        [
            ["Left horn of sinus venosus", "Coronary sinus"],
            ["Truncus arteriosus", "Aorta"],
            ["Bulbus cordis", "RV outflow"],
        ]
        .map(|[a, b]| [a.to_string(), b.to_string()])
        .to_vec()
    }

    /// The step with these pairs already made, as a reopened lesson shows it.
    fn step() -> Element {
        let body = StepBody::Match { prompt: "Match".into(), pairs: pairs(), why: None };
        let done = DONE.with_borrow(Clone::clone);
        let mut attempt = Attempt::new(1, 0);
        attempt.put(0, sig(&body), Draft::Match { done, misses: 0 });
        use_context_provider(|| StepAt(0));
        use_context_provider(|| Responses { attempt: Signal::new(attempt), put: Callback::new(|_| {}) });
        rsx! { Match { prompt: "Match", pairs: pairs(), why: None, seed: "x#3", on_answer: |_| {} } }
    }

    fn render(done: Vec<usize>) -> String {
        DONE.with_borrow_mut(|d| *d = done);
        let mut dom = VirtualDom::new(step);
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    /// The right column's words, top to bottom.
    fn right_column(html: &str) -> Vec<&'static str> {
        let col = html.split(r#"class="ls-match-col""#).nth(2).expect("the right column");
        let mut at: Vec<(usize, &'static str)> = ["Coronary sinus", "Aorta", "RV outflow"]
            .into_iter()
            .filter_map(|w| Some((col.find(&format!(">{w}<"))?, w)))
            .collect();
        at.sort();
        at.into_iter().map(|(_, w)| w).collect()
    }

    #[test]
    fn made_pairs_share_a_number_and_a_finished_grid_lines_up() {
        // One pair made: both its tiles carry «1»; the other tiles none.
        let html = render(vec![1]);
        assert_eq!(html.matches(r#"class="ls-pair""#).count(), 2, "{html}");
        assert!(html.contains(r#"aria-hidden="true">1</span>"#), "{html}");
        assert!(html.contains(&tx::paired_with("Truncus arteriosus", "Aorta")), "{html}");
        // Every pair made: rows line up with the left column, no numbers.
        let html = render(vec![2, 0, 1]);
        assert_eq!(right_column(&html), ["Coronary sinus", "Aorta", "RV outflow"], "{html}");
        assert!(!html.contains(r#"class="ls-pair""#), "{html}");
    }
}
