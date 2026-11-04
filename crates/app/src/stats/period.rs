//! The progress tab's head (2026-10-29 redesign): a real week/month switch,
//! retention over that period against the one before, minutes per day, and
//! three counts. Every number comes from `steps_core::stats::period`, and
//! every card opens where its number is explained.

use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::stats::{self, Period};

use super::{StudyData, Tab, open};
use crate::i18n::plan::WEEKDAYS_SHORT;
use crate::i18n::stats as tx;

/// Week or month; kept while the app runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    Week,
    Month,
}

impl Span {
    fn days(self) -> u32 {
        match self {
            Span::Week => 7,
            Span::Month => 30,
        }
    }
}

static SPAN: GlobalSignal<Span> = Signal::global(|| Span::Week);

/// How retention moved against the period before: (better, points), or
/// `None` when either period has nothing to compare.
pub fn change(now: &Period, before: &Period) -> Option<(bool, u32)> {
    let (a, b) = (now.retention.percent()?, before.retention.percent()?);
    Some((a >= b, a.abs_diff(b)))
}

#[component]
pub fn PeriodHead(data: Rc<StudyData>) -> Element {
    let ui = use_context::<crate::Ui>();
    let span = SPAN();
    let week = span == Span::Week;
    let now = stats::period(&data.events, &data.marks, data.clock, span.days(), 0);
    let before = stats::period(&data.events, &data.marks, data.clock, span.days(), 1);
    let pct = now.retention.percent();
    let shown = super::pct(now.retention);
    let peak = now.days.iter().map(|d| d.minutes).max().unwrap_or(0).max(1);
    let last = now.days.len().saturating_sub(1);
    rsx! {
        div { class: "segmented st-span", role: "radiogroup", aria_label: tx::PERIOD_LABEL,
            for (s , label) in [(Span::Week, tx::WEEK), (Span::Month, tx::MONTH)] {
                button {
                    key: "{label}",
                    role: "radio",
                    "aria-checked": span == s,
                    onclick: move |_| *SPAN.write() = s,
                    {label}
                }
            }
        }
        // Retention opens the mistakes: what pulled it down.
        crate::ui::CardButton {
            class: "tap st-hero",
            title: tx::RECALL_HINT,
            onclick: move |_| open(ui, Tab::Mistakes),
            span {
                class: "st-ring",
                role: "img",
                aria_label: "{tx::RECALL} {shown}",
                style: "--p: {pct.unwrap_or(0)}%",
                span { class: "st-ring-hole",
                    bdi { dir: "ltr", "{shown}" }
                }
            }
            span { class: "st-hero-text",
                strong { {tx::RECALL} }
                span { class: "muted small",
                    {if pct.is_some() { tx::RECALL_SUB } else { tx::RECALL_NONE }}
                }
                match change(&now, &before) {
                    Some((true, 0..=1)) | Some((false, 0..=1)) => rsx! { span { class: "chip", {tx::same(week)} } },
                    Some((true, d)) => rsx! { span { class: "chip", "data-tone": "good", {tx::better(week, d)} } },
                    Some((false, d)) => rsx! { span { class: "chip", "data-tone": "accent", {tx::worse(week, d)} } },
                    None => rsx! {},
                }
            }
            span { class: "chev", aria_hidden: "true" }
        }
        // Minutes per day; the planner shows what's ahead.
        crate::ui::CardButton {
            class: "tap st-activity",
            title: tx::ACTIVITY_HINT,
            onclick: move |_| open(ui, Tab::Plan),
            span { class: "st-activity-head",
                strong { {tx::ACTIVITY} }
                span { class: "muted small", {tx::ACTIVITY_UNIT} }
            }
            if now.answers == 0 {
                span { class: "muted small", {tx::ACTIVITY_EMPTY} }
            }
            span { class: "st-bars", "data-span": if week { "week" } else { "month" }, dir: "ltr", aria_hidden: "true",
                for (i , d) in now.days.iter().enumerate() {
                    span {
                        key: "{d.day.0}",
                        class: "st-bar",
                        "data-today": i == last,
                        title: tx::day_bar(d.minutes, d.answers),
                        style: format!("block-size: {}%", bar_height(d.minutes, d.answers, peak)),
                    }
                }
            }
            span { class: "st-bar-labels small muted", dir: "ltr",
                if week {
                    for d in now.days.iter() {
                        span { key: "{d.day.0}", {WEEKDAYS_SHORT[d.day.weekday()]} }
                    }
                } else {
                    span { {tx::MONTH_START} }
                    span { {tx::TODAY} }
                }
            }
        }
        div { class: "stat-tiles st-minis",
            button { class: "tap stat-tile", title: tx::ACTIVITY_HINT, onclick: move |_| open(ui, Tab::Plan),
                span { class: "stat-num", "{now.study_days}" }
                span { class: "stat-label", {tx::DAYS_STUDIED} }
            }
            button { class: "tap stat-tile", title: tx::ACTIVITY_HINT, onclick: move |_| open(ui, Tab::Plan),
                span { class: "stat-num", "{now.minutes}" }
                span { class: "stat-label", {tx::MINUTES} }
            }
            button { class: "tap stat-tile", title: tx::RECALL_HINT, onclick: move |_| open(ui, Tab::Mistakes),
                span { class: "stat-num", "{now.answers}" }
                span { class: "stat-label", {tx::ANSWERS} }
            }
        }
    }
}

/// A bar's height in percent: a day with answers always shows a little.
fn bar_height(minutes: u32, answers: u32, peak: u32) -> u32 {
    (minutes * 100 / peak.max(1)).max(if answers > 0 { 6 } else { 2 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use steps_core::stats::Ratio;

    fn with(right: u32, total: u32) -> Period {
        Period { retention: Ratio { right, total }, ..Period::default() }
    }

    #[test]
    fn retention_change_needs_both_periods() {
        assert_eq!(change(&with(8, 10), &with(6, 10)), Some((true, 20)));
        assert_eq!(change(&with(5, 10), &with(7, 10)), Some((false, 20)));
        assert_eq!(change(&with(5, 10), &Period::default()), None);
    }
}
