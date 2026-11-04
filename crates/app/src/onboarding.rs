//! First run: a few calm questions that set up the planner, the session
//! length, the reminders and the colours. Every step can be skipped, and
//! Settings › Study opens it again.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use steps_core::plan::Day;
use steps_core::theme::Theme;

use crate::Ui;
use crate::content::Library;
use crate::i18n::onboard as tx;
use crate::reminders::{MAX_TIMES, RemindPrefs, hhmm, parse_hhmm};
use crate::store::{now_ms, shared};

const CSS: &str = r#"
.onboard-backdrop { position: fixed; inset: 0; z-index: 50; background: var(--bg); overflow-y: auto; }
.onboard { max-inline-size: 36rem; min-block-size: 100%; margin-inline: auto; padding: var(--sp-4); display: flex; flex-direction: column; gap: var(--sp-3); }
.onboard-top { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-2); min-block-size: var(--tap); }
.onboard-body { display: flex; flex-direction: column; gap: var(--sp-3); }
.onboard-body h1 { font-size: 1.55rem; font-weight: 600; line-height: 1.6; margin: 0; }
.onboard-body p { margin: 0; }
/* A quiet picture: the app's pulse in a soft panel with faint rings. */
.onboard-art { position: relative; overflow: hidden; display: grid; place-items: center; block-size: 10rem;
  border-radius: var(--radius-sheet, 22px); background: var(--accent-soft); color: var(--accent-ink); }
.onboard-art::before, .onboard-art::after { content: ""; position: absolute; border-radius: 50%; border: 1px solid var(--line);
  inline-size: 10rem; block-size: 10rem; inset-inline-end: -3rem; inset-block-start: -5.5rem; }
.onboard-art::after { inline-size: 16rem; block-size: 16rem; inset-inline-end: -5rem; inset-block-start: -8.5rem; }
.onboard-art svg { inline-size: 4.5rem; block-size: 4.5rem; stroke-width: 1.2; }
.onboard-art-done { block-size: 6rem; background: var(--good-soft); color: var(--good); }
.onboard-art-done svg { inline-size: 3rem; block-size: 3rem; }
.onboard-body .segmented { align-self: stretch; }
.onboard-body .segmented > button { flex: 1; }
.onboard-chips { display: flex; flex-wrap: wrap; gap: var(--sp-2); }
.onboard-chips button { min-block-size: 44px; padding-inline: var(--sp-3); border-radius: var(--radius-pill); border: 1px solid var(--line); background: var(--surface); }
.onboard-chips button[aria-pressed="true"] { border-color: var(--accent); background: var(--accent-soft); color: var(--accent); font-weight: 700; }
/* The action stays at the foot; the last line of a long step scrolls clear of it. */
.onboard-actions { display: flex; flex-direction: column; gap: var(--sp-1); margin-block-start: auto;
  position: sticky; inset-block-end: 0; padding-block: var(--sp-2) calc(var(--sp-2) + env(safe-area-inset-bottom, 0px)); background: var(--bg); }
.onboard-actions .btn { justify-content: center; gap: var(--sp-2); }
.onboard-lectures { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--sp-1); }
.onboard-lectures li { display: flex; align-items: center; gap: var(--sp-2); }
.onboard-lecture { flex: 1; display: flex; align-items: center; gap: var(--sp-2); min-block-size: 44px; padding-inline: var(--sp-3);
  text-align: start; border-radius: var(--radius); border: 1px solid var(--line); background: var(--surface); }
.onboard-lecture[aria-pressed="true"] { border-color: var(--accent); background: var(--accent-soft); }
.onboard-check { flex: none; inline-size: 1.1rem; block-size: 1.1rem; border-radius: 4px; border: 2px solid var(--line); }
.onboard-lecture[aria-pressed="true"] .onboard-check { background: var(--accent); border-color: var(--accent); }
.onboard-num { flex: none; font-variant-numeric: tabular-nums; color: var(--muted); }
.onboard-lectures li.onboard-course { margin-block-start: var(--sp-2); font-weight: 700; color: var(--accent); }
.onboard-upto { flex: none; min-block-size: 44px; padding-inline: var(--sp-2); border: 0; background: none; color: var(--accent); }
/* Six themes with long names: whole-word rows, two columns once there is room (#232). */
.onboard-themes { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 12rem), 1fr)); gap: var(--sp-2); }
.onboard-theme { display: flex; align-items: center; gap: var(--sp-2); min-block-size: 44px; padding-inline: var(--sp-3);
  text-align: start; border-radius: var(--radius); border: 1px solid var(--line); background: var(--surface); }
.onboard-theme[aria-checked="true"] { border-color: var(--accent); background: var(--accent-soft); color: var(--accent); font-weight: 700; }
.onboard-theme .onboard-swatch { flex: none; margin-inline-end: 0; }
.onboard-swatch { display: inline-block; vertical-align: middle; inline-size: 1.1rem; block-size: 1.1rem; margin-inline-end: var(--sp-1);
  border-radius: 50%; border: 1px solid var(--line); background: linear-gradient(135deg, var(--sw-a) 50%, var(--sw-b) 50%); }
"#;

/// Session lengths offered, in minutes (the study screen's choices).
pub const MINUTE_CHOICES: [u32; 4] = [5, 10, 15, 20];

/// His answers that no other settings own, synced as `onboard`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Answers {
    /// Minutes a day; the study screen's default session length.
    pub minutes: Option<u32>,
    /// Lectures he had already studied before Steps (`hb.L01`, ...).
    pub studied: Vec<String>,
}

impl Answers {
    const KEY: &str = "onboard";

    pub fn load() -> Self {
        shared().setting(Self::KEY).unwrap_or_default()
    }

    fn save(&self) {
        crate::store::save_setting(Self::KEY, self);
    }
}

/// Default study session length: his first-run answer, else 10 minutes.
pub fn session_minutes() -> u32 {
    let _ = MINUTES_REV();
    Answers::load().minutes.filter(|m| MINUTE_CHOICES.contains(m)).unwrap_or(10)
}

/// Bumped when Today's chips change the session length, so readers re-read it.
static MINUTES_REV: GlobalSignal<u64> = Signal::global(|| 0);

/// Today's session-length chips: saves the new default length.
pub fn set_session_minutes(m: u32) {
    let mut a = Answers::load();
    if a.minutes != Some(m) {
        a.minutes = Some(m);
        a.save();
        *MINUTES_REV.write() += 1;
    }
}

/// Shown on this device (device-only: each device asks once, or never
/// once he has skipped or finished it). STEPS_ONBOARD=1 forces it open;
/// a launch that previews another screen or Settings skips it.
pub static OPEN: GlobalSignal<bool> = Signal::global(|| {
    let env = |k: &str| std::env::var_os(k).is_some();
    if env("STEPS_ONBOARD") {
        return true;
    }
    let previewing = ["STEPS_OPEN_SCREEN", "STEPS_OPEN_SETTINGS", "STEPS_STATS_TAB", "STEPS_PLAN_EXAM"];
    !previewing.iter().any(|k| env(k)) && !shared().local_setting::<bool>(SEEN).unwrap_or(false)
});

const SEEN: &str = "onboarded";

fn close() {
    if let Err(e) = shared().put_local_setting(SEEN, &true) {
        crate::diag::report!("saving first-run flag: {e}");
    }
    *OPEN.write() = false;
}

/// Opens the questions again (Settings › Study).
pub fn reopen() {
    *OPEN.write() = true;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Welcome,
    /// Session length and, if he knows it, the exam day.
    Time,
    /// The end: start, or set any of the optional details.
    Ready,
    Days,
    Studied,
    Reminders,
    Theme,
}

/// The quick path; the rest open from [`Step::Ready`].
const STEPS: [Step; 3] = [Step::Welcome, Step::Time, Step::Ready];

/// The first-run sheet; renders nothing once seen.
#[component]
pub fn Onboarding() -> Element {
    if !OPEN() {
        return rsx! {};
    }
    rsx! { Questions {} }
}

#[component]
fn Questions() -> Element {
    let mut ui = use_context::<Ui>();
    // STEPS_ONBOARD=3 opens on that screen (previews and screenshots).
    let mut step = use_signal(|| {
        let n = std::env::var("STEPS_ONBOARD").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
        n.clamp(1, STEPS.len()) - 1
    });
    // An optional detail opened from the last screen.
    let mut detail = use_signal(|| None::<Step>);
    let plan = crate::plan::SETTINGS.peek().clone();
    let answers = Answers::load();
    let today = crate::store::zone().study_day(now_ms());
    let mut exam = use_signal(|| plan.exam.map(Day::iso).unwrap_or_default());
    let mut weekdays = use_signal(|| plan.weekdays);
    let mut minutes = use_signal(|| answers.minutes.unwrap_or(10));
    let mut studied = use_signal(|| answers.studied.clone());
    // Reminder times as they stand: none while reminders are off, so the
    // last screen never shows a time that isn't set.
    let remind_now = || {
        let p = crate::reminders::PREFS.peek();
        if p.on { p.times.clone() } else { Vec::new() }
    };
    let mut times = use_signal(remind_now);
    // What each step showed at first. A step he clicks through unchanged
    // saves nothing, so a new device that hasn't synced yet can't overwrite
    // the real settings with its defaults (#32).
    let initial = use_hook(|| (plan.clone(), answers.clone(), remind_now()));
    let mut error = use_signal(|| None::<&'static str>);

    let current = detail().unwrap_or(STEPS[step()]);
    // Saves one step's answer; false keeps him on it with a gentle note.
    let mut commit = move |s: Step| -> bool {
        match s {
            Step::Time => {
                // The exam day first: a wrong date keeps him here.
                let text = exam.read().clone();
                if !text.is_empty() && Some(text.as_str()) != initial.0.exam.map(Day::iso).as_deref() {
                    let Some(day) = Day::parse(&text).filter(|d| *d > today) else {
                        error.set(Some(tx::PICK_FUTURE));
                        return false;
                    };
                    if !steps_core::plan::exam_day_ok(today, day) {
                        error.set(Some(tx::PICK_FAR));
                        return false;
                    }
                    let mut s = crate::plan::SETTINGS.write();
                    s.set_exam(day, today);
                    crate::plan::save(&s);
                }
                if Some(minutes()) != initial.1.minutes && (initial.1.minutes.is_some() || minutes() != 10) {
                    set_session_minutes(minutes());
                }
            }
            Step::Days => {
                if !weekdays.read().contains(&true) {
                    error.set(Some(tx::PICK_A_DAY));
                    return false;
                }
                if *weekdays.read() != initial.0.weekdays {
                    let mut s = crate::plan::SETTINGS.write();
                    s.weekdays = *weekdays.read();
                    crate::plan::save(&s);
                }
            }
            Step::Studied => {
                if *studied.read() != initial.1.studied {
                    let mut a = Answers::load();
                    a.studied = studied.read().clone();
                    a.save();
                }
            }
            Step::Reminders => {
                let t = times.read().clone();
                if t == initial.2 {
                    return true;
                }
                let on = !t.is_empty();
                RemindPrefs { on, times: if on { t } else { RemindPrefs::default().times } }.save();
                if on && crate::reminders::HERE() {
                    crate::platform::reminders::request_permission();
                }
            }
            Step::Welcome | Step::Ready | Step::Theme => {}
        }
        error.set(None);
        true
    };

    let body = match current {
        Step::Welcome => rsx! {
            div { class: "onboard-art", aria_hidden: "true", {crate::icons::pulse()} }
            h1 { {tx::WELCOME_TITLE} }
            p { class: "muted", {tx::WELCOME_BODY} }
            p { class: "note", {tx::WELCOME_NOTE} }
        },
        Step::Time => rsx! {
            h1 { {tx::TIME_TITLE} }
            p { class: "muted", {tx::TIME_BODY} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::MINUTES_TITLE,
                for m in MINUTE_CHOICES {
                    button {
                        key: "{m}",
                        role: "radio",
                        "aria-checked": minutes() == m,
                        onclick: move |_| minutes.set(m),
                        {crate::i18n::count::count(m, &crate::i18n::count::MINUTE)}
                    }
                }
            }
            label { class: "field", "data-invalid": error().is_some(),
                span { {tx::EXAM_OPTIONAL} }
                input {
                    r#type: "date",
                    min: "{today.plus(1).iso()}",
                    max: "{today.plus(steps_core::plan::MAX_DAYS_AHEAD).iso()}",
                    value: "{exam}",
                    oninput: move |e| {
                        exam.set(e.value());
                        error.set(None);
                    },
                }
                span { class: "field-hint", {tx::EXAM_HINT} }
            }
        },
        Step::Ready => {
            let days = weekdays.read().iter().filter(|d| **d).count();
            let theme = theme_name(ui.prefs.read().theme);
            let reminders = if times.read().is_empty() {
                tx::REMIND_OFF.to_string()
            } else {
                times.read().iter().map(|t| hhmm(*t)).collect::<Vec<_>>().join(" · ")
            };
            rsx! {
                div { class: "onboard-art onboard-art-done", aria_hidden: "true", {crate::icons::check()} }
                h1 { {tx::READY_TITLE} }
                p { class: "muted", {tx::READY_BODY} }
                ul { class: "tap-list",
                    for (s , title , sub) in [
                        (Step::Days, tx::DAYS_ROW, tx::days_sub(days)),
                        (Step::Studied, tx::STUDIED_ROW, tx::studied_sub(studied.read().len())),
                        (Step::Reminders, tx::REMIND_ROW, reminders),
                        (Step::Theme, tx::THEME_ROW, theme.to_string()),
                    ]
                    {
                        li { key: "{title}",
                            button { class: "tap tap-row", onclick: move |_| detail.set(Some(s)),
                                span { class: "tap-row-main",
                                    strong { {title} }
                                    span { class: "muted small", bdi { {sub} } }
                                }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }
        }
        Step::Days => rsx! {
            h1 { {tx::DAYS_TITLE} }
            p { class: "muted", {tx::DAYS_BODY} }
            div { class: "onboard-chips", role: "group", aria_label: tx::DAYS_TITLE,
                for (w , name) in crate::i18n::plan::WEEKDAYS.iter().enumerate() {
                    button {
                        key: "{w}",
                        "aria-pressed": if weekdays.read()[w] { "true" } else { "false" },
                        onclick: move |_| {
                            weekdays.write()[w] ^= true;
                            error.set(None);
                        },
                        {*name}
                    }
                }
            }
        },
        Step::Studied => {
            let ids: Vec<String> = Library::get().index.lectures.iter().map(|l| l.id.clone()).collect();
            let all = ids.clone();
            rsx! {
                h1 { {tx::STUDIED_TITLE} }
                p { class: "muted", {tx::STUDIED_BODY} }
                div { class: "row",
                    button { class: "btn btn-secondary", onclick: move |_| studied.set(all.clone()), {tx::ALL} }
                    button { class: "btn btn-secondary", onclick: move |_| studied.set(Vec::new()), {tx::NONE} }
                }
                ul { class: "onboard-lectures", aria_label: tx::STUDIED_TITLE,
                    for group in Library::get().by_course(Library::get().index.lectures.iter().enumerate().collect(), |(_, l)| l.id.as_str()) {
                        li { key: "{group.id}", class: "onboard-course", "{group.short()}" }
                        for (n , l) in group.rows {
                            li { key: "{l.id}",
                                button {
                                    class: "onboard-lecture",
                                    "aria-pressed": if studied.read().contains(&l.id) { "true" } else { "false" },
                                    onclick: {
                                        let id = l.id.clone();
                                        move |_| {
                                            let mut s = studied.write();
                                            match s.iter().position(|x| *x == id) {
                                                Some(i) => {
                                                    s.remove(i);
                                                }
                                                None => s.push(id.clone()),
                                            }
                                        }
                                    },
                                    span { class: "onboard-check", aria_hidden: "true" }
                                    bdi { class: "onboard-num", "{crate::content::lecture_no(&l.id)}" }
                                    bdi { {l.title.clone()} }
                                }
                                button {
                                    class: "onboard-upto small",
                                    title: tx::UP_TO_HERE,
                                    onclick: {
                                        // Up to here in this course; other courses keep their picks.
                                        let course = steps_core::program::course_of(&l.id).to_string();
                                        let upto: Vec<String> = ids[..=n]
                                            .iter()
                                            .filter(|x| steps_core::program::course_of(x) == course)
                                            .cloned()
                                            .collect();
                                        move |_| {
                                            let mut s = studied.write();
                                            s.retain(|x| steps_core::program::course_of(x) != course);
                                            s.extend(upto.iter().cloned());
                                        }
                                    },
                                    {tx::UP_TO_HERE}
                                }
                            }
                        }
                    }
                }
            }
        }
        Step::Reminders => rsx! {
            h1 { {tx::REMIND_TITLE} }
            p { class: "muted", {tx::REMIND_BODY} }
            for (i , t) in times.read().iter().copied().enumerate() {
                div { key: "{i}-{t}", class: "row",
                    input {
                        r#type: "time",
                        value: hhmm(t),
                        aria_label: tx::REMIND_TITLE,
                        onchange: move |e: FormEvent| {
                            if let Some(v) = parse_hhmm(&e.value()) {
                                times.write()[i] = v;
                            }
                        },
                    }
                    button { class: "btn btn-secondary", onclick: move |_| { times.write().remove(i); }, {tx::REMOVE} }
                }
            }
            if times.read().len() < MAX_TIMES {
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        let next = times.read().last().map_or(19 * 60, |t| (t + 120).min(23 * 60));
                        times.write().push(next);
                    },
                    {tx::ADD_TIME}
                }
            }
            if times.read().is_empty() {
                p { class: "muted small", {tx::NO_REMINDERS} }
            }
        },
        Step::Theme => rsx! {
            h1 { {tx::THEME_TITLE} }
            p { class: "muted", {tx::THEME_BODY} }
            div { class: "onboard-themes", role: "radiogroup", aria_label: tx::THEME_TITLE,
                for t in Theme::ALL {
                    button {
                        key: "{t.as_str()}",
                        class: "onboard-theme",
                        role: "radio",
                        "aria-checked": ui.prefs.read().theme == t,
                        onclick: move |_| {
                            ui.prefs.write().theme = t;
                            ui.prefs.peek().save();
                        },
                        span { class: "onboard-swatch", aria_hidden: "true", style: swatch_style(t) }
                        {theme_name(t)}
                    }
                }
            }
        },
    };

    let is_detail = detail().is_some();
    let last = step() + 1 == STEPS.len();
    rsx! {
        style { {CSS} }
        div { class: "onboard-backdrop",
            div { class: "onboard", role: "dialog", aria_modal: "true", aria_label: tx::TITLE,
                div { class: "onboard-top",
                    span { class: "chip", {tx::progress(step() + 1, STEPS.len())} }
                    if !last {
                        button { class: "btn btn-ghost", "data-back": if step() == 0 && detail().is_none() { "true" } else { "false" }, onclick: move |_| close(), {tx::LATER} }
                    }
                }
                section { class: "onboard-body", {body} }
                if let Some(msg) = error() {
                    p { class: "field-error", role: "alert", {msg} }
                }
                div { class: "onboard-actions",
                    if is_detail {
                        // A detail returns to the last screen once saved.
                        button {
                            class: "btn btn-primary btn-lg btn-block",
                            onclick: move |_| {
                                if commit(current) {
                                    detail.set(None);
                                }
                            },
                            {tx::DETAIL_DONE}
                        }
                    } else if last {
                        button {
                            class: "btn btn-primary btn-lg btn-block",
                            onclick: move |_| {
                                close();
                                match crate::learn::next_lesson(&[]) {
                                    Some(l) => crate::learn::open(ui, &l.id),
                                    None => ui.screen.set(crate::Screen::Learn),
                                }
                            },
                            {tx::START_LESSON}
                            {crate::icons::arrow_go()}
                        }
                        button {
                            class: "btn btn-ghost btn-block",
                            onclick: move |_| {
                                close();
                                ui.screen.set(crate::Screen::Today);
                            },
                            {tx::START_TODAY}
                        }
                    } else {
                        button {
                            class: "btn btn-primary btn-lg btn-block",
                            onclick: move |_| {
                                if commit(current) {
                                    step += 1;
                                }
                            },
                            {tx::CONTINUE}
                            {crate::icons::arrow_go()}
                        }
                    }
                    if step() > 0 && !is_detail {
                        button { class: "btn btn-ghost btn-block", "data-back": "true", onclick: move |_| { error.set(None); step -= 1; }, {tx::BACK} }
                    } else if is_detail {
                        button { class: "btn btn-ghost btn-block", "data-back": "true", onclick: move |_| { error.set(None); detail.set(None); }, {tx::BACK} }
                    }
                }
            }
        }
    }
}

/// The page and accent colours (System themes: the day and night page), as
/// in Settings › Look.
fn swatch_style(t: Theme) -> String {
    let (a, b) = match t.palette() {
        Some(p) => (p.bg, p.accent),
        None => (t.day_and_night().0.bg, t.day_and_night().1.bg),
    };
    format!("--sw-a:{a};--sw-b:{b};")
}

fn theme_name(t: Theme) -> &'static str {
    use crate::i18n::settings as s;
    match t {
        Theme::System => s::THEME_SYSTEM,
        Theme::SystemBlack => s::THEME_SYSTEM_BLACK,
        Theme::Paper => s::THEME_PAPER,
        Theme::Night => s::THEME_NIGHT,
        Theme::Black => s::THEME_BLACK,
        Theme::NightShift => s::THEME_SHIFT,
    }
}

/// Settings › Study: go through the first-run questions again.
#[component]
pub fn ReopenCard() -> Element {
    rsx! {
        crate::ui::Card { class: "onboard-reopen",
            h2 { {tx::REOPEN_TITLE} }
            button { class: "btn btn-secondary", onclick: move |_| reopen(), {tx::REOPEN} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_tolerate_missing_fields() {
        let a: Answers = serde_json::from_str(r#"{"minutes":15}"#).unwrap();
        assert_eq!(a, Answers { minutes: Some(15), studied: Vec::new() });
    }
}
