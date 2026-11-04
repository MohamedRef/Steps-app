//! One component per lesson step kind. Each takes the step and calls
//! `on_answer(right)` once when a question step is answered (the player
//! counts first tries of question steps only), or `on_answer(true)` when a
//! reveal opens (that only unlocks Continue).

mod table;
use table::*;

mod mini_case;
use mini_case::*;

mod sorting;
use sorting::*;

mod matching;
use matching::*;

mod order;
use order::*;

mod check;
use check::*;

mod presentation;
use presentation::*;

use dioxus::prelude::*;
use steps_core::content::CaseStep;
use steps_core::lesson::{MnemonicPart, Picture, SortCard, StepBody, Tone};
use steps_core::response::{Attempt, Draft};

use super::text::{Line, Rich, is_latin};
use crate::i18n::learn as tx;
use crate::order::grade::{misplaced, shuffled};

mod decision;
use decision::{answer_state, is_value};

const LETTERS: [&str; 5] = ["A", "B", "C", "D", "E"];

/// The player's footer gate, one slot per step (visited steps stay
/// mounted). A step whose response is complete but not checked yet puts its
/// check here; the footer's primary then reads "صحّح" and runs it: one
/// primary, always in the same place.
#[derive(Clone, Copy)]
pub struct CheckBar(pub Signal<Vec<Gate>>);

#[derive(Clone, Default, PartialEq)]
pub struct Gate {
    pub check: Option<Callback<()>>,
}

/// The index of the step a component belongs to.
#[derive(Clone, Copy)]
pub struct StepAt(pub usize);

/// The open attempt's saved responses (`steps_core::response`) and a way to
/// keep a step's response. Every interactive step starts from its saved
/// response and writes each change back, so leaving and coming back (Back,
/// the page view, a rotation, a restart) shows exactly what he did.
#[derive(Clone, Copy)]
pub struct Responses {
    pub attempt: Signal<Attempt>,
    pub put: Callback<(usize, Draft)>,
}

/// The step's saved response (taken once, at mount) and a setter.
fn use_draft() -> (Draft, Callback<Draft>) {
    let r = try_use_context::<Responses>();
    let k = try_use_context::<StepAt>().map_or(0, |s| s.0);
    let saved = use_hook(|| {
        r.and_then(|r| r.attempt.peek().drafts.get(k).map(|s| s.draft.clone())).unwrap_or_default()
    });
    let put = use_callback(move |d: Draft| {
        if let Some(r) = r {
            r.put.call((k, d));
        }
    });
    (saved, put)
}

/// Wires a step to the footer: while `can_check` holds, the footer's
/// primary runs `check`.
pub(super) fn use_check_bar(can_check: impl Fn() -> bool + 'static, check: impl FnMut(()) + 'static) {
    let Some(CheckBar(mut slots)) = try_use_context::<CheckBar>() else { return };
    let k = try_use_context::<StepAt>().map_or(0, |s| s.0);
    let check = use_callback(check);
    use_effect(move || {
        let v = Gate { check: can_check().then_some(check) };
        if slots.peek().get(k).is_some_and(|g| *g != v) {
            slots.write()[k] = v;
        }
    });
    use_drop(move || {
        if let Some(slot) = slots.write().get_mut(k) {
            *slot = Gate::default();
        }
    });
}

#[component]
pub fn StepView(body: StepBody, seed: String, on_answer: EventHandler<bool>) -> Element {
    match body {
        StepBody::Explain { title, text } => rsx! {
            if let Some(t) = title { h2 { class: "ls-h", Line { text: t } } }
            Rich { text }
        },
        StepBody::Figure { title, svg, image, caption, alt } => rsx! {
            Figure { title, svg, image, caption, alt }
        },
        StepBody::Reveal { prompt, answer } => rsx! { Reveal { prompt, answer, on_answer } },
        StepBody::Mnemonic { word, parts, hook } => rsx! { Mnemonic { word, parts, hook } },
        StepBody::Aside { tone, text } => rsx! { Aside { tone, text } },
        StepBody::Update { text, reference } => rsx! {
            div { class: "ls-aside", "data-tone": "update",
                p { class: "ls-aside-label", {tx::UPDATE} }
                Rich { text }
                p { class: "muted small", bdi { dir: "ltr", lang: "en", "{reference}" } }
                p { class: "small", {tx::BOOK_WINS} }
            }
        },
        StepBody::Check { question, options, answer, why } => rsx! {
            Check { question, options, answer, why, seed, on_answer }
        },
        StepBody::Order { prompt, steps, why } => rsx! { Order { prompt, steps, why, seed, on_answer } },
        StepBody::Match { prompt, pairs, why } => rsx! { Match { prompt, pairs, why, seed, on_answer } },
        StepBody::Sort { prompt, bins, cards, why } => {
            rsx! { Sort { prompt, bins, cards, why, seed, on_answer } }
        }
        StepBody::Case { stem, steps } => rsx! { MiniCase { stem, steps, seed, on_answer } },
        StepBody::Table { title, columns, rows, hide } => rsx! { Table { title, columns, rows, hide } },
        StepBody::Try { items } => rsx! { super::practice::TryItems { items, on_answer } },
    }
}

fn cue(right: bool) {
    crate::sounds::play(if right { crate::sounds::Cue::Right } else { crate::sounds::Cue::Wrong });
}

/// A question answered before its response was saved (an older version of
/// the app), or whose content changed since: the answer key, read-only,
/// with "حاول تاني" under it (player.rs). Never an empty task with Continue
/// already on, and no explanatory notes (Mohamed, 2026-10-29).
#[component]
pub fn AnsweredBefore(body: StepBody) -> Element {
    let (kicker, prompt) = match &body {
        StepBody::Check { question, .. } => (tx::CHECK, question.clone()),
        StepBody::Order { prompt, .. } => (tx::ORDER, prompt.clone()),
        StepBody::Match { prompt, .. } => (tx::MATCH, prompt.clone()),
        StepBody::Sort { prompt, .. } => (tx::SORT, prompt.clone()),
        StepBody::Case { stem, .. } => (tx::CASE, stem.clone()),
        _ => (tx::CHECK, String::new()),
    };
    let key = match body {
        StepBody::Check { options, answer, why, .. } => rsx! {
            p { class: "ls-key-line", Line { text: options.get(answer).cloned().unwrap_or_default() } }
            p { class: "ls-why small", Line { text: why.get(answer).cloned().unwrap_or_default() } }
        },
        StepBody::Order { steps, why, .. } => rsx! {
            ol { class: "ls-key small", dir: "ltr", lang: "en",
                for (i , s) in steps.iter().enumerate() { li { key: "{i}", "{s}" } }
            }
            if let Some(w) = why { div { class: "ls-why-box", Rich { text: w } } }
        },
        StepBody::Match { pairs, why, .. } => rsx! {
            ul { class: "ls-key small", dir: "ltr", lang: "en",
                for (i , p) in pairs.iter().enumerate() { li { key: "{i}", bdi { "{p[0]}" } " → " bdi { "{p[1]}" } } }
            }
            if let Some(w) = why { div { class: "ls-why-box", Rich { text: w } } }
        },
        StepBody::Sort { bins, cards, why, .. } => rsx! {
            ul { class: "ls-key small",
                for (b , name) in bins.iter().enumerate() {
                    li { key: "{b}",
                        strong { Line { text: name.clone() } }
                        ": "
                        for (i , c) in cards.iter().enumerate().filter(|(_, c)| c.bin == b) {
                            span { key: "{i}", class: "ls-key-item", Line { text: c.text.clone() } }
                        }
                    }
                }
            }
            if let Some(w) = why { div { class: "ls-why-box", Rich { text: w } } }
        },
        StepBody::Case { steps, .. } => rsx! {
            ol { class: "ls-key small", dir: "ltr", lang: "en",
                for (si , s) in steps.iter().enumerate() {
                    li { key: "{si}",
                        strong { Line { text: s.question.clone() } }
                        if let Some(a) = s.answer.and_then(|a| s.options.get(a)) {
                            " → "
                            Line { text: a.clone() }
                        }
                    }
                }
            }
        },
        _ => rsx! {},
    };
    rsx! {
        div { class: "ls-before",
            p { class: "ls-kicker", {kicker} }
            div { class: "ls-q", Rich { text: prompt } }
            p { class: "ls-kicker", {tx::KEY} }
            {key}
        }
    }
}
