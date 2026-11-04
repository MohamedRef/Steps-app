//! Learn (الدروس): short interactive lessons that teach each lecture before
//! its cards are reviewed. Explanations in Egyptian Arabic, drawn figures,
//! mnemonics and small checks between them (`steps_core::lesson`).
//!
//! The list shows every lecture's lessons; a lesson opens in the player
//! (`player.rs`). Finishing one is saved as a synced setting
//! `lesson.<id>` ([`Mark`]), and its practice items start a study session.

#[cfg(test)]
mod browser;
mod player;
mod practice;
mod steps;
mod text;
pub use text::Rich;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use steps_core::lesson::Lesson;

use crate::content::Library;
use crate::i18n::count::{MINUTE, QUESTION, count};
use crate::i18n::learn as tx;
use crate::look::{Css, css};
use crate::parts::{Collection, Layout, Sort, ViewBar, sort_by};
use crate::{Screen, Ui};

pub(crate) const CSS: Css = css!("src/learn/learn.css");

/// The lesson open in the player; set it, then switch to [`Screen::Learn`].
/// `STEPS_OPEN_LESSON=cvs.L04.lesson1` opens one at launch (previews, screenshots).
pub static OPEN: GlobalSignal<Option<String>> = Signal::global(|| std::env::var("STEPS_OPEN_LESSON").ok());

/// Where the open lesson was opened from: closing it goes back there.
static FROM: GlobalSignal<Screen> = Signal::global(|| Screen::Learn);

/// Following another lesson should keep the original return destination.
fn lesson_origin(current: Screen, has_open_lesson: bool, previous: Screen) -> Screen {
    if has_open_lesson && current == Screen::Learn { previous } else { current }
}

/// Opens a lesson from anywhere (Today, a study item, the planner).
pub fn open(mut ui: Ui, id: &str) {
    let origin = lesson_origin(*ui.screen.peek(), OPEN.peek().is_some(), *FROM.peek());
    *FROM.write() = origin;
    *OPEN.write() = Some(id.to_string());
    ui.screen.set(Screen::Learn);
}

/// Closes the player, back to where the lesson was opened from.
pub fn close(mut ui: Ui) {
    *OPEN.write() = None;
    let from = std::mem::replace(&mut *FROM.write(), Screen::Learn);
    if from != Screen::Learn {
        ui.screen.set(from);
    }
}

/// Lets go of the open lesson without switching screen, for a jump
/// elsewhere (a book page's "study this page"): the Learn tab then shows the
/// list, not a half-finished lesson.
pub fn forget() {
    *OPEN.write() = None;
    *FROM.write() = Screen::Learn;
}

/// Enter a chosen course's catalogue without a stale search or selected
/// lecture; completion data and saved list-view preferences remain intact.
pub(crate) fn reset_course_browse() {
    forget();
    *SHOWN.write() = None;
    QUERY.write().clear();
}

/// A finished lesson, saved as the synced setting `lesson.<id>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub done_ms: i64,
    /// Questions right on the first try, out of `asked`.
    pub right: u32,
    pub asked: u32,
}

const PREFIX: &str = "lesson.";

/// Bumped when a lesson is finished or a sync lands, so lists re-read the marks.
static MARKS_REV: GlobalSignal<u64> = Signal::global(|| 0);

pub fn mark(id: &str) -> Option<Mark> {
    let _ = MARKS_REV();
    crate::store::shared().setting(&format!("{PREFIX}{id}"))
}

fn save_mark(id: &str, m: Mark) {
    if let Err(e) = crate::store::shared().put_setting(&format!("{PREFIX}{id}"), &m) {
        crate::diag::report!("saving lesson mark {id} failed: {e}");
    }
    reload();
}

/// Re-reads the marks and the lesson view (after a sync or a restore).
pub fn reload() {
    *MARKS_REV.write() += 1;
    let fresh = load_view();
    if *VIEW.peek() != fresh {
        *VIEW.write() = fresh;
    }
}

/// How a lesson is shown: one step at a time with Back and Continue
/// (`Steps`), or as book pages turned by swiping, with small arrows
/// (`Pages`). Swiping works in both. A synced setting, `lesson_view`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    #[default]
    Steps,
    Pages,
}

const VIEW_KEY: &str = "lesson_view";

fn load_view() -> View {
    // STEPS_LESSON_VIEW=pages, for previews and screenshots.
    match std::env::var("STEPS_LESSON_VIEW").as_deref() {
        Ok("pages") => View::Pages,
        Ok("steps") => View::Steps,
        _ => crate::store::shared().setting(VIEW_KEY).unwrap_or_default(),
    }
}

pub static VIEW: GlobalSignal<View> = Signal::global(load_view);

pub fn set_view(v: View) {
    crate::store::save_setting(VIEW_KEY, &v);
    *VIEW.write() = v;
}

/// Settings › Review: Steps or Pages.
#[component]
pub fn ViewSection() -> Element {
    let now = VIEW();
    rsx! {
        crate::ui::Card {
            h2 { {tx::VIEW_TITLE} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::VIEW_TITLE,
                for (v , label , hint) in [(View::Steps, tx::VIEW_STEPS, tx::VIEW_STEPS_HINT), (View::Pages, tx::VIEW_PAGES, tx::VIEW_PAGES_HINT)] {
                    button {
                        key: "{label}",
                        role: "radio",
                        "aria-checked": now == v,
                        onclick: move |_| set_view(v),
                        {label}
                        span { class: "seg-hint", {hint} }
                    }
                }
            }
        }
    }
}

/// The lesson to do next: the first unfinished one of the earliest lecture
/// in `prefer` that has one left, else the first unfinished in book order.
/// The course in view (`courses::focus`) goes first.
pub fn next_lesson(prefer: &[String]) -> Option<&'static Lesson> {
    let lib = Library::get();
    let left = |l: &&Lesson| mark(&l.id).is_none();
    let from = |shown: &dyn Fn(&str) -> bool| {
        prefer
            .iter()
            .filter(|lec| shown(lec))
            .filter_map(|lec| lib.lessons.get(lec))
            .find_map(|f| f.lessons.iter().find(left))
            .or_else(|| lib.all_lessons().filter(|(lec, _)| shown(lec)).map(|(_, l)| l).find(left))
    };
    from(&crate::courses::focus::shows).or_else(|| from(&|_| true))
}

/// The lesson that teaches an item: one that practises it, else the first
/// lesson of its lecture that covers its topic.
pub fn lesson_for_item(item_id: &str) -> Option<&'static Lesson> {
    let lib = Library::get();
    let lecture = item_id.rsplit_once('.').map(|(l, _)| l)?;
    let file = lib.lessons.get(lecture)?;
    let listed = file.lessons.iter().find(|l| {
        l.practice.iter().any(|p| p == item_id)
            || l.steps.iter().any(|s| matches!(&s.body, steps_core::lesson::StepBody::Try { items } if items.iter().any(|i| i == item_id)))
    });
    listed.or_else(|| {
        let topic = practice::find(item_id)?.1.topic.as_deref()?;
        file.lessons.iter().find(|l| l.topics.iter().any(|t| t == topic))
    })
}

#[component]
pub fn Learn() -> Element {
    let open = OPEN();
    rsx! {
        style { {crate::study::STUDY_CSS.text()} }
        style { {CSS.text()} }
        match open.and_then(|id| Library::get().lesson(&id).map(|(lec, l)| (lec, l.clone()))) {
            Some((lecture, lesson)) => rsx! {
                player::Player { lesson, lecture_title: Library::get().title(&lecture.lecture).to_string() }
            },
            None => rsx! { LessonList {} },
        }
    }
}

/// The lecture the list shows: the one picked under "other lectures", else
/// the one with the next lesson. Kept while the app runs.
static SHOWN: GlobalSignal<Option<String>> = Signal::global(|| None);
/// What the search box holds, kept when a lesson opens and closes.
static QUERY: GlobalSignal<String> = Signal::global(String::new);

/// Where an unfinished lesson stands (on any of his devices): the step it reopens at
/// (0-based), read from the player's saved position (`player.rs`).
pub fn resume_step(id: &str) -> Option<usize> {
    part_way(id).map(|(at, _)| at)
}

/// An unfinished lesson's saved attempt on this device: the step it reopens
/// at and when the attempt began (0 in records saved before that was kept).
fn part_way(id: &str) -> Option<(usize, i64)> {
    #[derive(Deserialize)]
    struct At {
        at: usize,
        #[serde(default)]
        started_ms: i64,
    }
    if mark(id).is_some() {
        return None;
    }
    crate::store::shared()
        .place::<At>(&format!("lessonpos.{id}"))
        .filter(|p| p.at > 0)
        .map(|p| (p.at, p.started_ms))
}

/// The item started most recently; the first one wins a tie, so records
/// without a start time fall back to book order.
fn latest<T>(items: impl IntoIterator<Item = (T, i64)>) -> Option<T> {
    items
        .into_iter()
        .fold(None, |best: Option<(T, i64)>, (t, ms)| match best {
            Some((_, b)) if b >= ms => best,
            _ => Some((t, ms)),
        })
        .map(|(t, _)| t)
}

/// The lesson left part-way most recently among `lessons`.
fn left_off<'a>(lessons: impl IntoIterator<Item = &'a Lesson>) -> Option<&'a Lesson> {
    latest(lessons.into_iter().filter_map(|l| part_way(&l.id).map(|(_, ms)| (l, ms))))
}

/// A lesson id's lecture (`cvs.L04.lesson1` → `cvs.L04`).
fn lecture_of(lesson_id: &str) -> &str {
    lesson_id.rsplit_once('.').map(|(lec, _)| lec).unwrap_or(lesson_id)
}

/// A lesson's state in words: finished, part-way, the next one, or new.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Done(Mark),
    Current { at: usize },
    Next,
    New,
}

pub fn state(lesson: &Lesson, next: Option<&str>) -> State {
    if let Some(m) = mark(&lesson.id) {
        State::Done(m)
    } else if let Some(at) = resume_step(&lesson.id) {
        State::Current { at }
    } else if next == Some(lesson.id.as_str()) {
        State::Next
    } else {
        State::New
    }
}

/// Does a lesson match the search? Its title, hook, topics and lecture.
fn matches(q: &str, lecture: &str, lecture_title: &str, l: &Lesson) -> bool {
    let lib = Library::get();
    let code =
        format!("l{:0>2} {}", crate::content::lecture_no(lecture), lib.lecture_tag(lecture)).to_lowercase();
    let course = lib.course_of(lecture).map(|c| c.title.as_str()).unwrap_or_default();
    [l.title.as_str(), l.hook.as_str(), lecture_title, code.as_str(), course]
        .into_iter()
        .chain(l.topics.iter().map(String::as_str))
        .any(|t| t.to_lowercase().contains(q))
}

#[component]
fn LessonList() -> Element {
    use crate::courses::focus;
    let lib = Library::get();
    // Only the course in view (the chip above the title picks it).
    let _ = focus::current();
    let in_view = || lib.all_lessons().filter(|(lec, _)| focus::shows(lec));
    let total = in_view().count();
    let done = in_view().filter(|(_, l)| mark(&l.id).is_some()).count();
    let next = next_lesson(&[]).filter(|l| focus::shows(&l.id));
    let next_id = next.map(|l| l.id.clone());
    let next_lecture = next.map(|l| lecture_of(&l.id).to_string());
    // Where the learner left off: its lecture opens first, and while another
    // lecture is picked a card leads back to it.
    let resume = left_off(in_view().map(|(_, l)| l));
    let resume_lecture = resume.map(|l| lecture_of(&l.id).to_string());
    let shown = SHOWN()
        .filter(|id| lib.lessons.contains_key(id) && focus::shows(id))
        .or(resume_lecture)
        .or(next_lecture)
        .or_else(|| {
            lib.index
                .lectures
                .iter()
                .find(|e| lib.lessons.contains_key(&e.id) && focus::shows(&e.id))
                .map(|e| e.id.clone())
        });
    let query = QUERY();
    let q = query.trim().to_lowercase();
    rsx! {
        header { class: "page-head",
            crate::courses::CourseChip {}
            h1 { {tx::HEAD} }
            if total > 0 {
                p { class: "page-sub", span { class: "ls-nowrap", {tx::done_of(done, total)} } }
            }
        }
        if total > 0 {
            div { class: "ls-tools", crate::abbrdict::OpenButton {} }
        }
        if total == 0 && focus::current().is_none() {
            p { class: "note", {tx::NONE_YET} }
        } else if total > 0 {
            label { class: "search ls-search",
                {crate::icons::search()}
                input {
                    r#type: "search",
                    placeholder: tx::SEARCH,
                    aria_label: tx::SEARCH_LABEL,
                    value: "{query}",
                    oninput: move |e| *QUERY.write() = e.value(),
                }
                if !query.is_empty() {
                    button {
                        class: "ls-search-clear",
                        aria_label: tx::CLEAR_SEARCH,
                        title: tx::CLEAR_SEARCH,
                        onclick: move |_| QUERY.write().clear(),
                        "×"
                    }
                }
            }
        }
        if !q.is_empty() {
            Results { q: q.clone(), next_id: next_id.clone() }
        } else if let Some(shown) = shown {
            if let Some(l) = resume.filter(|l| lecture_of(&l.id) != shown) {
                Resume { id: l.id.clone() }
            }
            Shown { id: shown.clone(), next_id: next_id.clone() }
            p { class: "note small", {tx::UNVERIFIED_NOTE} }
            if done == total && total > 0 {
                p { class: "ls-verdict good", {tx::ALL_LESSONS_DONE} }
            }
            div { class: "section-head", h2 { {tx::OTHER_LECTURES} } }
            Others { shown: shown.clone() }
        } else {
            // The course in view has lectures but no lessons yet: its
            // lectures still open their pages (book pages, cards).
            p { class: "note", {tx::COURSE_NO_LESSONS} }
            Others { shown: String::new() }
        }
    }
}

/// The other lectures' cards: the screen's view switcher picks cards, rows
/// or one line each, and the order.
pub(crate) const LECTURES_VIEW: Collection = Collection {
    key: "lessons.lectures",
    layout: Layout::DEFAULT,
    sorts: &[Sort::Book, Sort::Left, Sort::Name],
    groups: false,
};
/// A lecture's lessons on this screen (the picked lecture, search results).
pub(crate) const LESSONS_VIEW: Collection = Collection {
    key: "lessons.rows",
    layout: Layout::DEFAULT,
    sorts: &[Sort::Book, Sort::Left],
    groups: false,
};
/// A lecture's lessons on its page.
pub(crate) const LECTURE_PAGE_VIEW: Collection = Collection {
    key: "lecture.lessons",
    layout: Layout::DEFAULT,
    sorts: &[Sort::Book, Sort::Left],
    groups: false,
};

/// Every lecture but the shown one. One with lessons moves to the top when
/// picked; one not written yet opens its page (the book's pages and cards).
#[component]
fn Others(shown: String) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let pick = LECTURES_VIEW.pick();
    let mut others: Vec<_> =
        lib.index.lectures.iter().filter(|e| e.id != shown && crate::courses::focus::shows(&e.id)).collect();
    let left =
        |id: &str| lib.lessons.get(id).is_some_and(|f| f.lessons.iter().any(|l| mark(&l.id).is_none()));
    sort_by(&mut others, pick.sort, |e| left(&e.id), |_| false, |e| e.title.clone());
    rsx! {
        ViewBar { of: LECTURES_VIEW }
        for group in lib.by_course(others, |e| e.id.as_str()) {
            crate::parts::CourseHead { key: "{group.id}", short: group.short().to_string(), title: group.title().to_string() }
            ul { class: "coll ls-others", "data-layout": pick.layout.as_str(),
                for entry in group.rows {
                    {
                        let id = entry.id.clone();
                        let code = crate::content::lecture_no(&entry.id);
                        match lib.lessons.get(&entry.id) {
                            Some(file) => {
                                let n = file.lessons.len();
                                let d = file.lessons.iter().filter(|l| mark(&l.id).is_some()).count();
                                let started = file.lessons.iter().any(|l| resume_step(&l.id).is_some());
                                rsx! {
                                    li { key: "{entry.id}",
                                        button {
                                            class: "coll-btn ls-other",
                                            title: tx::SHOW_LECTURE,
                                            onclick: move |_| {
                                                *SHOWN.write() = Some(id.clone());
                                                document::eval(r#"document.querySelector("main.content")?.scrollTo(0, 0);"#);
                                            },
                                            span { class: "coll-lead",
                                                span { class: "coll-badge", "data-tone": if d == n && n > 0 { "good" } else { "" }, "{code}" }
                                            }
                                            span { class: "coll-main",
                                                bdi { class: "coll-title", dir: "ltr", "{entry.title}" }
                                                span { class: "coll-sub",
                                                    BookPages { id: entry.id.clone(), pages: entry.pages }
                                                    " · "
                                                    {tx::lessons_n(n)}
                                                    " · "
                                                    {tx::done_of(d, n)}
                                                    if started {
                                                        " · "
                                                        span { class: "ls-started", {tx::LESSON_STARTED} }
                                                    }
                                                }
                                                span { class: "coll-extra", Meter { done: d, total: n } }
                                            }
                                            span { class: "coll-end coll-when-compact",
                                                crate::parts::Count { done: d, total: n }
                                            }
                                            span { class: "chev", aria_hidden: "true" }
                                        }
                                    }
                                }
                            }
                            None => rsx! {
                                li { key: "{entry.id}",
                                    button {
                                        class: "coll-btn ls-other ls-soon",
                                        title: tx::OPEN_LECTURE,
                                        onclick: move |_| crate::lecture::open(ui, &id),
                                        span { class: "coll-lead",
                                            span { class: "coll-badge", "data-tone": "quiet", "{code}" }
                                        }
                                        span { class: "coll-main",
                                            bdi { class: "coll-title", dir: "ltr", "{entry.title}" }
                                            span { class: "coll-sub",
                                                BookPages { id: entry.id.clone(), pages: entry.pages }
                                                " · "
                                                {tx::OPEN_LECTURE}
                                            }
                                        }
                                        span { class: "coll-end", span { class: "chip", {tx::WRITING} } }
                                        span { class: "chev", aria_hidden: "true" }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Where a lecture or lesson sits in its source: "صفحات 1–22" in the
/// book, or "سلايدز 1–53" for a slide-deck course.
#[component]
pub(crate) fn BookPages(id: String, pages: [u32; 2]) -> Element {
    let label = if Library::get().program.slides(&id) { crate::i18n::lecture::SLIDES } else { tx::PAGES };
    let [first, last] = pages;
    rsx! {
        span { class: "ls-nowrap ls-pages",
            "{label} "
            bdi { dir: "ltr", class: "num-ltr", if first == last { "{first}" } else { "{first}–{last}" } }
        }
    }
}

/// A thin progress line, `done` of `total`.
#[component]
fn Meter(done: usize, total: usize) -> Element {
    let pct = (done * 100).checked_div(total).unwrap_or(0);
    rsx! {
        span {
            class: "progress ls-meter",
            role: "progressbar",
            aria_valuemin: "0",
            aria_valuemax: "{total}",
            aria_valuenow: "{done}",
            span { class: "progress-fill", style: "inline-size: {pct}%" }
        }
    }
}

/// The picked lecture: a header with its progress and the way into its
/// next lesson, then every lesson as a row.
#[component]
fn Shown(id: String, next_id: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let Some(file) = lib.lessons.get(&id) else { return rsx! {} };
    let title = lib.title(&id).to_string();
    let kicker = tx::lecture_n(lib.course_short(&id), &crate::content::lecture_no(&id));
    let n = file.lessons.len();
    let d = file.lessons.iter().filter(|l| mark(&l.id).is_some()).count();
    let pct = (d * 100).checked_div(n).unwrap_or(0);
    // This lecture's way in: the lesson left part-way most recently, else
    // the first unfinished.
    let go = left_off(&file.lessons).or_else(|| file.lessons.iter().find(|l| mark(&l.id).is_none()));
    let go_at = go.and_then(|l| resume_step(&l.id)).zip(go.map(|l| l.steps.len()));
    let go_id = go.map(|l| l.id.clone());
    let cta_id = go_id.clone();
    let next = next_id.as_deref();
    let pick = LESSONS_VIEW.pick();
    let rows = numbered(&file.lessons, pick.sort);
    rsx! {
        // The card and its explicit keyboard CTA open the same next lesson,
        // without double activation from button event bubbling.
        crate::ui::Card {
            class: if go_id.is_some() { "card-flat card-action ls-lec" } else { "card-flat ls-lec" },
            onclick: move |_| if let Some(g) = &go_id { open(ui, g) },
            div { class: "ls-lec-meta",
                span { "{kicker}" }
                button {
                    class: "btn-link ls-lec-page",
                    onclick: { let id = id.clone(); move |e: MouseEvent| { e.stop_propagation(); crate::lecture::open(ui, &id) } },
                    {tx::LECTURE_PAGE}
                    span { class: "chev", aria_hidden: "true" }
                }
            }
            h2 { bdi { dir: "ltr", "{title}" } }
            p { class: "small ls-lec-sub",
                {tx::lessons_n(n)}
                if let Some(e) = lib.index.lectures.iter().find(|e| e.id == id) {
                    " · "
                    BookPages { id: id.clone(), pages: e.pages }
                }
            }
            Meter { done: d, total: n }
            div { class: "ls-lec-caption small",
                span { {tx::done_of(d, n)} }
                bdi { dir: "ltr", "{pct}%" }
            }
            if let Some(l) = go {
                button {
                    class: "btn btn-primary ls-lec-go",
                    r#type: "button",
                    onclick: move |e: MouseEvent| {
                        e.stop_propagation();
                        if let Some(id) = &cta_id {
                            open(ui, id);
                        }
                    },
                    span { class: "ls-btn-label",
                        {if go_at.is_some() || d > 0 { tx::CONTINUE } else { tx::START_FIRST }}
                        " · "
                        text::Line { text: l.title.clone() }
                    }
                    {crate::icons::arrow_go()}
                }
                if let Some((at, steps)) = go_at {
                    span { class: "small ls-lec-at", {tx::resume_at(at + 1, steps)} }
                }
            }
        }
        ViewBar { of: LESSONS_VIEW }
        ol { class: "coll ls-rows", "data-layout": pick.layout.as_str(),
            for (n , l) in rows {
                LessonRow { key: "{l.id}", n, lesson: l.clone(), next: next.map(str::to_string) }
            }
        }
    }
}

/// The lesson left part-way, shown while another lecture is picked: one
/// tap back to it, with its lecture and the step it reopens at.
#[component]
fn Resume(id: String) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let Some((_, l)) = lib.lesson(&id) else { return rsx! {} };
    let Some(at) = resume_step(&id) else { return rsx! {} };
    let steps = l.steps.len();
    rsx! {
        crate::ui::CardButton { class: "tap ls-now ls-resume",
            onclick: move |_| open(ui, &id),
            span { class: "ls-now-top",
                span { class: "chip", "data-tone": "accent", {tx::LEFT_OFF_KICKER} }
                span { class: "muted small ls-now-where",
                    {tx::step_of(at + 1, steps)}
                    span { class: "chev", aria_hidden: "true" }
                }
            }
            strong { class: "ls-now-title", text::Line { text: l.title.clone() } }
            span { class: "muted small", bdi { "{lib.crumb(lecture_of(&l.id))}" } }
            Meter { done: at, total: steps }
        }
    }
}

/// A lecture's lessons with their numbers, in the picked order.
pub(crate) fn numbered(lessons: &[Lesson], sort: Sort) -> Vec<(usize, &Lesson)> {
    let mut rows: Vec<(usize, &Lesson)> = lessons.iter().enumerate().map(|(i, l)| (i + 1, l)).collect();
    sort_by(&mut rows, sort, |(_, l)| mark(&l.id).is_none(), |_| false, |(_, l)| l.title.clone());
    rows
}

/// One lecture's search hits: its id, title, and (number, lesson) pairs.
type Hits = (String, String, Vec<(usize, Lesson)>);

/// Search results, by lecture.
#[component]
fn Results(q: String, next_id: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let groups: Vec<Hits> = lib
        .index
        .lectures
        .iter()
        .filter(|e| crate::courses::focus::shows(&e.id))
        .filter_map(|e| {
            let file = lib.lessons.get(&e.id)?;
            let hits: Vec<(usize, Lesson)> = file
                .lessons
                .iter()
                .enumerate()
                .filter(|(_, l)| matches(&q, &e.id, &e.title, l))
                .map(|(i, l)| (i + 1, l.clone()))
                .collect();
            (!hits.is_empty()).then(|| (e.id.clone(), e.title.clone(), hits))
        })
        .collect();
    let layout = LESSONS_VIEW.pick().layout;
    rsx! {
        if groups.is_empty() {
            p { class: "note", role: "status", {tx::no_match(&q)} }
            if crate::courses::focus::current().is_some() {
                button {
                    class: "btn btn-ghost ls-search-all",
                    onclick: move |_| crate::courses::focus::set(None),
                    {tx::SEARCH_ALL_COURSES}
                }
            }
        }
        for (id , title , hits) in groups {
            section { key: "{id}", class: "ls-group",
                h2 { class: "ls-sec",
                    button {
                        class: "tap tap-plain ls-group-head",
                        title: tx::OPEN_LECTURE,
                        onclick: { let id = id.clone(); move |_| crate::lecture::open(ui, &id) },
                        span { class: "num", "{lib.lecture_tag(&id)}" }
                        bdi { dir: "ltr", "{title}" }
                        span { class: "chev", aria_hidden: "true" }
                    }
                }
                ol { class: "coll ls-rows", "data-layout": layout.as_str(),
                    for (n , l) in hits {
                        LessonRow { key: "{l.id}", n, lesson: l, next: next_id.clone() }
                    }
                }
            }
        }
    }
}

/// One lesson as a row: number (or a tick), title and hook, and its state
/// in words with the time it takes.
#[component]
fn LessonRow(n: usize, lesson: Lesson, next: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let st = state(&lesson, next.as_deref());
    let steps = lesson.steps.len();
    let id = lesson.id.clone();
    let key = match st {
        State::Done(_) => "done",
        State::Current { .. } => "current",
        State::Next => "next",
        State::New => "new",
    };
    let tone = match st {
        State::Done(_) => "good",
        State::Current { .. } | State::Next => "strong",
        State::New => "quiet",
    };
    rsx! {
        li { class: "ls-item", "data-state": key,
            button { class: "coll-btn", onclick: move |_| open(ui, &id),
                span { class: "coll-lead", aria_hidden: "true",
                    span { class: "coll-badge", "data-tone": tone,
                        if matches!(st, State::Done(_)) { {crate::icons::check()} } else { "{n}" }
                    }
                }
                span { class: "coll-main",
                    strong { class: "coll-title", text::Line { text: lesson.title.clone() } }
                    span { class: "coll-sub", text::Line { text: lesson.hook.clone() } }
                }
                span { class: "coll-end",
                    match st {
                        State::Done(m) => rsx! {
                            span { class: "ls-state", {tx::STATE_DONE} }
                            // No question answered (all skipped): no "0/0".
                            if m.asked > 0 {
                                bdi { class: "num-ltr", "{m.right}/{m.asked}" }
                            } else {
                                span { {count(lesson.minutes, &MINUTE)} }
                            }
                        },
                        State::Current { at } => rsx! {
                            span { class: "ls-state", {tx::STATE_CURRENT} }
                            span { {tx::step_of(at + 1, steps)} }
                        },
                        State::Next => rsx! {
                            span { class: "ls-state", {tx::STATE_NEXT} }
                            span { {count(lesson.minutes, &MINUTE)} }
                        },
                        State::New => rsx! {
                            span { class: "ls-state", {tx::STATE_NEW} }
                            span { {count(lesson.minutes, &MINUTE)} }
                        },
                    }
                }
                span { class: "chev", aria_hidden: "true" }
            }
        }
    }
}

/// One lesson on its lecture's page: number (or a tick), title and hook,
/// time, questions and book pages, and whether it is done.
#[component]
pub fn LessonCard(n: usize, lesson: Lesson) -> Element {
    let ui = use_context::<Ui>();
    let m = mark(&lesson.id);
    let id = lesson.id.clone();
    let questions = lesson.steps.iter().filter(|s| s.body.is_question()).count();
    rsx! {
        li { class: "ls-card", "data-done": if m.is_some() { "yes" } else { "no" },
            button { class: "coll-btn ls-card-btn", onclick: move |_| open(ui, &id),
                span { class: "coll-lead", aria_hidden: "true",
                    span { class: "coll-badge", "data-tone": if m.is_some() { "good" } else { "quiet" }, "{n}" }
                }
                span { class: "coll-main",
                    strong { class: "coll-title", text::Line { text: lesson.title.clone() } }
                    span { class: "coll-sub", text::Line { text: lesson.hook.clone() } }
                    span { class: "coll-extra ls-card-meta small",
                        span { class: "ls-nowrap", {count(lesson.minutes, &MINUTE)} }
                        span { class: "ls-nowrap", {count(questions, &QUESTION)} }
                        BookPages { id: lesson.id.clone(), pages: lesson.pages }
                    }
                }
                span { class: "coll-end",
                    match m {
                        Some(m) if m.asked > 0 => rsx! { span { class: "chip ls-done", "✓ {m.right}/{m.asked}" } },
                        Some(_) => rsx! { span { class: "chip ls-done", "✓" } },
                        None => rsx! { span { class: "chip", {tx::NEW} } },
                    }
                }
                span { class: "chev", aria_hidden: "true" }
            }
        }
    }
}

/// Today's card: the lesson to go on with (part-way through first, else
/// the next one), one tap away, with how far along it is.
#[component]
pub fn TodayLesson(prefer: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    // The course in view first (`courses::focus`): a lesson part-way, else its next.
    let part_way = |shown: &dyn Fn(&str) -> bool| {
        lib.all_lessons().map(|(_, l)| l).find(|l| shown(&l.id) && resume_step(&l.id).is_some())
    };
    let next = next_lesson(&prefer);
    let Some(l) = part_way(&crate::courses::focus::shows)
        .or(next.filter(|l| crate::courses::focus::shows(&l.id)))
        .or_else(|| part_way(&|_| true))
        .or(next)
    else {
        return rsx! {};
    };
    let lecture = l.id.rsplit_once('.').map(|(l, _)| l).unwrap_or(&l.id);
    let steps = l.steps.len();
    let at = resume_step(&l.id);
    let done_steps = at.unwrap_or(0);
    rsx! {
        style { {CSS.text()} }
        crate::ui::CardButton { class: "tap ls-now",
            onclick: { let id = l.id.clone(); move |_| open(ui, &id) },
            span { class: "ls-now-top",
                span { class: "chip", "data-tone": "accent", {if at.is_some() { tx::NOW_KICKER } else { tx::NEXT_KICKER }} }
                span { class: "muted small ls-now-where",
                    match at {
                        Some(at) => rsx! { {tx::step_of(at + 1, steps)} },
                        None => rsx! { {count(l.minutes, &MINUTE)} },
                    }
                    span { class: "chev", aria_hidden: "true" }
                }
            }
            strong { class: "ls-now-title", text::Line { text: l.title.clone() } }
            span { class: "muted small", bdi { "{lib.crumb(lecture)}" } }
            span { class: "small ls-now-hook", text::Line { text: l.hook.clone() } }
            Meter { done: done_steps, total: steps }
        }
    }
}

/// A small "explain it to me" link under a study item whose lesson exists.
#[component]
pub fn ExplainLink(item_id: String) -> Element {
    let ui = use_context::<Ui>();
    let Some(l) = lesson_for_item(&item_id) else {
        return rsx! {};
    };
    let id = l.id.clone();
    rsx! {
        button { class: "st-link ls-explain", title: tx::EXPLAIN_HINT, onclick: move |_| open(ui, &id),
            "📖 {tx::EXPLAIN} · "
            text::Line { text: l.title.clone() }
        }
    }
}

/// Screen tests: the player on a lesson built by the test, not one of the
/// bundled lessons (so every step kind can be played).
#[cfg(test)]
pub(crate) fn test_player(lesson: Lesson) -> Element {
    rsx! {
        style { {CSS.text()} }
        player::Player { lesson, lecture_title: "Cardiovascular › Embryology".to_string() }
    }
}

#[cfg(test)]
mod list_tests {
    use super::*;

    #[test]
    fn search_finds_lessons_by_title_topic_and_lecture() {
        let lib = Library::get();
        let (lec, l) = lib.all_lessons().next().expect("a lesson is written");
        let title = lib.title(lec).to_string();
        let code = Library::get().lecture_tag(lec).to_lowercase();
        assert!(matches(&code, lec, &title, l));
        assert!(matches(&title.to_lowercase(), lec, &title, l));
        if let Some(t) = l.topics.first() {
            assert!(matches(&t.to_lowercase(), lec, &title, l));
        }
        assert!(!matches("zzz-no-such-word", lec, &title, l));
    }

    #[test]
    fn selected_lecture_cta_is_directly_keyboard_operable() {
        let source = include_str!("mod.rs");
        let shown = source.split("fn Shown(").nth(1).unwrap().split("pub(crate) fn numbered").next().unwrap();
        let cta = shown.split("class: \"btn btn-primary ls-lec-go\"").nth(1).unwrap();
        assert!(cta.contains("r#type: \"button\""));
        assert!(cta.contains("e.stop_propagation();"));
        assert!(cta.contains("open(ui, id);"));
    }

    #[test]
    fn following_next_lesson_keeps_starting_screen() {
        assert_eq!(lesson_origin(Screen::Learn, true, Screen::Today), Screen::Today);
        assert_eq!(lesson_origin(Screen::Learn, true, Screen::Study), Screen::Study);
        assert_eq!(lesson_origin(Screen::Learn, false, Screen::Today), Screen::Learn);
        assert_eq!(lesson_origin(Screen::Study, true, Screen::Today), Screen::Study);
    }

    #[test]
    fn left_off_is_the_latest_start_and_book_order_breaks_ties() {
        assert_eq!(latest([("a", 5), ("b", 9), ("c", 7)]), Some("b"));
        assert_eq!(latest([("a", 0), ("b", 0)]), Some("a"));
        assert_eq!(latest::<&str>([]), None);
        assert_eq!(lecture_of("cvs.L04.lesson1"), "cvs.L04");
    }

    #[test]
    fn resume_card_only_leads_back_from_another_lecture() {
        let source = include_str!("mod.rs");
        let list = source
            .split("fn LessonList(")
            .nth(1)
            .unwrap()
            .split("pub(crate) const LECTURES_VIEW")
            .next()
            .unwrap();
        assert!(list.contains("resume.filter(|l| lecture_of(&l.id) != shown)"));
        assert!(list.contains(".or(resume_lecture)"));
    }

    #[test]
    fn an_item_no_lesson_practises_is_taught_by_the_lesson_on_its_topic() {
        let lib = Library::get();
        for (lec, file) in &lib.lessons {
            let Some(lecture) = lib.lectures.get(lec) else { continue };
            for item in &lecture.items {
                let Some(l) = lesson_for_item(&item.id) else { continue };
                let practised = l.practice.contains(&item.id)
                    || l.steps.iter().any(|s| matches!(&s.body, steps_core::lesson::StepBody::Try { items } if items.contains(&item.id)));
                if !practised {
                    // Found by topic: the lesson covers the item's topic.
                    let topic = item.topic.as_deref().expect("matched by its topic");
                    assert!(l.topics.iter().any(|t| t == topic), "{} -> {}", item.id, l.id);
                }
                assert!(file.lessons.iter().any(|x| x.id == l.id));
            }
        }
        assert!(lesson_for_item("nolecture").is_none());
        // An item of a lecture with no lessons has none.
        if let Some(item) = lib
            .lectures
            .iter()
            .find(|(id, _)| !lib.lessons.contains_key(*id))
            .and_then(|(_, l)| l.items.first())
        {
            assert!(lesson_for_item(&item.id).is_none(), "{}", item.id);
        }
        assert!(lesson_for_item("cvs.L01.zzz-no-such-item").is_none());
    }

    #[test]
    fn lesson_counts_read_naturally() {
        assert_eq!(tx::lessons_n(1), "درس واحد");
        assert_eq!(tx::lessons_n(2), "درسين");
        assert_eq!(tx::lessons_n(7), "7 دروس");
        assert_eq!(tx::lessons_n(12), "12 درس");
    }
}
