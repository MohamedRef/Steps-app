//! The MSc map (More › المقررات): every course of the program as a tile,
//! grouped by where it stands, with the credit hours behind him. A tile
//! opens its course: hours, lectures, and the status he can change.

use dioxus::prelude::*;
use steps_core::program::{CourseStatus, Group};

use super::CSS;
use super::{Summary, fmt_hours, hours, set_status, summaries};
use crate::content::Library;
use crate::i18n::courses as tx;

/// The order of the map's groups.
const ORDER: [CourseStatus; 4] =
    [CourseStatus::Active, CourseStatus::Maintenance, CourseStatus::Later, CourseStatus::Archived];

pub(super) fn status_label(s: CourseStatus) -> &'static str {
    match s {
        CourseStatus::Active => tx::ACTIVE,
        CourseStatus::Maintenance => tx::MAINTENANCE,
        CourseStatus::Archived => tx::ARCHIVED,
        CourseStatus::Later => tx::LATER,
    }
}

fn status_hint(s: CourseStatus) -> &'static str {
    match s {
        CourseStatus::Active => tx::ACTIVE_HINT,
        CourseStatus::Maintenance => tx::MAINTENANCE_HINT,
        CourseStatus::Archived => tx::ARCHIVED_HINT,
        CourseStatus::Later => tx::LATER_HINT,
    }
}

fn group_label(g: Group) -> &'static str {
    match g {
        Group::General => tx::G_GENERAL,
        Group::Systems => tx::G_SYSTEMS,
    }
}

pub(super) fn group_key(g: Group) -> &'static str {
    match g {
        Group::General => "general",
        Group::Systems => "systems",
    }
}

#[component]
pub fn Map() -> Element {
    let ui = use_context::<crate::Ui>();
    let lib = Library::get();
    let all = summaries(lib, super::status);
    let (done, now, total) = hours(&all);
    let mut open = use_signal(|| None::<String>);
    // The ring: done hours solid, this semester's lighter after them.
    let c = 2.0 * std::f32::consts::PI * 42.0;
    let done_len = c * done / total.max(1.0);
    let now_len = c * now / total.max(1.0);

    rsx! {
        style { {CSS.text()} }
        style { {crate::lecture::CSS.text()} }
        div { class: "lec-page courses",
            div { class: "refs-head",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: crate::i18n::lecture::BACK,
                    title: crate::i18n::lecture::BACK,
                    onclick: move |_| crate::lecture::close(),
                    span { aria_hidden: "true", "→" }
                }
                header { class: "page-head refs-title",
                    h1 { {tx::TITLE} }
                    p { class: "page-sub", bdi { "{lib.program.title}" } }
                }
            }
            crate::ui::Card { class: "cm-hours",
                svg {
                    class: "cm-ring",
                    view_box: "0 0 104 104",
                    width: "104",
                    height: "104",
                    "aria-hidden": "true",
                    circle { class: "cm-ring-track", cx: "52", cy: "52", r: "42" }
                    circle {
                        class: "cm-ring-now",
                        cx: "52",
                        cy: "52",
                        r: "42",
                        stroke_dasharray: "{done_len + now_len} {c}",
                        transform: "rotate(-90 52 52)",
                    }
                    if done > 0.0 {
                        circle {
                            class: "cm-ring-done",
                        cx: "52",
                        cy: "52",
                        r: "42",
                            stroke_dasharray: "{done_len} {c}",
                            transform: "rotate(-90 52 52)",
                        }
                    }
                    text { class: "cm-ring-num", x: "52", y: "58", text_anchor: "middle",
                        "{fmt_hours(done + now)}/{fmt_hours(total)}"
                    }
                }
                div { class: "cm-hours-text",
                    p { class: "cm-big", {tx::hours_line(&fmt_hours(total), all.len())} }
                    p { span { class: "cm-key cm-key-done" } {tx::hours_done(&fmt_hours(done))} }
                    p { span { class: "cm-key cm-key-now" } {tx::hours_now(&fmt_hours(now))} }
                }
            }
            for st in ORDER {
                if all.iter().any(|s| s.status == st) {
                    section { key: "{st.key()}", class: "cm-group", aria_label: status_label(st),
                        h2 { {status_label(st)} }
                        p { class: "muted small", {status_hint(st)} }
                        div { class: "cm-grid",
                            for s in all.iter().filter(|s| s.status == st).cloned() {
                                CourseTile {
                                    key: "{s.course.id}",
                                    open: open() == Some(s.course.id.clone()),
                                    on_toggle: move |id: String| {
                                        let same = open() == Some(id.clone());
                                        open.set(if same { None } else { Some(id) });
                                    },
                                    on_lectures: move |id: String| open_course_lessons(ui, &id),
                                    summary: s,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Browse the selected course's lectures and lessons, not References.
fn open_course_lessons(mut ui: crate::Ui, id: &str) {
    super::focus::set(Some(id.to_string()));
    crate::learn::reset_course_browse();
    crate::lecture::close_all();
    ui.screen.set(crate::Screen::Learn);
}

#[component]
fn CourseTile(
    summary: Summary,
    open: bool,
    on_toggle: EventHandler<String>,
    on_lectures: EventHandler<String>,
) -> Element {
    let s = summary;
    let id = s.course.id.clone();
    let lecture_id = id.clone();
    let half = s.course.hours.fract() != 0.0;
    let whole = s.course.hours.trunc() as usize;
    rsx! {
        div { class: "cm-cell", "data-open": open,
            crate::ui::CardButton {
                class: "tap tile cm-tile",
                "data-group": group_key(s.course.group),
                "aria-expanded": open,
                "aria-controls": "cm-detail-{s.course.id}",
                onclick: move |_| on_toggle.call(id.clone()),
                span { class: "cm-bar", aria_hidden: "true" }
                span { class: "cm-name", bdi { "{s.course.title}" } }
                span { class: "cm-meta",
                    span { class: "cm-dots", aria_hidden: "true",
                        for k in 0..whole {
                            i { key: "{k}" }
                        }
                        if half {
                            i { class: "half" }
                        }
                    }
                    span { class: "muted small", {tx::tile_line(&s.course.short, &fmt_hours(s.course.hours))} }
                }
                if s.lectures > 0 {
                    span { class: "cm-coverage muted small", {tx::ready_lectures(s.written, s.lectures)} }
                }
            }
            if open {
                crate::ui::Card {
                    element: crate::ui::CardElement::Div,
                    class: "cm-detail",
                    id: "cm-detail-{s.course.id}",
                    role: "region",
                    aria_label: s.course.title.clone(),
                    p { class: "small",
                        {group_label(s.course.group)}
                        if let Some(n) = s.course.semester {
                            " · "
                            {tx::semester(n)}
                        }
                    }
                    if s.lectures > 0 {
                        p { class: "small", {tx::content_line(s.written, s.lectures, s.items, s.lessons)} }
                        button {
                            class: "btn btn-ghost cm-open",
                            onclick: move |_| on_lectures.call(lecture_id.clone()),
                            {tx::OPEN_LECTURES}
                        }
                    } else {
                        p { class: "muted small", {tx::NO_CONTENT} }
                    }
                    h3 { class: "small cm-status-title", {tx::STATUS} }
                    div { class: "segmented cm-status", role: "radiogroup", aria_label: tx::STATUS,
                        for st in CourseStatus::ALL {
                            button {
                                key: "{st.key()}",
                                role: "radio",
                                "aria-checked": s.status == st,
                                onclick: {
                                    let id = s.course.id.clone();
                                    move |_| set_status(&id, st)
                                },
                                {status_label(st)}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Course discovery must lead into the chosen course, with native controls.
#[cfg(test)]
mod ux_tests {
    use super::*;

    #[test]
    fn courses_open_their_lessons_instead_of_the_references_page() {
        let source = include_str!("map.rs");
        let production = source.split("/// Course discovery must lead").next().unwrap();
        assert!(production.contains("on_lectures: move |id: String| open_course_lessons(ui, &id)"));
        assert!(production.contains("super::focus::set(Some(id.to_string()))"));
        assert!(production.contains("crate::learn::reset_course_browse()"));
        assert!(production.contains("crate::lecture::close_all()"));
        assert!(!production.contains("open_references(ui)"));
    }

    #[test]
    fn an_expanded_course_exposes_real_readiness_and_its_detail_region() {
        fn app() -> Element {
            let lib = Library::get();
            let summary = summaries(lib, |c| c.status).into_iter().find(|s| s.course.id == "cvs").unwrap();
            rsx! {
                CourseTile {
                    summary,
                    open: true,
                    on_toggle: move |_: String| {},
                    on_lectures: move |_: String| {},
                }
            }
        }
        let lib = Library::get();
        let summary = summaries(lib, |c| c.status).into_iter().find(|s| s.course.id == "cvs").unwrap();
        let expected = tx::ready_lectures(summary.written, summary.lectures);
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for required in [
            "aria-expanded=true",
            "aria-controls=\"cm-detail-cvs\"",
            "id=\"cm-detail-cvs\"",
            tx::OPEN_LECTURES,
            expected.as_str(),
        ] {
            assert!(html.contains(required), "missing {required}: {html}");
        }
    }
}
