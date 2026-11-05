//! After a paper: the score, per lecture and per topic, the weakest topics,
//! and a review of every question with its explanation.

use dioxus::prelude::*;
use steps_core::content::Body;
use steps_core::exam::{self, Mode, Score};

use super::question::QuestionView;
use super::{PHASE, Phase, back_to_setup, lookup, minutes, start, topic_title};
use crate::i18n::bank as tx;
use crate::i18n::count::{MINUTE, count};

#[derive(Clone, Copy, PartialEq)]
enum Show {
    All,
    Wrong,
    Flagged,
}

#[component]
pub fn Result() -> Element {
    let Phase::Done { paper, outcome } = PHASE.read().clone() else {
        return rsx! {};
    };
    let mut show =
        use_signal(|| if outcome.total.right < outcome.total.total { Show::Wrong } else { Show::All });
    let timed = paper.mode.timed();
    let ui = use_context::<crate::Ui>();

    let lectures: Vec<(String, Score)> = outcome.by_lecture.iter().map(|(l, s)| (l.clone(), *s)).collect();
    let topics: Vec<(String, String, Score)> = outcome
        .by_topic
        .iter()
        .map(|(k, s)| {
            let (l, t) = k.split_once('/').unwrap_or((k, ""));
            (l.to_string(), t.to_string(), *s)
        })
        .collect();
    let weak: Vec<(String, String, Score)> = exam::focus_topics(&outcome, 60)
        .into_iter()
        .take(5)
        .map(|(k, s)| {
            let (l, t) = k.split_once('/').unwrap_or((k, ""));
            (l.to_string(), t.to_string(), s)
        })
        .collect();
    let several_lectures = lectures.len() > 1;

    let wrong: Vec<_> = paper
        .questions
        .iter()
        .zip(&paper.answers)
        .filter(|(q, a)| !a.correct(q))
        .map(|(q, _)| q.clone())
        .collect();
    let has_wrong = !wrong.is_empty();
    let filter = *show.read();
    let rows: Vec<_> = paper
        .questions
        .iter()
        .zip(&paper.answers)
        .enumerate()
        .filter(|(_, (q, a))| match filter {
            Show::All => true,
            Show::Wrong => !a.correct(q),
            Show::Flagged => a.flagged,
        })
        .map(|(i, (q, a))| (i, q.clone(), a.clone()))
        .collect();
    let any_flagged = paper.answers.iter().any(|a| a.flagged);
    let mut open = use_signal(|| None::<usize>);
    let detail = open()
        .and_then(|i| Some((i, paper.questions.get(i)?.clone(), paper.answers.get(i)?.clone())))
        .map(|(i, q, a)| {
            rsx! {
                div { class: "review-detail",
                    div { class: "review-detail-head",
                        span { class: "run-count", "{tx::QUESTION} " bdi { "{i + 1}" } }
                        button {
                            class: "btn-link bank-close",
                            "aria-keyshortcuts": "Escape",
                            // Android Back closes the question before leaving the result.
                            "data-back": "true",
                            onclick: move |_| open.set(None),
                            {tx::CLOSE}
                        }
                    }
                    QuestionView { key: "{i}", q, chosen: a.chosen, reveal: true }
                }
            }
        });

    rsx! {
        crate::ui::Card { class: "result",
            // Submit is gone: land focus on the result so it is read out.
            h1 {
                tabindex: "-1",
                onmounted: move |e: MountedEvent| async move {
                    let _ = e.set_focus(true).await;
                },
                if paper.mode == Mode::Mock { {tx::MOCK_TITLE} } else { {tx::RESULT} }
            }
            div { class: "big-score",
                bdi { class: "pct", "{outcome.total.percent()}%" }
                span { class: "muted", bdi { "{outcome.total.right}" } " {tx::OF} " bdi { "{outcome.total.total}" } }
            }
            div { class: "muted small",
                if timed {
                    "{tx::TOOK} " {count(minutes(outcome.took_ms), &MINUTE)}
                }
                if outcome.unanswered > 0 {
                    " · " bdi { "{outcome.unanswered}" } " {tx::UNANSWERED}"
                }
            }
            if has_wrong {
                p { class: "small", {tx::SENT_TO_REVIEW} }
            }
            // The one next step sits with the score, not under the review list.
            div { class: "start-row",
                if has_wrong {
                    button {
                        class: "btn btn-primary btn-lg",
                        onclick: { let wrong = wrong.clone(); move |_| start(Mode::Tutor, wrong.clone()) },
                        {tx::RETRY_WRONG}
                    }
                } else {
                    button { class: "btn btn-primary btn-lg", onclick: move |_| back_to_setup(), {tx::NEW_PAPER} }
                }
            }
        }

        if several_lectures {
            crate::ui::Card { class: "",
                h2 { {tx::BY_LECTURE} }
                ScoreRows {
                    rows: lectures.into_iter().map(|(l, s)| (crate::content::Library::get().crumb(&l), s, l, None)).collect::<Vec<_>>(),
                }
            }
        }
        if !topics.is_empty() {
            crate::ui::Card { class: "",
                h2 { {tx::BY_TOPIC} }
                ScoreRows {
                    rows: topics.into_iter().map(|(l, t, s)| (topic_title(&l, &t), s, l, Some(t))).collect::<Vec<_>>(),
                }
                h2 { class: "label", {tx::WEAK} }
                // Praise only a score that earns it; otherwise point at the
                // review list below (#233).
                if weak.is_empty() && outcome.total.percent() >= 60 {
                    p { class: "muted small", {tx::ALL_STRONG} }
                } else if weak.is_empty() {
                } else {
                    div { class: "chips",
                        // A weak topic opens the bank set to practise just it.
                        for (l, t, s) in weak {
                            button {
                                key: "{l}/{t}",
                                class: "chip weak tap",
                                title: tx::PRACTISE_TOPIC,
                                onclick: { let (l, t) = (l.clone(), t.clone()); move |_| super::practise(ui, &l, Some(t.clone())) },
                                bdi { {topic_title(&l, &t)} }
                                " · " bdi { "{s.right}/{s.total}" }
                            }
                        }
                    }
                }
            }
        }

        crate::ui::Card { class: "",
            h2 { {tx::REVIEW} }
            div { class: "chips",
                for (value, label) in [(Show::All, tx::SHOW_ALL), (Show::Wrong, tx::SHOW_WRONG), (Show::Flagged, tx::SHOW_FLAGGED)] {
                    if value != Show::Flagged || any_flagged {
                        button {
                            class: "chip toggle",
                            "aria-pressed": if filter == value { "true" } else { "false" },
                            onclick: move |_| show.set(value),
                            {label}
                        }
                    }
                }
            }
        }
        crate::desk::ListDetail {
            list: rsx! {
                div { class: "review-list",
                    for (i, q, a) in rows {
                        button {
                            key: "{i}",
                            class: "review-item",
                            "data-right": if a.correct(&q) { "yes" } else { "no" },
                            "aria-current": if open() == Some(i) { "true" } else { "false" },
                            onclick: move |_| open.set(Some(i)),
                            span { class: "mark", aria_hidden: "true", if a.correct(&q) { "✓" } else { "✗" } }
                            span { "{tx::QUESTION} " bdi { "{i + 1}" } }
                            span { class: "bank-sr", if a.correct(&q) { "، {tx::RIGHT}" } else { "، {tx::WRONG}" } }
                            bdi { class: "stem-peek", {stem_peek(&q.id)} }
                            if a.flagged {
                                span { class: "muted", aria_hidden: "true", " ⚑" }
                                span { class: "bank-sr", "، {tx::FLAGGED}" }
                            }
                        }
                    }
                }
            },
            detail: detail,
        }

        div { class: "start-row",
            button {
                class: "btn btn-secondary",
                // Android Back on the result returns to the bank; an open
                // question's Close takes it first.
                "data-back": if open().is_none() { "true" } else { "false" },
                onclick: move |_| back_to_setup(),
                {tx::BACK}
            }
        }
    }
}

/// Score per lecture (a row opens the lecture's page) or per topic (a row
/// opens the bank set to that topic).
#[component]
fn ScoreRows(rows: Vec<(String, Score, String, Option<String>)>) -> Element {
    let ui = use_context::<crate::Ui>();
    rsx! {
        ul { class: "score-rows",
            for (i, (label, s, lecture, topic)) in rows.into_iter().enumerate() {
                li { key: "{i}",
                    button {
                        class: "tap tap-plain score-row",
                        title: if topic.is_some() { tx::PRACTISE_TOPIC } else { tx::OPEN_LECTURE },
                        onclick: move |_| match &topic {
                            Some(t) => super::practise(ui, &lecture, Some(t.clone())),
                            None => crate::lecture::open(ui, &lecture),
                        },
                        // The cell stays right to left with the name inline in
                        // it, so an English topic starts at the row's edge like
                        // a lecture does, not beside the bar (UX #586).
                        span { class: "score-label", bdi { "{label}" } }
                        div { class: "score-bar", aria_hidden: "true",
                            div { class: "score-fill", style: "inline-size: {s.percent()}%;" }
                        }
                        bdi { class: "score-num", "{s.right}/{s.total}" }
                    }
                }
            }
        }
    }
}

fn stem_peek(id: &str) -> String {
    match lookup(id).map(|(_, i)| i.body) {
        Some(Body::Mcq { stem, .. }) => stem.chars().take(90).collect(),
        _ => String::new(),
    }
}
