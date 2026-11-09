//! What one Study grade costs in the review backend as the log grows: the
//! FSRS replay of every answer, and the whole grade path (save the answer,
//! add it to the log, replay, tell the screens). Not the checkpoint marker
//! (`store::marker_perf`) and not the WebView's paint. Informational, never
//! a timing gate.
//! Run: cargo test -p steps-app --no-default-features store::backend::replay_perf -- --ignored --nocapture
//! Adapted from batna-app main: crates/app/src/store/backend/replay_perf.rs.

use std::hint::black_box;
use std::time::Instant;

use steps_core::review::{Grade, Origin};

use super::super::marker_perf::spread;
use super::*;

fn answer(n: usize) -> ReviewEvent {
    let grade = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy][n % 4];
    ReviewEvent::new(
        format!("cvs.L04.replay.{}", n % 64),
        grade,
        Origin::Study,
        1_793_610_000_000 + n as i64 * 60_000,
        1_200,
    )
}

#[test]
#[ignore = "informational cost baseline for CI's summary, not a timing gate"]
fn profile_review_replay() {
    for count in [100_usize, 1_000, 3_000] {
        let db: &'static Store = Box::leak(Box::new(Store::open_in_memory().unwrap()));
        for n in 0..count {
            db.append_review(answer(n)).expect("seed answers");
        }
        let mut backend = ReviewBackend::new(db, Catalog::new());
        assert_eq!(backend.reviews_logged(), count);
        let mut replay = Vec::new();
        for _ in 0..7 {
            let t = Instant::now();
            backend.replay();
            replay.push(t.elapsed().as_micros());
            black_box(backend.states.len());
        }
        let (p50, p95, max) = spread(&mut replay);

        let t = Instant::now();
        let id = StudyStore::record_logged(&mut backend, answer(count)).expect("saved").expect("an id");
        let grade_us = t.elapsed().as_micros();
        assert_eq!(backend.reviews_logged(), count + 1);
        assert!(db.reviews().unwrap().iter().any(|r| r.id == id));
        println!(
            "STUDY_REPLAY_BASELINE events={count} replay_p50_us={p50} replay_p95_us={p95} replay_max_us={max} grade_store_replay_us={grade_us}"
        );
    }
}
