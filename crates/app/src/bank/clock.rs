//! The clock in a paper's top bar: what is left of a timed paper's time,
//! or in tutor mode and in review the time on the question on screen. Only
//! this chip redraws as it ticks, once a second; the paper's drain bar
//! (`player::Timer`) is what hands a timed paper in when its time is up.

use std::time::Duration;

use dioxus::prelude::*;
use steps_core::exam;

use super::now_ms;
use crate::i18n::bank as tx;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clock {
    /// A timed paper of `n` questions started at `started_ms`.
    Left { n: usize, started_ms: i64 },
    /// The time on the question: `took_ms` so far, ticking on from
    /// `since_ms` while it is still open.
    On { took_ms: u64, since_ms: Option<i64> },
}

/// `ms` as `m:ss` (`h:mm:ss` from an hour).
pub fn face(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

#[component]
pub fn ClockChip(clock: Clock) -> Element {
    match clock {
        Clock::On { took_ms, since_ms: None } => rsx! { Face { left: false, ms: took_ms } },
        _ => rsx! { Ticking { clock } },
    }
}

#[component]
fn Ticking(clock: Clock) -> Element {
    let mut now = use_signal(now_ms);
    use_future(move || async move {
        loop {
            crate::platform::time::sleep(Duration::from_secs(1)).await;
            now.set(now_ms());
        }
    });
    let now = now();
    let (left, ms) = match clock {
        // Rounded up, so it shows 0:00 only when the time is out.
        Clock::Left { n, started_ms } => (true, exam::remaining_ms(n, started_ms, now).div_ceil(1000) * 1000),
        Clock::On { took_ms, since_ms } => {
            (false, took_ms + since_ms.map_or(0, |s| u64::try_from(now.saturating_sub(s)).unwrap_or(0)))
        }
    };
    rsx! { Face { left, ms } }
}

#[component]
fn Face(left: bool, ms: u64) -> Element {
    let label = if left { tx::time_left(ms) } else { tx::time_on(ms) };
    rsx! {
        span { class: "exam-clock", "data-kind": if left { "left" } else { "on" }, role: "timer", aria_label: "{label}",
            span { class: "exam-clock-icon", aria_hidden: "true", {crate::icons::clock()} }
            if !left {
                span { class: "exam-clock-word", aria_hidden: "true", {tx::ON_QUESTION} }
            }
            bdi { class: "num-ltr", aria_hidden: "true", {face(ms)} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::face;

    #[test]
    fn the_clock_reads_minutes_and_seconds() {
        assert_eq!(face(0), "0:00");
        assert_eq!(face(72_400), "1:12");
        assert_eq!(face(1_800_000), "30:00");
        assert_eq!(face(3_725_000), "1:02:05");
    }

    #[test]
    fn the_clock_says_its_time_in_words() {
        use crate::i18n::bank::{time_left, time_on};
        assert_eq!(time_left(1_122_000), "الوقت الباقي 18 دقيقة و42 ثانية");
        assert_eq!(time_left(120_000), "الوقت الباقي دقيقتين");
        assert_eq!(time_on(72_000), "الوقت على السؤال ده دقيقة و12 ثانية");
        assert_eq!(time_on(5_000), "الوقت على السؤال ده 5 ثواني");
    }
}
