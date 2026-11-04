//! Oral mode: the structured oral exam, hands-free. The app reads a question
//! aloud, gives Mohamed time to answer out loud, reads the model answer, and
//! he rates himself Again or Good. Again comes back later in the session.
//! No recording and no speech recognition: nothing leaves the device.

mod session;

use std::time::Duration;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

use std::collections::BTreeSet;

use dioxus::prelude::*;
use steps_core::review::{Grade, Origin, ReviewEvent, StudyStoreHandle};

use crate::content::Library;
use crate::i18n::count::{QUESTION, SECOND, count};
use crate::i18n::oral as tx;
use crate::look::{Css, css};
use crate::platform::background::{self, Labels, Remote};
use crate::platform::tts;
use session::{Entry, Pick, Session};

const ORAL_CSS: Css = css!("src/oral/oral.css");

/// Think-timer choices, in seconds (the brief's 10–30 s range).
const THINK_MIN: u32 = 10;
const THINK_MAX: u32 = 30;
const SIZES: [usize; 3] = [10, 20, 30];
/// With auto-continue on, how long the answer waits for a rating.
const AUTO_WAIT: Duration = Duration::from_secs(8);
/// After a rating, "show answer" waits this long, so a double tap or a held
/// key cannot flash the next question's answer.
const SETTLE: Duration = Duration::from_millis(700);

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Settings {
    lecture: Option<String>,
    size: usize,
    think: u32,
    voice: bool,
    /// Index into [`tts::RATES`].
    rate: usize,
    auto_continue: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { lecture: None, size: 10, think: 15, voice: true, rate: 1, auto_continue: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// Reading the question.
    Asking,
    /// The think timer: seconds left.
    Thinking(u32),
    /// "And what else?", then a shorter timer.
    WhatElse(u32),
    /// Reading the answer, then waiting for a rating.
    Answer,
}

// Kept across screen changes, for this run of the app. Ratings go to the
// review store (FSRS), like the study player's.
static SETTINGS: GlobalSignal<Settings> = Signal::global(Settings::default);
/// The setup he used last, kept on this device.
const SETTINGS_KEY: &str = "oral.settings";
static RESTORED: GlobalSignal<bool> = Signal::global(|| false);

fn restore_settings() {
    if *RESTORED.peek() {
        return;
    }
    *RESTORED.write() = true;
    if let Some(mut s) = crate::store::shared().local_setting::<Settings>(SETTINGS_KEY) {
        s.size = if SIZES.contains(&s.size) { s.size } else { Settings::default().size };
        s.think = s.think.clamp(THINK_MIN, THINK_MAX);
        s.rate = s.rate.min(tts::RATES.len() - 1);
        if s.lecture.as_ref().is_some_and(|l| !Library::get().lectures.contains_key(l)) {
            s.lecture = None;
        }
        *SETTINGS.write() = s;
    }
}
static SESSION: GlobalSignal<Option<Session>> = Signal::global(|| None);

#[component]
pub fn Oral() -> Element {
    use_drop(tts::stop);
    let active = SESSION.read().as_ref().map(|s| s.current.is_some() || s.left() > 0);
    rsx! {
        style { {ORAL_CSS.text()} }
        match active {
            None => rsx! { Setup {} },
            Some(true) => rsx! { Runner {} },
            Some(false) => rsx! { Summary {} },
        }
    }
}

/// The review store the app provides (FSRS, Mohamed's ✓/✎/✗).
fn use_store() -> CopyValue<StudyStoreHandle> {
    use_hook(|| CopyValue::new(consume_context::<StudyStoreHandle>()))
}

/// Askable items, minus what Mohamed marked wrong, with his edits applied.
fn pool(store: &StudyStoreHandle, lecture: Option<&str>) -> Vec<session::Prompt> {
    let s = store.0.borrow();
    session::pool_with(Library::get().lectures.values(), lecture, |item| {
        (!s.is_hidden(&item.id)).then(|| s.effective(item))
    })
}

fn start(store: &StudyStoreHandle) {
    let settings = SETTINGS.peek().clone();
    let pool = pool(store, settings.lecture.as_deref());
    let due: BTreeSet<String> = store.0.borrow().due(now_ms()).into_iter().collect();
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    let mut s = Session::new(pool, &Pick { lecture: settings.lecture, size: settings.size, seed, due });
    s.advance();
    *SESSION.write() = Some(s);
}

#[component]
fn Setup() -> Element {
    let store = use_store();
    use_hook(restore_settings);
    use_effect(|| {
        let s = SETTINGS.read().clone();
        let _ = crate::store::shared().put_local_setting(SETTINGS_KEY, &s);
    });
    let settings = SETTINGS.read().clone();
    let lib = Library::get();
    let all = pool(&store.read(), None);
    let lectures: Vec<(String, String)> = lib
        .index
        .lectures
        .iter()
        .filter(|e| all.iter().any(|p| p.lecture == e.id))
        .map(|e| (e.id.clone(), e.title.clone()))
        .collect();
    let available = all.iter().filter(|p| settings.lecture.as_ref().is_none_or(|l| *l == p.lecture)).count();

    rsx! {
        // Tablet landscape and the Mac: what oral mode is and Start on one
        // side, its settings on the other.
        div { class: "tab-cols oral-cols",
            div { class: "tab-main tab-sticky",
                // With questions to ask, the whole card starts (the button's click bubbles up).
                crate::ui::Card {
                    class: if available > 0 { "card-action oral-setup" } else { "oral-setup" },
                    onclick: move |_| if available > 0 { start(&store.read()) },
                    h1 { {tx::TITLE} }
                    p { class: "note small", {tx::HONEST} }
                    if available > 0 {
                        p { class: "muted small", "{tx::AVAILABLE} " {count(available, &QUESTION)} }
                        button {
                            class: "oral-start",
                            "aria-keyshortcuts": "Enter",
                            {tx::START}
                        }
                    }
                }
            }
            div { class: "tab-aside",
                if lectures.is_empty() {
                    p { class: "note", {tx::EMPTY} }
                } else {
                    crate::ui::Card { class: "oral-setup",
                        label { class: "oral-field",
                            span { {tx::LECTURE} }
                            select {
                                onchange: move |e| {
                                    let v = e.value();
                                    SETTINGS.write().lecture = (!v.is_empty()).then_some(v);
                                },
                                option { value: "", selected: settings.lecture.is_none(), {tx::ALL_LECTURES} }
                                for (id, title) in lectures {
                                    option {
                                        key: "{id}",
                                        value: "{id}",
                                        selected: settings.lecture.as_deref() == Some(id.as_str()),
                                        "{title}"
                                    }
                                }
                            }
                        }
                        div { class: "oral-field",
                            span { {tx::SIZE} }
                            div { class: "segmented", role: "radiogroup",
                                for size in SIZES {
                                    button {
                                        key: "{size}",
                                        role: "radio",
                                        "aria-checked": "{settings.size == size}",
                                        onclick: move |_| SETTINGS.write().size = size,
                                        "{size}"
                                    }
                                }
                            }
                        }
                        label { class: "oral-field",
                            span { "{tx::THINK}: " {count(settings.think, &SECOND)} }
                            input {
                                r#type: "range",
                                min: "{THINK_MIN}",
                                max: "{THINK_MAX}",
                                step: "5",
                                value: "{settings.think}",
                                oninput: move |e| {
                                    if let Ok(v) = e.value().parse::<u32>() {
                                        SETTINGS.write().think = v.clamp(THINK_MIN, THINK_MAX);
                                    }
                                },
                            }
                        }
                        label { class: "oral-check",
                            input {
                                r#type: "checkbox",
                                checked: settings.voice,
                                onchange: move |e| SETTINGS.write().voice = e.checked(),
                            }
                            span { {tx::VOICE} }
                        }
                        if settings.voice {
                            div { class: "oral-field",
                                span { {tx::RATE} }
                                div { class: "segmented", role: "radiogroup",
                                    for (i, name) in tx::RATES.iter().enumerate() {
                                        button {
                                            key: "{i}",
                                            role: "radio",
                                            "aria-checked": "{settings.rate == i}",
                                            onclick: move |_| SETTINGS.write().rate = i,
                                            {*name}
                                        }
                                    }
                                }
                            }
                        }
                        label { class: "oral-check",
                            input {
                                r#type: "checkbox",
                                checked: settings.auto_continue,
                                onchange: move |e| SETTINGS.write().auto_continue = e.checked(),
                            }
                            span { {tx::AUTO} }
                        }
                        p { class: "muted small", {tx::AUTO_HINT} }
                    }
                }
            }
        }
    }
}

/// What the driver should do next for the current question.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Ask,
    Reveal,
}

#[component]
fn Runner() -> Element {
    // A running round gets the whole phone screen; End is the way out.
    crate::parts::use_focus_mode(true);
    let store = use_store();
    let mut phase = use_signal(|| Phase::Asking);
    let no_voice = use_signal(|| false);
    // Bumped by every new step, so a stale timer or voice stops on its own.
    let mut run = use_signal(|| 0u64);
    let mut asked_at = use_signal(Instant::now);
    let mut rated_at = use_signal(|| None::<Instant>);
    // The think timer is his to stop and restart; a new question runs it again.
    let mut paused = use_signal(|| false);
    // Answer points he says he covered, and his own typed points: this
    // question only, never saved or sent anywhere.
    let mut ticked = use_signal(BTreeSet::<usize>::new);
    let mut notes = use_signal(String::new);
    let settled = move || rated_at.peek().is_none_or(|at| at.elapsed() >= SETTLE);
    // Rates question `turn` once and shows the next one as unasked right away
    // (the driver's own task runs later), so a second tap finds nothing to grade.
    let mut grade_turn = move |turn: u64, grade: Option<Grade>| {
        if rate_current(&store.read(), turn, grade, *asked_at.peek()) {
            phase.set(Phase::Asking);
            rated_at.set(Some(Instant::now()));
            tts::stop();
        }
    };

    let mut drive = move |step: Step| {
        let id = *run.peek() + 1;
        run.set(id);
        paused.set(false);
        tts::stop();
        if step == Step::Reveal && !settled() {
            return;
        }
        let Some((entry, turn)) = SESSION.peek().as_ref().and_then(|s| Some((s.current.clone()?, s.asked)))
        else {
            return;
        };
        let Settings { think, voice, rate, auto_continue, .. } = SETTINGS.peek().clone();
        let rate = tts::RATES[rate.min(tts::RATES.len() - 1)];
        let say = move |text: String| speak(text, voice, rate, no_voice);
        spawn(async move {
            let live = move || *run.peek() == id;
            if step == Step::Ask {
                asked_at.set(Instant::now());
                phase.set(Phase::Asking);
                say(spoken_question(&entry)).await;
                if !live() || !countdown(think, Phase::Thinking, phase, paused, live).await {
                    return;
                }
                if entry.prompt.what_else {
                    let more = (think / 2).max(THINK_MIN);
                    phase.set(Phase::WhatElse(more));
                    say("And what else?".into()).await;
                    if !live() || !countdown(more, Phase::WhatElse, phase, paused, live).await {
                        return;
                    }
                }
            }
            phase.set(Phase::Answer);
            say(entry.prompt.spoken_answer()).await;
            if auto_continue && live() {
                crate::platform::time::sleep(AUTO_WAIT).await;
                if live() {
                    // Moving on changes the session, and the effect below asks the next one.
                    grade_turn(turn, None);
                }
            }
        });
    };

    // Ask the first question, and the next one each time the session moves on.
    let asked_now = SESSION.read().as_ref().map_or(0, |s| s.asked);
    let mut asked = use_signal(|| 0u64);
    use_effect(use_reactive!(|asked_now| {
        if asked_now != 0 && *asked.peek() != asked_now {
            asked.set(asked_now);
            ticked.set(BTreeSet::new());
            notes.set(String::new());
            drive(Step::Ask);
        }
    }));

    // Screen off (Android): a foreground service keeps the session going,
    // shows it on the lock screen and passes on headset and notification buttons.
    use_hook(|| {
        background::start(&Labels {
            show: tx::SHOW,
            again: tx::AGAIN,
            good: tx::GOOD,
            end: tx::END,
            channel: tx::CHANNEL,
            channel_hint: tx::CHANNEL_HINT,
        });
    });
    use_drop(background::stop);
    let revealed_now = use_memo(move || *phase.read() == Phase::Answer);
    use_effect(move || {
        let revealed = revealed_now();
        let session = SESSION.read();
        let Some(entry) = session.as_ref().and_then(|s| s.current.as_ref()) else { return };
        let title = format!("{} · {} {}", tx::TITLE, tx::LEFT, session.as_ref().map_or(0, Session::left));
        let lines = if revealed { &entry.prompt.answer } else { &entry.prompt.question };
        background::update(&title, &lines.join(" · "), revealed);
    });
    use_future(move || async move {
        loop {
            crate::platform::time::sleep(Duration::from_millis(300)).await;
            let Some(remote) = background::take() else { continue };
            let revealed = revealed_now();
            let turn = SESSION.peek().as_ref().map_or(0, |s| s.asked);
            match remote {
                Remote::End => end_session(),
                Remote::Again if revealed => grade_turn(turn, Some(Grade::Again)),
                Remote::Main | Remote::Good if revealed => grade_turn(turn, Some(Grade::Good)),
                Remote::Main | Remote::Good => drive(Step::Reveal),
                Remote::Again => {}
            }
        }
    });

    let session = SESSION.read();
    let Some(s) = session.as_ref() else { return rsx! {} };
    let Some(entry) = s.current.clone() else { return rsx! {} };
    let left = s.left();
    let total = s.total.max(1);
    let turn = s.asked;
    drop(session);
    // Questions behind him, out of the session (Agains add to the end).
    let done_n = total.saturating_sub(left.min(total));
    let progress = 100 * done_n / total;
    let points = entry.prompt.answer.clone();
    let said = ticked.read().len();
    let enough = entry.prompt.pass.is_some_and(|(pass, _)| said >= pass);
    let phase_now = *phase.read();
    let revealed = phase_now == Phase::Answer;
    let p = &entry.prompt;
    // The course and the lecture, not a bare "L02": a mixed session can
    // come from any course (UX #300).
    let lecture = crate::content::Library::get().crumb(&p.lecture);
    // hover.js reads this course's glossary first (#318).
    let course = steps_core::program::course_of(&p.item_id).to_string();

    rsx! {
        div {
            class: "oral-run",
            "data-phase": if revealed { "answer" } else { "ask" },
            header { class: "oral-top",
                div {
                    class: "progress oral-progress",
                    role: "progressbar",
                    "aria-valuemin": "0",
                    "aria-valuemax": "100",
                    "aria-valuenow": "{progress}",
                    div { class: "progress-fill", style: "inline-size: {progress}%;" }
                }
                span { class: "muted small", "{tx::LEFT} " bdi { class: "num-ltr", "{left}" } }
                if entry.follow_up {
                    span { class: "chip accent", {tx::FOLLOW_UP} }
                }
                if entry.again {
                    span { class: "chip", {tx::AGAIN_BADGE} }
                }
                span { class: "muted small", bdi { "{lecture}" } }
                crate::hover::BookPage { page: p.page, quote: p.quote.clone(), of: p.lecture.clone() }
                button {
                    class: "oral-end",
                    "data-back": "true",
                    "aria-keyshortcuts": "Escape",
                    onclick: move |_| end_session(),
                    {tx::END}
                }
            }
            if *no_voice.read() {
                p { class: "note small", {tx::NO_VOICE} }
            }
            crate::desk::Panes {
                primary: rsx! {
                    crate::ui::Card { class: "oral-question", "data-course": "{course}",
                        h2 { class: "muted small", {tx::LISTEN} }
                        div { class: "en", dir: "ltr", lang: "en",
                            for (i, line) in p.question.iter().enumerate() {
                                p { key: "{i}", class: if i + 1 == p.question.len() { "oral-ask" } else { "" }, "{line}" }
                            }
                        }
                        if let Some((pass, of)) = p.pass {
                            span { class: "chip", "{tx::PASS} " bdi { "{pass}" } " {tx::OF} " bdi { "{of}" } }
                        }
                    }
                    if !revealed {
                        details { class: "oral-notes",
                            summary { {tx::NOTES} }
                            textarea {
                                lang: "en",
                                dir: "auto",
                                rows: "4",
                                placeholder: tx::NOTES_HINT,
                                value: "{notes}",
                                oninput: move |e| notes.set(e.value()),
                            }
                        }
                    }
                    match phase_now {
                        Phase::Asking => rsx! {
                            section { class: "oral-timer", "aria-live": "polite",
                                span { class: "oral-status", {tx::LISTEN} }
                            }
                        },
                        Phase::Thinking(n) | Phase::WhatElse(n) => {
                            let what_else = matches!(phase_now, Phase::WhatElse(_));
                            let full = if what_else { (SETTINGS.peek().think / 2).max(THINK_MIN) } else { SETTINGS.peek().think };
                            let pct = 100 * n / full.max(1);
                            rsx! {
                                section { class: "oral-timer", "aria-live": "polite", "data-paused": if paused() { "yes" } else { "no" },
                                    div { class: "oral-ring", style: "--p: {pct}", span { "{n}" } }
                                    span { class: "oral-status",
                                        if paused() { {tx::PAUSED} } else if what_else { {tx::WHAT_ELSE} } else { {tx::THINKING} }
                                    }
                                    div { class: "row oral-timer-controls",
                                        button {
                                            class: "btn btn-secondary",
                                            "aria-pressed": if paused() { "true" } else { "false" },
                                            onclick: move |_| { let p = *paused.peek(); paused.set(!p); },
                                            if paused() { "▶ {tx::RESUME_TIMER}" } else { "⏸ {tx::PAUSE_TIMER}" }
                                        }
                                        button {
                                            class: "btn btn-ghost",
                                            onclick: move |_| drive(Step::Ask),
                                            "↺ {tx::RESTART_TIMER}"
                                        }
                                    }
                                }
                            }
                        }
                        Phase::Answer => rsx! {},
                    }
                },
                secondary: revealed.then(|| rsx! {
                    crate::ui::Card { class: "oral-answer", "data-course": "{course}",
                        h2 { class: "small", {tx::ANSWER} }
                        p { class: "muted small", {tx::TICK_HINT} }
                        // The model answer as a checklist he ticks against
                        // what he said; the rating stays his.
                        ul { class: "oral-points", lang: "en",
                            for (i, line) in points.iter().cloned().enumerate() {
                                li { key: "{i}",
                                    button {
                                        class: "oral-point",
                                        dir: "ltr",
                                        "aria-pressed": if ticked.read().contains(&i) { "true" } else { "false" },
                                        onclick: move |_| {
                                            let mut t = ticked.write();
                                            if !t.remove(&i) {
                                                t.insert(i);
                                            }
                                        },
                                        span { class: "oral-tick", aria_hidden: "true", if ticked.read().contains(&i) { "✓" } else { "" } }
                                        span { "{line}" }
                                    }
                                }
                            }
                        }
                        p { class: "small oral-said", "data-enough": if enough { "yes" } else { "no" },
                            "{tx::SAID} " bdi { class: "num-ltr", "{said} / {points.len()}" }
                            if let Some((pass, _)) = p.pass {
                                " · {tx::PASS} " bdi { class: "num-ltr", "{pass}" }
                                if enough { " · {tx::ENOUGH}" }
                            }
                        }
                        if !notes.read().trim().is_empty() {
                            div { class: "oral-your-notes",
                                h3 { class: "small", {tx::YOUR_NOTES} }
                                p { lang: "en", dir: "auto", "{notes}" }
                            }
                        }
                    }
                }),
            }
            if revealed {
                crate::verify::VerifyBar { item_id: p.item_id.clone() }
            }
            div { class: "oral-actions",
                if revealed {
                    button {
                        class: "oral-big again",
                        "aria-keyshortcuts": "1",
                        onclick: move |_| grade_turn(turn, Some(Grade::Again)),
                        span { class: "big-label", {tx::AGAIN} }
                        span { class: "big-hint", {tx::AGAIN_HINT} }
                    }
                    button {
                        class: "oral-big good",
                        "data-suggest": if enough { "yes" } else { "no" },
                        "aria-keyshortcuts": "3 2 Space",
                        onclick: move |_| grade_turn(turn, Some(Grade::Good)),
                        span { class: "big-label", {tx::GOOD} }
                        span { class: "big-hint", {tx::GOOD_HINT} }
                    }
                } else {
                    button { class: "oral-small", onclick: move |_| drive(Step::Ask), {tx::REPEAT} }
                    button {
                        class: "oral-big show",
                        "aria-keyshortcuts": "Space Enter",
                        onclick: move |_| drive(Step::Reveal),
                        span { class: "big-label", {tx::SHOW} }
                    }
                }
            }
        }
    }
}

/// Counts down `secs`, showing each second; time stands still while
/// `paused`. `false` if the step was replaced.
async fn countdown(
    secs: u32,
    show: fn(u32) -> Phase,
    mut phase: Signal<Phase>,
    paused: Signal<bool>,
    live: impl Fn() -> bool,
) -> bool {
    const TICK: Duration = Duration::from_millis(100);
    for n in (1..=secs).rev() {
        phase.set(show(n));
        let mut ran = Duration::ZERO;
        while ran < Duration::from_secs(1) {
            crate::platform::time::sleep(TICK).await;
            if !live() {
                return false;
            }
            if !*paused.peek() {
                ran += TICK;
            }
        }
    }
    true
}

/// Says `text` when the voice is on and working; on a missing voice, turns
/// it off for the session and shows why (the text is on screen anyway).
async fn speak(text: String, voice: bool, rate: f32, mut no_voice: Signal<bool>) {
    if !voice || *no_voice.peek() {
        return;
    }
    if let Err(e) = tts::say(&text, rate).await {
        if !matches!(e, tts::SpeakError::Failed(_)) {
            no_voice.set(true);
        }
        crate::diag::report!("oral: speech failed: {e:?}");
    }
}

/// The question as the examiner says it.
fn spoken_question(entry: &Entry) -> String {
    let lead = if entry.follow_up {
        "Following on from that. "
    } else if entry.again {
        "Once more. "
    } else {
        ""
    };
    format!("{lead}{}", entry.prompt.spoken_question())
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Stops early; the summary shows what was done.
fn end_session() {
    tts::stop();
    if let Some(s) = SESSION.write().as_mut() {
        s.end();
    }
}

/// Records the rating of question `turn` (none when heard without one) and
/// moves on. `false` when that question is already past (a second tap).
fn rate_current(store: &StudyStoreHandle, turn: u64, grade: Option<Grade>, asked_at: Instant) -> bool {
    let mut guard = SESSION.write();
    let Some(s) = guard.as_mut() else { return false };
    let Some(prompt) = s.rate_turn(turn, grade) else { return false };
    if let Some(grade) = grade {
        store.0.borrow_mut().record(ReviewEvent {
            item_id: prompt.item_id,
            grade,
            origin: Origin::Oral,
            at_ms: now_ms(),
            took_ms: asked_at.elapsed().as_millis().min(u32::MAX as u128) as u32,
            confidence: None,
            reason: None,
        });
    }
    true
}

/// One missed item in a study session, or its lecture's page when the study
/// player can't show it.
fn study_one(mut ui: crate::Ui, id: String) {
    if crate::study::is_playable(&id) {
        *crate::study::START.write() =
            Some(crate::study::Start { lectures: None, items: Some(vec![id]), minutes: 5 });
        ui.screen.set(crate::Screen::Study);
    } else if let Some((lecture, _)) = id.rsplit_once('.') {
        crate::lecture::open(ui, lecture);
    }
}

#[component]
fn Summary() -> Element {
    let ui = use_context::<crate::Ui>();
    let store = use_store();
    let session = SESSION.read();
    let Some(s) = session.as_ref() else { return rsx! {} };
    let t = s.tally.clone();
    drop(session);
    rsx! {
        crate::ui::Card { class: "oral-summary",
            h1 { {tx::DONE} }
            dl { class: "oral-tally",
                div { dt { {tx::DONE_GOOD} } dd { "{t.good}" } }
                div { dt { {tx::DONE_AGAIN} } dd { "{t.again}" } }
                if t.heard > 0 {
                    div { dt { {tx::DONE_HEARD} } dd { "{t.heard}" } }
                }
            }
        }
        crate::ui::Card {
            h2 { {tx::MISSED} }
            if t.missed.is_empty() {
                p { class: "muted", {tx::NONE_MISSED} }
            } else {
                // A missed question opens on its card in a study session.
                ul { class: "en oral-missed tap-list", dir: "ltr", lang: "en",
                    for p in t.missed {
                        li { key: "{p.item_id}",
                            button {
                                class: "tap tap-row",
                                lang: "ar",
                                title: tx::MISSED_HINT,
                                onclick: { let id = p.item_id.clone(); move |_| study_one(ui, id.clone()) },
                                span { class: "tap-row-main", lang: "en",
                                    span { "{p.question.last().cloned().unwrap_or_default()}" }
                                    span { class: "muted small", " (p. {p.page})" }
                                }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
            }
        }
        div { class: "oral-actions",
            button { class: "oral-small", onclick: move |_| *SESSION.write() = None, {tx::BACK} }
            button {
                class: "oral-big good",
                "aria-keyshortcuts": "Enter",
                onclick: move |_| start(&store.read()),
                span { class: "big-label", {tx::ANOTHER} }
            }
        }
    }
}
