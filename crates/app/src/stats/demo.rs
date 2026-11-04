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
                    confidence: None,
                    reason: None,
                });
            }
        }
    }
    out
}

pub fn marks() -> BTreeMap<String, Status> {
    (1..=30).map(|n| (format!("hb.L01.{n:03}"), Status::Approved)).collect()
}
