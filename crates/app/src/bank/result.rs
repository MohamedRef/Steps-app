//! The end of a paper: its score (n / m and %), the time used of the time
//! allowed and per question, tiles for right, wrong, blank and marked, a
//! table of the questions, the score per course (system or discipline), the
//! weakest topics, and the two next steps: review the wrong ones, or a new
//! block of the wrong and marked ones. A row, a tile or «راجع الأسئلة
//! الغلط» opens the review (`review.rs`) on what it shows.

use dioxus::prelude::*;
use steps_core::exam::{self, Graded, Score};

use super::clock::face;
use super::review::{Review, Show};
use super::{PHASE, Phase, back_to_setup, retake, to_top, topic_title};
use crate::content::Library;
use crate::i18n::bank as tx;

#[component]
pub fn Result() -> Element {
    let mut review = use_signal(|| None::<(usize, Show)>);
    let ui = use_context::<crate::Ui>();
    let Phase::Done { paper, outcome } = PHASE.read().clone() else {
        return rsx! {};
    };
    if let Some((at, show)) = review() {
        return rsx! {
            Review {
                at,
                show,
                ongo: move |i| review.set(Some((i, show))),
                onclose: move |_| {
                    review.set(None);
                    to_top();
                },
            }
        };
    }
    let timed = paper.mode.timed();
    let s = exam::summary(&paper.questions, &paper.answers, paper.mode, outcome.took_ms);
    let lib = Library::get();
    let first =
        |show: Show| (0..paper.questions.len()).find(|&i| show.takes(&paper.questions[i], &paper.answers[i]));
    let mut open = move |show: Show, from: Option<usize>| {
        if let Some(at) = from {
            review.set(Some((at, show)));
            to_top();
        }
    };
    let missed = first(Show::Missed);
    let can_retake = !exam::retake(&paper.questions, &paper.answers).is_empty();
    let rows: Vec<(usize, Option<String>, String, Graded, u32, bool)> = paper
        .questions
        .iter()
        .zip(&paper.answers)
        .enumerate()
        .map(|(i, (q, a))| {
            (i, crate::vq::name(&q.id), lib.crumb(&q.lecture), a.graded(q), a.took_ms, a.flagged)
        })
        .collect();
    // A VQ question can be found again by its number in quick search.
    let numbered = rows.iter().any(|r| r.1.is_some());
    // Courses in the book's order; any the program doesn't list, last.
    let mut courses: Vec<(String, Score)> = lib
        .program
        .courses
        .iter()
        .filter_map(|c| s.by_course.get(&c.id).map(|score| (c.title.clone(), *score)))
        .collect();
    courses.extend(
        s.by_course
            .iter()
            .filter(|(id, _)| lib.program.course(id).is_none())
            .map(|(id, sc)| (id.clone(), *sc)),
    );
    let weak: Vec<(String, String, Score)> = exam::focus_topics(&outcome, 60)
        .into_iter()
        .take(5)
        .map(|(k, s)| {
            let (l, t) = k.split_once('/').unwrap_or((k, ""));
            (l.to_string(), t.to_string(), s)
        })
        .collect();
    let pct = s.total.percent();
    let tiles = [
        (Show::Right, s.total.right, tx::RIGHT, "good"),
        (Show::Wrong, s.wrong, tx::WRONG, "bad"),
        (Show::Blank, s.blank, tx::TILE_BLANK, ""),
        (Show::Marked, s.marked, tx::MARKED, "gold"),
    ]
    .map(|(show, n, label, tone)| (show, n, label, tone, first(show)));

    rsx! {
        div { class: "sum",
            header { class: "sum-head",
                // The paper is gone: land focus on the result so it is read out.
                h1 {
                    tabindex: "-1",
                    onmounted: move |e: MountedEvent| async move {
                        let _ = e.set_focus(true).await;
                    },
                    if paper.block { {tx::MOCK_TITLE} } else { {tx::RESULT} }
                }
                span { class: "chip", "data-tone": "accent", {tx::kind(paper.block, timed)} }
                span { class: "sum-grow" }
                // The two next steps sit with the score, not under the table.
                if missed.is_some() {
                    button { class: "btn btn-secondary", onclick: move |_| open(Show::Missed, missed), {tx::REVIEW_WRONG} }
                }
                if can_retake {
                    button { class: "btn btn-primary", onclick: move |_| retake(), {tx::RETAKE} }
                } else {
                    button { class: "btn btn-primary", onclick: move |_| back_to_setup(), {tx::NEW_PAPER} }
                }
            }
            div { class: "sum-top",
                crate::ui::Card { class: "sum-score",
                    span { class: "muted small", {tx::RESULT} }
                    div { class: "big-score",
                        bdi { class: "pct", "{pct}%" }
                        bdi { class: "num-ltr muted", "{s.total.right} / {s.total.total}" }
                    }
                    p { class: "small",
                        {tx::used_of(&face(s.used_ms), s.allowed_ms.map(face).as_deref())}
                        // The answers' own times: a blank is no answer's time.
                        " · {tx::PER_QUESTION} "
                        bdi { class: "num-ltr", {s.per_question_ms.map_or_else(|| tx::NO_TIME.to_string(), face)} }
                    }
                }
                // Each tile opens the review on what it counts.
                div { class: "stat-tiles sum-tiles",
                    for (show, n, label, tone, from) in tiles {
                        button {
                            key: "{label}",
                            class: "tap stat-tile",
                            "data-tone": tone,
                            disabled: n == 0,
                            onclick: move |_| open(show, from),
                            span { class: "stat-num", "{n}" }
                            span { class: "stat-label", {label} }
                        }
                    }
                }
            }
            if s.wrong > 0 {
                p { class: "small", {tx::SENT_TO_REVIEW} }
            }
            div { class: "sum-cols",
                crate::ui::Card { class: "sum-table-card",
                    h2 { {tx::TABLE} }
                    div { class: "sum-table-wrap",
                        table { class: "sum-table",
                            thead {
                                tr {
                                    th { scope: "col", {tx::COL_NO} }
                                    // Only VQ questions have an id to show.
                                    if numbered {
                                        th { scope: "col", {tx::COL_ID} }
                                    }
                                    th { scope: "col", {tx::COL_LECTURE} }
                                    th { scope: "col", {tx::COL_RESULT} }
                                    th { scope: "col", {tx::COL_TIME} }
                                }
                            }
                            tbody {
                                for (i, label, crumb, graded, took, marked) in rows {
                                    tr {
                                        key: "{i}",
                                        class: "sum-row",
                                        "data-graded": match graded { Graded::Right => "right", Graded::Wrong => "wrong", Graded::Blank => "blank" },
                                        "data-flag": if marked { "yes" } else { "no" },
                                        onclick: move |_| open(Show::All, Some(i)),
                                        td {
                                            button {
                                                class: "sum-open",
                                                "aria-label": tx::open_question(i + 1),
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    open(Show::All, Some(i));
                                                },
                                                bdi { class: "num-ltr", "{i + 1}" }
                                                if marked {
                                                    span { class: "sum-flag", aria_hidden: "true", {crate::icons::flag()} }
                                                    span { class: "bank-sr", "، {tx::MARKED}" }
                                                }
                                            }
                                        }
                                        if numbered {
                                            td { if let Some(label) = label { bdi { dir: "ltr", lang: "en", "{label}" } } }
                                        }
                                        td { bdi { "{crumb}" } }
                                        td { class: "sum-result",
                                            match graded {
                                                Graded::Right => rsx! { "✓ {tx::RIGHT}" },
                                                Graded::Wrong => rsx! { "✗ {tx::WRONG}" },
                                                Graded::Blank => rsx! { {tx::TILE_BLANK} },
                                            }
                                        }
                                        // A question never opened has no time, not 0:00.
                                        td {
                                            if took == 0 {
                                                span { class: "muted", aria_hidden: "true", title: tx::NOT_OPENED, {tx::NO_TIME} }
                                                span { class: "bank-sr", {tx::NOT_OPENED} }
                                            } else {
                                                bdi { class: "num-ltr", {face(u64::from(took))} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    p { class: "muted small",
                        {tx::TABLE_HINT}
                        if numbered { " " {tx::TABLE_HINT_VQ} }
                    }
                }
                crate::ui::Card { class: "sum-courses-card",
                    h2 { {tx::BY_COURSE} }
                    ul { class: "sum-courses",
                        for (title, score) in courses {
                            li { key: "{title}",
                                div { class: "sum-course-head",
                                    bdi { lang: "en", "{title}" }
                                    bdi { class: "num-ltr", "{score.right}/{score.total}" }
                                }
                                div { class: "score-bar", aria_hidden: "true",
                                    div { class: "score-fill", style: "inline-size: {score.percent()}%;" }
                                }
                            }
                        }
                    }
                }
            }
            // What to revise next: a weak topic opens the bank set to it.
            if !weak.is_empty() || pct >= 60 {
                crate::ui::Card { class: "sum-weak",
                    h2 { class: "label", {tx::WEAK} }
                    // Praise only a score that earns it (#233).
                    if weak.is_empty() {
                        p { class: "muted small", {tx::ALL_STRONG} }
                    }
                    div { class: "chips",
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
            div { class: "start-row",
                button {
                    class: "btn btn-secondary",
                    // Android Back on the result returns to the bank.
                    "data-back": "true",
                    onclick: move |_| back_to_setup(),
                    {tx::BACK}
                }
            }
        }
    }
}
