//! Catch-up plan for a review backlog (research brief A7): when far more
//! reviews are due than he usually does in a day, Today offers a calm plan
//! instead of a scary number: spread the backlog over 2–7 days, optionally
//! pausing new items until it is done, with the load said in minutes.
//! Nothing is hidden for good: each day's share is the most overdue
//! reviews first, and whatever is left keeps its due date.
//! Inspired by Anki's "stop new cards until you catch up" and RemNote's
//! catch-up period.
//!
//! The plan is a synced setting (`catchup`), last writer wins, so the
//! phone and the tablet agree.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::catchup as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/catchup/catchup.css");

const KEY: &str = "catchup";
/// Days over which recent answers are averaged.
pub const HISTORY_DAYS: usize = 14;
/// Below this many due reviews there is no backlog, whatever the average.
pub const MIN_BACKLOG: usize = 30;
pub const DAY_CHOICES: [u32; 4] = [2, 3, 5, 7];
const DEFAULT_DAYS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// The study day it started on.
    pub start_day: i64,
    pub days: u32,
    /// Reviews due when it started.
    pub backlog: u32,
    /// No new items while it runs.
    pub pause_new: bool,
}

impl Plan {
    /// Reviews to do each day.
    pub fn share(&self) -> u32 {
        self.backlog.div_ceil(self.days.max(1))
    }

    /// Still running on study day `today`.
    pub fn running(&self, today: i64) -> bool {
        (self.start_day..self.start_day + i64::from(self.days)).contains(&today)
    }

    /// Reviews still in today's share after `answered` answers today.
    pub fn left_today(&self, answered: usize) -> usize {
        (self.share() as usize).saturating_sub(answered)
    }
}

/// The usual day: answers per day over the last [`HISTORY_DAYS`] days
/// before today (today is still going on).
pub fn usual(per_day: &[u32]) -> f32 {
    let past = &per_day[..per_day.len().saturating_sub(1)];
    if past.is_empty() { 0.0 } else { past.iter().sum::<u32>() as f32 / past.len() as f32 }
}

/// A backlog: more than twice the usual day, and at least [`MIN_BACKLOG`].
pub fn is_backlog(due: usize, usual: f32) -> bool {
    due > MIN_BACKLOG.max((usual * 2.0).ceil() as usize)
}

/// The plan, when one is running today.
pub fn running(today: i64) -> Option<Plan> {
    let _ = REV();
    load().filter(|p| p.running(today))
}

fn load() -> Option<Plan> {
    crate::store::shared().setting::<Plan>(KEY)
}

fn store(plan: &Plan) -> bool {
    crate::store::save_setting(KEY, plan)
}

/// Bumped on every change, so screens reading the plan draw again.
static REV: GlobalSignal<u64> = Signal::global(|| 0);

fn save(plan: Option<Plan>) {
    // An ended plan is saved as one that never runs, so the stop syncs.
    let plan = plan.unwrap_or(Plan { start_day: 0, days: 0, backlog: 0, pause_new: false });
    if store(&plan) {
        *REV.write() += 1;
    }
}

/// After a sync or restore: draw the plan again.
pub fn reload() {
    *REV.write() += 1;
}

/// How many new items may start today, with the plan applied.
pub fn new_left(today: i64, without: usize) -> usize {
    if running(today).is_some_and(|p| p.pause_new) { 0 } else { without }
}

/// Today's due reviews, cut to the plan's share (most overdue first, as
/// `due` comes).
pub fn share_of(today: i64, answered_today: usize, mut due: Vec<String>) -> Vec<String> {
    if let Some(p) = running(today) {
        due.truncate(p.left_today(answered_today));
    }
    due
}

/// Today's card: the offer when there is a backlog, the plan while it runs.
#[component]
pub fn CatchUpCard(due: usize, due_minutes: u32) -> Element {
    let review = crate::store::use_review();
    let now = crate::store::now_ms();
    let (today, per_day, answered) = {
        let b = review.backend.borrow();
        let per_day = b.answers_per_day(HISTORY_DAYS + 1, now);
        let answered = per_day.last().copied().unwrap_or(0) as usize;
        (b.study_day(now), per_day, answered)
    };
    let mut days = use_signal(|| DEFAULT_DAYS);
    let mut pause = use_signal(|| true);
    if let Some(p) = running(today) {
        let left = p.left_today(answered);
        let day_n = (today - p.start_day + 1).max(1);
        let per_item = if due > 0 { due_minutes as f32 / due as f32 } else { 0.0 };
        let minutes = (left as f32 * per_item).ceil() as u32;
        return rsx! {
            crate::ui::Card { class: "catchup", "data-state": "running",
                style { {CSS.text()} }
                h2 { {tx::RUNNING_TITLE} }
                p { {tx::running_line(day_n, p.days)} }
                p { class: "small",
                    if left > 0 { {tx::today_left(left, minutes)} } else { {tx::TODAY_DONE} }
                }
                if p.pause_new {
                    p { class: "muted small", {tx::NEW_PAUSED} }
                }
                button { class: "btn btn-ghost", onclick: move |_| save(None), {tx::STOP} }
            }
        };
    }
    if !is_backlog(due, usual(&per_day)) {
        return rsx! {};
    }
    let d = days();
    let per_day_minutes = due_minutes.div_ceil(d);
    rsx! {
        crate::ui::Card { class: "catchup", "data-state": "offer",
            style { {CSS.text()} }
            h2 { {tx::OFFER_TITLE} }
            p { {tx::offer_line(due, due_minutes)} }
            div { class: "catchup-days", role: "radiogroup", aria_label: tx::DAYS_LABEL,
                span { class: "muted small", {tx::DAYS_LABEL} }
                div { class: "chips",
                    for n in DAY_CHOICES {
                        button {
                            key: "{n}",
                            class: "chip toggle",
                            role: "radio",
                            "aria-checked": d == n,
                            onclick: move |_| days.set(n),
                            {tx::days(n)}
                        }
                    }
                }
            }
            label { class: "catchup-pause",
                input { r#type: "checkbox", checked: pause(), onchange: move |e| pause.set(e.checked()) }
                span { {tx::PAUSE_NEW} }
            }
            p { class: "small", {tx::per_day(per_day_minutes)} }
            button {
                class: "btn btn-primary",
                onclick: move |_| {
                    save(Some(Plan { start_day: today, days: d, backlog: due as u32, pause_new: pause() }))
                },
                {tx::START}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backlog_is_twice_the_usual_day_and_never_small() {
        assert_eq!(usual(&[10, 20, 30, 999]), 20.0, "today is left out");
        assert_eq!(usual(&[5]), 0.0);
        assert!(!is_backlog(30, 0.0));
        assert!(is_backlog(31, 0.0));
        assert!(!is_backlog(80, 40.0));
        assert!(is_backlog(81, 40.0));
    }

    #[test]
    fn a_plan_spreads_the_backlog_and_ends_on_time() {
        let p = Plan { start_day: 100, days: 3, backlog: 100, pause_new: true };
        assert_eq!(p.share(), 34);
        assert!(!p.running(99) && p.running(100) && p.running(102) && !p.running(103));
        assert_eq!(p.left_today(0), 34);
        assert_eq!(p.left_today(30), 4);
        assert_eq!(p.left_today(50), 0);
        let none = Plan { start_day: 0, days: 0, backlog: 0, pause_new: false };
        assert!(!none.running(0), "the cleared plan never runs");
        assert_eq!(none.share(), 0);
    }

    #[test]
    fn the_running_plan_caps_reviews_and_can_pause_new_items() {
        let mut dom = VirtualDom::new(|| {
            use_hook(|| save(Some(Plan { start_day: 7, days: 2, backlog: 5, pause_new: true })));
            rsx! {}
        });
        dom.rebuild_in_place();
        let due: Vec<String> = (0..5).map(|i| format!("x.{i}")).collect();
        dom.in_runtime(|| {
            assert_eq!(share_of(7, 0, due.clone()).len(), 3);
            assert_eq!(share_of(7, 2, due.clone()).len(), 1);
            assert_eq!(share_of(9, 0, due.clone()).len(), 5, "an ended plan cuts nothing");
            assert_eq!(new_left(7, 10), 0);
            assert_eq!(new_left(9, 10), 10);
            save(None);
            assert_eq!(new_left(7, 10), 10);
        });
    }

    #[derive(Props, Clone, PartialEq)]
    struct CardProps {
        due: usize,
        plan: Option<Plan>,
    }

    #[allow(non_snake_case)]
    fn Card(p: CardProps) -> Element {
        crate::store::use_provide_review();
        let today = crate::store::use_review().backend.borrow().study_day(crate::store::now_ms());
        use_hook(move || save(p.plan.map(|plan| Plan { start_day: today, ..plan })));
        rsx! {
            crate::browser_fixture::Shell { screen: crate::Screen::Today, focus: false,
                CatchUpCard { due: p.due, due_minutes: 40 }
            }
        }
    }

    fn render(due: usize, plan: Option<Plan>) -> String {
        let mut dom = VirtualDom::new_with_props(Card, CardProps { due, plan });
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    #[test]
    fn today_offers_a_plan_only_for_a_real_backlog() {
        assert!(!render(20, None).contains("catchup"), "a small pile needs no plan");
        let html = render(120, None);
        assert!(html.contains(r#"data-state="offer""#), "{html}");
        assert!(html.contains(tx::OFFER_TITLE));
        assert_eq!(html.matches(r#"role="radio""#).count(), DAY_CHOICES.len());
        assert!(html.contains("aria-checked=true>3"), "the default is three days");
        assert!(html.contains(&tx::per_day(40u32.div_ceil(DEFAULT_DAYS))));
    }

    #[test]
    fn a_running_plan_shows_the_day_and_what_is_left() {
        let plan = Plan { start_day: 0, days: 3, backlog: 90, pause_new: true };
        let html = render(90, Some(plan));
        assert!(html.contains(r#"data-state="running""#), "{html}");
        assert!(html.contains(&tx::running_line(1, 3)));
        assert!(html.contains(tx::NEW_PAUSED));
        assert!(!html.contains(tx::OFFER_TITLE));
    }

    #[test]
    fn the_plan_round_trips_as_a_synced_setting() {
        let plan = Plan { start_day: -5, days: 2, backlog: 40, pause_new: false };
        assert!(crate::store::save_setting(KEY, &plan));
        assert_eq!(crate::store::shared().setting::<Plan>(KEY), Some(plan));
    }
}
