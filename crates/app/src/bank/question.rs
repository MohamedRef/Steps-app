//! One MCQ on screen: stem (with his highlights, `stem.rs`), its figure when
//! it has one (VQ questions, `crate::vq`), options he can strike out, and
//! once revealed the verdict, the right answer, why each option is right or
//! wrong, a VQ question's educational objective, the lecture and topic, the
//! book page (or `VQ-0001`), the ✓ / ✎ / ✗ check and any guideline update.
//! The explanation sits under the question, and beside it on the full 72rem
//! canvas (`desk::Panes`, bank.css).

use dioxus::prelude::*;
use steps_core::content::Body;
use steps_core::exam::{Answer, Graded, Question};
use steps_core::review::verify::Check;

use super::stem::Stem;
use super::{lookup, topic_title};
use crate::i18n::bank as tx;
use crate::store::use_review;

/// The letter an option is shown under: A for the first shown.
pub fn letter(shown: usize) -> char {
    char::from(b'A' + (shown.min(25)) as u8)
}

/// The right option's letter as shown, and the picked one's.
pub fn letters(q: &Question, chosen: Option<usize>) -> (char, Option<char>) {
    let shown = |orig: usize| q.order.iter().position(|&o| o == orig).map(letter);
    (shown(q.answer).unwrap_or('?'), chosen.and_then(shown))
}

#[component]
pub fn QuestionView(
    q: Question,
    /// What he did with it on the paper: struck options, highlights, time.
    a: Answer,
    /// The option shown as picked: the draft (tutor) or the answer.
    chosen: Option<usize>,
    reveal: bool,
    /// The question's place in its paper, for the stem's highlights.
    at: usize,
    /// Called with the original option index; `None` makes the options read-only.
    onchoose: Option<EventHandler<usize>>,
    /// A long press or a secondary click on an option (original index).
    #[props(default)]
    onstrike: Option<EventHandler<usize>>,
    /// Keep the explanation's place before the answer is out (a tutor
    /// paper), so the question never narrows when it comes (UX-18).
    #[props(default)]
    reserve: bool,
) -> Element {
    let Some((lecture, item)) = lookup(&q.id) else {
        return rsx! { p { class: "note", {tx::NOT_FOUND} } };
    };
    let Body::Mcq { stem, options, why, .. } = &item.body else {
        return rsx! { p { class: "note", {tx::NOT_FOUND} } };
    };
    let topic = item.topic.as_deref().map(|t| topic_title(&lecture.lecture, t));
    let ui = use_context::<crate::Ui>();
    let rows = q.order.iter().enumerate().filter_map(|(shown, &orig)| {
        let text = options.get(orig)?.clone();
        let reason = why.get(orig).cloned().unwrap_or_default();
        let state = answer_state(reveal, orig == q.answer, chosen == Some(orig));
        Some((orig, letter(shown), text, reason, state, a.struck.contains(&orig)))
    });
    let rows: Vec<_> = rows.collect();
    let locked = onchoose.is_none();
    let quote = item.src.quote.clone();
    let primary = rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Article, class: "q",
            "data-course": steps_core::program::course_of(&item.id),
            if reveal {
                Verdict { q: q.clone(), chosen, took_ms: a.took_ms }
                // The lecture and topic say what the question tested, so
                // they wait for the answer: the topic is often the diagnosis
                // the stem asks for (#743), the lecture a cue in a mixed
                // block. The lecture chip opens the lecture's page.
                div { class: "q-meta",
                    button {
                        class: "chip tap",
                        title: tx::OPEN_LECTURE,
                        onclick: { let id = lecture.lecture.clone(); move |_| crate::lecture::open(ui, &id) },
                        bdi { "{crate::content::Library::get().crumb(&lecture.lecture)}" }
                    }
                    if let Some(t) = topic {
                        span { class: "chip", bdi { "{t}" } }
                    }
                }
            }
            Stem { text: stem.clone(), highlights: a.highlights.clone(), at }
            crate::vq::Figure { item_id: item.id.clone() }
            ol { class: "q-options", lang: "en", dir: "ltr",
                for (orig, letter, text, reason, state, struck) in rows.clone() {
                    li { key: "{orig}",
                        button {
                            class: "q-option answer",
                            "data-state": state,
                            "data-struck": if struck { "true" } else { "false" },
                            disabled: locked,
                            "aria-pressed": if chosen == Some(orig) { "true" } else { "false" },
                            // A–E; F marks the question (bar.rs).
                            "aria-keyshortcuts": if locked || letter > 'E' { String::new() } else { letter.to_string() },
                            "data-peek": if reveal { reason.clone() } else { String::new() },
                            onclick: move |_| {
                                if let Some(h) = onchoose {
                                    h.call(orig);
                                }
                            },
                            // A long press (touch) or a secondary click strikes it out.
                            oncontextmenu: move |e: MouseEvent| {
                                if let Some(h) = onstrike {
                                    e.prevent_default();
                                    h.call(orig);
                                }
                            },
                            span { class: "answer-key", "{letter}" }
                            span { class: "q-text answer-text", "{text}" }
                            if struck {
                                span { class: "bank-sr", lang: "ar", dir: "rtl", "، {tx::STRUCK}" }
                            }
                            // The state in words, so it never rests on colour alone;
                            // under 840dp the words are read out only (bank.css).
                            if let Some((icon, words)) = mark(reveal, state) {
                                span { class: "answer-mark", lang: "ar", dir: "rtl",
                                    "{icon}"
                                    span { class: "answer-mark-words", " {words}" }
                                }
                            }
                        }
                        // Under each option on phones; the side pane has them on wide windows.
                        if reveal && !reason.is_empty() {
                            p { class: "q-why", "{reason}" }
                        }
                    }
                }
            }
        }
    };
    if !reveal {
        if reserve {
            return rsx! {
                crate::desk::Panes { primary, secondary: None, split: SPLIT }
            };
        }
        return primary;
    }
    // The explanation: why for every option (wide windows), the objective,
    // the book page or the VQ number, the ✓ / ✎ / ✗ check and any update.
    let secondary = rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Div, class: "q-explain",
            h2 { class: "q-explain-head", {tx::EXPLANATION} }
            ol { class: "q-why-list", lang: "en", dir: "ltr",
                for (orig, letter, _, reason, state, _) in rows {
                    li { key: "{orig}", "data-state": if matches!(state, "correct" | "missed") { "right" } else { state },
                        span { class: "q-letter", "{letter}" }
                        if matches!(state, "correct" | "missed") {
                            span { class: "q-why-mark", lang: "ar", dir: "rtl", "✓ {tx::RIGHT}" }
                        } else if state == "wrong" {
                            span { class: "q-why-mark", lang: "ar", dir: "rtl", "✗ {tx::YOUR_ANSWER}" }
                        }
                        span { "{reason}" }
                    }
                }
            }
            // A VQ question's educational objective, after its why lines,
            // then the video that explains it.
            crate::vq::Objective { item_id: item.id.clone() }
            crate::vq::VideoLink { item_id: item.id.clone() }
            div { class: "q-foot",
                // A question bank's item is named by its number, not a page.
                match crate::vq::source_label(&item) {
                    Some(label) => rsx! { crate::vq::SourceChip { label } },
                    None => rsx! { crate::hover::BookPage { page: item.src.page, quote, of: item.id.clone() } },
                }
            }
            // A miss shows the cards that hold what it tested.
            if chosen != Some(q.answer) {
                super::behind::Behind { lecture: lecture.lecture.clone(), item: item.clone() }
            }
            Report { item_id: q.id.clone() }
            if let Some(u) = &item.update {
                aside { class: "q-update",
                    strong { {tx::UPDATE} }
                    p { lang: "en", dir: "ltr", "{u.text}" }
                    p { class: "muted small", bdi { "{u.reference}" } " · " {tx::UPDATE_NOTE} }
                }
            }
        }
    };
    rsx! {
        crate::desk::Panes { primary, secondary: Some(secondary), split: SPLIT }
    }
}

/// «إجابة غلط · اخترت C، والإجابة الصح B · الوقت 1:12» over an answered
/// question. Its words are on screen; the player's live region reads the
/// verdict out once.
#[component]
fn Verdict(q: Question, chosen: Option<usize>, took_ms: u32) -> Element {
    let a = Answer { chosen, ..Answer::default() };
    let graded = a.graded(&q);
    let (right, picked) = letters(&q, chosen);
    let (tone, word, icon) = match graded {
        Graded::Right => ("good", tx::VERDICT_RIGHT, "✓"),
        Graded::Wrong => ("bad", tx::VERDICT_WRONG, "✗"),
        Graded::Blank => ("blank", tx::VERDICT_BLANK, "–"),
    };
    rsx! {
        // Brought into view below the bar when part of it is hidden (UX-01).
        div { class: "q-verdict", "data-tone": tone, onmounted: move |_| super::run::verdict(),
            span { class: "q-verdict-icon", aria_hidden: "true", "{icon}" }
            strong { {word} }
            span { class: "q-verdict-picked", {tx::picked(picked, right)} }
            if took_ms > 0 {
                span { class: "q-verdict-time muted small",
                    "{tx::TOOK} " bdi { class: "num-ltr", {super::clock::face(u64::from(took_ms))} }
                }
            }
        }
    }
}

/// The question's share where the explanation stands beside it: half, so
/// the explanation is no narrow column (bank.css's 50%, UX-18).
const SPLIT: Option<u8> = Some(50);

/// An answered option's mark: its icon and its words.
fn mark(reveal: bool, state: &str) -> Option<(char, &'static str)> {
    match (reveal, state) {
        (true, "correct") => Some(('✓', tx::RIGHT_PICKED)),
        (true, "missed") => Some(('✓', tx::RIGHT_ANSWER)),
        (true, "wrong") => Some(('✗', tx::YOUR_ANSWER)),
        _ => None,
    }
}

/// The shared `button.answer` state (docs/DESIGN.md): a draft is
/// "selected"; once revealed the right option is "correct" when he picked
/// it and "missed" when he didn't, and his wrong pick is "wrong".
fn answer_state(reveal: bool, right: bool, picked: bool) -> &'static str {
    match (reveal, right, picked) {
        (true, true, true) => "correct",
        (true, true, false) => "missed",
        (true, false, true) => "wrong",
        (false, _, true) => "selected",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::answer_state;

    #[test]
    fn answer_states_follow_the_shared_part() {
        assert_eq!(answer_state(false, true, true), "selected");
        assert_eq!(answer_state(false, false, false), "");
        assert_eq!(answer_state(true, true, true), "correct");
        assert_eq!(answer_state(true, true, false), "missed");
        assert_eq!(answer_state(true, false, true), "wrong");
        assert_eq!(answer_state(true, false, false), "");
    }
}

/// ✓ / ✎ / ✗ the first time he sees the question (the review store's verify
/// bar), and afterwards a "report a problem" link that reopens that check.
#[component]
fn Report(item_id: String) -> Element {
    let review = use_review();
    let mut reopened = use_signal(|| false);
    let check = review.backend.borrow().check(&item_id);
    if check == Check::Ask || reopened() {
        return rsx! { crate::verify::VerifyBar { item_id } };
    }
    if check == Check::Hidden {
        return rsx! {};
    }
    rsx! {
        button {
            class: "btn-link small report",
            onclick: move |_| {
                review.backend.borrow_mut().undo_judgement(&item_id);
                reopened.set(true);
            },
            {tx::REPORT}
        }
    }
}
