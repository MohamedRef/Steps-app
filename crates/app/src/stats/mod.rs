//! The progress screen (nav "تقدّمي"): stats per lecture and topic, the
//! weakest topics, the mistakes log, and the exam planner's tab.
//! The numbers come from `steps_core::stats`; this module only draws them.

#[cfg(test)]
mod browser;
pub mod charts;
mod demo;
mod mistakes;
mod period;
mod scores;
mod uworld;
pub use uworld::reload as reload_uworld;

use std::collections::BTreeMap;

use dioxus::prelude::*;
use steps_core::content::{Lecture, Status};
use steps_core::review::{ReviewEvent, StudyStore};
use steps_core::stats::{self, Clock, Counts, LectureStats, Ratio, WeakTopic};

use crate::content::{Library, lecture_no};
use crate::i18n::stats as tx;
use crate::look::{Css, css};
use crate::plan::Planner;

/// The study day starts when the scheduler's does (4 am).
const ROLLOVER_HOUR: u32 = steps_core::srs::ROLLOVER_HOUR as u32;
/// How far ahead the scheduler's forecast is read: past any exam this year.
const FORECAST_DAYS: usize = 400;
const STATS_CSS: Css = css!("src/stats/stats.css");

/// What the stats read: the content, the review log, Mohamed's marks and the
/// scheduler's due forecast. Rebuilt after every answer or verdict.
#[derive(PartialEq)]
pub struct StudyData {
    pub lectures: Vec<Lecture>,
    pub events: Vec<ReviewEvent>,
    pub marks: BTreeMap<String, Status>,
    /// Reviews due per study day from FSRS, today first. `None` in demo mode,
    /// where the planner falls back to `stats::rough_forecast`.
    pub forecast: Option<Vec<u32>>,
    pub clock: Clock,
    /// Demo numbers (STEPS_STATS_DEMO=1), labelled on screen.
    pub demo: bool,
}

impl StudyData {
    /// Today's data from the review store, or demo numbers for previews.
    pub fn load(store: &dyn StudyStore) -> StudyData {
        let lib = Library::get();
        // Book order, from the index.
        let lectures: Vec<Lecture> =
            lib.index.lectures.iter().filter_map(|e| lib.lectures.get(&e.id).cloned()).collect();
        let now_ms = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();
        let clock = Clock { now_ms, zone: crate::store::zone(), rollover_hour: ROLLOVER_HOUR };
        if std::env::var("STEPS_STATS_DEMO").is_ok_and(|v| v == "1") {
            let events = demo::events(&lectures, now_ms);
            return StudyData { lectures, events, marks: demo::marks(), forecast: None, clock, demo: true };
        }
        StudyData {
            lectures,
            events: store.log(),
            marks: store.marks(),
            forecast: Some(store.forecast(FORECAST_DAYS, now_ms)),
            clock,
            demo: false,
        }
    }

    /// Reviews due per day for `days` days, today first.
    pub fn due_forecast(&self, days: usize) -> Vec<u32> {
        match &self.forecast {
            Some(f) => f.iter().copied().chain(std::iter::repeat(0)).take(days).collect(),
            None => stats::rough_forecast(&self.lectures, &self.events, &self.marks, self.clock, days),
        }
    }

    pub fn stats(&self) -> Vec<LectureStats> {
        stats::lecture_stats(&self.lectures, &self.events, &self.marks, self.clock)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Progress,
    Mistakes,
    Plan,
    Scores,
}

impl Tab {
    fn from_env() -> Option<Tab> {
        // STEPS_STATS_TAB=plan opens that tab (previews and screenshots).
        match std::env::var("STEPS_STATS_TAB").ok()?.as_str() {
            "mistakes" => Some(Tab::Mistakes),
            "plan" => Some(Tab::Plan),
            "scores" => Some(Tab::Scores),
            _ => None,
        }
    }
}

/// The tab the next visit opens on, set by [`open`].
static WANT: GlobalSignal<Option<Tab>> = Signal::global(|| None);

/// Opens the progress screen on a tab (Today's summary and week cards, a
/// count that the progress screen explains).
pub fn open(mut ui: crate::Ui, tab: Tab) {
    *WANT.write() = Some(tab);
    ui.screen.set(crate::Screen::Stats);
}

#[component]
pub fn Stats() -> Element {
    let mut tab = use_signal(|| WANT.write().take().or_else(Tab::from_env).unwrap_or(Tab::Progress));
    // Asked for a tab while already here (the planner's progress card).
    use_effect(move || {
        if let Some(t) = WANT() {
            tab.set(t);
            *WANT.write() = None;
        }
    });
    // Subscribed: the screen refreshes after every answer and verdict.
    let review = crate::store::use_review();
    let data = std::rc::Rc::new(StudyData::load(&*review.backend.borrow()));
    let current = *tab.read();
    let tabs = [
        (Tab::Progress, tx::TAB_PROGRESS),
        (Tab::Mistakes, tx::TAB_MISTAKES),
        (Tab::Plan, tx::TAB_PLAN),
        (Tab::Scores, crate::i18n::scores::TAB),
    ];
    rsx! {
        style { {STATS_CSS.text()} }
        header { class: "page-head",
            h1 { {tx::HEAD} }
        }
        div { class: "tabs", role: "tablist", aria_label: tx::TABS_LABEL,
            for (t, label) in tabs {
                button {
                    key: "{t:?}",
                    class: "tab",
                    role: "tab",
                    "aria-selected": if current == t { "true" } else { "false" },
                    onclick: move |_| tab.set(t),
                    {label}
                }
            }
        }
        if data.demo {
            p { class: "note", {tx::DEMO} }
        }
        match current {
            Tab::Progress => rsx! { Progress { data: data.clone() } },
            Tab::Mistakes => rsx! { mistakes::Mistakes { data: data.clone() } },
            Tab::Plan => rsx! { Planner { data: data.clone() } },
            Tab::Scores => rsx! { scores::Scores { data: data.clone() } },
        }
    }
}

#[component]
fn Progress(data: std::rc::Rc<StudyData>) -> Element {
    let rows = data.stats();
    let mut all = Counts::default();
    for l in &rows {
        all.total += l.counts.total;
        all.seen += l.counts.seen;
        all.seen_twice += l.counts.seen_twice;
        all.approved += l.counts.approved;
    }
    let weak = stats::weakest_topics(&rows, 3);
    let weak_ids = stats::topics_session(&data.lectures, &weak, &data.events, &data.marks);
    let mut ui = use_context::<crate::Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    rsx! {
        period::PeriodHead { data: data.clone() }
        // Tablet landscape and the Mac: the totals and the weakest topics side by side.
        div { class: "tab-cols", "data-align": "stretch",
            div { class: "tab-main",
                crate::ui::Card {
                    h2 { {tx::SUMMARY} }
                    div { class: "summary",
                        // Seen and seen twice grow by studying; approved by the
                        // checks, whose export is in Settings › Data.
                        Big { n: all.seen, of: all.total, label: tx::SEEN, hint: tx::STUDY_HINT, onclick: move |_| ui.screen.set(crate::Screen::Study) }
                        Big { n: all.seen_twice, of: all.total, label: tx::SEEN_TWICE, hint: tx::STUDY_HINT, onclick: move |_| ui.screen.set(crate::Screen::Study) }
                        Big {
                            n: all.approved,
                            of: all.total,
                            label: tx::APPROVED,
                            hint: tx::APPROVED_HINT,
                            onclick: move |_| settings.show(Some(crate::settings::Section::Data)),
                        }
                    }
                    if all.seen == 0 {
                        p { class: "muted", {tx::NOTHING_YET} }
                    }
                }
            }
            div { class: "tab-aside", WeakCard { weak, ids: weak_ids, data: data.clone() } }
        }
        section { class: "stat-lectures",
            for group in Library::get().by_course(rows.iter().collect(), |l| l.id.as_str()) {
                crate::parts::CourseHead { key: "{group.id}", short: group.short().to_string(), title: group.title().to_string() }
                for l in group.rows {
                    LectureCard { key: "{l.id}", l: l.clone(), data: data.clone() }
                }
            }
        }
        if rows.len() < Library::get().index.lectures.len() {
        }
    }
}

#[component]
fn Big(
    n: u32,
    of: u32,
    label: &'static str,
    hint: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: "big tap",
            aria_label: "{n} {tx::OF} {of} {label}. {hint}",
            title: hint,
            onclick: move |e| onclick.call(e),
            span { class: "big-n", "{n}" }
            span { class: "muted small", "{tx::OF} {of}" }
            span { class: "big-label", {label} }
        }
    }
}

#[component]
fn WeakCard(weak: Vec<WeakTopic>, ids: Vec<String>, data: std::rc::Rc<StudyData>) -> Element {
    rsx! {
        crate::ui::Card {
            h2 { {tx::NEXT_STEP} }
            p { class: "muted small", {tx::WEAK_TITLE} }
            if weak.is_empty() {
                p { class: "muted", {tx::WEAK_EMPTY} }
            } else {
                // Each topic starts a session on just that topic.
                ol { class: "weak-list tap-list",
                    for w in weak.iter() {
                        li { key: "{w.lecture}-{w.title}",
                            TopicButton { lecture: w.lecture.clone(), topic: w.topic.clone(), data: data.clone(),
                                span { class: "tap-row-main",
                                    span { class: "weak-title", bdi { dir: "ltr", "{w.title}" } }
                                    span { class: "muted small",
                                        "{Library::get().crumb(&w.lecture)} · {tx::COL_RETENTION} "
                                        bdi { {pct(w.retention)} }
                                        " · "
                                        bdi { "MCQ" }
                                        " "
                                        bdi { {pct(w.mcq)} }
                                    }
                                }
                            }
                        }
                    }
                }
                SessionButton { label: tx::WEAK_START, ids }
            }
        }
    }
}

/// A row (or, `inline`, a link in a table cell) that starts a session on one
/// topic of a lecture: its cards when it has any, else the bank's questions on it.
#[component]
pub fn TopicButton(
    lecture: String,
    topic: Option<String>,
    data: std::rc::Rc<StudyData>,
    #[props(default)] inline: bool,
    children: Element,
) -> Element {
    let mut ui = use_context::<crate::Ui>();
    let ids: Vec<String> = crate::lecture::topic_session(&data, &lecture, topic.clone())
        .into_iter()
        .filter(|i| crate::study::is_playable(i))
        .collect();
    rsx! {
        button {
            class: if inline { "tap tap-plain topic-link" } else { "tap tap-row" },
            title: tx::TOPIC_HINT,
            onclick: move |_| {
                if ids.is_empty() {
                    crate::bank::practise(ui, &lecture, topic.clone());
                } else {
                    *crate::study::START.write() =
                        Some(crate::study::Start { lectures: None, items: Some(ids.clone()), minutes: 10 });
                    ui.screen.set(crate::Screen::Study);
                }
            },
            {children}
            if !inline {
                span { class: "chev", aria_hidden: "true" }
            }
        }
    }
}

/// Starts a study session on exactly these items, in this order. Items the
/// player can't show yet are left out of the count and the session.
#[component]
pub fn SessionButton(label: &'static str, ids: Vec<String>) -> Element {
    let mut ui = use_context::<crate::Ui>();
    let lib = Library::get();
    let playable: Vec<String> = ids
        .into_iter()
        .filter(|id| {
            lib.lectures
                .values()
                .flat_map(|l| &l.items)
                .find(|i| &i.id == id)
                .is_some_and(|i| crate::study::PLAYABLE.contains(&i.body.kind()))
        })
        .collect();
    let count = playable.len();
    rsx! {
        div { class: "session-start",
            button {
                class: "btn btn-primary",
                disabled: playable.is_empty(),
                // Inside a card that opens on a tap (Today's hero), this button's own job wins.
                onclick: move |e: MouseEvent| {
                    e.stop_propagation();
                    *crate::study::START.write() =
                        Some(crate::study::Start { lectures: None, items: Some(playable.clone()), minutes: 10 });
                    ui.screen.set(crate::Screen::Study);
                },
                {label}
            }
            span { class: "muted small", {crate::i18n::count::count(count, &crate::i18n::count::QUESTION)} " {tx::SESSION_COUNT}" }
        }
    }
}

#[component]
fn LectureCard(l: LectureStats, data: std::rc::Rc<StudyData>) -> Element {
    let ui = use_context::<crate::Ui>();
    let c = &l.counts;
    let seen = Ratio { right: c.seen, total: c.total };
    let approved = Ratio { right: c.approved, total: c.total };
    rsx! {
        details { class: "stat-lecture",
            summary {
                span { class: "num", "{lecture_no(&l.id)}" }
                div { class: "lecture-main",
                    span { class: "lecture-title", bdi { dir: "ltr", {Library::get().title(&l.id)} } }
                    div { class: "bars",
                        Bar { label: tx::COL_SEEN, r: seen }
                        Bar { label: tx::COL_RETENTION, r: c.retention }
                        Bar { label: tx::COL_MCQ, r: c.mcq }
                        Bar { label: tx::COL_APPROVED, r: approved }
                    }
                }
            }
            div { class: "topic-table", role: "table", aria_label: tx::TOPICS,
                div { class: "topic-row head", role: "row",
                    span { role: "columnheader", {tx::TOPICS} }
                    span { role: "columnheader", {tx::COL_SEEN} }
                    span { role: "columnheader", {tx::COL_RETENTION} }
                    span { role: "columnheader", {tx::COL_MCQ} }
                }
                for t in l.topics.iter() {
                    div { class: "topic-row", role: "row", key: "{t.title}",
                        span { role: "cell",
                            TopicButton { lecture: l.id.clone(), topic: t.id.clone(), data: data.clone(), inline: true,
                                if t.id.is_some() { bdi { dir: "ltr", "{t.title}" } } else { {tx::UNTAGGED} }
                            }
                        }
                        span { role: "cell", bdi { "{t.counts.seen}/{t.counts.total}" } }
                        span { role: "cell", bdi { {pct(t.counts.retention)} } }
                        span { role: "cell", bdi { {pct(t.counts.mcq)} } }
                    }
                }
            }
            button {
                class: "btn btn-tonal stat-open",
                onclick: { let id = l.id.clone(); move |_| crate::lecture::open(ui, &id) },
                {tx::OPEN_LECTURE}
            }
        }
    }
}

/// One labelled bar: the share as a filled track plus the number.
#[component]
fn Bar(label: &'static str, r: Ratio) -> Element {
    let width = r.share().map_or(0.0, |s| s * 100.0);
    rsx! {
        div { class: "bar", "data-empty": if r.total == 0 { "yes" } else { "no" },
            span { class: "bar-label small", {label} }
            span { class: "bar-track", span { class: "bar-fill", style: "inline-size: {width:.0}%" } }
            bdi { class: "bar-value small", {pct(r)} }
        }
    }
}

pub fn pct(r: Ratio) -> String {
    r.percent().map_or_else(|| tx::NO_DATA.to_string(), |p| format!("{p}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_numbers() {
        assert_eq!(pct(Ratio { right: 1, total: 4 }), "25%");
        assert_eq!(pct(Ratio::default()), tx::NO_DATA);
    }
}
