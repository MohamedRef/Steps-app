//! Today: the screen the app opens on. With an exam day it leads with the
//! plan's target for the day (#99). Then what's due, what's new, one button
//! for a short session, the lectures in progress, and the library.

mod settings;

use dioxus::prelude::*;

pub use settings::StudySection;

use steps_core::plan::{Focus, PlanStatus, TightFix};
use steps_core::review::StudyStore;

use crate::content::Library;
use crate::i18n::count::{ITEM, count};
use crate::i18n::today as tx;
use crate::look::{Css, css};
use crate::plan::DailyTarget;
use crate::stats::StudyData;
use crate::store::{self, LectureProgress, Overview, use_review};
use crate::{Screen, Ui};

pub(crate) const CSS: Css = css!("src/today/today.css");

#[component]
pub fn Today() -> Element {
    // A failed preference save has its own notice with Retry above the
    // screen; the note below then only repeats it.
    let preferences_pending = crate::persistence::use_pending_saves();
    let write_error = store::write_error().filter(|e| {
        !(preferences_pending && (e.starts_with("setting ") || e.starts_with("review settings: ")))
    });
    let lib = Library::get();
    let review = use_review();
    let o = review.backend.borrow().overview(lib, store::now_ms());
    // With an exam day, the plan sets today's new items (#99).
    let target = crate::plan::target_now(&*review.backend.borrow());
    let new_today = target.as_ref().map_or(o.new_today, |t| (t.new_left() as usize).min(o.unseen));
    let mut ui = use_context::<Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    let ready = o.due + new_today > 0;
    let minutes = crate::onboarding::session_minutes();
    // Everything today, reviews and the new items' first looks.
    let all_minutes = o.due_minutes + (new_today as u32 * NEW_ITEM_SECS).div_ceil(60);

    rsx! {
        style { {CSS.text()} }
        // At expanded widths the plan stays beside the lecture library.
        // Compact and medium widths use the full Lessons screen for browsing.
        div { class: "tab-cols today-cols", "data-wide": "aside",
            div { class: "tab-main tab-sticky",
                header { class: "page-head td-greet",
                    div { class: "td-greet-text",
                        p { class: "page-eyebrow", {tx::EYEBROW} }
                        h1 { {if ready { tx::READY_Q } else { tx::DONE_Q }} }
                    }
                    span { class: "td-art", aria_hidden: "true", {crate::icons::pulse()} }
                }
                // The session length: the start button below uses it, and
                // it stays his default (the first-run answer).
                // The hint sits on the question only: on the whole row the
                // tooltip covered the lecture list's heading beside it.
                div { class: "td-time", role: "radiogroup", aria_label: tx::TIME_Q,
                    span { class: "td-time-q", title: tx::TIME_HINT, {tx::TIME_Q} }
                    for m in crate::onboarding::MINUTE_CHOICES {
                        button {
                            key: "{m}",
                            class: "td-chip",
                            role: "radio",
                            "aria-checked": minutes == m,
                            onclick: move |_| crate::onboarding::set_session_minutes(m),
                            bdi { {tx::minutes_short(m)} }
                        }
                    }
                }
                Hero { target: target.clone(), due: o.due, new_today, unseen: o.unseen, due_minutes: o.due_minutes, due_tomorrow: o.due_tomorrow, nothing: o.seen == 0 && o.unseen == 0 && o.locked == 0, minutes }
                // Each count opens what it counts.
                div { class: "stat-tiles",
                    Tile {
                        value: o.due,
                        label: tx::DUE,
                        hint: if o.due > 0 { tx::DUE_HINT } else { tx::FORECAST_HINT },
                        accent: o.due > 0,
                        onclick: {
                            let due = review.backend.borrow().session_due(store::now_ms());
                            move |_| if due.is_empty() { crate::stats::open(ui, crate::stats::Tab::Plan) } else { session(ui, due.clone()) }
                        },
                    }
                    Tile {
                        value: new_today,
                        label: tx::NEW,
                        hint: if new_today > 0 { tx::NEW_HINT } else { tx::FORECAST_HINT },
                        onclick: {
                            let fresh = unseen(&*review.backend.borrow(), &o, new_today);
                            move |_| if fresh.is_empty() { crate::stats::open(ui, crate::stats::Tab::Plan) } else { session(ui, fresh.clone()) }
                        },
                    }
                }
                // New items waiting for their First Aid page (A2): one quiet
                // line, so a 0 above never reads as "nothing written".
                if o.locked > 0 {
                    p { class: "muted small td-locked", {tx::locked(o.locked)} }
                }
                // The active plan already shows its estimated minutes; without
                // a plan this stays a quiet link, not another large metric tile.
                if target.is_none() && all_minutes > 0 {
                    button {
                        class: "btn-link td-estimate",
                        title: tx::MINUTES_HINT,
                        aria_label: "{tx::minutes_all_label(all_minutes)}. {tx::MINUTES_HINT}",
                        onclick: move |_| crate::stats::open(ui, crate::stats::Tab::Plan),
                        bdi { class: "num-ltr", "{all_minutes}" }
                        span { {tx::minutes_all(all_minutes)} }
                        span { class: "chev", aria_hidden: "true" }
                    }
                }
                if !o.due_cases.is_empty() {
                    DueCases { ids: o.due_cases.clone() }
                }
                if let Some(e) = store::open_error() {
                    p { class: "note", "{tx::STORE_FAILED} " bdi { dir: "ltr", "{e}" } }
                } else if let Some(e) = write_error {
                    p { class: "note", "{tx::WRITE_FAILED} " bdi { dir: "ltr", "{e}" } }
                }
                div { class: "section-head",
                    h2 { {tx::CONTINUE_HEAD} }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            crate::learn::forget();
                            ui.screen.set(Screen::Learn);
                        },
                        {tx::ALL_LESSONS}
                        span { class: "chev", aria_hidden: "true" }
                    }
                }
                crate::learn::TodayLesson { prefer: o.lectures.iter().map(|l| l.id.clone()).collect::<Vec<_>>() }
                if !o.lectures.is_empty() {
                    section { class: "in-progress",
                        div { class: "section-head", h2 { {tx::IN_PROGRESS} } }
                        ul { class: "tap-list progress-list",
                            for l in o.lectures.iter() {
                                Progress { key: "{l.id}", lecture: l.clone() }
                            }
                        }
                    }
                }
                crate::recap::TodayRecap { due_left: o.due, due_tomorrow: o.due_tomorrow }
                // Extra shortcuts remain available without competing with today's work.
                details { class: "td-extras",
                    summary { class: "section-head td-extras-toggle",
                        h2 { {tx::MORE_HEAD} }
                        span { class: "chev", aria_hidden: "true" }
                    }
                    ul { class: "tap-list",
                        li {
                            button { class: "tap tap-row", onclick: move |_| ui.screen.set(Screen::Bank),
                                span { class: "row-icon", aria_hidden: "true", {crate::icons::nav(Screen::Bank)} }
                                span { class: "tap-row-main",
                                    strong { {tx::QUICK_BANK} }
                                }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                        // Content checks are a separate thing from memory: their
                        // marks and export live in Settings › Data.
                        li {
                            button { class: "tap tap-row", title: tx::CHECKED_HINT, onclick: move |_| settings.show(Some(crate::settings::Section::Data)),
                                span { class: "row-icon", aria_hidden: "true", {crate::icons::check()} }
                                span { class: "tap-row-main",
                                    strong { {tx::CHECKED_ROW} }
                                    span { class: "muted small", {tx::checked_sub(o.checked)} }
                                }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }
            div { class: "tab-aside", LibraryList {} }
        }
    }
}

/// The one recommended start: what today holds in a sentence and the
/// button that starts it. With an exam plan, the plan's day leads.
#[component]
fn Hero(
    target: Option<DailyTarget>,
    due: usize,
    new_today: usize,
    unseen: usize,
    due_minutes: u32,
    due_tomorrow: u32,
    nothing: bool,
    minutes: u32,
) -> Element {
    let ui = use_context::<Ui>();
    let review = use_review();
    let ready = due + new_today > 0;
    // The card's one job: start the session, or with nothing due, the next lesson.
    let next = crate::learn::next_lesson(&[]).map(|l| l.id.clone());
    let cta_next = next.clone();
    rsx! {
        crate::ui::Card {
            class: "hero card-action td-hero",
            onclick: move |_| open_recommended(ui, ready, next.as_deref(), minutes),
            div { class: "hero-meta",
                span { {if target.is_some() { tx::HERO_KICKER_PLAN } else if due > 0 { tx::HERO_KICKER } else { tx::HERO_KICKER_NEW }} }
                match target.clone() {
                    Some(t) => rsx! {
                        button {
                            class: "td-pill tap",
                            title: tx::PLAN_HINT,
                            onclick: move |e: MouseEvent| {
                                e.stop_propagation();
                                crate::stats::open(ui, crate::stats::Tab::Plan)
                            },
                            {tx::exam_in(t.days_left)}
                        }
                    },
                    None => rsx! {},
                }
            }
            if let Some(t) = target.clone() {
                // The plan's day says what today holds; the start follows it.
                PlanHero { t, due, due_minutes }
            } else if ready {
                h2 { {tx::headline(due, new_today)} }
                p { class: "td-body", {tx::hero_body(due, new_today)} }
            }
            if !ready {
                if nothing {
                    p { class: "td-body", {tx::NOTHING_WRITTEN} }
                } else {
                    h2 { {tx::ALL_DONE} }
                    if unseen > 0 {
                        p { class: "td-body", "{tx::NEW_CAP_REACHED} · {tx::TOMORROW} {due_tomorrow}" }
                    } else {
                        p { class: "td-body", "{tx::TOMORROW} {due_tomorrow}" }
                    }
                    if target.is_some() && unseen > 0 {
                        crate::stats::SessionButton { label: tx::AHEAD, ids: unseen_ids(&review, unseen) }
                    }
                }
            }
            // A direct keyboard action; prevent also firing the full-card click.
            button {
                class: "btn btn-lg btn-on-hero td-go",
                r#type: "button",
                onclick: move |e: MouseEvent| {
                    e.stop_propagation();
                    open_recommended(ui, ready, cta_next.as_deref(), minutes);
                },
                if ready {
                    {tx::start_session(minutes)}
                } else {
                    {tx::OPEN_NEXT_LESSON}
                }
                {crate::icons::arrow_go()}
            }
        }
    }
}

/// The whole-card tap and visible button share one route and study budget.
fn open_recommended(mut ui: Ui, ready: bool, next: Option<&str>, minutes: u32) {
    if ready {
        *crate::study::START.write() = Some(crate::study::Start { lectures: None, items: None, minutes });
        ui.screen.set(Screen::Study);
    } else if let Some(id) = next {
        crate::learn::open(ui, id);
    } else {
        ui.screen.set(Screen::Learn);
    }
}

fn unseen_ids(review: &crate::store::Review, unseen: usize) -> Vec<String> {
    let b = review.backend.borrow();
    let o = b.overview(Library::get(), store::now_ms());
    self::unseen(&*b, &o, AHEAD_ITEMS.min(unseen))
}

/// Cases that fell due: they play on the Cases screen, not
/// in a session, so Today points there with how many are waiting (#242).
#[component]
fn DueCases(ids: Vec<String>) -> Element {
    let ui = use_context::<Ui>();
    rsx! {
        button {
            class: "btn-link due-cases",
            onclick: move |_| crate::case::open_due(ui),
            {tx::due_cases(ids.len())}
        }
    }
}

/// Rough time for a new item: a first look takes longer than a review.
const NEW_ITEM_SECS: u32 = 40;
/// Items in a "get ahead" session once today's plan is done.
const AHEAD_ITEMS: usize = 10;

/// The exam plan's day on Today: days to the exam, today's target with a
/// ring for how much of it is done, and a calm note if the plan is tight.
#[component]
fn PlanHero(t: DailyTarget, due: usize, due_minutes: u32) -> Element {
    let Some(day) = t.today().cloned() else { return rsx! {} };
    let goal = t.new_goal();
    let left = t.new_left();
    // Reviews: what he has answered today plus what is still due.
    let target_total = goal + t.done.reviews + due as u32;
    let done_total = t.done.new.min(goal) + t.done.reviews;
    let pct = (done_total * 100).checked_div(target_total).unwrap_or(100);
    let minutes = due_minutes + (left * NEW_ITEM_SECS).div_ceil(60);
    let tight = match t.plan.status {
        // Inside the final window the hero already says mocks and weak topics.
        PlanStatus::Tight { fix, .. } => match fix {
            TightFix::MoreDays(n) => Some(tx::tight_days(n)),
            TightFix::HigherCap(cap) => Some(tx::tight_cap(cap)),
            TightFix::HighYield => Some(tx::TIGHT_HIGH_YIELD.to_string()),
            TightFix::FinalWindow => None,
        },
        _ => None,
    };
    let mut ui = use_context::<Ui>();
    rsx! {
        div { class: "plan-line",
            button {
                class: "tap tap-plain ring-tap",
                aria_label: tx::PLAN_HINT,
                title: tx::PLAN_HINT,
                onclick: move |e: MouseEvent| {
                    e.stop_propagation();
                    crate::stats::open(ui, crate::stats::Tab::Plan)
                },
                div {
                    class: "ring",
                    role: "progressbar",
                    aria_label: tx::RING_LABEL,
                    aria_valuemin: "0",
                    aria_valuemax: "100",
                    aria_valuenow: "{pct}",
                    style: "--p: {pct}%",
                    span { class: "ring-hole", bdi { "{pct}%" } }
                }
            }
            div { class: "plan-text",
                match day.focus {
                    Focus::Learn | Focus::Revisit => rsx! {
                        p { class: "plan-target",
                            "{tx::PLAN_TODAY} "
                            if goal > 0 && due > 0 {
                                {tx::plan_line(goal, due)}
                            } else if goal > 0 {
                                "{goal} {tx::PLAN_NEW}"
                            } else {
                                {tx::plan_reviews_only(due)}
                            }
                            if minutes > 0 {
                                " ({tx::ABOUT} " {count(minutes, &crate::i18n::count::MINUTE)} ")"
                            }
                        }
                        if !day.revisit.is_empty() {
                            p { class: "muted small",
                                "{tx::REVISIT} "
                                bdi { {crate::plan::lecture_list(&day.revisit)} }
                            }
                        }
                        if goal > 0 && left == 0 && due > 0 {
                            p { class: "muted small", {tx::NEW_DONE} }
                        } else if t.done.new > 0 && left > 0 {
                            p { class: "muted small", "{tx::NEW_LEFT_1} {left} {tx::NEW_LEFT_2}" }
                        }
                    },
                    Focus::Mock => rsx! {
                        p { class: "plan-target", {tx::MOCK} }
                        button {
                            class: "btn-link",
                            onclick: move |e: MouseEvent| {
                                e.stop_propagation();
                                ui.screen.set(Screen::Bank)
                            },
                            {tx::OPEN_BANK}
                        }
                    },
                    Focus::WeakTopics => rsx! {
                        p { class: "plan-target", {tx::WEAK} }
                        WeakStart {}
                    },
                    Focus::Rest => rsx! {
                        p { class: "plan-target", {tx::REST} }
                        if due > 0 {
                            p { class: "muted small", {tx::REST_DUE} }
                        }
                    },
                }
            }
        }
        if let Some(note) = tight {
            p { class: "note small", {note} }
        }
    }
}

/// A session on the weakest topics, as on the progress screen.
#[component]
fn WeakStart() -> Element {
    let review = use_review();
    let data = StudyData::load(&*review.backend.borrow());
    let weak = steps_core::stats::weakest_topics(&data.stats(), 3);
    let ids = steps_core::stats::topics_session(&data.lectures, &weak, &data.events, &data.marks);
    rsx! {
        crate::stats::SessionButton { label: tx::WEAK_START, ids }
    }
}

/// The next `n` unseen items the player can show: lectures already started
/// first, then book order (today's new items, or "get ahead" once today's
/// plan is done).
fn unseen(store: &dyn StudyStore, o: &Overview, n: usize) -> Vec<String> {
    let lib = Library::get();
    let mut order: Vec<&str> = o.lectures.iter().map(|l| l.id.as_str()).collect();
    for e in &lib.index.lectures {
        if !order.contains(&e.id.as_str()) {
            order.push(&e.id);
        }
    }
    order
        .into_iter()
        .filter_map(|id| lib.lectures.get(id))
        .flat_map(|l| l.items.iter().map(move |i| (l, i)))
        .filter(|(_, i)| crate::study::PLAYABLE.contains(&i.body.kind()))
        .filter(|(_, i)| store.is_new(&i.id) && !store.is_hidden(&i.id))
        .filter(|(l, i)| steps_core::session::page_read(i, l, store.pages_read()))
        .map(|(_, i)| i)
        .take(n)
        .map(|i| i.id.clone())
        .collect()
}

/// A session on exactly these items, from a count on Today.
fn session(mut ui: Ui, items: Vec<String>) {
    let minutes = crate::onboarding::session_minutes();
    *crate::study::START.write() = Some(crate::study::Start { lectures: None, items: Some(items), minutes });
    ui.screen.set(Screen::Study);
}

/// A count that opens what it counts; `hint` says where it goes.
#[component]
fn Tile(
    value: usize,
    label: &'static str,
    hint: &'static str,
    #[props(default)] accent: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: "tap stat-tile",
            "data-accent": accent,
            aria_label: "{value} {label}. {hint}",
            title: hint,
            onclick: move |e| onclick.call(e),
            span { class: "stat-num", "{value}" }
            span { class: "stat-label", {label} }
        }
    }
}

#[component]
fn Progress(lecture: LectureProgress) -> Element {
    let ui = use_context::<Ui>();
    let pct = lecture.seen * 100 / lecture.total.max(1);
    let number = crate::content::lecture_no(&lecture.id);
    let course = Library::get().course_short(&lecture.id).to_string();
    rsx! {
        li {
            button {
                class: "tap tap-row progress-row",
                onclick: { let id = lecture.id.clone(); move |_| crate::lecture::open(ui, &id) },
                span { class: "num", "{number}" }
                span { class: "tap-row-main",
                    bdi { class: "lecture-title", dir: "ltr", "{lecture.title}" }
                    span {
                        class: "progress",
                        role: "progressbar",
                        aria_valuemin: "0",
                        aria_valuemax: "{lecture.total}",
                        aria_valuenow: "{lecture.seen}",
                        span { class: "progress-fill", style: "inline-size: {pct}%" }
                    }
                    span { class: "muted small", "{course} · {lecture.seen} {tx::OF} {lecture.total} {tx::STARTED}" }
                }
                if lecture.due > 0 {
                    span { class: "chip due", "{lecture.due} {tx::DUE_CHIP}" }
                }
                span { class: "chev", aria_hidden: "true" }
            }
        }
    }
}

/// Every lecture under its course, and how many items each has.
#[component]
fn LibraryList() -> Element {
    let ui = use_context::<Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    let lib = Library::get();
    let rows = lib.index.lectures.iter().map(|entry| {
        let items = lib.lectures.get(&entry.id).map(|l| l.items.len());
        let number = crate::content::lecture_no(&entry.id);
        let [first, last] = entry.pages;
        (entry.id.clone(), number, entry.title.clone(), items, format!("{first}–{last}"))
    });
    let groups = lib.by_course(rows.collect(), |r| r.0.as_str());
    rsx! {
        // A heading, not a card: the lectures under it are what open.
        div { class: "library-head",
            div { class: "section-head", h2 { {tx::LIBRARY} } }
            // The note says to mark items; the marks are sent from Settings › Data.
            button {
                class: "note tap library-note",
                title: tx::UNVERIFIED_HINT,
                onclick: move |_| settings.show(Some(crate::settings::Section::Data)),
                {tx::UNVERIFIED_NOTE}
            }
        }
        for group in groups {
            crate::parts::CourseHead { key: "{group.id}", short: group.short().to_string(), title: group.title().to_string() }
            ol { class: "tap-list lectures-list",
                for (id, number, title, items, pages) in group.rows {
                    li { key: "{id}",
                        button {
                            class: "tap tap-row lecture-row",
                            "data-ready": if items.is_some() { "yes" } else { "no" },
                            onclick: { let id = id.clone(); move |_| crate::lecture::open(ui, &id) },
                            span { class: "num", "{number}" }
                            span { class: "tap-row-main",
                                bdi { class: "lecture-title", dir: "ltr", "{title}" }
                                span { class: "muted small", if lib.program.slides(&id) { "{crate::i18n::lecture::SLIDES} " } else { "{tx::PAGES} " } bdi { dir: "ltr", "{pages}" } }
                            }
                            match items {
                                Some(n) => rsx! { span { class: "chip ready", "{tx::READY} · " {count(n, &ITEM)} } },
                                None => rsx! { span { class: "chip", {tx::WRITING} } },
                            }
                            span { class: "chev", aria_hidden: "true" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod hierarchy_tests {
    #[test]
    fn daily_study_and_continuation_come_before_secondary_shortcuts() {
        let source = include_str!("mod.rs");
        let today = source.split("/// The one recommended start").next().unwrap();
        let sections = [
            "Hero { target:",
            "div { class: \"stat-tiles\"",
            "crate::learn::TodayLesson {",
            "section { class: \"in-progress\"",
            "crate::recap::TodayRecap {",
            "details { class: \"td-extras\"",
        ];
        let positions: Vec<usize> = sections
            .iter()
            .map(|section| today.find(section).unwrap_or_else(|| panic!("Missing Today section: {section}")))
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(today.matches("Tile {").count(), 2);
        assert!(today.contains("if target.is_none() && all_minutes > 0"));
    }

    #[test]
    fn visible_hero_button_has_its_own_non_bubbling_click_handler() {
        let source = include_str!("mod.rs");
        let hero = source.split("fn Hero(").nth(1).unwrap().split("fn unseen_ids(").next().unwrap();
        assert!(hero.contains("onclick: move |_| open_recommended("));
        assert!(hero.contains("r#type: \"button\""));
        assert!(hero.contains("e.stop_propagation();\n                    open_recommended("));
    }

    #[test]
    fn full_library_remains_in_lessons_at_compact_and_medium_widths() {
        let css = include_str!("today.css");
        assert!(css.contains("@media (max-width: 839.98px)"));
        assert!(css.contains(".today-cols > .tab-aside { display: none; }"));
    }
}
