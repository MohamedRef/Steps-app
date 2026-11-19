//! Quick search, Ctrl/⌘ + K (or the search button on the rail): one box
//! that jumps to a screen, a tool, a lecture, a lesson, a First Aid page
//! ("ص 32" or just "32"), a Settings section, a lecture's questions
//! ("questions heart failure", "أسئلة …"), or one VQ question once his key
//! opened them ("VQ-0001", "vq0001", "vq 1"). Everything comes from the library on the device;
//! nothing is saved or sent. Results open through the same calls as the
//! buttons that open them, so no session or answer is thrown away.
//! Adapted from batna-app main: crates/app/src/shortcuts/palette.rs (#743,
//! Settings sections and lecture questions from #744);
//! the page jump and the tools are Steps' own.

use dioxus::prelude::*;

use crate::content::Library;
use crate::i18n::keys as tx;
use crate::settings::{Section, SettingsNav};
use crate::{Screen, Ui};

static OPEN: GlobalSignal<bool> = Signal::global(|| false);

pub fn open(on: bool) {
    if *OPEN.peek() != on {
        *OPEN.write() = on;
    }
}

pub fn is_open() -> bool {
    *OPEN.peek()
}

/// Closing search (Esc, ✕ or the backdrop) gives focus back to the control
/// that opened it, if it is still on screen; following a result does not,
/// since that leaves the old screen. Adapted from batna-app main:
/// crates/app/src/shortcuts/palette.rs (#746).
fn dismiss() {
    open(false);
    document::eval(
        "requestAnimationFrame(() => { const el = window.__stepsSearchOpener; \
         if (el?.isConnected && el.getClientRects().length) el.focus({ preventScroll: true }); });",
    );
}

/// The phone's way in: a search button on the top bar (which shows under
/// 600dp; the rail and sidebar have their own). It opens the same box as
/// Ctrl/⌘ + K.
#[component]
pub fn SearchButton() -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "icon-button topbar-search",
            aria_label: tx::QS_TITLE,
            title: tx::QS_TITLE,
            onclick: move |_| open(true),
            {crate::icons::search()}
        }
    }
}

const MAX: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Go {
    Screen(Screen),
    Tool(&'static str),
    Settings,
    Section(Section),
    Questions(String),
    /// One question, as a tutor set (a VQ question by its number).
    Question(String),
    /// A link to open in the browser (the video of a VQ question).
    Link(String),
    Page(u32, String),
    Lecture(String),
    Lesson(String),
    /// A lecture's topic: the lesson that teaches it, else the lecture.
    Topic(String, String),
    /// An abbreviation, in the dictionary of its course.
    Term(String, String),
    /// Nothing found here: the same words in the Lessons search, or in the
    /// dictionary.
    SearchLessons(String),
    SearchTerms(String),
}

#[derive(Clone, Debug)]
struct Hit {
    key: String,
    name: String,
    sub: String,
    english: bool,
    go: Go,
    /// English shown after `sub`, isolated (a term's meaning).
    detail: Option<String>,
}

/// Abbreviations shown at most, so a word common in meanings ("histone")
/// can't push the rest out.
const MAX_TERMS: usize = 4;

/// Every word of the query is somewhere in `words`, in any case.
fn matches(query: &str, words: &str) -> bool {
    let words = words.to_lowercase();
    query.to_lowercase().split_whitespace().all(|w| words.contains(w))
}

/// "questions <lecture>" (or أسئلة / bank / mcq): only then are lectures
/// offered as question practice, so a plain search stays short.
fn questions_query(query: &str) -> Option<&str> {
    ["questions ", "question ", "bank ", "mcq ", "أسئلة ", "اسئلة ", "سؤال "]
        .into_iter()
        .find_map(|prefix| query.strip_prefix(prefix))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// With nothing typed: the screens, the tools and Settings. Typed: those
/// that match, Settings sections, a First Aid page when it is a number, a
/// lecture's questions when asked for, then lectures and lessons, best
/// match first.
fn find(query: &str, lib: &Library) -> Vec<Hit> {
    let q = query.trim().to_lowercase();
    let mut hits = Vec::new();
    for screen in Screen::NAV {
        let name = screen.label();
        if q.is_empty() || matches(&q, &format!("{name} {screen:?}")) {
            hits.push(Hit {
                key: format!("s-{screen:?}"),
                name: name.into(),
                sub: tx::KIND_SCREEN.into(),
                english: false,
                go: Go::Screen(screen),
                detail: None,
            });
        }
    }
    for tool in crate::more::TOOLS {
        if q.is_empty() || matches(&q, tool.label) {
            hits.push(Hit {
                key: format!("t-{}", tool.label),
                name: tool.label.into(),
                sub: tx::KIND_TOOL.into(),
                english: false,
                go: Go::Tool(tool.label),
                detail: None,
            });
        }
    }
    if q.is_empty() || matches(&q, &format!("{} settings", Screen::Settings.label())) {
        hits.push(Hit {
            key: "settings".into(),
            name: Screen::Settings.label().into(),
            sub: tx::KIND_TOOL.into(),
            english: false,
            go: Go::Settings,
            detail: None,
        });
    }
    if q.is_empty() {
        return hits;
    }
    // The same words the Settings search box finds them by.
    for section in Section::ALL {
        if section.applies() && section.matches(&q) {
            hits.push(Hit {
                key: format!("c-{}", section.id()),
                name: section.title().into(),
                sub: tx::KIND_SECTION.into(),
                english: false,
                go: Go::Section(section),
                detail: None,
            });
        }
    }
    if let Some(needle) = questions_query(&q) {
        for e in &lib.index.lectures {
            let Some(lecture) = lib.lectures.get(&e.id) else { continue };
            let tag = lib.lecture_tag(&e.id);
            if matches(needle, &format!("{} {} {tag}", e.id, e.title))
                && lecture.items.iter().any(|i| matches!(i.body, steps_core::content::Body::Mcq { .. }))
            {
                hits.push(Hit {
                    key: format!("q-{}", e.id),
                    name: e.title.clone(),
                    sub: format!("{} · {tag}", tx::KIND_QUESTIONS),
                    english: true,
                    go: Go::Questions(e.id.clone()),
                    detail: None,
                });
            }
        }
    }
    // A VQ question by its number, and the video that explains it, named
    // and linked; then the number is no First Aid page.
    let vq = crate::vq::search_number(&q);
    if let Some(id) = vq.and_then(|n| lib.vq.question(n)) {
        let name = crate::vq::name(id).unwrap_or_default();
        let video = lib.vq.video(id);
        hits.push(Hit {
            key: format!("v-{id}"),
            name: name.clone(),
            sub: format!("{} · {}", crate::i18n::vq::KIND, lib.crumb(id)),
            english: true,
            go: Go::Question(id.to_string()),
            detail: video.map(|v| v.title.clone()),
        });
        if let Some(v) = video {
            hits.push(Hit {
                key: format!("y-{id}"),
                name: v.title.clone(),
                sub: format!("{} · {name}", crate::i18n::vq::KIND_VIDEO),
                english: true,
                go: Go::Link(v.url()),
                detail: Some(v.url()),
            });
        }
    }
    // One of his own questions by its id (`P-0001`), in any of his categories.
    for mine in lib.mine.find(&q) {
        let id = mine.item_id();
        hits.push(Hit {
            key: format!("m-{id}"),
            name: format!("{} · {}", mine.category, mine.id),
            sub: format!("{} · {}", crate::i18n::ownq::KIND, lib.crumb(&id)),
            english: true,
            go: Go::Question(id),
            detail: None,
        });
    }
    if let (None, crate::gopage::Target::Page { page, lecture }) = (vq, crate::gopage::target(lib, &q)) {
        hits.push(Hit {
            key: format!("p-{page}"),
            name: format!("{} {page}", tx::KIND_PAGE),
            sub: lib.crumb(&lecture),
            english: false,
            go: Go::Page(page, lecture),
            detail: None,
        });
    }
    for e in &lib.index.lectures {
        if !lib.lectures.contains_key(&e.id) {
            continue;
        }
        let tag = lib.lecture_tag(&e.id);
        if matches(&q, &format!("{} {} {tag}", e.id, e.title)) {
            hits.push(Hit {
                key: format!("l-{}", e.id),
                name: e.title.clone(),
                sub: format!("{} · {tag}", tx::KIND_LECTURE),
                english: true,
                go: Go::Lecture(e.id.clone()),
                detail: None,
            });
        }
    }
    for (lecture, lesson) in lib.all_lessons() {
        let crumb = lib.crumb(lecture);
        if matches(&q, &format!("{} {} {crumb}", lesson.id, lesson.title)) {
            hits.push(Hit {
                key: format!("n-{}", lesson.id),
                name: lesson.title.clone(),
                sub: format!("{} · {crumb}", tx::KIND_LESSON),
                english: false,
                go: Go::Lesson(lesson.id.clone()),
                detail: None,
            });
        }
    }
    // A lecture's topics (UX-15): "histone" finds where it is taught.
    for e in &lib.index.lectures {
        let Some(lecture) = lib.lectures.get(&e.id) else { continue };
        for t in &lecture.topics {
            if matches(&q, &format!("{} {}", t.id, t.title)) {
                hits.push(Hit {
                    key: format!("o-{}/{}", e.id, t.id),
                    name: t.title.clone(),
                    sub: format!("{} · {}", tx::KIND_TOPIC, lib.crumb(&e.id)),
                    english: true,
                    go: Go::Topic(e.id.clone(), t.id.clone()),
                    detail: None,
                });
            }
        }
    }
    // Abbreviations by their spelling first, then by a word of their
    // meaning; one of each, in the first course that explains it.
    let mut terms: Vec<(u8, Hit)> = Vec::new();
    for c in &lib.program.courses {
        let Some(list) = crate::abbrdict::index().get(&c.id) else { continue };
        for t in list {
            let Some(rank) = crate::abbrdict::rank(t.entry, &q) else { continue };
            if terms.iter().any(|(_, h)| h.name == t.entry.term) {
                continue;
            }
            terms.push((
                rank,
                Hit {
                    key: format!("g-{}/{}", c.id, t.entry.term),
                    name: t.entry.term.clone(),
                    sub: format!("{} · {}", tx::KIND_TERM, c.short),
                    english: true,
                    go: Go::Term(c.id.clone(), t.entry.term.clone()),
                    detail: Some(t.entry.text.clone()),
                },
            ));
        }
    }
    terms.sort_by_key(|(rank, _)| *rank);
    hits.extend(terms.into_iter().take(MAX_TERMS).map(|(_, h)| h));
    // Exact, then starts with, then the rest; a stable sort keeps book order.
    hits.sort_by_key(|h| {
        let name = h.name.to_lowercase();
        if name == q || h.key.ends_with(&format!("-{q}")) {
            0
        } else if name.starts_with(&q) {
            1
        } else {
            2
        }
    });
    hits.truncate(MAX);
    // Nothing here: the same words where longer texts are searched.
    if hits.is_empty() {
        let typed = query.trim().to_string();
        hits.push(Hit {
            key: "x-lessons".into(),
            name: tx::search_lessons(&typed),
            sub: tx::KIND_SCREEN_LESSONS.into(),
            english: false,
            go: Go::SearchLessons(typed.clone()),
            detail: None,
        });
        hits.push(Hit {
            key: "x-terms".into(),
            name: tx::SEARCH_TERMS.into(),
            sub: crate::i18n::abbr::DICT_TITLE.into(),
            english: false,
            go: Go::SearchTerms(typed),
            detail: None,
        });
    }
    hits
}

/// Only the ways to search elsewhere: nothing matched.
fn nothing_found(hits: &[Hit]) -> bool {
    hits.iter().all(|h| matches!(h.go, Go::SearchLessons(_) | Go::SearchTerms(_)))
}

/// The lesson that teaches `topic` of `lecture`, if one does.
fn lesson_of(lib: &Library, lecture: &str, topic: &str) -> Option<String> {
    let file = lib.lessons.get(lecture)?;
    file.lessons.iter().find(|l| l.topics.iter().any(|t| t == topic)).map(|l| l.id.clone())
}

fn follow(go: Go, mut ui: Ui, settings: SettingsNav) {
    open(false);
    if !matches!(go, Go::Settings | Go::Section(_)) {
        settings.close();
    }
    match go {
        Go::Screen(screen) => ui.screen.set(screen),
        Go::Tool(label) => {
            if let Some(tool) = crate::more::TOOLS.iter().find(|t| t.label == label) {
                (tool.open)(ui);
            }
        }
        Go::Settings => settings.show(None),
        Go::Section(section) => settings.show(Some(section)),
        Go::Questions(id) => crate::bank::practise(ui, &id, None),
        Go::Question(id) => crate::bank::open_question(ui, &id),
        Go::Link(url) => {
            if let Err(e) = crate::cloud::open_url(&url) {
                crate::diag::report!("quick search didn't open {url}: {e}");
            }
        }
        Go::Page(page, lecture) => crate::pagesheet::open(page, &lecture, None, false, None),
        Go::Lecture(id) => crate::lecture::open(ui, &id),
        Go::Lesson(id) => crate::learn::open(ui, &id),
        Go::Topic(lecture, topic) => match lesson_of(Library::get(), &lecture, &topic) {
            Some(lesson) => crate::learn::open(ui, &lesson),
            None => crate::lecture::open(ui, &lecture),
        },
        Go::Term(course, term) => crate::abbrdict::open_search(ui, Some(course), &term),
        Go::SearchLessons(q) => crate::learn::search(ui, &q),
        Go::SearchTerms(q) => crate::abbrdict::open_search(ui, None, &q),
    }
}

/// Drawn once by the shell; empty while closed.
#[component]
pub fn Palette() -> Element {
    let ui = use_context::<Ui>();
    let settings = use_context::<SettingsNav>();
    let mut query = use_signal(String::new);
    let _ = crate::content::REV.read();
    if !OPEN() {
        if !query.peek().is_empty() {
            query.set(String::new());
        }
        return rsx! {};
    }
    let hits = find(&query.read(), Library::get());
    rsx! {
        div { class: "sheet-backdrop qs-backdrop", onclick: move |_| dismiss() }
        section {
            class: "qs",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "qs-title",
            onmounted: move |_| {
                document::eval("window.__stepsSearchOpener = document.activeElement;");
            },
            h2 { id: "qs-title", class: "qs-sr", {tx::QS_TITLE} }
            div { class: "qs-field",
                span { class: "qs-icon", aria_hidden: "true", {crate::icons::search()} }
                input {
                    class: "qs-input",
                    r#type: "search",
                    "aria-label": tx::QS_TITLE,
                    "aria-controls": "qs-list",
                    placeholder: tx::QS_PLACEHOLDER,
                    autocomplete: "off",
                    spellcheck: "false",
                    value: "{query}",
                    onmounted: move |e| async move {
                        let _ = e.set_focus(true).await;
                    },
                    oninput: move |e| query.set(e.value()),
                }
                button {
                    r#type: "button",
                    class: "qs-close",
                    "aria-label": tx::CLOSE,
                    "aria-keyshortcuts": "Escape",
                    "data-back": "true",
                    onclick: move |_| dismiss(),
                    kbd { "Esc" }
                }
            }
            div { id: "qs-list", class: "qs-list", role: "list",
                if nothing_found(&hits) && !query.read().trim().is_empty() {
                    p { class: "qs-empty", role: "status", {tx::QS_NONE} }
                }
                for hit in hits {
                    button {
                        key: "{hit.key}",
                        r#type: "button",
                        role: "listitem",
                        class: "qs-row",
                        onclick: {
                            let go = hit.go.clone();
                            move |_| follow(go.clone(), ui, settings)
                        },
                        if hit.english {
                            bdi { class: "qs-name", dir: "ltr", lang: "en", "{hit.name}" }
                        } else {
                            span { class: "qs-name", "{hit.name}" }
                        }
                        span { class: "qs-sub",
                            "{hit.sub}"
                            if let Some(d) = hit.detail.as_ref() {
                                " · "
                                bdi { dir: "ltr", lang: "en", "{d}" }
                            }
                        }
                    }
                }
            }
            p { class: "qs-hint", {tx::QS_HINT} }
        }
    }
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod tests;
