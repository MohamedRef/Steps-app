//! The bank's start screen: filters, tutor/timed start, the Step 1 style
//! block (systems he picks, 10–20 questions, 90 s each, timed or with the
//! explanation after each), and the history of finished papers.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use steps_core::exam::{self, Filter, MOCK_SIZE, Mode, Only};
use steps_core::program::{Group, course_of};

use super::{
    HISTORY, PAUSED, date, lecture_title, minutes, now_ms, resume, short_lecture_title, start,
    study_lectures, study_lectures_where, submit_paused, topic_title,
};
use crate::content::Library;
use crate::i18n::bank as tx;
use crate::i18n::count::{LECTURE, MINUTE, QUESTION, TOPIC, count as counted};

/// The filters stay put between papers, and across launches (`SETUP_KEY`).
static FILTER: GlobalSignal<Filter> = Signal::global(Filter::default);
static COUNT: GlobalSignal<usize> = Signal::global(|| 20);
static MOCK_COUNT: GlobalSignal<usize> = Signal::global(|| MOCK_SIZE);
/// The block's systems and disciplines (course ids); empty = all of them.
static BLOCK_COURSES: GlobalSignal<BTreeSet<String>> = Signal::global(BTreeSet::new);
/// A block with the explanation after each question instead of the clock.
static BLOCK_TUTOR: GlobalSignal<bool> = Signal::global(|| false);
/// Practice with the explanation, practice against the clock, or a mock.
static MODE: GlobalSignal<Mode> = Signal::global(|| Mode::Tutor);
/// Whether the lecture, topic and tag filters are open.
static ADVANCED: GlobalSignal<bool> = Signal::global(|| false);
static RESTORED: GlobalSignal<bool> = Signal::global(|| false);

/// The last scope, count and mode, kept on this device: the bank opens the
/// way he left it.
const SETUP_KEY: &str = "bank.setup";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Saved {
    #[serde(default = "tutor")]
    mode: Mode,
    #[serde(default)]
    filter: Filter,
    #[serde(default = "twenty")]
    count: usize,
    #[serde(default = "mock_size")]
    mock_count: usize,
    #[serde(default)]
    advanced: bool,
    #[serde(default)]
    block_courses: BTreeSet<String>,
    #[serde(default)]
    block_tutor: bool,
}

fn tutor() -> Mode {
    Mode::Tutor
}
fn twenty() -> usize {
    20
}
fn mock_size() -> usize {
    MOCK_SIZE
}

impl Saved {
    fn now() -> Self {
        Saved {
            mode: *MODE.peek(),
            filter: FILTER.peek().clone(),
            count: *COUNT.peek(),
            mock_count: *MOCK_COUNT.peek(),
            advanced: *ADVANCED.peek(),
            block_courses: BLOCK_COURSES.peek().clone(),
            block_tutor: *BLOCK_TUTOR.peek(),
        }
    }

    /// Drop lectures, topics and tags the content no longer has, and counts
    /// the screen doesn't offer, so an old scope never hides every question.
    fn pruned(mut self, pool: &[exam::Mcq]) -> Self {
        let lectures: BTreeSet<&str> = pool.iter().map(|q| q.lecture).collect();
        let topics: BTreeSet<(&str, &str)> =
            pool.iter().filter_map(|q| Some((q.lecture, q.topic?))).collect();
        let tags: BTreeSet<&str> = pool.iter().flat_map(|q| q.tags.iter().map(String::as_str)).collect();
        self.filter.lectures.retain(|l| lectures.contains(l.as_str()));
        self.filter.topics.retain(|(l, t)| topics.contains(&(l.as_str(), t.as_str())));
        self.filter.tags.retain(|t| tags.contains(t.as_str()));
        let lib = Library::get();
        self.block_courses.retain(|c| lib.program.course(c).is_some());
        if !COUNTS.contains(&self.count) {
            self.count = twenty();
        }
        if !MOCK_COUNTS.contains(&self.mock_count) {
            self.mock_count = mock_size();
        }
        self
    }
}

/// Once per launch: the scope and mode he used last.
fn restore(pool: &[exam::Mcq]) {
    if *RESTORED.peek() {
        return;
    }
    *RESTORED.write() = true;
    let Some(saved) = crate::store::shared().local_setting::<Saved>(SETUP_KEY) else { return };
    let saved = saved.pruned(pool);
    *MODE.write() = saved.mode;
    *FILTER.write() = saved.filter;
    *COUNT.write() = saved.count;
    *MOCK_COUNT.write() = saved.mock_count;
    *ADVANCED.write() = saved.advanced;
    *BLOCK_COURSES.write() = saved.block_courses;
    *BLOCK_TUTOR.write() = saved.block_tutor;
}

/// The MCQs a block draws from: every written lecture of the chosen
/// systems, whatever course is in view.
fn block_lectures() -> Vec<steps_core::content::Lecture> {
    let chosen = BLOCK_COURSES.peek().clone();
    study_lectures_where(|l| chosen.is_empty() || chosen.contains(course_of(l)))
}

/// Courses with MCQs, in the book's order, with how many: the block's
/// system chips.
fn block_courses(pool: &[exam::Mcq]) -> Vec<(Group, String, String, usize)> {
    Library::get()
        .program
        .courses
        .iter()
        .map(|c| {
            (
                c.group,
                c.id.clone(),
                c.short.clone(),
                pool.iter().filter(|q| course_of(q.lecture) == c.id).count(),
            )
        })
        .filter(|(.., n)| *n > 0)
        .collect()
}

/// Narrows the filters to one lecture, or one of its topics (a lecture
/// page's "questions", a weak topic after a paper), as practice.
pub fn focus(lecture: &str, topic: Option<String>) {
    *FILTER.write() = Filter {
        lectures: BTreeSet::from([lecture.to_string()]),
        topics: topic.into_iter().map(|t| (lecture.to_string(), t)).collect(),
        ..Filter::default()
    };
    if *MODE.peek() == Mode::Mock {
        *MODE.write() = Mode::Tutor;
    }
}

/// Keeps only the lecture and topic picks of the course in view.
pub fn course_changed() {
    let shows = crate::courses::focus::shows;
    let mut f = FILTER.peek().clone();
    f.lectures.retain(|l| shows(l));
    f.topics.retain(|(l, _)| shows(l));
    if f != *FILTER.peek() {
        *FILTER.write() = f;
    }
}

/// "All lectures", or "all of <course>" with one course in view.
fn all_lectures() -> String {
    let lib = Library::get();
    crate::courses::focus::current()
        .and_then(|id| lib.program.courses.iter().find(|c| c.id == id))
        .map_or_else(|| tx::ALL_LECTURES.to_string(), |c| tx::all_of(&c.short))
}

const COUNTS: [usize; 4] = [10, 20, 40, usize::MAX];
/// Step 1 blocks hold up to 20 questions (30 minutes) since May 2026.
const MOCK_COUNTS: [usize; 3] = [10, 15, 20];
const TAGS: [(&str, &str); 3] =
    [("high-yield", tx::TAG_HIGH_YIELD), ("book-question", tx::TAG_BOOK), ("numbers", tx::TAG_NUMBERS)];
const MODES: [(Mode, &str, &str); 3] = [
    (Mode::Tutor, tx::MODE_TUTOR_NAME, tx::MODE_TUTOR_HINT),
    (Mode::Timed, tx::MODE_TIMED_NAME, tx::MODE_TIMED_HINT),
    (Mode::Mock, tx::MODE_MOCK_NAME, tx::MODE_MOCK_HINT),
];

/// Why a scope has no questions, so the empty state can say it and offer
/// the one change that brings questions back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Empty {
    /// Every question in scope was right last time.
    AllRight,
    /// No question of the chosen kinds in scope.
    Tags,
    /// The chosen topics have no questions.
    Topics,
}

fn why_empty(filter: &Filter, pool: &[exam::Mcq], last: &exam::LastResults) -> Empty {
    let any = |f: &Filter| pool.iter().any(|q| f.matches(q, last));
    if filter.only != Only::All && any(&Filter { only: Only::All, ..filter.clone() }) {
        Empty::AllRight
    } else if !filter.tags.is_empty()
        && any(&Filter { tags: BTreeSet::new(), only: Only::All, ..filter.clone() })
    {
        Empty::Tags
    } else {
        Empty::Topics
    }
}

/// Undo just the restriction that emptied the scope.
fn widen(reason: Empty) {
    let mut f = FILTER.write();
    match reason {
        Empty::AllRight => f.only = Only::All,
        Empty::Tags => {
            f.tags.clear();
            f.only = Only::All;
        }
        Empty::Topics => *f = Filter::default(),
    }
}

/// The scope in a few words: its title (English titles flagged `latin`) and
/// what else narrows it.
fn scope_title(filter: &Filter, lectures: &[(String, usize)]) -> (String, bool) {
    match (filter.lectures.len(), filter.topics.len()) {
        (_, 1) => {
            let (l, t) = filter.topics.iter().next().expect("one topic");
            (topic_title(l, t), true)
        }
        (_, n) if n > 1 => (counted(n, &TOPIC), false),
        (1, _) => (short_lecture_title(filter.lectures.iter().next().expect("one lecture")), true),
        (0, _) if lectures.len() == 1 => (short_lecture_title(&lectures[0].0), true),
        (0, _) => (all_lectures(), false),
        (n, _) => (counted(n, &LECTURE), false),
    }
}

#[component]
pub fn Setup() -> Element {
    let lib = Library::get();
    let owned = study_lectures();
    let pool = exam::mcqs(&owned);
    use_hook(|| restore(&pool));
    // Keep the scope and mode for the next launch whenever they change.
    use_effect(|| {
        let _ = (MODE.read(), FILTER.read(), COUNT.read(), MOCK_COUNT.read(), ADVANCED.read());
        let _ = (BLOCK_COURSES.read(), BLOCK_TUTOR.read());
        let _ = crate::store::shared().put_local_setting(SETUP_KEY, &Saved::now());
    });
    if pool.is_empty() {
        return rsx! {
            crate::ui::Card {
                h1 { {tx::TITLE} }
                h2 { class: "muted", {tx::EMPTY_TITLE} }
                p { {tx::EMPTY_BODY} }
            }
            History {}
        };
    }

    let mode = *MODE.read();
    let advanced = *ADVANCED.read() && mode != Mode::Mock;
    let filter = FILTER.read().clone();
    let history = HISTORY.read();
    let selected = filter.select(&pool, &history.last);
    let empty_reason = selected.is_empty().then(|| why_empty(&filter, &pool, &history.last));
    drop(history);
    let available = selected.len();
    let count = *COUNT.read();
    let mock_count = *MOCK_COUNT.read();

    // Lectures that have MCQs, with how many.
    let mut lectures: Vec<(String, usize)> = Vec::new();
    for q in &pool {
        match lectures.last_mut() {
            Some((l, n)) if l == q.lecture => *n += 1,
            _ => lectures.push((q.lecture.to_string(), 1)),
        }
    }
    // Topics of the chosen lectures, in book order. With several lectures and
    // none chosen there would be dozens, so they wait for a lecture pick.
    let several_lectures = lectures.len() > 1;
    let mut topics: Vec<(String, String)> = Vec::new();
    if !several_lectures || !filter.lectures.is_empty() {
        for q in pool.iter().filter(|q| filter.lectures.is_empty() || filter.lectures.contains(q.lecture)) {
            if let Some(t) = q.topic {
                let key = (q.lecture.to_string(), t.to_string());
                if !topics.contains(&key) {
                    topics.push(key);
                }
            }
        }
        let rank = |(l, t): &(String, String)| {
            let pos = lib.lectures.get(l).and_then(|lec| lec.topics.iter().position(|x| &x.id == t));
            (l.clone(), pos.unwrap_or(usize::MAX))
        };
        topics.sort_by_key(rank);
    }
    let topic_prefix = several_lectures && filter.lectures.len() != 1;
    let tags_present: BTreeSet<&str> = pool.iter().flat_map(|q| q.tags.iter().map(String::as_str)).collect();
    let tags: Vec<(&str, &str)> = TAGS.into_iter().filter(|(t, _)| tags_present.contains(t)).collect();
    // What else narrows the scope, said under its title.
    let mut narrowing: Vec<&str> =
        TAGS.iter().filter(|(t, _)| filter.tags.contains(*t)).map(|(_, label)| *label).collect();
    if filter.only == Only::WrongOrUnseen {
        narrowing.push(tx::ONLY_WRONG);
    }
    let (title, latin) = scope_title(&filter, &lectures);
    let filtered = filter != Filter::default();

    let start_practice = move |_| {
        let filter = FILTER.read().clone();
        let lectures = study_lectures();
        let pool = exam::mcqs(&lectures);
        let selected = filter.select(&pool, &HISTORY.read().last);
        start(*MODE.read(), exam::practice(&selected, *COUNT.read(), now_ms() as u64));
    };
    let start_mock = move |_| {
        let lectures = block_lectures();
        let pool = exam::mcqs(&lectures);
        let mode = if *BLOCK_TUTOR.read() { Mode::Tutor } else { Mode::Mock };
        start(mode, exam::mock(&pool, *MOCK_COUNT.read(), now_ms() as u64));
    };
    let shown = count.min(available);
    // The block draws from every course, not just the one in view.
    let all_owned = study_lectures_where(|_| true);
    let all_pool = exam::mcqs(&all_owned);
    let systems = block_courses(&all_pool);
    let chosen = BLOCK_COURSES.read().clone();
    let block_tutor = *BLOCK_TUTOR.read();
    let pool_size =
        all_pool.iter().filter(|q| chosen.is_empty() || chosen.contains(course_of(q.lecture))).count();
    let mock_shown = mock_count.min(pool_size);
    let has_history = !HISTORY.read().papers.is_empty();
    let mode_hint = if mode == Mode::Tutor { tx::PRESET_TUTOR } else { tx::PRESET_TIMED };

    rsx! {
        // Tablet landscape and the Mac: the setup in the wide column, past
        // papers beside it.
        div { class: if has_history { "tab-cols bank-cols" } else { "bank-cols" }, "data-wide": "main",
            div { class: "tab-main bank-setup",
                Paused {}
                header { class: "page-head",
                    crate::courses::CourseChip {}
                    h1 { {tx::TITLE} }
                }
                div { class: "segmented bank-modes", role: "radiogroup", "aria-label": tx::MODE_LABEL,
                    for (m, name, hint) in MODES {
                        button {
                            key: "{m:?}",
                            role: "radio",
                            "aria-checked": if mode == m { "true" } else { "false" },
                            onclick: move |_| *MODE.write() = m,
                            span { {name} }
                            span { class: "seg-hint", {hint} }
                        }
                    }
                }

                if mode == Mode::Mock {
                    crate::ui::Card { class: "bank-preset",
                        span { class: "chip", "data-tone": "accent", {tx::MOCK_SCOPE_CHIP} }
                        h2 { class: "bank-scope",
                            if chosen.is_empty() { {tx::BLOCK_ALL_SYSTEMS} } else { {tx::block_systems(chosen.len())} }
                        }
                        div { class: "bank-systems", role: "group", aria_label: tx::BLOCK_SYSTEMS,
                            span { class: "muted small", {tx::BLOCK_SYSTEMS} }
                            div { class: "chips",
                                Chip { on: chosen.is_empty(), onclick: move |_| BLOCK_COURSES.write().clear(), {tx::BLOCK_ALL_SYSTEMS} }
                            }
                            for g in [Group::General, Group::Systems] {
                                if systems.iter().any(|s| s.0 == g) {
                                    h3 { key: "{g:?}", class: "label course-label",
                                        bdi { if g == Group::General { {crate::i18n::courses::G_GENERAL} } else { {crate::i18n::courses::G_SYSTEMS} } }
                                    }
                                    div { class: "chips",
                                        for (_, id, short, n) in systems.iter().filter(|s| s.0 == g).cloned() {
                                            Chip {
                                                key: "{id}",
                                                on: chosen.contains(&id),
                                                onclick: {
                                                    let id = id.clone();
                                                    move |_| {
                                                        let mut c = BLOCK_COURSES.write();
                                                        if !c.remove(&id) {
                                                            c.insert(id.clone());
                                                        }
                                                    }
                                                },
                                                span { class: "chip-label", "{short}" }
                                                span { class: "count", {tx::questions(n)} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "bank-count",
                            span { class: "muted small", {tx::COUNT} }
                            div { class: "chips",
                                for c in MOCK_COUNTS {
                                    Chip { key: "{c}", on: mock_count == c, onclick: move |_| *MOCK_COUNT.write() = c, bdi { "{c}" } }
                                }
                            }
                        }
                        div { class: "segmented bank-block-mode", role: "radiogroup", aria_label: tx::MODE_LABEL,
                            button {
                                role: "radio",
                                "aria-checked": if block_tutor { "false" } else { "true" },
                                onclick: move |_| *BLOCK_TUTOR.write() = false,
                                span { {tx::BLOCK_TIMED} }
                                span { class: "seg-hint", {tx::BLOCK_TIMED_HINT} }
                            }
                            button {
                                role: "radio",
                                "aria-checked": if block_tutor { "true" } else { "false" },
                                onclick: move |_| *BLOCK_TUTOR.write() = true,
                                span { {tx::BLOCK_TUTOR} }
                                span { class: "seg-hint", {tx::BLOCK_TUTOR_HINT} }
                            }
                        }
                        p { class: "bank-sum",
                            strong { {counted(mock_shown, &QUESTION)} }
                            if !block_tutor {
                                " · " {counted(minutes(exam::time_limit_ms(mock_shown)), &MINUTE)}
                            }
                            span { class: "muted", " · {tx::POOL_OF} " {counted(pool_size, &QUESTION)} }
                        }
                        button { class: "btn btn-primary btn-lg bank-go", disabled: mock_shown == 0, onclick: start_mock,
                            {tx::START_MOCK}
                        }
                    }
                    p { class: "note bank-scope-note", {tx::MOCK_SCOPE_NOTE} }
                } else {
                    crate::ui::Card { class: "bank-preset",
                        // The scope is what the card shows: tapping it opens
                        // the lecture, topic and tag choices.
                        button {
                            class: "tap tap-plain bank-scope-tap",
                            "aria-expanded": if advanced { "true" } else { "false" },
                            "aria-controls": "bank-advanced",
                            title: tx::CUSTOMISE,
                            onclick: move |_| { let open = *ADVANCED.peek(); *ADVANCED.write() = !open; },
                            span { class: "chip", "data-tone": "accent", if filtered { {tx::SCOPE_CHOSEN} } else { {tx::SCOPE_ALL} } }
                            if latin {
                                bdi { class: "bank-scope", lang: "en", "{title}" }
                            } else {
                                span { class: "bank-scope", "{title}" }
                            }
                            if !narrowing.is_empty() {
                                span { class: "muted small", {narrowing.join(" · ")} }
                            }
                        }
                        p { class: "muted small", {mode_hint} }
                        div { class: "bank-count",
                            span { class: "muted small", {tx::COUNT} }
                            div { class: "chips",
                                for c in COUNTS {
                                    Chip {
                                        key: "{c}",
                                        on: count == c,
                                        onclick: move |_| *COUNT.write() = c,
                                        if c == usize::MAX { {tx::ALL} } else { bdi { "{c}" } }
                                    }
                                }
                            }
                        }
                        if let Some(reason) = empty_reason {
                            div { class: "note bank-empty", role: "status",
                                p { strong { {tx::NONE_MATCH} } }
                                p { class: "small",
                                    match reason {
                                        Empty::AllRight => tx::EMPTY_ALL_RIGHT,
                                        Empty::Tags => tx::EMPTY_TAGS,
                                        Empty::Topics => tx::EMPTY_TOPICS,
                                    }
                                }
                                button { class: "btn btn-tonal", onclick: move |_| widen(reason),
                                    match reason {
                                        Empty::AllRight => tx::WIDEN_ALL_RIGHT,
                                        Empty::Tags => tx::WIDEN_TAGS,
                                        Empty::Topics => tx::WIDEN_ALL,
                                    }
                                }
                            }
                        } else {
                            p { class: "bank-sum",
                                strong { {counted(shown, &QUESTION)} }
                                " · " {tx::ABOUT} " " {counted(minutes(exam::time_limit_ms(shown)), &MINUTE)}
                                span { class: "muted", " · {tx::POOL_OF} " {counted(available, &QUESTION)} }
                            }
                            button { class: "btn btn-primary btn-lg bank-go", "aria-keyshortcuts": "Enter", onclick: start_practice,
                                if mode == Mode::Tutor { {tx::START_TUTOR} } else { {tx::START_TIMED} }
                            }
                        }
                    }

                    button {
                        class: "btn btn-secondary bank-customise",
                        "aria-expanded": if advanced { "true" } else { "false" },
                        "aria-controls": "bank-advanced",
                        onclick: move |_| { let open = *ADVANCED.peek(); *ADVANCED.write() = !open; },
                        span { aria_hidden: "true", "⚙" }
                        if advanced { {tx::CUSTOMISE_CLOSE} } else { {tx::CUSTOMISE} }
                    }

                    if advanced {
                        crate::ui::Card { class: "bank-advanced", id: "bank-advanced",
                            if several_lectures {
                                h2 { class: "label", {tx::LECTURES} }
                                for group in crate::content::Library::get().by_course(lectures, |(id, _)| id.as_str()) {
                                    h3 { key: "{group.id}", class: "label course-label", "{group.short()}" }
                                    div { class: "chips",
                                        for (id, n) in group.rows {
                                            Chip {
                                                key: "{id}",
                                                on: filter.lectures.contains(&id),
                                                onclick: {
                                                    let id = id.clone();
                                                    move |_| {
                                                        let mut f = FILTER.write();
                                                        if !f.lectures.remove(&id) {
                                                            f.lectures.insert(id.clone());
                                                        }
                                                        let keep = f.lectures.clone();
                                                        f.topics.retain(|(l, _)| keep.is_empty() || keep.contains(l));
                                                    }
                                                },
                                                title: lecture_title(&id),
                                                bdi { class: "chip-label", "{short_lecture_title(&id)}" }
                                                span { class: "count", {tx::questions(n)} }
                                            }
                                        }
                                    }
                                }
                            }

                            h2 { class: "label", {tx::TOPICS} }
                            if topics.is_empty() {
                                p { class: "muted small", {tx::PICK_LECTURE_FOR_TOPICS} }
                            } else {
                                div { class: "chips",
                                    for (lecture, topic) in topics {
                                        Chip {
                                            key: "{lecture}/{topic}",
                                            on: filter.topics.contains(&(lecture.clone(), topic.clone())),
                                            onclick: {
                                                let key = (lecture.clone(), topic.clone());
                                                move |_| {
                                                    let mut f = FILTER.write();
                                                    if !f.topics.remove(&key) {
                                                        f.topics.insert(key.clone());
                                                    }
                                                }
                                            },
                                            if topic_prefix {
                                                bdi { "{short_lecture_title(&lecture)} › {topic_title(&lecture, &topic)}" }
                                            } else {
                                                bdi { {topic_title(&lecture, &topic)} }
                                            }
                                        }
                                    }
                                }
                            }

                            h2 { class: "label", {tx::TAGS} }
                            div { class: "chips",
                                for (tag, label) in tags {
                                    Chip {
                                        key: "{tag}",
                                        on: filter.tags.contains(tag),
                                        onclick: move |_| {
                                            let mut f = FILTER.write();
                                            if !f.tags.remove(tag) {
                                                f.tags.insert(tag.to_string());
                                            }
                                        },
                                        {label}
                                    }
                                }
                                Chip {
                                    on: filter.only == Only::WrongOrUnseen,
                                    onclick: move |_| {
                                        let mut f = FILTER.write();
                                        f.only = if f.only == Only::All { Only::WrongOrUnseen } else { Only::All };
                                    },
                                    {tx::ONLY_WRONG}
                                }
                            }
                            if filtered {
                                button { class: "btn btn-ghost bank-reset", onclick: move |_| *FILTER.write() = Filter::default(),
                                    {tx::RESET_SCOPE}
                                }
                            }
                        }
                    }
                    p { class: "note bank-scope-note",
                        span { aria_hidden: "true", "ⓘ " }
                        "{tx::SCOPE_NOW} "
                        if latin { bdi { lang: "en", "{title}" } } else { "{title}" }
                        " · " {counted(available, &QUESTION)}
                    }
                }
            }
            if has_history {
                div { class: "tab-aside tab-sticky",
                    History {}
                }
            }
        }
    }
}

#[component]
fn Chip(
    on: bool,
    onclick: EventHandler<MouseEvent>,
    /// The full text when the label is shortened (long press or hover).
    #[props(default)]
    title: String,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "chip toggle",
            title,
            "aria-pressed": if on { "true" } else { "false" },
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

#[component]
fn History() -> Element {
    let ui = use_context::<crate::Ui>();
    let papers = HISTORY.read().papers.clone();
    if papers.is_empty() {
        return rsx! {};
    }
    rsx! {
        crate::ui::Card {
            h2 { {tx::HISTORY} }
            ol { class: "history", reversed: true,
                // A paper opens to its score per lecture; each lecture opens its page.
                for (i, o) in papers.iter().enumerate().rev() {
                    li { key: "{i}",
                        details { class: "history-paper",
                            summary {
                                span { class: "chip", if o.mode == Mode::Mock { {tx::MODE_MOCK} } else { {tx::MODE_TIMED} } }
                                bdi { class: "muted small", {date(crate::store::zone().local_ms(o.at_ms))} }
                                span { class: "grow" }
                                bdi { class: "score", "{o.total.right}/{o.total.total}" }
                                bdi { class: "pct", "{o.total.percent()}%" }
                            }
                            ul { class: "tap-list",
                                for (l, s) in o.by_lecture.iter() {
                                    li { key: "{l}",
                                        button {
                                            class: "tap tap-row",
                                            title: tx::OPEN_LECTURE,
                                            onclick: { let l = l.clone(); move |_| crate::lecture::open(ui, &l) },
                                            bdi { class: "tap-row-main", "{crate::content::Library::get().crumb(l)}" }
                                            bdi { class: "score", "{s.right}/{s.total}" }
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

/// A timed paper he left part-way: go back to it, or submit it as it is.
#[component]
fn Paused() -> Element {
    let Some(p) = PAUSED.read().clone() else {
        return rsx! {};
    };
    let n = p.questions.len();
    let answered = p.answers.iter().filter(|a| a.chosen.is_some()).count();
    let left = exam::remaining_ms(n, p.started_ms, now_ms());
    rsx! {
        crate::ui::Card { class: "paused",
            h2 { {tx::PAUSED_TITLE} }
            p { class: "muted small",
                "{tx::PAUSED_BODY} " bdi { "{answered}/{n}" }
                if left > 0 {
                    " {tx::PAUSED_LEFT} " {counted(minutes(left), &MINUTE)}
                } else {
                    " · {tx::PAUSED_OVER}"
                }
            }
            div { class: "start-row",
                button { class: "btn btn-primary", onclick: move |_| resume(), {tx::RESUME} }
                button { class: "btn btn-secondary", onclick: move |_| submit_paused(), {tx::SUBMIT_NOW} }
            }
        }
    }
}

#[cfg(test)]
mod block_tests {
    use super::*;

    #[test]
    fn block_systems_follow_the_book_and_only_list_courses_with_questions() {
        let lib = Library::get();
        let pool = exam::mcqs(lib.lectures.values());
        let systems = block_courses(&pool);
        assert!(systems.iter().all(|(.., n)| *n > 0));
        assert_eq!(systems.iter().map(|s| s.3).sum::<usize>(), pool.len());
        let order: Vec<usize> = systems
            .iter()
            .map(|(_, id, ..)| lib.program.courses.iter().position(|c| &c.id == id).unwrap())
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]));
    }

    #[component]
    fn BlockSetup(tutor: bool) -> Element {
        use_hook(|| {
            *RESTORED.write() = true;
            *MODE.write() = Mode::Mock;
            *BLOCK_TUTOR.write() = tutor;
            BLOCK_COURSES.write().clear();
        });
        rsx! {
            crate::browser_fixture::Shell { screen: crate::Screen::Bank, focus: false, Setup {} }
        }
    }

    fn render_block(tutor: bool) -> String {
        let mut dom = VirtualDom::new_with_props(BlockSetup, BlockSetupProps { tutor });
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    #[test]
    fn the_block_card_offers_systems_sizes_and_both_modes() {
        let html = render_block(false);
        for needle in [
            tx::MODE_MOCK_NAME,
            tx::BLOCK_SYSTEMS,
            tx::BLOCK_ALL_SYSTEMS,
            tx::BLOCK_TIMED_HINT,
            tx::BLOCK_TUTOR,
            tx::START_MOCK,
        ] {
            assert!(html.contains(needle), "missing {needle}");
        }
        // The written lectures' courses are offered as chips, in Arabic.
        let lib = Library::get();
        let pool = exam::mcqs(lib.lectures.values());
        for (_, _, short, _) in block_courses(&pool) {
            assert!(html.contains(&short), "missing chip {short}");
        }
        for n in MOCK_COUNTS {
            assert!(html.contains(&format!(">{n}<")), "missing size {n}");
        }
        // Timed blocks show their clock; tutor blocks have none.
        let timed_minutes = minutes(exam::time_limit_ms(MOCK_SIZE.min(pool.len())));
        let clock = counted(timed_minutes, &MINUTE);
        assert!(html.contains(&clock), "missing {clock}");
        let tutor = render_block(true);
        assert!(!tutor.contains(&clock), "a tutor block shows no clock");
    }

    #[test]
    fn a_setup_saved_before_blocks_still_loads() {
        let old = r#"{"mode":"mock","filter":{"lectures":[],"topics":[],"tags":[],"only":"all"},"count":20,"mock_count":100}"#;
        let saved: Saved = serde_json::from_str(old).unwrap();
        assert!(saved.block_courses.is_empty());
        assert!(!saved.block_tutor);
        // 100 is no longer a block size: it falls back to a full block.
        let saved = saved.pruned(&[]);
        assert_eq!(saved.mock_count, MOCK_SIZE);
        assert!(MOCK_COUNTS.iter().all(|&n| (10..=20).contains(&n)));
    }
}
