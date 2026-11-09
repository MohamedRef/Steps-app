//! Quick search, Ctrl/⌘ + K (or the search button on the rail): one box
//! that jumps to a screen, a tool, a lecture, a lesson, a First Aid page
//! ("ص 32" or just "32"), a Settings section, or a lecture's questions
//! ("questions heart failure", "أسئلة …"). Everything comes from the library on the device;
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

const MAX: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Go {
    Screen(Screen),
    Tool(&'static str),
    Settings,
    Section(Section),
    Questions(String),
    Page(u32, String),
    Lecture(String),
    Lesson(String),
}

#[derive(Clone, Debug)]
struct Hit {
    key: String,
    name: String,
    sub: String,
    english: bool,
    go: Go,
}

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
                });
            }
        }
    }
    if let crate::gopage::Target::Page { page, lecture } = crate::gopage::target(lib, &q) {
        hits.push(Hit {
            key: format!("p-{page}"),
            name: format!("{} {page}", tx::KIND_PAGE),
            sub: lib.crumb(&lecture),
            english: false,
            go: Go::Page(page, lecture),
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
            });
        }
    }
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
    hits
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
        Go::Page(page, lecture) => crate::pagesheet::open(page, &lecture, None, false, None),
        Go::Lecture(id) => crate::lecture::open(ui, &id),
        Go::Lesson(id) => crate::learn::open(ui, &id),
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
        div { class: "sheet-backdrop qs-backdrop", onclick: move |_| open(false) }
        section {
            class: "qs",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "qs-title",
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
                    onclick: move |_| open(false),
                    kbd { "Esc" }
                }
            }
            div { id: "qs-list", class: "qs-list", role: "list",
                if hits.is_empty() {
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
                        span { class: "qs-sub", "{hit.sub}" }
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
