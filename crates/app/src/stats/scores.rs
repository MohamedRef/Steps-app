//! The self-assessment tab (A14 / research item 5): Mohamed logs NBME, UWSA,
//! Free 120 and other practice-test scores he takes outside Steps, sees them
//! on a chart with the low-pass band, and reads one plain verdict from
//! `steps_core::selfassess::readiness`. Only numbers he typed are shown;
//! nothing here predicts a score or a pass probability.

use std::fmt::Write as _;
use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::plan::Day;
use steps_core::selfassess::{
    self as sa, FLAT_TOLERANCE, Kind, LOW_PASS_BAND, Problem, READY_PERCENT, Readiness, SelfAssessment,
};

use super::StudyData;
use super::charts::esc;
use crate::i18n::scores as tx;
use crate::store::{self, selfassess::new_id};

/// Test-only: open the add form at once (the browser fixture).
#[cfg(test)]
pub static FORM_FOR_FIXTURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The entry being written; the form shows while it's set.
#[derive(Clone, Debug, PartialEq)]
struct Draft {
    entry: SelfAssessment,
    new: bool,
}

#[component]
pub fn Scores(data: Rc<StudyData>) -> Element {
    // Re-render after a sync or a restore (they bump the review revision).
    let _review = crate::store::use_review();
    let mut version = use_signal(|| 0u64);
    let _ = version();
    let entries = store::shared().self_assessments();
    let today = data.clock.today();
    let mut draft = use_signal(|| {
        #[cfg(test)]
        if FORM_FOR_FIXTURE.load(std::sync::atomic::Ordering::Relaxed) {
            return Some(Draft { entry: blank(today), new: true });
        }
        None::<Draft>
    });
    let points = sa::readiness_points(&entries);
    let verdict = sa::readiness(&points);
    let has_probability = entries.iter().any(|e| e.pass_probability.is_some());
    let has_practice = entries.iter().any(|e| !e.kind.counts_for_readiness());
    rsx! {
        div { class: "tab-cols scores", "data-align": "stretch",
            div { class: "tab-main",
                Verdict { verdict, points: points.clone() }
                if !entries.is_empty() {
                    crate::ui::Card { class: "chart-card",
                        h2 { {tx::CHART_TITLE} }
                        div { class: "chart-box score-chart", dangerous_inner_html: "{chart(&entries, tx::CHART_LABEL)}" }
                        div { class: "chart-key small muted",
                            span { span { class: "key key-band" } {tx::KEY_BAND} }
                            span { span { class: "key key-ready" } {tx::KEY_READY} }
                            span { span { class: "key key-nbme" } bdi { "NBME" } }
                            if has_practice {
                                span { span { class: "key key-practice" } {tx::KEY_PRACTICE} }
                            }
                        }
                    }
                }
            }
            div { class: "tab-aside",
                crate::ui::Card {
                    div { class: "score-head",
                        h2 { {tx::LIST_TITLE} }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| draft.set(Some(Draft { entry: blank(today), new: true })),
                            {tx::ADD}
                        }
                    }
                    if let Some(d) = draft() {
                        Form {
                            key: "{d.entry.id}",
                            draft: d,
                            today,
                            on_done: move |saved: bool| {
                                draft.set(None);
                                if saved {
                                    *version.write() += 1;
                                }
                            },
                        }
                    }
                    if entries.is_empty() {
                        p { class: "muted", {tx::EMPTY} }
                    } else {
                        ul { class: "score-list",
                            for e in entries.iter().rev() {
                                Row {
                                    key: "{e.id}",
                                    entry: e.clone(),
                                    on_edit: move |e: SelfAssessment| draft.set(Some(Draft { entry: e, new: false })),
                                    on_deleted: move |_| *version.write() += 1,
                                }
                            }
                        }
                    }
                }
                crate::ui::Card { class: "score-notes",
                    p { class: "muted small", {tx::RULE_NOTE} }
                    if has_practice {
                        p { class: "muted small", {tx::PRACTICE_NOTE} }
                    }
                    if has_probability {
                        p { class: "muted small", {tx::PROBABILITY_NOTE} }
                    }
                    p { class: "muted small", {tx::FREE120_NOTE} }
                }
            }
        }
    }
}

fn blank(today: Day) -> SelfAssessment {
    SelfAssessment {
        id: new_id(),
        kind: Kind::Nbme,
        form: None,
        day: today,
        percent: 0,
        pass_probability: None,
        note: None,
        created_ms: store::now_ms(),
    }
}

/// What the verdict card says, from the rule alone.
pub fn verdict_text(v: Readiness) -> &'static str {
    match v {
        Readiness::NoScores => tx::V_NONE,
        Readiness::OneScore => tx::V_ONE,
        Readiness::Ready => tx::V_READY,
        Readiness::Close => tx::V_CLOSE,
        Readiness::NotYet => tx::V_NOT_YET,
    }
}

#[component]
fn Verdict(verdict: Readiness, points: Vec<(Day, u8)>) -> Element {
    let last_two: Vec<u8> = points.iter().rev().take(2).rev().map(|(_, p)| *p).collect();
    let trend = sa::trend(&points);
    rsx! {
        crate::ui::Card { class: "score-verdict",
            h2 { {tx::VERDICT_TITLE} }
            p { class: "score-verdict-text", "data-verdict": "{verdict:?}", {verdict_text(verdict)} }
            if !last_two.is_empty() {
                p { class: "muted small",
                    {tx::LAST_NBME}
                    " "
                    if let [prev, last] = last_two[..] {
                        bdi { "{prev}%" }
                        " {tx::THEN} "
                        bdi { "{last}%" }
                    } else {
                        bdi { "{last_two[0]}%" }
                    }
                    if let Some(t) = trend {
                        " · "
                        {trend_text(t)}
                    }
                }
            }
        }
    }
}

pub fn trend_text(change: i16) -> String {
    if change > 0 {
        format!("{} {change} {}", tx::TREND_UP, tx::POINTS)
    } else if change < -i16::from(FLAT_TOLERANCE) {
        format!("{} {} {}", tx::TREND_DOWN, -change, tx::POINTS)
    } else {
        tx::TREND_FLAT.to_string()
    }
}

/// The kind's name on screen: the test's own name, or Arabic for "other".
pub fn kind_name(k: Kind) -> &'static str {
    if k == Kind::Other { tx::KIND_OTHER } else { k.label() }
}

#[component]
fn Row(entry: SelfAssessment, on_edit: EventHandler<SelfAssessment>, on_deleted: EventHandler) -> Element {
    let mut confirm = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    let e = entry.clone();
    let title =
        if e.kind == Kind::Other && e.form.is_none() { tx::KIND_OTHER.to_string() } else { e.title() };
    let practice = !e.kind.counts_for_readiness();
    rsx! {
        li { class: "score-entry",
            div { class: "score-text",
                span { class: "score-title",
                    bdi { dir: "ltr", "{title}" }
                    " · "
                    bdi { class: "score-pct", "{e.percent}%" }
                }
                span { class: "muted small",
                    bdi { dir: "ltr", "{e.day.iso()}" }
                    if let Some(p) = e.pass_probability {
                        " · {tx::PROBABILITY_SHORT} "
                        bdi { "{p}%" }
                    }
                    if practice {
                        " · {tx::PRACTICE_TAG}"
                    }
                }
                if let Some(n) = &e.note {
                    span { class: "small", bdi { dir: "auto", "{n}" } }
                }
                if let Some(m) = failed() {
                    span { class: "field-error", role: "alert", "{m}" }
                }
            }
            div { class: "score-actions",
                if confirm() {
                    button {
                        class: "btn btn-danger",
                        onclick: {
                            let id = e.id.clone();
                            move |_| match store::shared().delete_self_assessment(&id) {
                                Ok(()) => on_deleted.call(()),
                                Err(err) => failed.set(Some(format!("{} {err}", tx::FAILED))),
                            }
                        },
                        {tx::CONFIRM_DELETE}
                    }
                    button { class: "btn btn-ghost", onclick: move |_| confirm.set(false), {tx::CANCEL} }
                } else {
                    button { class: "btn btn-secondary", onclick: move |_| on_edit.call(entry.clone()), {tx::EDIT} }
                    button { class: "btn btn-ghost", onclick: move |_| confirm.set(true), {tx::DELETE} }
                }
            }
        }
    }
}

pub fn problem_text(p: Problem) -> &'static str {
    match p {
        Problem::PercentTooHigh | Problem::ProbabilityTooHigh => tx::BAD_PERCENT,
        Problem::FormTooLong => tx::FORM_TOO_LONG,
        Problem::NoteTooLong => tx::NOTE_TOO_LONG,
        Problem::InTheFuture => tx::IN_THE_FUTURE,
    }
}

#[component]
fn Form(draft: Draft, today: Day, on_done: EventHandler<bool>) -> Element {
    let mut entry = use_signal(|| draft.entry.clone());
    // The typed text, kept as typed so a half-written number isn't lost.
    let mut percent = use_signal(|| if draft.new { String::new() } else { draft.entry.percent.to_string() });
    let mut prob = use_signal(|| draft.entry.pass_probability.map(|p| p.to_string()).unwrap_or_default());
    let mut date = use_signal(|| draft.entry.day.iso());
    let mut problem = use_signal(|| None::<String>);
    let mut save = move |_: ()| {
        let mut e = entry().tidy();
        let Some(day) = Day::parse(&date()) else {
            problem.set(Some(tx::BAD_DATE.into()));
            return;
        };
        let Some(pct) = percent().trim().parse::<u8>().ok() else {
            problem.set(Some(tx::BAD_PERCENT.into()));
            return;
        };
        let pr = prob();
        let pass_probability = match pr.trim() {
            "" => None,
            t => match t.parse::<u8>() {
                Ok(p) => Some(p),
                Err(_) => {
                    problem.set(Some(tx::BAD_PERCENT.into()));
                    return;
                }
            },
        };
        e.day = day;
        e.percent = pct;
        e.pass_probability = pass_probability;
        if let Some(p) = e.problem(today) {
            problem.set(Some(problem_text(p).into()));
            return;
        }
        match store::shared().put_self_assessment(&e) {
            Ok(()) => on_done.call(true),
            Err(err) => problem.set(Some(format!("{} {err}", tx::FAILED))),
        }
    };
    let e = entry();
    rsx! {
        form {
            class: "score-form",
            aria_label: if draft.new { tx::FORM_NEW } else { tx::FORM_EDIT },
            onsubmit: move |ev: FormEvent| {
                ev.prevent_default();
                save(());
            },
            div { class: "score-fields",
                label { class: "field score-field",
                    span { {tx::KIND} }
                    select {
                        onchange: move |ev| {
                            if let Some(k) = Kind::parse(&ev.value()) {
                                entry.write().kind = k;
                            }
                        },
                        for k in Kind::ALL {
                            option { key: "{k.as_str()}", value: k.as_str(), selected: e.kind == k, {kind_name(k)} }
                        }
                    }
                }
                label { class: "field score-field",
                    span { {tx::FORM} }
                    input {
                        dir: "ltr",
                        placeholder: tx::FORM_HINT,
                        value: e.form.clone().unwrap_or_default(),
                        oninput: move |ev| entry.write().form = Some(ev.value()),
                    }
                }
            }
            div { class: "score-fields",
                label { class: "field score-field",
                    span { {tx::DATE} }
                    input {
                        r#type: "date",
                        dir: "ltr",
                        max: today.iso(),
                        value: "{date}",
                        oninput: move |ev| date.set(ev.value()),
                    }
                }
                label { class: "field score-field score-narrow",
                    span { {tx::PERCENT} }
                    input {
                        r#type: "number",
                        inputmode: "numeric",
                        min: "0",
                        max: "100",
                        dir: "ltr",
                        required: true,
                        value: "{percent}",
                        oninput: move |ev| percent.set(ev.value()),
                    }
                }
            }
            if e.kind == Kind::Nbme {
                label { class: "field score-field",
                    span { {tx::PROBABILITY} }
                    input {
                        r#type: "number",
                        inputmode: "numeric",
                        min: "0",
                        max: "100",
                        dir: "ltr",
                        value: "{prob}",
                        oninput: move |ev| prob.set(ev.value()),
                    }
                    span { class: "field-hint", {tx::PROBABILITY_HINT} }
                }
            }
            label { class: "field score-field",
                span { {tx::NOTE} }
                input {
                    dir: "auto",
                    value: e.note.clone().unwrap_or_default(),
                    oninput: move |ev| entry.write().note = Some(ev.value()),
                }
            }
            if let Some(m) = problem() {
                p { class: "field-error", role: "alert", "{m}" }
            }
            div { class: "score-actions",
                button { class: "btn btn-primary", r#type: "submit", {tx::SAVE} }
                button { class: "btn btn-ghost", r#type: "button", onclick: move |_| on_done.call(false), {tx::CANCEL} }
            }
        }
    }
}

/// The scores over time, the newest on the left (the axis is mirrored for
/// the Arabic UI, as in `charts`): the low-pass band shaded, the ready line
/// dashed, NBME forms joined by a line, and other tests as hollow marks.
pub fn chart(entries: &[SelfAssessment], label: &str) -> String {
    let (w, h, pad) = (320.0f32, 120.0f32, 8.0f32);
    let sorted = sa::sorted(entries);
    // Room for the scores and the band, in steps of ten.
    let low = sorted.iter().map(|e| e.percent).min().unwrap_or(0).min(*LOW_PASS_BAND.start());
    let high = sorted.iter().map(|e| e.percent).max().unwrap_or(0).max(READY_PERCENT);
    let lo = f32::from(low.saturating_sub(5) / 10 * 10);
    let hi = (((f32::from(high) + 5.0) / 10.0).ceil() * 10.0).min(100.0);
    let (first, last) = match (sorted.first(), sorted.last()) {
        (Some(f), Some(l)) => (f.day, l.day),
        _ => (Day(0), Day(0)),
    };
    let span = first.until(last).max(1) as f32;
    let x = |d: Day| {
        if first == last { w / 2.0 } else { w - pad - first.until(d) as f32 / span * (w - 2.0 * pad) }
    };
    let y = |p: f32| h - pad - (p.clamp(lo, hi) - lo) / (hi - lo) * (h - 2.0 * pad);
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="chart score-svg" viewBox="0 0 {w} {h}" role="img" aria-label="{l}" direction="ltr" preserveAspectRatio="xMidYMid meet"><title>{l}</title>"#,
        l = esc(label)
    );
    let (b0, b1) = (y(f32::from(*LOW_PASS_BAND.end())), y(f32::from(*LOW_PASS_BAND.start())));
    let _ = write!(s, r#"<rect class="band" x="0" y="{b0:.1}" width="{w}" height="{:.1}"/>"#, b1 - b0);
    let r = y(f32::from(READY_PERCENT));
    let _ = write!(s, r#"<line class="ready" x1="0" x2="{w}" y1="{r:.1}" y2="{r:.1}"/>"#);
    let _ = write!(s, r#"<line class="baseline" x1="0" x2="{w}" y1="{}" y2="{}"/>"#, h - pad, h - pad);
    let nbme: Vec<&SelfAssessment> = sorted.iter().filter(|e| e.kind.counts_for_readiness()).collect();
    if nbme.len() >= 2 {
        let d: Vec<String> = nbme
            .iter()
            .enumerate()
            .map(|(i, e)| {
                format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x(e.day), y(f32::from(e.percent)))
            })
            .collect();
        let _ = write!(s, r#"<path class="line-nbme" d="{}"/>"#, d.join(" "));
    }
    for e in &sorted {
        let class = if e.kind.counts_for_readiness() { "pt nbme" } else { "pt practice" };
        let _ = write!(
            s,
            r#"<circle class="{class}" cx="{:.1}" cy="{:.1}" r="4"><title>{} · {}% · {}</title></circle>"#,
            x(e.day),
            y(f32::from(e.percent)),
            esc(&e.title()),
            e.percent,
            e.day.iso()
        );
    }
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: Kind, day: &str, percent: u8) -> SelfAssessment {
        SelfAssessment {
            id: format!("sa.{day}"),
            kind,
            form: None,
            day: Day::parse(day).unwrap(),
            percent,
            pass_probability: None,
            note: None,
            created_ms: 0,
        }
    }

    #[test]
    fn chart_marks_every_test_and_joins_nbme_only() {
        let svg = chart(
            &[
                entry(Kind::Nbme, "2027-06-01", 60),
                entry(Kind::Uwsa, "2027-06-04", 72),
                entry(Kind::Nbme, "2027-06-08", 70),
            ],
            "a < b",
        );
        assert!(svg.contains(r#"aria-label="a &lt; b""#));
        assert_eq!(svg.matches("<circle").count(), 3);
        assert_eq!(svg.matches("pt practice").count(), 1);
        assert_eq!(svg.matches("line-nbme").count(), 1);
        assert!(svg.contains(r#"class="band""#) && svg.contains(r#"class="ready""#));
        // The plot spans 50-80% here: 80% would be the top, 50% the bottom.
        assert!(svg.contains(r#"<line class="ready" x1="0" x2="320" y1="49.6""#));
        // Mirrored: the first test on the right edge, the last on the left.
        assert!(svg.contains(r#"cx="312.0""#) && svg.contains(r#"cx="8.0""#));
    }

    #[test]
    fn a_single_test_sits_in_the_middle_without_a_line() {
        let svg = chart(&[entry(Kind::Nbme, "2027-06-01", 100)], "x");
        assert!(svg.contains(r#"cx="160.0""#) && !svg.contains("line-nbme"));
        // 100% is the top of the plot.
        assert!(svg.contains(r#"cy="8.0""#));
    }

    #[test]
    fn words_for_every_verdict_trend_and_problem() {
        for v in
            [Readiness::NoScores, Readiness::OneScore, Readiness::Ready, Readiness::Close, Readiness::NotYet]
        {
            assert!(!verdict_text(v).is_empty());
        }
        assert!(trend_text(3).contains('3'));
        assert_eq!(trend_text(-2), tx::TREND_FLAT);
        assert!(trend_text(-5).contains('5'));
        for p in [
            Problem::PercentTooHigh,
            Problem::ProbabilityTooHigh,
            Problem::FormTooLong,
            Problem::NoteTooLong,
            Problem::InTheFuture,
        ] {
            assert!(!problem_text(p).is_empty());
        }
        assert_eq!(kind_name(Kind::Other), tx::KIND_OTHER);
        assert_eq!(kind_name(Kind::Free120), "Free 120");
    }
}
