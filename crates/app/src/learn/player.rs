//! The lesson player: one step at a time, a segmented progress bar, Back and
//! Continue at the foot. Questions must be answered (or skipped) before
//! Continue; only first tries count. The end screen recaps the lesson and
//! hands its practice items to a study session. The layout follows Batna's
//! lesson player (adapted from batna-app main: crates/app/src/lesson.rs).

use dioxus::prelude::*;
use steps_core::lesson::{Lesson, Step, StepBody};

use steps_core::response::{Attempt, Draft, StepState, sig};

use super::practice::Tries;
use super::steps::{AnsweredBefore, CheckBar, Gate, Responses, StepAt, StepView};
use super::text::Line;
use super::{Mark, VIEW, View, close, mark, next_lesson, save_mark, set_view};
use crate::i18n::learn as tx;
use crate::{Screen, Ui};

/// Where an unfinished lesson stands (`Attempt`): the step, every answer
/// and note, and the score, so it reopens exactly there, on this device or
/// another one (`Store::put_place`, synced).
pub(super) fn pos_key(id: &str) -> String {
    format!("lessonpos.{id}")
}

fn save_pos(id: &str, pos: &Attempt) {
    if let Err(e) = crate::store::shared().put_place(&pos_key(id), pos) {
        crate::diag::report!("saving lesson position {id} failed: {e}");
    }
}

/// Swiping between steps, with the page following the finger (swipe.js).
const SWIPE_JS: &str = include_str!("swipe.js");

#[component]
pub fn Player(lesson: Lesson, lecture_title: String) -> Element {
    let ui = use_context::<Ui>();
    // The lesson gets the whole phone screen; × (data-back) is the way out.
    crate::parts::use_focus_mode(true);
    crate::hover::use_course_hover(lesson.id.split('.').next().unwrap_or_default());
    let n = lesson.steps.len();
    let id = lesson.id.clone();
    // The saved attempt, restored before any step decides Check or Continue.
    let start = use_hook(|| {
        let now = crate::store::now_ms();
        // STEPS_LESSON_STEP=4 opens at that step (1-based), for screenshots.
        let preview = std::env::var("STEPS_LESSON_STEP").ok().and_then(|s| s.parse::<usize>().ok());
        if let Some(s) = preview {
            return Attempt { at: s.saturating_sub(1).min(n), ..Attempt::new(n, now) };
        }
        let saved: Option<Attempt> = crate::store::shared().place(&pos_key(&id));
        Attempt::restore(&lesson, saved, mark(&id).map(|m| m.done_ms), now)
    });
    let sigs = use_hook(|| lesson.steps.iter().map(|s| sig(&s.body)).collect::<Vec<_>>());
    let mut attempt = use_signal(|| start.clone());
    let mut at = use_signal(|| start.at);
    // Saves where he is now: after every response and every graded card,
    // not only on Back/Continue, since the lesson can close mid-step (the
    // page chip's lecture, the rail).
    let save_now = {
        let id = id.clone();
        move || {
            if *at.peek() < n {
                let mut a = attempt.peek().clone();
                a.at = *at.peek();
                save_pos(&id, &a);
            }
        }
    };
    use_context_provider({
        let save_now = save_now.clone();
        move || Tries { attempt, save: Callback::new(move |_| save_now()) }
    });
    use_context_provider({
        let save_now = save_now.clone();
        move || Responses {
            attempt,
            put: Callback::new(move |(k, d): (usize, Draft)| {
                let sig = sigs.get(k).cloned().unwrap_or_default();
                attempt.write().put(k, sig, d);
                save_now();
            }),
        }
    });
    let check = use_signal(|| vec![Gate::default(); n]);
    use_context_provider(|| CheckBar(check));
    // Steps stay mounted once seen (hidden when not current), so Back and
    // the page view keep a step as he left it, and never grade it again.
    let mut seen = use_signal(|| (0..n).map(|k| k == start.at).collect::<Vec<_>>());
    // Questions answered before their responses were saved, and Try steps
    // answered in an earlier session (their grades are reviews already):
    // both show a done note instead of the task.
    let before: Vec<Option<bool>> = (0..n)
        .map(|k| match start.state(&lesson, k) {
            StepState::AnsweredBefore => start.answers[k],
            StepState::Done if matches!(lesson.steps[k].body, StepBody::Try { .. }) => Some(true),
            _ => None,
        })
        .collect();
    // Which way the last move went, for the enter animation.
    let mut dir = use_signal(|| "fwd");
    use_hook(|| {
        document::eval(SWIPE_JS);
    });
    super::highlight::use_highlights(&lesson.id);
    let view = VIEW();
    let pages = view == View::Pages;
    // The reading choices (learn/reading.rs) and whether their popover is open.
    let reading = ui.prefs.read().reading;
    let [(_, read_size), (_, read_spacing), (_, read_width)] = super::reading::attrs(reading);
    let read_open = use_signal(|| false);
    let question_steps: Vec<bool> = lesson.steps.iter().map(|step| step.body.is_question()).collect();

    let go = {
        let id = id.clone();
        let lesson = lesson.clone();
        move |to: usize| {
            dir.set(if to < *at.peek() { "back" } else { "fwd" });
            at.set(to);
            if to < n {
                seen.write()[to] = true;
            }
            if to == n {
                let (right, asked) = attempt.peek().score(&lesson);
                save_mark(&id, Mark { done_ms: crate::store::now_ms(), right, asked });
                save_pos(&id, &Attempt::default());
                crate::sounds::play(crate::sounds::Cue::SessionEnd);
            } else {
                let mut a = attempt.peek().clone();
                a.at = to;
                save_pos(&id, &a);
            }
            document::eval(
                "document.querySelector('.content')?.scrollTo({top: 0}); window.scrollTo({top: 0});",
            );
        }
    };

    if at() >= n {
        let (right, asked) = attempt.read().score(&lesson);
        return rsx! { Summary { lesson: lesson.clone(), right, asked } };
    }

    let i = at();
    let state = attempt.read().state(&lesson, i);
    let can_go = state.can_continue();
    let gate = check.read().get(i).cloned().unwrap_or_default();
    let ready = gate.check;
    let needs_answer = !can_go && ready.is_none();
    let mut go_next = go.clone();
    let mut go_back = go.clone();
    let mut go_skip = go.clone();

    rsx! {
        div {
            class: "ls-player",
            "data-view": if pages { "pages" } else { "steps" },
            "data-dir": dir(),
            "data-read-size": read_size,
            "data-read-spacing": read_spacing,
            "data-read-width": read_width,
            header { class: "ls-bar",
                button {
                    class: "ls-close",
                    "data-back": "true",
                    // Escape closes the reading popover first, when it is open.
                    "aria-keyshortcuts": if read_open() { None } else { Some("Escape") },
                    aria_label: tx::CLOSE,
                    title: tx::CLOSE,
                    onclick: move |_| close(ui),
                    "×"
                }
                StepMeter { current: i + 1, question_steps }
                span { class: "muted small ls-count num-ltr", "{i + 1}/{n}" }
                super::reading::ReadingButton { open: read_open }
                button {
                    class: "ls-view",
                    aria_label: if pages { tx::TO_STEPS } else { tx::TO_PAGES },
                    title: if pages { tx::TO_STEPS } else { tx::TO_PAGES },
                    onclick: move |_| set_view(if pages { View::Steps } else { View::Pages }),
                    {if pages { crate::icons::steps() } else { crate::icons::book() }}
                }
            }
            // Name the lesson independently of the exercise step below.
            header { class: "ls-context",
                h1 { class: "ls-context-title", Line { text: lesson.title.clone() } }
                p { class: "ls-context-lecture muted small",
                    bdi { dir: "ltr", lang: "en", "{lecture_title}" }
                }
            }
            // B&B and UWorld for this lecture, at the start of the lesson.
            if i == 0 {
                crate::qbank::LessonStrip { lecture: super::lecture_of(&lesson.id).to_string() }
            }
            for (k , st) in lesson.steps.iter().enumerate().filter(|(k, _)| seen.read()[*k]) {
                Stage {
                    key: "{k}",
                    k,
                    current: k == i,
                    step: st.clone(),
                    course: lesson.id.split('.').next().unwrap_or_default().to_string(),
                    seed: format!("{}#{}", lesson.id, k),
                    before: before[k],
                    quote_held: attempt.read().quote_held(&lesson, k),
                    retry: {
                        let st_k = attempt.read().state(&lesson, k);
                        let q = st.body.is_question();
                        (q && st_k == StepState::Done, q && st_k == StepState::AnsweredBefore)
                    },
                    on_retry: {
                        let sig = sig(&st.body);
                        let save_now = save_now.clone();
                        move |_| {
                            attempt.write().put(k, sig.clone(), Draft::None);
                            save_now();
                        }
                    },
                    on_answer: {
                        let save_now = save_now.clone();
                        move |right: bool| {
                            // Only the first result counts (Attempt::answer).
                            if attempt.write().answer(k, right) {
                                save_now();
                            }
                        }
                    },
                }
            }
            footer { class: "ls-nav",
                button {
                    class: if pages { "ls-arrow" } else { "btn btn-secondary" },
                    "data-nav": "back",
                    // Right-to-left: the earlier step is on the right.
                    "aria-keyshortcuts": "ArrowRight",
                    aria_label: tx::BACK,
                    title: tx::BACK,
                    disabled: i == 0,
                    onclick: move |_| go_back(i - 1),
                    if pages {
                        {crate::icons::chevron(false)}
                    } else {
                        {tx::BACK}
                    }
                }
                if needs_answer {
                    button {
                        class: "btn-link ls-skip",
                        onclick: move |_| {
                            if let Some(s) = attempt.write().skipped.get_mut(i) {
                                *s = true;
                            }
                            go_skip(i + 1);
                        },
                        {tx::SKIP}
                    }
                } else if pages {
                    span { class: "ls-page-of muted small", "{tx::PAGE_OF} {i + 1} {tx::OF_N} {n}" }
                }
                if let Some(run) = ready {
                    button {
                        class: "btn btn-primary btn-lg ls-next",
                        "data-nav": "check",
                        "aria-keyshortcuts": "Enter",
                        onclick: move |_| run.call(()),
                        {tx::CHECK_IT}
                    }
                } else if pages && i + 1 < n {
                    button {
                        class: "ls-arrow ls-arrow-next",
                        "data-nav": "next",
                        "aria-keyshortcuts": "Enter ArrowLeft",
                        aria_label: tx::NEXT,
                        title: tx::NEXT,
                        disabled: !can_go,
                        aria_describedby: if needs_answer { Some("ls-gate-hint") } else { None },
                        onclick: move |_| go_next(i + 1),
                        {crate::icons::chevron(true)}
                    }
                } else {
                    button {
                        class: "btn btn-primary btn-lg ls-next",
                        "data-nav": "next",
                        "aria-keyshortcuts": "Enter ArrowLeft",
                        disabled: !can_go,
                        aria_describedby: if needs_answer { Some("ls-gate-hint") } else { None },
                        onclick: move |_| go_next(i + 1),
                        {if i + 1 == n { tx::FINISH } else { tx::NEXT }}
                    }
                }
                // Screen readers only: the lesson page stays free of helper notes
                // (Mohamed, 2026-10-29); the Skip button is the visible way on.
                if needs_answer {
                    p { id: "ls-gate-hint", class: "ls-sr-only", {tx::NAV_REQUIRED} }
                }
            }
        }
    }
}

/// Visual progress indicates the current step, not completion or a score.
#[component]
fn StepMeter(current: usize, question_steps: Vec<bool>) -> Element {
    let total = question_steps.len();
    rsx! {
        div {
            class: "ls-segs",
            role: "progressbar",
            aria_label: tx::PROGRESS_LABEL,
            aria_valuemin: "1",
            aria_valuemax: "{total}",
            aria_valuenow: "{current}",
            aria_valuetext: tx::step_of(current, total),
            for (k , is_question) in question_steps.iter().enumerate() {
                span {
                    key: "{k}",
                    class: "ls-seg",
                    "data-q": if *is_question { "yes" } else { "no" },
                    "data-state": if k + 1 < current { "done" } else if k + 1 == current { "now" } else { "todo" },
                }
            }
        }
    }
}

/// One step's card. It stays mounted after he leaves it, hidden.
#[component]
fn Stage(
    k: usize,
    current: bool,
    step: Step,
    /// The lesson's course (`cvs`), for its own glossary meanings.
    course: String,
    seed: String,
    /// Answered before its response was saved: Some(right) (Try steps: done).
    before: Option<bool>,
    /// The book quote waits until he has answered (`response::quote_held`).
    quote_held: bool,
    /// (checked now, answered before): a question he can do again.
    retry: (bool, bool),
    on_retry: EventHandler<()>,
    on_answer: EventHandler<bool>,
) -> Element {
    use_context_provider(|| StepAt(k));
    let try_done = before.is_some() && matches!(step.body, StepBody::Try { .. });
    // "حاول تاني": the step starts over as a fresh task. Practice only: the
    // first result stays the one that counts (Attempt::answer).
    let mut round = use_signal(|| 0u32);
    let mut fresh = use_signal(|| false);
    let (checked, answered_before) = retry;
    // A case has its own ↻ under every checked part (mini_case.rs), so the
    // step-wide one only shows for a record opened with no saved parts.
    let per_part = matches!(step.body, StepBody::Case { .. });
    let can_retry =
        if per_part { before.is_some() && !fresh() } else { checked || (answered_before && !fresh()) };
    let texts = use_hook(|| step.shown_text());
    rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Article, class: "ls-stage", hidden: !current, "data-kind": step.body.kind(), "data-n": k + 1,
            if try_done {
                div { class: "ls-try-done",
                    p { class: "ls-kicker", {tx::TRY} }
                    p { {tx::TRY_DONE} }
                }
            } else if before.is_some() && !fresh() {
                AnsweredBefore { body: step.body.clone() }
            } else {
                for r in std::iter::once(round()) {
                    StepView { key: "{r}", body: step.body.clone(), seed: seed.clone(), on_answer }
                }
            }
            // A تحشيشة this topic already has, from this lesson or another.
            if !try_done && !step.remind.is_empty() {
                super::remind::Reminders { remind: step.remind.clone(), here: seed.split('#').next().unwrap_or_default().to_string() }
            }
            if can_retry && !try_done {
                button {
                    class: "icon-button ls-retry",
                    aria_label: tx::RETRY,
                    title: tx::RETRY,
                    onclick: move |_| {
                        on_retry.call(());
                        fresh.set(true);
                        *round.write() += 1;
                    },
                    span { aria_hidden: "true", "↻" }
                }
            }
            // The page chip, with the step's abbreviations and his note beside it.
            footer { class: "ls-src",
                crate::abbrsheet::AbbrChip { course: course.clone(), texts: texts.clone() }
                if let Some(page) = step.page {
                    crate::hover::BookPage {
                        page,
                        quote: if quote_held { None } else { step.quote.clone() },
                        held: quote_held && step.quote.is_some(),
                        of: seed.clone(),
                        hook: step_hook(&step.body),
                    }
                    crate::weak::StepMark { of: seed.clone(), page }
                }
                crate::notes::NoteChip { anchor: steps_core::note::Anchor::Lesson(seed.clone()), page: step.page }
            }
        }
    }
}

/// A completed lesson always offers exactly one primary next action.
fn summary_return_class(has_follow_up: bool) -> &'static str {
    if has_follow_up { "btn-link" } else { "btn btn-primary" }
}

#[component]
fn Summary(lesson: Lesson, right: u32, asked: u32) -> Element {
    let mut ui = use_context::<Ui>();
    let lecture = lesson.id.rsplit_once('.').map(|(l, _)| l.to_string()).unwrap_or_default();
    let next = next_lesson(std::slice::from_ref(&lecture)).filter(|l| l.id != lesson.id);
    let practice = lesson.practice.clone();
    let n_practice = practice.len();
    let return_class = summary_return_class(n_practice > 0 || next.is_some());
    rsx! {
        crate::ui::Card { class: "ls-summary",
            p { class: "ls-confetti", aria_hidden: "true", "🎉" }
            // The finished button is gone: land focus on the result.
            h1 {
                tabindex: "-1",
                onmounted: move |e: MountedEvent| async move {
                    let _ = e.set_focus(true).await;
                },
                {tx::DONE_TITLE}
            }
            p { class: "muted", Line { text: lesson.title.clone() } }
            if asked > 0 {
                p { class: "ls-score",
                    strong { "{right}/{asked}" }
                    " {tx::FIRST_TRY}"
                }
                p { class: "small", {tx::score_line(right, asked)} }
            }
            h2 { {tx::RECAP} }
            ul { class: "ls-recap",
                for (k , r) in lesson.recap.iter().enumerate() {
                    li { key: "{k}", Line { text: r.clone() } }
                }
            }
            div { class: "ls-summary-actions",
                if n_practice > 0 {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            *crate::study::START.write() = Some(crate::study::Start {
                                lectures: None,
                                items: Some(practice.clone()),
                                minutes: crate::onboarding::session_minutes(),
                            });
                            *super::OPEN.write() = None;
                            ui.screen.set(Screen::Study);
                        },
                        "{tx::PRACTICE} ({n_practice})"
                    }
                }
                if let Some(l) = next {
                    button {
                        class: if n_practice > 0 { "btn btn-secondary" } else { "btn btn-primary" },
                        onclick: move |_| super::open(ui, &l.id),
                        span { class: "ls-btn-label",
                            "{tx::NEXT_LESSON}: "
                            Line { text: l.title.clone() }
                        }
                    }
                }
                button { class: return_class, "data-back": "true", "aria-keyshortcuts": "Escape", onclick: move |_| close(ui), {tx::BACK_TO_LIST} }
            }
            if n_practice > 0 {
            }
        }
    }
}

/// Focused navigation regressions: no study records or score changes.
/// A mnemonic step's تحشيشة and letters, for its page chip's sheet.
fn step_hook(body: &steps_core::lesson::StepBody) -> Option<crate::pagesheet::Hook> {
    match body {
        steps_core::lesson::StepBody::Mnemonic { parts, hook, .. } => Some(crate::pagesheet::Hook {
            text: hook.clone().unwrap_or_default(),
            parts: parts.iter().map(|p| (p.key.clone(), p.text.clone())).collect(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod ux_tests {
    use super::*;

    #[test]
    fn step_progress_announces_position_and_question_steps() {
        fn app() -> Element {
            rsx! { StepMeter { current: 2, question_steps: vec![false, true, false] } }
        }
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for attribute in [
            "role=\"progressbar\"",
            "aria-label=\"موضعك في الدرس\"",
            "aria-valuemin=\"1\"",
            "aria-valuemax=\"3\"",
            "aria-valuenow=\"2\"",
            "aria-valuetext=\"خطوة 2 من 3\"",
            "data-q=\"yes\"",
            "data-state=\"done\"",
            "data-state=\"now\"",
        ] {
            assert!(html.contains(attribute), "missing {attribute}: {html}");
        }
        assert_eq!(html.matches("class=\"ls-seg\"").count(), 3, "{html}");
    }

    #[test]
    fn gated_navigation_describes_the_available_skip() {
        let source = include_str!("player.rs");
        let production = source.split("/// Focused navigation regressions").next().unwrap();
        assert!(production.contains("let needs_answer = !can_go && ready.is_none()"));
        assert!(production.contains("aria_describedby: if needs_answer"));
        assert!(production.contains("id: \"ls-gate-hint\", class: \"ls-sr-only\""));
        assert!(tx::NAV_REQUIRED.contains(tx::SKIP));
    }

    #[test]
    fn last_lesson_has_a_primary_exit_without_practice_or_next_lesson() {
        assert_eq!(summary_return_class(false), "btn btn-primary");
        assert_eq!(summary_return_class(true), "btn-link");
    }

    #[test]
    fn summary_android_back_and_compact_touch_targets() {
        let source = include_str!("player.rs");
        let summary = source
            .split("fn Summary(")
            .nth(1)
            .unwrap()
            .split("/// Focused navigation regressions")
            .next()
            .unwrap();
        assert!(summary.contains("\"data-back\": \"true\""));
        assert!(summary.contains("onclick: move |_| close(ui)"));
        let css = include_str!("learn.css");
        assert!(css.contains(
            ".ls-nav :is(.btn, .btn-link), .ls-summary-actions button { min-block-size: max(var(--target), var(--tap)); }"
        ));
        assert!(css.contains(".ls-nav .btn-lg { min-block-size: max(var(--target), var(--tap-lg)); }"));
    }

    #[test]
    fn in_step_controls_keep_48dp_targets() {
        let css = include_str!("learn.css");
        let rule = |sel: &str| -> String {
            let at = css.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("no rule for {sel}"));
            css[at..].split('}').next().unwrap().to_string()
        };
        for sel in [".ls-tile", ".ls-bin-head", ".ls-cell-hidden"] {
            assert!(rule(sel).contains("min-block-size: var(--target);"), "{sel}");
        }
        assert!(css.contains(".ls-chiplet { cursor: pointer; min-block-size: var(--target); }"));
        // Short English terms stay on one line in prose, but must wrap inside
        // a sort bin or card, or they spill out of it on a phone.
        assert!(css.contains(".ls-bin-name .ls-nw { white-space: normal; }"));
        assert!(css.contains(":is(.ls-tile, .ls-chiplet, .ls-chiplet-done) .ls-nw { white-space: normal; }"));
        assert!(css.contains(".ls-undo { min-block-size: var(--target); }"));
        assert!(
            css.contains(
                ".ls-viewer-zoom .ls-arrow { inline-size: var(--target); block-size: var(--target);"
            )
        );
        assert!(rule(".ls-retry").contains("min-block-size: max(var(--target), var(--tap));"));
        // No 36/40/44px tap sizes left in the lesson player.
        for small in ["min-height: 36px", "min-height: 44px", "min-block-size: 44px", "block-size: 44px"] {
            assert!(!css.contains(small), "{small}");
        }
    }
}
