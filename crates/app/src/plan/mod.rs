//! The exam planner tab: Mohamed sets the exam day and his study days, and
//! `steps_core::plan` spreads what's left over the days before it.

use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::plan::{self, Day, DayPlan, Focus, Plan, PlanInput, PlanSettings, PlanStatus, TightFix};
use steps_core::review::StudyStore;
use steps_core::stats;

use crate::i18n::count::{REVIEW, count};
use crate::i18n::plan as tx;
use crate::stats::{StudyData, charts};

/// The planner settings: a synced (LWW) setting in the review store, so the
/// phone and the tablet share one plan. STEPS_PLAN_EXAM=2027-01-20 presets
/// the exam day instead (previews and screenshots; nothing is saved then).
pub(crate) static SETTINGS: GlobalSignal<PlanSettings> = Signal::global(|| {
    if let Some(exam) = std::env::var("STEPS_PLAN_EXAM").ok().and_then(|v| Day::parse(&v)) {
        return PlanSettings { exam: Some(exam), ..PlanSettings::default() };
    }
    crate::store::shared().setting(KEY).unwrap_or_default()
});

const KEY: &str = "plan";

/// Re-reads the plan after a sync or restore (see `crate::synced`).
pub(crate) fn reload() {
    if std::env::var_os("STEPS_PLAN_EXAM").is_some() {
        return;
    }
    let fresh: PlanSettings = crate::store::shared().setting(KEY).unwrap_or_default();
    if *SETTINGS.peek() != fresh {
        *SETTINGS.write() = fresh;
    }
}

/// Best effort: if the store can't write, the plan still holds for this run.
pub(crate) fn save(settings: &PlanSettings) {
    if std::env::var_os("STEPS_PLAN_EXAM").is_none() {
        crate::store::save_setting(KEY, settings);
    }
}

/// The plan from the start of today's study day: progress as it stood this
/// morning, plus the reviews FSRS has due from now on. Today's share stays
/// put while he works through it.
pub fn plan_for(data: &StudyData, settings: &PlanSettings) -> Plan {
    let today = data.clock.today();
    let progress = stats::progress_at_day_start(&data.lectures, &data.events, &data.marks, data.clock);
    let span = settings.exam.map_or(0, |e| today.until(e).max(0) as usize);
    let forecast = data.due_forecast(span);
    plan::plan(settings, PlanInput { today, lectures: &progress, review_forecast: &forecast })
}

/// Today's share of the exam plan: the one daily target that Today, the
/// study session and Settings › Review all read (#99).
#[derive(Clone, Debug, PartialEq)]
pub struct DailyTarget {
    pub plan: Plan,
    pub exam: Day,
    /// Calendar days until the exam.
    pub days_left: i32,
    /// What today has done so far.
    pub done: stats::DayDone,
}

impl DailyTarget {
    /// Today's line of the plan (`None` once the exam day has come).
    pub fn today(&self) -> Option<&DayPlan> {
        self.plan.today()
    }

    /// New items the plan gives today.
    pub fn new_goal(&self) -> u32 {
        self.today().map_or(0, |d| d.new)
    }

    /// New items still to start today.
    pub fn new_left(&self) -> u32 {
        self.new_goal().saturating_sub(self.done.new)
    }
}

/// The plan's target for today, or `None` without an exam day, before any
/// study day is ticked, or once the exam has passed: the review settings'
/// daily cap rules then.
pub fn daily_target(data: &StudyData, settings: &PlanSettings) -> Option<DailyTarget> {
    let exam = settings.exam?;
    let p = plan_for(data, settings);
    if matches!(p.status, PlanStatus::NoExamDate | PlanStatus::NoStudyDays | PlanStatus::ExamDone) {
        return None;
    }
    let done = stats::done_today(&data.lectures, &data.events, &data.marks, data.clock);
    Some(DailyTarget { days_left: data.clock.today().until(exam), plan: p, exam, done })
}

/// The target from the store and the saved plan. Reading `SETTINGS` here
/// subscribes a component to plan edits.
pub fn target_now(store: &dyn StudyStore) -> Option<DailyTarget> {
    target_for(store, SETTINGS.read().clone())
}

/// Today's target is read on every render of Today, Settings and the study
/// setup, and building it replays the whole review log, so it is cached
/// until the store changes, the study day rolls over or the plan is edited
/// (#249).
#[derive(Clone, PartialEq)]
struct TargetKey {
    revision: u64,
    today: Day,
    settings: PlanSettings,
}

thread_local! {
    static TARGET: std::cell::RefCell<Option<(TargetKey, Option<DailyTarget>)>> =
        const { std::cell::RefCell::new(None) };
}

fn target_for(store: &dyn StudyStore, settings: PlanSettings) -> Option<DailyTarget> {
    settings.exam?;
    // No review context (tests, previews): no cache.
    let Some(review) = try_consume_context::<crate::store::Review>() else {
        return daily_target(&StudyData::load(store), &settings);
    };
    let clock = stats::Clock {
        now_ms: crate::store::now_ms(),
        zone: crate::store::zone(),
        rollover_hour: steps_core::srs::ROLLOVER_HOUR as u32,
    };
    let key = TargetKey { revision: *review.revision.peek(), today: clock.today(), settings };
    if let Some(hit) = TARGET.with_borrow(|c| c.as_ref().filter(|(k, _)| *k == key).map(|(_, t)| t.clone())) {
        return hit;
    }
    let target = daily_target(&StudyData::load(store), &key.settings);
    TARGET.set(Some((key, target.clone())));
    target
}

/// New items a session may still start today: the exam plan's when there
/// is one, else the review settings' daily cap.
pub fn new_left_today(store: &dyn StudyStore, now_ms: i64) -> usize {
    match target_for(store, SETTINGS.peek().clone()) {
        Some(t) => t.new_left() as usize,
        None => store.new_left_today(now_ms),
    }
}

#[component]
pub fn Planner(data: Rc<StudyData>) -> Element {
    let mut editing = use_signal(|| false);
    let settings = SETTINGS.read().clone();
    if settings.exam.is_none() || *editing.read() {
        return rsx! {
            Setup { today: data.clock.today(), settings, on_done: move |_| editing.set(false) }
        };
    }
    let p = plan_for(&data, &settings);
    // The plan starts from this morning; the progress bars show right now.
    let live = stats::progress(&data.stats());
    let mut now = p.clone();
    now.seen = live.iter().map(|l| l.seen.min(l.total)).sum();
    now.seen_twice = live.iter().map(|l| l.seen_twice.min(l.total)).sum();
    rsx! {
        StatusCard { p: p.clone(), exam: settings.exam }
        TodayCard { p: p.clone() }
        LoadCard { p: p.clone(), cap: settings.review_cap }
        PaceCard { p: p.clone(), past: past_pace(&data) }
        ProgressCard { p: now }
        WeekCard { p: p.clone() }
        div { class: "plan-actions",
            button { class: "btn btn-secondary", onclick: move |_| editing.set(true), {tx::EDIT} }
        }
    }
}

#[component]
fn Setup(today: Day, settings: PlanSettings, on_done: EventHandler<()>) -> Element {
    let mut date = use_signal(|| settings.exam.map(Day::iso).unwrap_or_default());
    let mut weekdays = use_signal(|| settings.weekdays);
    let mut cap = use_signal(|| settings.review_cap);
    let mut error = use_signal(|| None::<&'static str>);
    let had_exam = settings.exam.is_some();
    let min = today.plus(1).iso();
    let max = today.plus(plan::MAX_DAYS_AHEAD).iso();
    let save = move |_| {
        let Some(exam) = Day::parse(&date.read()).filter(|d| *d > today) else {
            error.set(Some(tx::PICK_DAY));
            return;
        };
        if !plan::exam_day_ok(today, exam) {
            error.set(Some(tx::PICK_DAY_FAR));
            return;
        }
        if !weekdays.read().contains(&true) {
            error.set(Some(tx::PICK_WEEKDAY));
            return;
        }
        let mut s = SETTINGS.write();
        s.exam = Some(exam);
        s.weekdays = *weekdays.read();
        s.review_cap = (*cap.read()).clamp(30, plan::MAX_REVIEW_CAP);
        save(&s);
        on_done.call(());
    };
    rsx! {
        crate::ui::Card { class: "plan-setup",
            h2 { {tx::SETUP_TITLE} }
            label { class: "field",
                span { {tx::EXAM_DAY} }
                input {
                    r#type: "date",
                    min: "{min}",
                    max: "{max}",
                    value: "{date}",
                    oninput: move |e| {
                        date.set(e.value());
                        error.set(None);
                    },
                }
            }
            fieldset { class: "field weekdays",
                legend { {tx::DAYS_LABEL} }
                div { class: "weekday-chips",
                    for (w, name) in tx::WEEKDAYS.iter().enumerate() {
                        button {
                            key: "{w}",
                            class: "weekday",
                            "aria-pressed": if weekdays.read()[w] { "true" } else { "false" },
                            onclick: move |_| {
                                weekdays.write()[w] ^= true;
                                error.set(None);
                            },
                            {*name}
                        }
                    }
                }
            }
            label { class: "field",
                span { {tx::CAP_LABEL} }
                input {
                    r#type: "number",
                    inputmode: "numeric",
                    min: "30",
                    max: "1000",
                    step: "10",
                    value: "{cap}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse() {
                            cap.set(n);
                        }
                    },
                }
            }
            if let Some(msg) = *error.read() {
                p { class: "error", role: "alert", {msg} }
            }
            div { class: "plan-actions",
                button { class: "btn btn-primary", onclick: save, {tx::SAVE} }
                if had_exam {
                    button { class: "btn btn-secondary", onclick: move |_| on_done.call(()), {tx::CANCEL} }
                }
            }
        }
    }
}

#[component]
fn StatusCard(p: Plan, exam: Option<Day>) -> Element {
    let message = match p.status {
        PlanStatus::OnTrack => tx::ON_TRACK.to_string(),
        PlanStatus::Snug => tx::SNUG.to_string(),
        PlanStatus::Tight { left_over, fix } => {
            let fix = match fix {
                TightFix::MoreDays(n) => format!("{} {n}.", tx::TIGHT_ADD),
                TightFix::HigherCap(cap) => tx::tight_cap(cap),
                TightFix::HighYield => tx::TIGHT_HIGH_YIELD.to_string(),
                TightFix::FinalWindow => tx::TIGHT_FINAL.to_string(),
            };
            format!("{} {left_over}. {fix}", tx::TIGHT)
        }
        PlanStatus::FinalWindow => tx::FINAL_WINDOW.to_string(),
        PlanStatus::ExamDone => tx::EXAM_DONE.to_string(),
        PlanStatus::NoExamDate | PlanStatus::NoStudyDays => String::new(),
    };
    let tight = matches!(p.status, PlanStatus::Tight { .. });
    rsx! {
        crate::ui::Card { class: "plan-status",
            if let Some(exam) = exam {
                h2 { "{tx::EXAM_ON}: {long_date(exam)}" }
            }
            if p.status != PlanStatus::ExamDone {
                p { class: "muted", {tx::days_left(p.study_days_left as usize)} }
            }
            if let Some(from) = p.final_from.filter(|_| p.status != PlanStatus::ExamDone) {
                p { class: "muted small", "{tx::FINAL_FROM} {long_date(from)}" }
            }
            p { class: if tight { "note" } else { "" }, {message} }
        }
    }
}

/// Starts a session on these lectures (all when empty) with the usual budget.
fn start_on(mut ui: crate::Ui, lectures: &[String]) {
    *crate::study::START.write() = Some(crate::study::Start {
        lectures: (!lectures.is_empty()).then(|| lectures.to_vec()),
        items: None,
        minutes: crate::onboarding::session_minutes(),
    });
    ui.screen.set(crate::Screen::Study);
}

#[component]
fn TodayCard(p: Plan) -> Element {
    let ui = use_context::<crate::Ui>();
    let Some(today) = p.today() else { return rsx! {} };
    if today.focus == Focus::Rest {
        let next = p.next_study_day();
        return rsx! {
            crate::ui::Card { class: "plan-today",
                h2 { {tx::TODAY} }
                p { {tx::REST_TODAY} }
                if let Some(next) = next {
                    DayLine { d: next.clone() }
                }
            }
        };
    }
    rsx! {
        crate::ui::Card { class: "plan-today",
            h2 { "{tx::TODAY}: {focus_label(today.focus)}" }
            div { class: "summary",
                // Both start today's session: on today's lectures, or on everything due.
                if today.new > 0 {
                    button {
                        class: "big tap",
                        title: tx::START_HINT,
                        onclick: { let l = today.new_lectures.clone(); move |_| start_on(ui, &l) },
                        span { class: "big-n", "{today.new}" }
                        span { class: "big-label", {tx::NEW_ITEMS} }
                    }
                }
                button {
                    class: "big tap",
                    title: tx::START_HINT,
                    onclick: move |_| start_on(ui, &[]),
                    span { class: "big-n", "{today.reviews}" }
                    span { class: "big-label", {tx::REVIEWS} }
                }
            }
            DayDetail { d: today.clone() }
        }
    }
}

/// Items seen by the end of each past day, as (offset from today, count),
/// over the last [`PACE_PAST_DAYS`] days. Empty with no history.
fn past_pace(data: &StudyData) -> Vec<(i32, u32)> {
    let today = data.clock.today();
    let by_day = stats::first_seen_by_day(&data.lectures, &data.events, &data.marks, data.clock);
    let Some(first) = by_day.keys().next().copied() else { return Vec::new() };
    let from = first.max(today.plus(-PACE_PAST_DAYS));
    let mut seen: u32 = by_day.range(..from).map(|(_, n)| n).sum();
    (today.until(from)..=0)
        .map(|off| {
            seen += by_day.get(&today.plus(off)).copied().unwrap_or(0);
            (off, seen)
        })
        .collect()
}

const PACE_PAST_DAYS: i32 = 30;
const LOAD_DAYS: usize = 14;

#[component]
fn LoadCard(p: Plan, cap: u32) -> Element {
    let days: Vec<(Day, charts::Load)> = p
        .days
        .iter()
        .take(LOAD_DAYS)
        .map(|d| (d.day, charts::Load { reviews: d.reviews, new: d.new, rest: d.focus == Focus::Rest }))
        .collect();
    if days.is_empty() {
        return rsx! {};
    }
    let loads: Vec<charts::Load> = days.iter().map(|(_, l)| *l).collect();
    let svg = charts::forecast(&loads, cap, tx::LOAD_LABEL, |i, l| {
        let day = short_date(days[i].0);
        if l.rest {
            format!("{day}: {}", tx::FOCUS_REST)
        } else {
            format!("{day}: {} {} · {} {}", count(l.reviews, &REVIEW), tx::APPROX, l.new, tx::NEW_ITEMS)
        }
    });
    let peak = days.iter().max_by_key(|(_, l)| l.reviews + l.new).map(|(d, l)| (*d, l.reviews + l.new));
    rsx! {
        crate::ui::Card { class: "chart-card",
            h2 { {tx::LOAD_TITLE} }
            div { class: "chart-box", dangerous_inner_html: "{svg}" }
            div { class: "chart-key small muted",
                span { span { class: "key key-bar" } {tx::LOAD_KEY_REVIEWS} }
                span { span { class: "key key-new" } {tx::LOAD_KEY_NEW} }
                span { span { class: "key key-cap" } "{tx::LOAD_KEY_CAP} ({cap})" }
            }
            if let Some((d, n)) = peak.filter(|(_, n)| *n > 0) {
                p { class: "muted small", "{tx::LOAD_PEAK}: {short_date(d)} ({n})" }
            }
        }
    }
}

#[component]
fn PaceCard(p: Plan, past: Vec<(i32, u32)>) -> Element {
    if p.days.is_empty() || p.total == 0 {
        return rsx! {};
    }
    let today = p.days[0].day;
    let mut planned = vec![(0, p.seen)];
    let mut seen = p.seen;
    let mut done_on = (p.seen >= p.total).then_some(today);
    for (i, d) in p.days.iter().enumerate() {
        seen += d.new;
        if done_on.is_none() && seen >= p.total {
            done_on = Some(d.day);
        }
        planned.push((i as i32 + 1, seen));
    }
    let to = p.days.len() as i32;
    let svg = charts::pace(
        &charts::Pace {
            past: &past,
            planned: &planned,
            total: p.total,
            from: past.first().map_or(0, |(o, _)| *o),
            to,
            final_from: p.final_from.map(|f| today.until(f)),
        },
        tx::PACE_LABEL,
    );
    rsx! {
        crate::ui::Card { class: "chart-card",
            h2 { {tx::PACE_TITLE} }
            div { class: "chart-box", dangerous_inner_html: "{svg}" }
            div { class: "chart-key small muted",
                span { span { class: "key key-past" } {tx::PACE_KEY_PAST} }
                span { span { class: "key key-plan" } {tx::PACE_KEY_PLAN} }
                span { span { class: "key key-final" } {tx::PACE_KEY_FINAL} }
            }
            p { class: "small",
                match done_on {
                    Some(d) => format!("{} {}", tx::PACE_DONE_ON, long_date(d)),
                    None => tx::PACE_NOT_DONE.to_string(),
                }
            }
        }
    }
}

#[component]
fn ProgressCard(p: Plan) -> Element {
    let ui = use_context::<crate::Ui>();
    let share = |n: u32| if p.total == 0 { 0.0 } else { n as f32 * 100.0 / p.total as f32 };
    rsx! {
        // The per-lecture numbers behind these bars are on the progress tab.
        crate::ui::CardButton {
            class: "tap",
            title: tx::PROGRESS_HINT,
            onclick: move |_| crate::stats::open(ui, crate::stats::Tab::Progress),
            h2 { {tx::PROGRESS} span { class: "chev", aria_hidden: "true" } }
            for (label, n) in [(tx::FIRST_PASS, p.seen), (tx::SECOND_PASS, p.seen_twice)] {
                div { class: "bar", key: "{label}",
                    span { class: "bar-label small", {label} }
                    span { class: "bar-track", span { class: "bar-fill", style: "inline-size: {share(n):.0}%" } }
                    span { class: "bar-value small", crate::parts::Count { done: n as usize, total: p.total as usize } }
                }
            }
        }
    }
}

#[component]
fn WeekCard(p: Plan) -> Element {
    let week: Vec<DayPlan> = p.days.iter().skip(1).take(7).cloned().collect();
    let rest: Vec<DayPlan> = p.days.iter().skip(8).cloned().collect();
    rsx! {
        crate::ui::Card {
            h2 { {tx::WEEK} }
            ol { class: "plan-days",
                for d in week {
                    li { key: "{d.day.0}", DayLine { d } }
                }
            }
            if !rest.is_empty() {
                details { class: "plan-all",
                    summary { {tx::ALL_DAYS} }
                    ol { class: "plan-days",
                        for d in rest {
                            li { key: "{d.day.0}", DayLine { d } }
                        }
                    }
                }
            }
        }
    }
}

/// A day of the plan. A day with lectures opens the first one's page; a
/// rest day refers to nothing, so it stays a plain line.
#[component]
fn DayLine(d: DayPlan) -> Element {
    let ui = use_context::<crate::Ui>();
    let first = d.new_lectures.first().or(d.revisit.first()).cloned();
    let body = rsx! {
        span { class: "plan-date", {short_date(d.day)} }
        span { class: "plan-focus", {focus_label(d.focus)} }
        if d.focus != Focus::Rest {
            DayDetail { d: d.clone() }
        }
    };
    match first {
        Some(id) => rsx! {
            button {
                class: "plan-day tap tap-plain",
                "data-focus": "{d.focus:?}",
                title: tx::DAY_HINT,
                onclick: move |_| crate::lecture::open(ui, &id),
                {body}
            }
        },
        None => rsx! {
            div { class: "plan-day", "data-focus": "{d.focus:?}", {body} }
        },
    }
}

#[component]
pub(crate) fn DayDetail(d: DayPlan) -> Element {
    rsx! {
        span { class: "plan-detail muted small",
            if d.new > 0 {
                "{d.new} {tx::NEW_ITEMS} {tx::FROM} "
                bdi { {lecture_list(&d.new_lectures)} }
                " · "
            }
            if !d.revisit.is_empty() {
                "{tx::REVISIT} "
                bdi { {lecture_list(&d.revisit)} }
                " · "
            }
            {count(d.reviews, &REVIEW)} " {tx::APPROX}"
        }
    }
}

/// `["hb.L01", "hb.L02"]` → `Liver Cirrhosis, Non-invasive assessment…`:
/// the lectures' real titles (their course shows where the plan is).
pub(crate) fn lecture_list(ids: &[String]) -> String {
    let lib = crate::content::Library::get();
    ids.iter().map(|id| lib.title(id)).collect::<Vec<_>>().join(", ")
}

pub(crate) fn focus_label(f: Focus) -> &'static str {
    match f {
        Focus::Learn => tx::FOCUS_LEARN,
        Focus::Revisit => tx::FOCUS_REVISIT,
        Focus::Mock => tx::FOCUS_MOCK,
        Focus::WeakTopics => tx::FOCUS_WEAK,
        Focus::Rest => tx::FOCUS_REST,
    }
}

/// `الأربع 20 يناير 2027`.
fn long_date(d: Day) -> String {
    let (y, m, day) = d.ymd();
    format!("{} {day} {} {y}", tx::WEEKDAYS[d.weekday()], tx::MONTHS[m as usize - 1])
}

/// `أربع 20 يناير`.
fn short_date(d: Day) -> String {
    let (_, m, day) = d.ymd();
    format!("{} {day} {}", tx::WEEKDAYS_SHORT[d.weekday()], tx::MONTHS[m as usize - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_dates() {
        let d = Day::from_ymd(2027, 1, 20).unwrap();
        assert_eq!(long_date(d), "الأربع 20 يناير 2027");
        assert_eq!(short_date(d), "أربع 20 يناير");
        let lib = crate::content::Library::get();
        assert_eq!(
            lecture_list(&["hb.L01".into(), "hb.L12".into()]),
            format!("{}, {}", lib.title("hb.L01"), lib.title("hb.L12"))
        );
    }

    #[test]
    fn todays_target_holds_still_while_he_studies() {
        use steps_core::review::{Grade, Origin, ReviewEvent};
        use steps_core::stats::Clock;

        let Some(lecture) =
            crate::content::Library::get().lectures.values().find(|l| l.items.len() >= 6).cloned()
        else {
            return; // no content embedded
        };
        const DAY_MS: i64 = 86_400_000;
        let now_ms = 20_400 * DAY_MS + 12 * 3_600_000;
        let clock = Clock { now_ms, zone: steps_core::zone::Zone::fixed(0), rollover_hour: 4 };
        let answer = |i: usize, days_ago: i64| ReviewEvent {
            item_id: lecture.items[i].id.clone(),
            grade: Grade::Good,
            origin: Origin::Review,
            at_ms: now_ms - days_ago * DAY_MS,
            took_ms: 5000,
        };
        let data = |events: Vec<ReviewEvent>| StudyData {
            lectures: vec![lecture.clone()],
            events,
            marks: Default::default(),
            forecast: Some(Vec::new()),
            clock,
            demo: false,
        };
        let settings = PlanSettings { exam: Some(clock.today().plus(60)), ..PlanSettings::default() };
        assert!(
            daily_target(&data(Vec::new()), &PlanSettings::default()).is_none(),
            "no exam day, no target"
        );

        let morning = daily_target(&data(vec![answer(0, 1)]), &settings).unwrap();
        let later =
            daily_target(&data(vec![answer(0, 1), answer(0, 0), answer(1, 0), answer(2, 0)]), &settings)
                .unwrap();
        assert_eq!(morning.days_left, 60);
        assert_eq!(later.new_goal(), morning.new_goal(), "today's share doesn't shrink as he works");
        assert_eq!(later.done, stats::DayDone { new: 2, reviews: 1 });
        assert_eq!(later.new_left(), morning.new_goal().saturating_sub(2));
    }
}
