//! The map (More › المواد): First Aid's General Principles and Organ
//! Systems as tiles, in the book's order, with how much of each he has
//! started. A tile opens its course: First Aid pages, lectures, and the
//! status he can change.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use steps_core::program::{CourseStatus, Group};

use super::CSS;
use super::{Summary, percent, set_status, started, summaries};
use crate::content::Library;
use crate::i18n::courses as tx;

/// The book's two sections, in its order.
const GROUPS: [Group; 2] = [Group::General, Group::Systems];

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

fn group_hint(g: Group) -> &'static str {
    match g {
        Group::General => tx::G_GENERAL_HINT,
        Group::Systems => tx::G_SYSTEMS_HINT,
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
    let review = crate::store::use_review();
    let lib = Library::get();
    let all = summaries(lib, super::status);
    let seen: BTreeMap<String, usize> =
        started(&review.backend.borrow().overview(lib, crate::store::now_ms()).lectures);
    let pages: u32 = all.iter().map(|s| s.page_count).sum();
    let items: usize = all.iter().map(|s| s.items).sum();
    let begun: usize = seen.values().sum();
    let pct = percent(begun, items);
    let mut open = use_signal(|| None::<String>);
    // The ring: cards started out of the cards written so far.
    let c = 2.0 * std::f32::consts::PI * 42.0;
    let done_len = c * pct as f32 / 100.0;

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
                    if pct > 0 {
                        circle {
                            class: "cm-ring-done",
                            cx: "52",
                            cy: "52",
                            r: "42",
                            stroke_dasharray: "{done_len} {c}",
                            transform: "rotate(-90 52 52)",
                        }
                    }
                    text { class: "cm-ring-num", x: "52", y: "58", text_anchor: "middle", "{pct}%" }
                }
                div { class: "cm-hours-text",
                    p { class: "cm-big", {tx::pages_line(pages, all.len())} }
                    p { span { class: "cm-key cm-key-done" } {tx::started(begun, items)} }
                    p { class: "muted small",
                        {tx::ready_lectures(all.iter().map(|s| s.written).sum(), all.iter().map(|s| s.lectures).sum())}
                    }
                }
            }
            for g in GROUPS {
                if all.iter().any(|s| s.course.group == g) {
                    section { key: "{group_key(g)}", class: "cm-group", aria_label: group_label(g),
                        h2 { bdi { {group_label(g)} } }
                        p { class: "muted small", {group_hint(g)} }
                        div { class: "cm-grid",
                            for s in all.iter().filter(|s| s.course.group == g).cloned() {
                                CourseTile {
                                    key: "{s.course.id}",
                                    open: open() == Some(s.course.id.clone()),
                                    seen: seen.get(&s.course.id).copied().unwrap_or(0),
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
    /// Cards of this course he has started.
    seen: usize,
    on_toggle: EventHandler<String>,
    on_lectures: EventHandler<String>,
) -> Element {
    let s = summary;
    let id = s.course.id.clone();
    let lecture_id = id.clone();
    let pct = percent(seen, s.items);
    rsx! {
        div { class: "cm-cell", "data-open": open,
            crate::ui::CardButton {
                class: "tap tile cm-tile",
                "data-group": group_key(s.course.group),
                "data-status": s.status.key(),
                "aria-expanded": open,
                "aria-controls": "cm-detail-{s.course.id}",
                onclick: move |_| on_toggle.call(id.clone()),
                span { class: "cm-bar", aria_hidden: "true" }
                span { class: "cm-name", bdi { "{s.course.title}" } }
                if s.status != CourseStatus::Active {
                    span { class: "chip cm-status-chip", {status_label(s.status)} }
                }
                if s.items > 0 {
                    span { class: "cm-progress", aria_hidden: "true",
                        i { style: "inline-size: {pct}%" }
                    }
                    span { class: "cm-coverage muted small", {tx::started(seen, s.items)} }
                } else if s.lectures > 0 {
                    span { class: "cm-coverage muted small", {tx::ready_lectures(s.written, s.lectures)} }
                }
                if let Some([from, to]) = s.pages {
                    span { class: "cm-meta",
                        span { class: "muted small", {tx::tile_line(&s.course.short, from, to)} }
                    }
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
                        bdi { {group_label(s.course.group)} }
                        if let Some([from, to]) = s.pages {
                            " · "
                            {tx::book_pages(from, to, s.page_count)}
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
                    p { class: "muted small", {status_hint(s.status)} }
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
                    seen: 0,
                    on_toggle: move |_: String| {},
                    on_lectures: move |_: String| {},
                }
            }
        }
        let lib = Library::get();
        let summary = summaries(lib, |c| c.status).into_iter().find(|s| s.course.id == "cvs").unwrap();
        let expected = tx::content_line(summary.written, summary.lectures, summary.items, summary.lessons);
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for required in [
            "aria-expanded=true",
            "aria-controls=\"cm-detail-cvs\"",
            "id=\"cm-detail-cvs\"",
            tx::OPEN_LECTURES,
            expected.as_str(),
            "First Aid ص 284–328",
        ] {
            assert!(html.contains(required), "missing {required}: {html}");
        }
    }
}
