//! The abbreviations dictionary (Mohamed, 2026-11-01): every abbreviation
//! the lessons use, one course at a time, reachable any time from the
//! Lessons screen, More (the rail from 600dp) and a lesson step's
//! abbreviations sheet. A–Z with a search over the term and its meaning; a
//! term opens to show the lessons that use it, each a way into the lesson.
//!
//! It is a page on the lecture page stack (`lecture::open`), like the
//! references list, so Back and leaving the screen close it.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use steps_core::content::Status;
use steps_core::glossary::Entry;

use crate::Ui;
use crate::content::{Derived, Library};
use crate::i18n::abbr as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/abbrdict/abbrdict.css");

/// The page id on the lecture page stack.
pub const PAGE: &str = "abbreviations";

/// One abbreviation of a course: its entry and the lessons that use it, in
/// book order.
#[derive(Debug)]
pub struct Term {
    pub entry: &'static Entry,
    pub lessons: Vec<&'static str>,
}

/// Each course's abbreviations, A–Z, by course id.
pub fn index() -> &'static BTreeMap<String, Vec<Term>> {
    static INDEX: Derived<BTreeMap<String, Vec<Term>>> = Derived::new();
    INDEX.get(|lib| {
        let mut by_course: BTreeMap<String, Vec<Term>> = BTreeMap::new();
        // Each course reads its own meaning of a term, as its lessons show it (#318).
        let mut finders = BTreeMap::new();
        for (lecture, lesson) in lib.all_lessons() {
            let course = steps_core::program::course_of(lecture);
            let finder = finders.entry(course).or_insert_with(|| crate::hover::glossary_for(course).finder());
            let terms = by_course.entry(course.to_string()).or_default();
            let texts: Vec<String> = lesson.steps.iter().flat_map(|s| s.shown_text()).collect();
            for e in finder.abbreviations_in(&texts) {
                match terms.iter_mut().find(|t| std::ptr::eq(t.entry, e)) {
                    Some(t) => t.lessons.push(lesson.id.as_str()),
                    None => terms.push(Term { entry: e, lessons: vec![lesson.id.as_str()] }),
                }
            }
        }
        for terms in by_course.values_mut() {
            terms.sort_by_cached_key(|t| (t.entry.term.to_lowercase(), t.entry.term.clone()));
        }
        by_course
    })
}

/// The course the page shows; set by whoever opens it.
pub(crate) static COURSE: GlobalSignal<Option<String>> = Signal::global(|| None);

/// Opens the dictionary over the current screen at `course` (a course id;
/// `None`: the course in view, else the first one with lessons).
pub fn open(ui: Ui, course: Option<String>) {
    crate::abbrsheet::close();
    crate::pagesheet::close();
    *COURSE.write() = course.or_else(crate::courses::focus::current).filter(|c| index().contains_key(c));
    crate::lecture::open(ui, PAGE);
}

/// The courses that have abbreviations, in the program's order.
fn courses() -> Vec<(&'static str, &'static str)> {
    let lib = Library::get();
    let idx = index();
    lib.program
        .courses
        .iter()
        .filter(|c| idx.get(&c.id).is_some_and(|t| !t.is_empty()))
        .map(|c| (c.id.as_str(), c.short.as_str()))
        .collect()
}

/// The terms whose spelling or meaning holds every word of `query`.
pub fn search<'a>(terms: &'a [Term], query: &str) -> Vec<&'a Term> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    terms
        .iter()
        .filter(|t| {
            let hay =
                format!("{} {} {}", t.entry.term, t.entry.aliases.join(" "), t.entry.text).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

/// The letter a term is filed under: `A`–`Z`, else `#` (digits, Greek).
fn letter(term: &str) -> char {
    term.chars().next().map(|c| c.to_ascii_uppercase()).filter(char::is_ascii_alphabetic).unwrap_or('#')
}

#[component]
pub fn Dictionary() -> Element {
    let ui = use_context::<Ui>();
    let mut query = use_signal(String::new);
    let list = courses();
    let course = COURSE()
        .filter(|c| list.iter().any(|(id, _)| id == c))
        .or_else(|| list.first().map(|(id, _)| id.to_string()));
    let Some(course) = course else {
        return rsx! {
            div { class: "lec-page abd",
                p { class: "note", {tx::DICT_EMPTY} }
            }
        };
    };
    let lib = Library::get();
    let title =
        lib.program.courses.iter().find(|c| c.id == course).map(|c| c.title.clone()).unwrap_or_default();
    let all = index().get(&course).map(Vec::as_slice).unwrap_or_default();
    let shown = search(all, &query.read());
    let mut groups: Vec<(char, Vec<&Term>)> = Vec::new();
    for t in shown.iter().copied() {
        let l = letter(&t.entry.term);
        match groups.last_mut() {
            Some((g, v)) if *g == l => v.push(t),
            _ => groups.push((l, vec![t])),
        }
    }
    // Digits and Greek after Z.
    groups.sort_by_key(|(l, _)| (*l == '#', *l));
    let unverified = shown.iter().any(|t| t.entry.status != Status::Approved);

    rsx! {
        style { {crate::lecture::CSS.text()} }
        style { {CSS.text()} }
        div { class: "lec-page abd", "data-course": "{course}",
            div { class: "abd-head",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: crate::i18n::lecture::BACK,
                    title: crate::i18n::lecture::BACK,
                    onclick: move |_| crate::lecture::close(),
                    span { aria_hidden: "true", "→" }
                }
                header { class: "page-head",
                    h1 { {tx::DICT_TITLE} }
                    p { class: "page-sub",
                        bdi { lang: "en", dir: "ltr", "{title}" }
                        " · {tx::count(all.len())}"
                    }
                }
            }
            div { class: "abd-courses", role: "group", aria_label: tx::DICT_COURSES,
                for (id, short) in list {
                    button {
                        key: "{id}",
                        class: "chip tap-chip abd-course",
                        "data-tone": if id == course { "accent" } else { "" },
                        aria_pressed: if id == course { "true" } else { "false" },
                        onclick: move |_| *COURSE.write() = Some(id.to_string()),
                        "{short}"
                    }
                }
            }
            label { class: "search",
                {crate::icons::search()}
                input {
                    r#type: "search",
                    dir: "auto",
                    aria_label: tx::DICT_SEARCH,
                    placeholder: tx::DICT_SEARCH,
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }
            if groups.is_empty() {
                p { class: "note", {tx::DICT_NO_MATCH} }
            }
            for (l, terms) in groups {
                section { key: "{l}", class: "abd-letter", lang: "en", dir: "ltr",
                    h2 { class: "abd-l", "{l}" }
                    for t in terms {
                        details { key: "{t.entry.term}", class: "abd-item",
                            summary { class: "abd-sum",
                                span { class: "abd-term", "{t.entry.term}" }
                                span { class: "abd-text", "{t.entry.text}" }
                            }
                            div { class: "abd-uses", lang: "ar", dir: "rtl",
                                p { class: "muted small", {tx::count_lessons(t.lessons.len())} }
                                ul { class: "abd-lessons",
                                    for id in t.lessons.iter().copied() {
                                        if let Some((lec, lesson)) = lib.lesson(id) {
                                            li { key: "{id}",
                                                button {
                                                    class: "abd-lesson tap",
                                                    onclick: move |_| {
                                                        crate::lecture::close_all();
                                                        crate::learn::open(ui, id);
                                                    },
                                                    bdi { class: "abd-lesson-title", "{lesson.title}" }
                                                    span { class: "muted small",
                                                        bdi { lang: "en", dir: "ltr", {lib.crumb(&lec.lecture)} }
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
            if unverified {
                p { class: "abd-note muted small", {crate::i18n::abbr::UNVERIFIED} }
            }
        }
    }
}

/// The Lessons screen's way in.
#[component]
pub fn OpenButton() -> Element {
    let ui = use_context::<Ui>();
    rsx! {
        button {
            class: "chip tap-chip abd-open",
            aria_haspopup: "dialog",
            onclick: move |_| open(ui, None),
            {tx::DICT_TITLE}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_course_with_lessons_has_its_own_list() {
        let idx = index();
        let cvs = idx.get("cvs").expect("cardiovascular has abbreviations");
        let pfo = cvs.iter().find(|t| t.entry.term == "PFO").expect("PFO in cardiovascular");
        assert!(pfo.lessons.iter().all(|id| id.starts_with("cvs.")), "{:?}", pfo.lessons);
        assert!(!pfo.lessons.is_empty());
        // Lists are A–Z and hold each term once.
        let terms: Vec<String> = cvs.iter().map(|t| t.entry.term.to_lowercase()).collect();
        let mut sorted = terms.clone();
        sorted.sort();
        assert_eq!(terms, sorted);
        // Each term once (`HBIG` and `HBIg` may both be there).
        let mut exact: Vec<&str> = cvs.iter().map(|t| t.entry.term.as_str()).collect();
        exact.sort();
        exact.dedup();
        assert_eq!(exact.len(), cvs.len());
        // Only courses with lessons have a list, and only real courses.
        let lib = Library::get();
        for course in idx.keys() {
            assert!(lib.program.courses.iter().any(|c| &c.id == course), "{course}");
            assert!(
                lib.lessons.keys().any(|l| steps_core::program::course_of(l) == course),
                "{course} has a list but no lessons"
            );
        }
    }

    #[test]
    fn search_finds_the_term_or_its_meaning() {
        let cvs = &index()["cvs"];
        assert!(search(cvs, "pfo").iter().any(|t| t.entry.term == "PFO"));
        assert!(search(cvs, "foramen ovale").iter().any(|t| t.entry.term == "PFO"));
        assert_eq!(search(cvs, "").len(), cvs.len());
        assert!(search(cvs, "zzzz-nothing").is_empty());
    }

    #[test]
    fn letters_file_digits_and_greek_under_hash() {
        assert_eq!(letter("hCG"), 'H');
        assert_eq!(letter("5HT3"), '#');
        assert_eq!(letter("β2GPI"), '#');
    }
}
