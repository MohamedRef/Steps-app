//! Demo numbers for previews and screenshots (STEPS_STATS_DEMO=1): a few
//! weeks of made-up answers on the first lectures. Never Mohamed's data, and
//! the screen says so.

use std::collections::BTreeMap;

use steps_core::content::{Lecture, Status};
use steps_core::review::{Grade, Origin, ReviewEvent};

const DAY_MS: i64 = 86_400_000;

/// A small deterministic hash, so the demo looks the same on every run.
fn mix(a: usize, b: usize) -> u32 {
    let mut x = (a as u32).wrapping_mul(0x9E37_79B9) ^ (b as u32).wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

pub fn events(lectures: &[Lecture], now_ms: i64) -> Vec<ReviewEvent> {
    let mut out = Vec::new();
    // Lecture 1 mostly done, lecture 2 half, lecture 3 started.
    for (l, share) in lectures.iter().zip([90, 55, 20]) {
        for (n, item) in l.items.iter().enumerate() {
            let h = mix(n, l.items.len());
            if h % 100 >= share {
                continue;
            }
            // Topics further into the lecture are made a little weaker.
            let topic_rank =
                item.topic.as_deref().and_then(|t| l.topics.iter().position(|x| x.id == t)).unwrap_or(0);
            let miss_odds = 8 + 6 * topic_rank as u32 % 30;
            let first = 20 - (h % 18) as i64;
            for (k, back) in [first, first - 3, first / 3, 1].into_iter().enumerate() {
                if back < 0 || (k == 3 && !h.is_multiple_of(3)) {
                    continue;
                }
                let wrong = mix(n, k + 7) % 100 < miss_odds;
                out.push(ReviewEvent {
                    item_id: item.id.clone(),
                    grade: if wrong { Grade::Again } else { Grade::Good },
                    origin: Origin::Review,
                    at_ms: now_ms - back * DAY_MS - 3_600_000,
                    took_ms: 8_000,
                });
            }
        }
    }
    out
}

pub fn marks() -> BTreeMap<String, Status> {
    (1..=30).map(|n| (format!("cvs.L04.{n:03}"), Status::Approved)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_is_the_same_every_run_and_only_in_the_past() {
        let lib = crate::content::Library::get();
        let lectures: Vec<Lecture> =
            lib.index.lectures.iter().filter_map(|e| lib.lectures.get(&e.id).cloned()).collect();
        let now = 20_400 * DAY_MS;
        let a = events(&lectures, now);
        assert_eq!(a, events(&lectures, now), "deterministic");
        assert!(!a.is_empty(), "the first lecture gets answers");
        assert!(a.iter().all(|e| e.at_ms < now && e.at_ms > now - 21 * DAY_MS));
        // Only the first three lectures, and only their own items.
        let first: Vec<&str> = lectures.iter().take(3).map(|l| l.lecture.as_str()).collect();
        assert!(a.iter().all(|e| first.iter().any(|l| e.item_id.starts_with(&format!("{l}.")))));
        // Mostly remembered, some missed.
        let again = a.iter().filter(|e| e.grade == Grade::Again).count();
        assert!(again < a.len() / 2, "{again} of {}", a.len());
        assert!(events(&[], now).is_empty());
    }

    #[test]
    fn demo_marks_approve_thirty_cardiovascular_items() {
        let m = marks();
        assert_eq!(m.len(), 30);
        assert_eq!(m.get("cvs.L04.001"), Some(&Status::Approved));
        assert_eq!(m.keys().last().map(String::as_str), Some("cvs.L04.030"));
        assert_ne!(mix(1, 2), mix(2, 1));
    }
}
