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

/// The same entry of the glossary, used by the same lessons.
impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.entry, other.entry) && self.lessons == other.lessons
    }
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
/// What the search box holds; quick search can open the page with it.
static QUERY: GlobalSignal<String> = Signal::global(String::new);

/// Opens the dictionary over the current screen at `course` (a course id;
/// `None`: the course in view, else the first one with lessons).
pub fn open(ui: Ui, course: Option<String>) {
    crate::abbrsheet::close();
    crate::pagesheet::close();
    *COURSE.write() = course.or_else(crate::courses::focus::current).filter(|c| index().contains_key(c));
    QUERY.write().clear();
    crate::lecture::open(ui, PAGE);
}

/// Opens it searching for `query` (quick search's way in), at `course`.
pub fn open_search(ui: Ui, course: Option<String>, query: &str) {
    open(ui, course);
    *QUERY.write() = query.to_string();
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

/// How well `entry` answers `query`, best first: the abbreviation itself
/// (or an alias), then one it starts, then one that holds the query, then
/// a meaning with a word starting with each word of the query. `None`: no
/// match. A meaning never matches inside a word, so "ACE" doesn't find
/// "surface" (UX-30).
pub fn rank(entry: &Entry, query: &str) -> Option<u8> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    let spellings: Vec<String> =
        std::iter::once(&entry.term).chain(&entry.aliases).map(|t| t.to_lowercase()).collect();
    if spellings.contains(&q) {
        return Some(0);
    }
    if spellings.iter().any(|t| t.starts_with(&q)) {
        return Some(1);
    }
    if spellings.iter().any(|t| t.contains(&q)) {
        return Some(2);
    }
    let text = entry.text.to_lowercase();
    let starts: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    q.split_whitespace().all(|w| starts.iter().any(|s| s.starts_with(w))).then_some(3)
}

/// The terms that answer `query`, best match first (A–Z within a rank);
/// every term when it is empty.
pub fn search<'a>(terms: &'a [Term], query: &str) -> Vec<&'a Term> {
    let mut hits: Vec<(u8, &Term)> =
        terms.iter().filter_map(|t| rank(t.entry, query).map(|r| (r, t))).collect();
    hits.sort_by_key(|(r, _)| *r);
    hits.into_iter().map(|(_, t)| t).collect()
}

/// The same search in every other course: each hit with its course id,
/// best first, a term explained the same way in two courses once.
pub fn search_elsewhere(course: &str, query: &str) -> Vec<(&'static str, &'static Term)> {
    let lib = Library::get();
    let idx = index();
    let mut hits: Vec<(u8, &'static str, &'static Term)> = Vec::new();
    for c in &lib.program.courses {
        if c.id == course {
            continue;
        }
        let Some(terms) = idx.get(&c.id) else { continue };
        for t in terms {
            let Some(r) = rank(t.entry, query) else { continue };
            let same =
                |(_, _, o): &(u8, &str, &Term)| o.entry.term == t.entry.term && o.entry.text == t.entry.text;
            if !hits.iter().any(same) {
                hits.push((r, c.id.as_str(), t));
            }
        }
    }
    hits.sort_by_key(|(r, _, _)| *r);
    hits.into_iter().map(|(_, c, t)| (c, t)).collect()
}

/// The letter a term is filed under: `A`–`Z`, else `#` (digits, Greek).
fn letter(term: &str) -> char {
    term.chars().next().map(|c| c.to_ascii_uppercase()).filter(char::is_ascii_alphabetic).unwrap_or('#')
}

#[component]
pub fn Dictionary() -> Element {
    let query = QUERY.signal();
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
    let typed = query.read().trim().to_string();
    let shown = search(all, &typed);
    // A search lists best matches first; the A–Z letters are for browsing.
    let mut groups: Vec<(char, Vec<&Term>)> = Vec::new();
    if typed.is_empty() {
        for t in shown.iter().copied() {
            let l = letter(&t.entry.term);
            match groups.last_mut() {
                Some((g, v)) if *g == l => v.push(t),
                _ => groups.push((l, vec![t])),
            }
        }
        // Digits and Greek after Z.
        groups.sort_by_key(|(l, _)| (*l == '#', *l));
    } else if !shown.is_empty() {
        groups.push(('*', shown.clone()));
    }
    // Every other course too, each hit tagged with its course (UX-30).
    let elsewhere = if typed.is_empty() { Vec::new() } else { search_elsewhere(&course, &typed) };
    let short_of =
        |c: &str| lib.program.courses.iter().find(|x| x.id == c).map(|x| x.short.clone()).unwrap_or_default();
    let unverified = shown
        .iter()
        .copied()
        .chain(elsewhere.iter().map(|(_, t)| *t))
        .any(|t| t.entry.status != Status::Approved);

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
            // A sideways scroller on phones (abbrdict.css): every course is
            // reachable by scrolling, which the browser check knows by data-scroll-x.
            div { class: "abd-courses", role: "group", aria_label: tx::DICT_COURSES, "data-scroll-x": "true",
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
                    oninput: move |e| *QUERY.write() = e.value(),
                }
            }
            if groups.is_empty() {
                p { class: "note", {tx::DICT_NO_MATCH} }
            }
            for (l, terms) in groups {
                section { key: "{l}", class: "abd-letter", lang: "en", dir: "ltr",
                    if l != '*' {
                        h2 { class: "abd-l", "{l}" }
                    }
                    for t in terms {
                        TermItem { key: "{t.entry.term}", term: t, course: None }
                    }
                }
            }
            if !elsewhere.is_empty() {
                section { class: "abd-letter abd-elsewhere", lang: "en", dir: "ltr",
                    h2 { class: "abd-l", lang: "ar", dir: "rtl", {tx::DICT_ELSEWHERE} }
                    for (c, t) in elsewhere {
                        TermItem { key: "{c}-{t.entry.term}", term: t, course: Some(short_of(c)) }
                    }
                }
            }
            if unverified {
                p { class: "abd-note muted small", {crate::i18n::abbr::UNVERIFIED} }
            }
        }
    }
}

/// One term: its spelling and meaning, opening to the lessons that use it.
/// `course` tags a hit from another course.
#[component]
fn TermItem(term: &'static Term, course: Option<String>) -> Element {
    let ui = use_context::<Ui>();
    let lib = Library::get();
    let t = term;
    rsx! {
        details { class: "abd-item",
            summary { class: "abd-sum",
                span { class: "abd-term", "{t.entry.term}" }
                span { class: "abd-text",
                    if let Some(c) = course {
                        span { class: "chip abd-in", lang: "ar", dir: "rtl", "{c}" }
                        " "
                    }
                    "{t.entry.text}"
                }
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
    fn the_abbreviation_itself_comes_before_a_meaning_and_no_match_is_inside_a_word() {
        use steps_core::glossary::Kind;
        let entry = |term: &str, text: &str| Entry {
            term: term.into(),
            aliases: Vec::new(),
            kind: Kind::Abbr,
            text: text.into(),
            status: Status::Unverified,
        };
        let ace = entry("ACE", "Angiotensin-converting enzyme");
        let acei = entry("ACEi", "ACE inhibitor");
        let bsa = entry("BSA", "Body surface area");
        assert_eq!(rank(&ace, "ace"), Some(0));
        assert_eq!(rank(&acei, "ace"), Some(1));
        assert_eq!(rank(&bsa, "ace"), None, "not inside a word of the meaning");
        assert_eq!(rank(&bsa, "surf"), Some(3), "a meaning's word may start with it");
        let terms: Vec<Term> = [bsa, acei, ace]
            .into_iter()
            .map(|e| Term { entry: Box::leak(Box::new(e)), lessons: Vec::new() })
            .collect();
        let found: Vec<&str> = search(&terms, "ace").iter().map(|t| t.entry.term.as_str()).collect();
        assert_eq!(found, ["ACE", "ACEi"]);
    }

    #[test]
    fn a_search_also_finds_other_courses_terms_each_once() {
        let idx = index();
        let (course, terms) = idx.iter().find(|(_, t)| !t.is_empty()).expect("a course with terms");
        let term = &terms[0].entry.term;
        let other = idx.keys().find(|c| *c != course).expect("another course");
        let hits = search_elsewhere(other, term);
        assert!(hits.iter().any(|(c, t)| c == course && &t.entry.term == term), "{term} from {course}");
        assert!(hits.iter().all(|(c, _)| c != other), "never the course in view");
    }

    #[test]
    fn letters_file_digits_and_greek_under_hash() {
        assert_eq!(letter("hCG"), 'H');
        assert_eq!(letter("5HT3"), '#');
        assert_eq!(letter("β2GPI"), '#');
    }
}
