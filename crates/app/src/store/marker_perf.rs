//! What saving a Study step costs as the review log grows: reading the
//! checkpoint marker (the v2 revision and, for comparison, the v1 whole-log
//! fingerprint) and writing the checkpoint. Informational, never a timing
//! gate: a hosted runner is not Mohamed's phone.
//! Run: cargo test -p steps-app --no-default-features store::marker_perf -- --ignored --nocapture
//! Adapted from batna-app main: crates/app/src/store/marker_perf.rs.

use std::hint::black_box;
use std::time::Instant;

use super::*;

/// Median, 95th percentile and slowest, in microseconds.
pub(crate) fn spread(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let last = samples.len() - 1;
    (samples[samples.len() / 2], samples[(last * 95).div_ceil(100)], samples[last])
}

#[test]
#[ignore = "informational cost baseline for CI's summary, not a timing gate"]
fn profile_study_checkpoint_marker() {
    let db = Store::open_in_memory().expect("an in-memory store");
    let mut inserted = 0;
    for count in [100_usize, 1_000, 3_000] {
        for n in inserted..count {
            let event = ReviewEvent::new(
                format!("cvs.L04.profile.{}", n % 64),
                Grade::Good,
                Origin::Study,
                1_800_000_000_000 + n as i64,
                1_200,
            );
            db.append_review(event).expect("seed answers");
        }
        inserted = count;
        let (mut marker, mut legacy, mut write) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..7 {
            let t = Instant::now();
            let m = black_box(db.study_review_marker().expect("marker"));
            marker.push(t.elapsed().as_micros());
            let t = Instant::now();
            black_box(db.study_review_marker_legacy().expect("legacy marker"));
            legacy.push(t.elapsed().as_micros());
            let t = Instant::now();
            db.put_local_setting("benchmark.study.marker", &m).expect("local write");
            write.push(t.elapsed().as_micros());
        }
        let (m50, m95, mmax) = spread(&mut marker);
        let (l50, l95, lmax) = spread(&mut legacy);
        let (w50, w95, wmax) = spread(&mut write);
        println!(
            "STUDY_CHECKPOINT_MARKER_BASELINE events={count} marker_p50_us={m50} marker_p95_us={m95} marker_max_us={mmax} legacy_p50_us={l50} legacy_p95_us={l95} legacy_max_us={lmax} local_write_p50_us={w50} local_write_p95_us={w95} local_write_max_us={wmax}"
        );
    }
}
