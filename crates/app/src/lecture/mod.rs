//! One lecture's page: what it covers, how far along it is, and every way in
//! (a session on it, what's due, its lessons, its questions, its cases, a
//! session on one topic). Every lecture row in the app opens it, so a row
//! is never a dead end.
//!
//! It opens over whichever screen it came from and closes with Back, the
//! page's own back button, or any move to another screen. Pages stack: a
//! lecture opened from the references list ([`refs`]) goes back to it.

pub mod refs;

use dioxus::prelude::*;
use steps_core::review::StudyStore;
use steps_core::stats::WeakTopic;

use crate::content::{Library, lecture_no};
use crate::i18n::lecture as tx;
use crate::look::{Css, css};
use crate::stats::{StudyData, pct};
use crate::store::{now_ms, use_review};
use crate::{Screen, Ui};

pub(crate) const CSS: Css = css!("src/lecture/lecture.css");

/// The open pages (last on top) and the screen they opened over. A page is a
/// lecture id or [`REFERENCES`]. `STEPS_OPEN_LECTURE=cvs.L04` (or
/// `references`) opens one at launch over the first screen (previews,
/// screenshots).
static OPEN: GlobalSignal<Option<(Vec<String>, Screen)>> = Signal::global(|| None);

/// The page id of the references list.
pub const REFERENCES: &str = "references";

/// Opens a lecture's page over the current screen, on top of any page
/// already open there.
pub fn open(ui: Ui, id: &str) {
    let screen = *ui.screen.peek();
    let mut open = OPEN.write();
    match open.as_mut() {
        Some((stack, on)) if *on == screen => {
            if stack.last().map(String::as_str) != Some(id) {
                stack.push(id.to_string());
            }
        }
        _ => *open = Some((vec![id.to_string()], screen)),
    }
    drop(open);
    to_top();
}

/// Opens the references list (More › المراجع) over the current screen.
pub fn open_references(ui: Ui) {
    *OPEN.write() = Some((vec![REFERENCES.to_string()], *ui.screen.peek()));
    to_top();
}

/// Closes the top page: back to the page under it, or to the screen.
pub fn close() {
    let mut open = OPEN.write();
    if let Some((stack, _)) = open.as_mut() {
        stack.pop();
        if stack.is_empty() {
            *open = None;
        }
    }
    drop(open);
    to_top();
}

/// Closes every page (a session is starting).
pub fn close_all() {
    *OPEN.write() = None;
    to_top();
}

fn to_top() {
    document::eval(r#"document.querySelector("main.content")?.scrollTo(0, 0);"#);
}

/// The page to show over `screen`, if one is open there.
pub fn shown(screen: Screen) -> Option<String> {
    OPEN.read().as_ref().filter(|(_, on)| *on == screen).and_then(|(stack, _)| stack.last().cloned())
}

/// Whether a lecture page is open (Back closes it before leaving the screen).
pub fn is_open() -> bool {
    OPEN.read().is_some()
}

/// Wires the page to the shell: the launch preview, and closing it once
/// the screen it opened over is left.
pub fn use_lecture_page(ui: Ui) {
    use_hook(move || {
        if let Ok(id) = std::env::var("STEPS_OPEN_LECTURE") {
            *OPEN.write() = Some((vec![id], *ui.screen.peek()));
        }
    });
    use_effect(move || {
        let now = *ui.screen.read();
        let stale = OPEN.peek().as_ref().is_some_and(|(_, on)| *on != now);
        if stale {
            *OPEN.write() = None;
        }
    });
}

/// Item ids for a session on one topic of a lecture (`None`: its untagged
/// items): what he has seen first, then the rest, rejected items left out.
pub fn topic_session(data: &StudyData, lecture: &str, topic: Option<String>) -> Vec<String> {
    let t = WeakTopic {
        lecture: lecture.to_string(),
        topic,
        title: String::new(),
        score: 0.0,
        retention: Default::default(),
        mcq: Default::default(),
    };
    steps_core::stats::topics_session(&data.lectures, &[t], &data.events, &data.marks)
}

/// Starts a study session: on exactly `items` when given, else on the
/// lecture with the usual time budget.
fn start(mut ui: Ui, lecture: &str, items: Option<Vec<String>>) {
    // Over Study itself the screen doesn't change, so close the pages here.
    close_all();
    *crate::study::START.write() = Some(crate::study::Start {
        lectures: Some(vec![lecture.to_string()]),
        items,
        minutes: crate::onboarding::session_minutes(),
    });
    ui.screen.set(Screen::Study);
}

#[component]
pub fn LecturePage(id: String) -> Element {
    if id == REFERENCES {
        return rsx! { refs::References {} };
    }
    if id == crate::courses::MAP {
        return rsx! { crate::courses::Map {} };
    }
    if id == crate::abbrdict::PAGE {
        return rsx! { crate::abbrdict::Dictionary {} };
    }
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let review = use_review();
    let Some(entry) = lib.index.lectures.iter().find(|e| e.id == id) else {
        return rsx! {};
    };
    let lecture = lib.lectures.get(&id);
    let lessons = lib.lessons.get(&id);
    let crate::parts::ViewPick { layout: lesson_layout, sort: lesson_sort, .. } =
        crate::learn::LECTURE_PAGE_VIEW.pick();
    let data = StudyData::load(&*review.backend.borrow());
    let stats = data.stats().into_iter().find(|l| l.id == id);
    let (due, playable, mcqs, cases) = {
        let b = review.backend.borrow();
        let prefix = format!("{id}.");
        let due: Vec<String> =
            b.session_due(now_ms()).into_iter().filter(|i| i.starts_with(&prefix)).collect();
        let items = lecture.map(|l| l.items.as_slice()).unwrap_or_default();
        let count = |kind: &[&str]| {
            items.iter().filter(|i| kind.contains(&i.body.kind()) && !b.is_hidden(&i.id)).count()
        };
        (due, count(crate::study::PLAYABLE), count(&["mcq"]), count(&["case"]))
    };
    let done_lessons =
        lessons.map_or(0, |f| f.lessons.iter().filter(|l| crate::learn::mark(&l.id).is_some()).count());
    let [first, last] = entry.pages;
    let number = lecture_no(&id);
    let course = lib.course_short(&id).to_string();
    let (seen, total) = stats.as_ref().map_or((0, 0), |s| (s.counts.seen, s.counts.total));
    let share = (seen * 100).checked_div(total).unwrap_or(0);
    let due_n = due.len();

    rsx! {
        style { {crate::learn::CSS.text()} }
        style { {CSS.text()} }
        div { class: "lec-page",
            div { class: "lec-top",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: tx::BACK,
                    title: tx::BACK,
                    onclick: move |_| close(),
                    span { aria_hidden: "true", "→" }
                }
                // Course › lecture: the course opens the course map.
                nav { class: "muted small lec-crumbs", aria_label: tx::KICKER,
                    button { class: "btn-link", title: crate::i18n::courses::TITLE, onclick: move |_| crate::courses::open_map(ui), "{course}" }
                    " › {tx::KICKER} {number}"
                }
            }
            crate::ui::Card { class: "lec-hero",
                div { class: "lec-head",
                    span { class: "num", "{number}" }
                    div { class: "lecture-main",
                        h1 { bdi { class: "lecture-title", "{entry.title}" } }
                        span { class: "muted small",
                            if lib.program.slides(&id) { "{tx::SLIDES} " } else { "{tx::PAGES} " }
                            bdi { dir: "ltr", "{first}–{last}" }
                        }
                    }
                }
                if lecture.is_some() {
                    div {
                        class: "progress",
                        "data-tone": "good",
                        role: "progressbar",
                        aria_label: tx::PROGRESS,
                        aria_valuemin: "0",
                        aria_valuemax: "{total}",
                        aria_valuenow: "{seen}",
                        div { class: "progress-fill", style: "inline-size: {share}%" }
                    }
                    if let Some(c) = stats.as_ref().map(|s| &s.counts) {
                        p { class: "small lec-check",
                            span {
                                class: "chip",
                                "data-tone": if c.approved >= c.total && c.total > 0 { "good" } else { "" },
                                "{tx::CHECKED} " span { class: "num-ltr", "{c.approved}/{c.total}" }
                            }
                        }
                    }
                    p { class: "muted small",
                        "{seen} {tx::OF} {total} {tx::STARTED}"
                        if due_n > 0 { " · " span { class: "lec-due", "{due_n} {tx::DUE}" } }
                    }
                    if !lib.program.slides(&id) {
                        PagesRead { first, last }
                    }
                    // When reviews are due, give them the one primary action.
                    // A full-lecture study session is still available.
                    div { class: "lec-actions",
                        if due_n > 0 {
                            button {
                                class: "btn btn-primary",
                                onclick: { let id = id.clone(); move |_| start(ui, &id, Some(due.clone())) },
                                "{tx::REVIEW_DUE} ({due_n})"
                            }
                        }
                        if playable > 0 {
                            button {
                                class: if due_n > 0 { "btn btn-tonal" } else { "btn btn-primary" },
                                onclick: { let id = id.clone(); move |_| start(ui, &id, None) },
                                {tx::STUDY}
                            }
                        }
                    }
                } else {
                    p { class: "note", {tx::WRITING} }
                }
            }

            if let Some(file) = lessons {
                section { class: "lec-section",
                    h2 {
                        {tx::LESSONS}
                        span { class: "muted small", " · {done_lessons} {tx::OF} {file.lessons.len()}" }
                    }
                    crate::parts::ViewBar { of: crate::learn::LECTURE_PAGE_VIEW }
                    ol { class: "coll ls-cards", "data-layout": lesson_layout.as_str(),
                        for (n , l) in crate::learn::numbered(&file.lessons, lesson_sort) {
                            crate::learn::LessonCard { key: "{l.id}", n, lesson: l.clone() }
                        }
                    }
                }
            }

            if mcqs > 0 || cases > 0 {
                section { class: "lec-section",
                    h2 { {tx::PRACTICE} }
                    div { class: "lec-ways",
                        if mcqs > 0 {
                            button {
                                class: "tap lec-way",
                                onclick: { let id = id.clone(); move |_| crate::bank::practise(ui, &id, None) },
                                span { class: "lec-way-n", "{mcqs}" }
                                span { class: "lec-way-label", {tx::MCQS} }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                        if cases > 0 {
                            button {
                                class: "tap lec-way",
                                onclick: move |_| { let mut ui = ui; ui.screen.set(Screen::Cases) },
                                span { class: "lec-way-n", "{cases}" }
                                span { class: "lec-way-label", {tx::CASES} }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }

            if let Some(s) = stats.filter(|s| !s.topics.is_empty()) {
                section { class: "lec-section",
                    h2 { {tx::TOPICS} }
                    ul { class: "tap-list",
                        for t in s.topics.iter() {
                            {
                                // Cards and the like start a session; a topic with only
                                // MCQs opens the bank on it instead.
                                let ids: Vec<String> =
                                    topic_session(&data, &id, t.id.clone()).into_iter().filter(|i| crate::study::is_playable(i)).collect();
                                let topic = t.id.clone();
                                let lecture_id = id.clone();
                                rsx! {
                                    li { key: "{t.title}",
                                        button {
                                            class: "tap tap-row",
                                            onclick: move |_| {
                                                if ids.is_empty() {
                                                    crate::bank::practise(ui, &lecture_id, topic.clone());
                                                } else {
                                                    start(ui, &lecture_id, Some(ids.clone()));
                                                }
                                            },
                                            span { class: "tap-row-main",
                                                if t.id.is_some() {
                                                    bdi { dir: "ltr", class: "lecture-title", "{t.title}" }
                                                } else {
                                                    span { class: "lecture-title", {crate::i18n::stats::UNTAGGED} }
                                                }
                                                span { class: "muted small",
                                                    "{t.counts.seen}/{t.counts.total} {tx::STARTED} · {crate::i18n::stats::COL_RETENTION} "
                                                    bdi { {pct(t.counts.retention)} }
                                                }
                                            }
                                            span { class: "chev", aria_hidden: "true" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What of the lecture's First Aid pages is read, and the ways to mark
/// them (A2): the whole lecture, or every page up to one.
#[component]
fn PagesRead(first: u32, last: u32) -> Element {
    let review = use_review();
    let read = review.backend.borrow().pages().all().within(first, last);
    let n: u32 = read.iter().map(|[a, b]| b - a + 1).sum();
    let all = last - first + 1;
    let mut upto = use_signal(|| read.last().map_or(first, |r| r[1]));
    let backend = review.backend.clone();
    let mark = move |a: u32, b: u32, on: bool| {
        let backend = backend.clone();
        move |_| {
            backend.borrow_mut().set_pages_read(Library::get(), a, b, on);
        }
    };
    rsx! {
        div { class: "lec-read", role: "group", aria_label: tx::READ_GROUP,
            p { class: "small",
                if n == 0 {
                    span { class: "muted", {tx::READ_NONE} }
                } else {
                    {tx::read_count(n, all)}
                    " · "
                    bdi { dir: "ltr", class: "num-ltr", {steps_core::pages::label(&read)} }
                }
            }
            div { class: "lec-read-actions",
                if n < all {
                    button { class: "btn btn-ghost", onclick: mark(first, last, true), {tx::READ_ALL} }
                } else {
                    button { class: "btn btn-ghost", onclick: mark(first, last, false), {tx::UNREAD_ALL} }
                }
                label { class: "lec-read-upto",
                    span { {tx::READ_UPTO} }
                    input {
                        r#type: "number",
                        inputmode: "numeric",
                        min: "{first}",
                        max: "{last}",
                        value: "{upto}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().trim().parse::<u32>() {
                                upto.set(v.clamp(first, last));
                            }
                        },
                    }
                }
                button {
                    class: "btn btn-tonal",
                    onclick: {
                        let backend = review.backend.clone();
                        move |_| {
                            backend.borrow_mut().set_pages_read(Library::get(), first, upto(), true);
                        }
                    },
                    {tx::READ_UPTO_GO}
                }
            }
        }
    }
}
