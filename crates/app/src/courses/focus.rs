//! The course in view (Mohamed, 2026-10-29): one tap on the course chip at
//! the top of Lessons, Review and Questions opens a sheet of his courses;
//! picking one narrows those screens to it, and "all courses" shows
//! everything again. A synced setting, `course_focus` (empty = all), so the
//! phone and the tablet show the same course.
//!
//! Only what he browses is narrowed. A lesson, lecture or paper opened by
//! id elsewhere (Today, a book page, a lecture page) always opens, and the
//! bank's "practise this lecture" moves the focus to that lecture's course.

use dioxus::prelude::*;
use steps_core::program::{CourseStatus, course_of};

use super::{Summary, summaries};
use crate::content::Library;
use crate::i18n::courses as tx;

const KEY: &str = "course_focus";

static FOCUS: GlobalSignal<Option<String>> = Signal::global(load);
/// `STEPS_COURSE_SHEET=1` opens the sheet at launch (previews, screenshots).
static PICKER: GlobalSignal<bool> = Signal::global(|| std::env::var_os("STEPS_COURSE_SHEET").is_some());

fn load() -> Option<String> {
    // STEPS_COURSE=hb (or "all"), for previews and screenshots.
    let id = std::env::var("STEPS_COURSE")
        .ok()
        .or_else(|| crate::store::shared().setting::<String>(KEY))
        .unwrap_or_default();
    known(&id).then_some(id)
}

/// A course of the program that has lectures in the index.
fn known(id: &str) -> bool {
    let lib = Library::get();
    lib.program.courses.iter().any(|c| c.id == id)
        && lib.index.lectures.iter().any(|e| course_of(&e.id) == id)
}

/// Re-reads the focus (a sync or restore changed the store).
pub fn reload() {
    let fresh = load();
    if *FOCUS.peek() != fresh {
        *FOCUS.write() = fresh;
    }
}

/// The course in view; `None` = every course.
pub fn current() -> Option<String> {
    FOCUS()
}

/// Does `id` (a lecture, item or lesson id) belong to the course in view?
/// Always true with every course in view.
pub fn shows(id: &str) -> bool {
    FOCUS.read().as_deref().is_none_or(|c| course_of(id) == c)
}

/// Puts `course` in view (`None` = all) and keeps it.
pub fn set(course: Option<String>) {
    let course = course.filter(|c| known(c));
    if *FOCUS.peek() == course {
        return;
    }
    crate::store::save_setting(KEY, &course.clone().unwrap_or_default());
    *FOCUS.write() = course;
    crate::bank::course_changed();
}

/// Moves the focus to `id`'s course, unless every course is in view.
pub fn follow(id: &str) {
    if FOCUS.peek().as_deref().is_some_and(|c| c != course_of(id)) {
        set(Some(course_of(id).to_string()));
    }
}

pub fn is_open() -> bool {
    PICKER()
}

/// Closes the course sheet when the screen changes: it belongs to the
/// screen whose chip opened it.
pub fn use_close_on_screen_change(ui: crate::Ui) {
    let mut last = use_signal(|| *ui.screen.peek());
    use_effect(move || {
        let now = *ui.screen.read();
        if *last.peek() != now {
            last.set(now);
            close();
        }
    });
}

/// Closes the course sheet (a pick, another screen).
pub fn close() {
    if *PICKER.peek() {
        *PICKER.write() = false;
    }
}

/// The course in view as a chip: the screen's way to the course sheet.
#[component]
pub fn CourseChip() -> Element {
    let lib = Library::get();
    let focus = current();
    let course = focus.as_deref().and_then(|id| lib.program.courses.iter().find(|c| c.id == id));
    let name = course.map_or(tx::ALL, |c| c.short.as_str()).to_string();
    let label = tx::chip_label(&name);
    rsx! {
        style { {super::CSS.text()} }
        button {
            class: "cf-chip",
            "aria-haspopup": "dialog",
            "aria-expanded": if is_open() { "true" } else { "false" },
            "data-all": if course.is_none() { "true" } else { "false" },
            aria_label: "{label}",
            title: "{label}",
            onclick: move |_| {
                let open = *PICKER.peek();
                *PICKER.write() = !open;
            },
            span { class: "cf-chip-icon", aria_hidden: "true", {crate::icons::courses()} }
            span { class: "cf-chip-name", "{name}" }
            if let Some(c) = course {
                bdi { class: "cf-chip-title", dir: "ltr", lang: "en", "{c.title}" }
            }
            span { class: "cf-chip-caret", aria_hidden: "true", {crate::icons::caret_down()} }
        }
    }
}

/// The order courses are listed in: this semester's first.
const ORDER: [CourseStatus; 4] =
    [CourseStatus::Active, CourseStatus::Maintenance, CourseStatus::Later, CourseStatus::Archived];

/// Lessons done and in all, for a course (or every course).
fn lesson_counts(lib: &Library, course: Option<&str>) -> (usize, usize) {
    let mut done = 0;
    let mut all = 0;
    for (lec, l) in lib.all_lessons() {
        if course.is_none_or(|c| course_of(lec) == c) {
            all += 1;
            done += usize::from(crate::learn::mark(&l.id).is_some());
        }
    }
    (done, all)
}

/// The courses the sheet lists: the ones with lectures in the index, this
/// semester's first, then the program's order.
pub fn listed(all: Vec<Summary>) -> Vec<Summary> {
    let mut shown: Vec<Summary> = all.into_iter().filter(|s| s.lectures > 0).collect();
    shown.sort_by_key(|s| ORDER.iter().position(|o| *o == s.status).unwrap_or(ORDER.len()));
    shown
}

/// The course sheet: every course with lectures, and "all courses".
#[component]
pub fn Picker() -> Element {
    let ui = use_context::<crate::Ui>();
    if !is_open() {
        return rsx! {};
    }
    let lib = Library::get();
    let focus = current();
    let courses = listed(summaries(lib, super::status));
    let (all_done, all_lessons) = lesson_counts(lib, None);
    let pick = move |c: Option<String>| {
        set(c);
        close();
        document::eval(r#"document.querySelector("main.content")?.scrollTo(0, 0);"#);
    };
    rsx! {
        style { {super::CSS.text()} }
        crate::sheet::Sheet {
            label_id: "cf-title",
            on_close: move |_| close(),
            head: rsx! {
                div {
                    h2 { id: "cf-title", {tx::PICK_TITLE} }
                }
            },
            ul { class: "cf-list", role: "radiogroup", aria_labelledby: "cf-title",
                li {
                    button {
                        class: "tap cf-row",
                        role: "radio",
                        "aria-checked": if focus.is_none() { "true" } else { "false" },
                        onclick: move |_| pick(None),
                        span { class: "cf-badge", "data-all": "true", aria_hidden: "true", {crate::icons::courses()} }
                        span { class: "cf-main",
                            strong { {tx::ALL} }
                            span { class: "muted small", {tx::all_sub(courses.len())} }
                        }
                        span { class: "cf-end small", {tx::lessons_done(all_done, all_lessons)} }
                        span { class: "cf-tick", aria_hidden: "true", {crate::icons::check()} }
                    }
                }
                for s in courses {
                    {
                        let (done, n) = lesson_counts(lib, Some(&s.course.id));
                        let on = focus.as_deref() == Some(s.course.id.as_str());
                        let id = s.course.id.clone();
                        rsx! {
                            li { key: "{s.course.id}",
                                button {
                                    class: "tap cf-row",
                                    role: "radio",
                                    "aria-checked": if on { "true" } else { "false" },
                                    onclick: move |_| pick(Some(id.clone())),
                                    span { class: "cf-badge", "data-group": super::map::group_key(s.course.group), aria_hidden: "true",
                                        {s.course.short.chars().next().map(String::from).unwrap_or_default()}
                                    }
                                    span { class: "cf-main",
                                        strong { "{s.course.short}" }
                                        bdi { class: "muted small cf-title", dir: "ltr", lang: "en", "{s.course.title}" }
                                    }
                                    span { class: "cf-end small",
                                        if n > 0 {
                                            {tx::lessons_done(done, n)}
                                        } else {
                                            {tx::lectures_only(s.lectures)}
                                        }
                                        if s.status != CourseStatus::Active {
                                            span { class: "chip", {super::map::status_label(s.status)} }
                                        }
                                    }
                                    span { class: "cf-tick", aria_hidden: "true", {crate::icons::check()} }
                                }
                            }
                        }
                    }
                }
            }
            button {
                class: "btn btn-ghost cf-map",
                onclick: move |_| {
                    close();
                    super::open_map(ui);
                },
                {crate::icons::courses()}
                {tx::MAP_LINK}
                span { class: "chev", aria_hidden: "true" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheet_lists_courses_with_lectures_this_semester_first() {
        let lib = Library::get();
        let listed = listed(summaries(lib, |c| c.status));
        assert!(listed.iter().all(|s| s.lectures > 0));
        assert!(listed.iter().any(|s| s.course.id == "cvs"));
        let first_later = listed.iter().position(|s| s.status != CourseStatus::Active);
        if let Some(i) = first_later {
            assert!(listed[i..].iter().all(|s| s.status != CourseStatus::Active));
        }
        assert!(known("cvs"));
        assert!(!known("nope"));
        assert!(!known(""));
    }
}
