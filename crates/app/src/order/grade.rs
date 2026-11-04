//! Pure logic for `order` items: the starting shuffle, which steps are out of
//! place, and the grade to suggest. Steps are named by their index in the
//! correct order, so the right answer is always `0, 1, 2, …`.

use steps_core::review::Grade;

/// The order the steps start in: a shuffle seeded by the item id, so an item
/// always opens the same way. It never starts solved, and at most a third of
/// the steps start at their own number, so it never starts nearly solved.
pub fn shuffled(n: usize, seed: &str) -> Vec<usize> {
    // FNV-1a, then xorshift64: small, stable across platforms and versions.
    let mut state =
        seed.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut out: Vec<usize> = (0..n).collect();
    // Fisher-Yates, redrawn until few enough steps sit at their own number.
    // Any n ≥ 2 passes within a few draws; the cap only bounds the loop.
    for _ in 0..64 {
        for i in (1..n).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            out.swap(i, j);
        }
        if scrambled_enough(&out) {
            return out;
        }
    }
    // A rotation has no step at its own number.
    out.sort_unstable();
    if n > 1 {
        out.rotate_left(1);
    }
    out
}

fn scrambled_enough(order: &[usize]) -> bool {
    let n = order.len();
    let fixed = order.iter().enumerate().filter(|&(pos, &step)| pos == step).count();
    n < 2 || (fixed < n && fixed * 3 <= n)
}

/// Positions in `answer` whose step is not at its own number. Only these
/// get "its place is N"; every other step is exactly where the book has it.
pub fn misplaced(answer: &[usize]) -> Vec<usize> {
    answer.iter().enumerate().filter(|&(pos, &step)| pos != step).map(|(pos, _)| pos).collect()
}

/// The fewest steps to move to put `answer` right: everything outside the
/// longest run already in the right relative order. One step put first
/// shifts every other step off its number but is still one slip, so this,
/// not [`misplaced`], drives the suggested grade.
pub fn slips(answer: &[usize]) -> usize {
    answer.len() - longest_increasing(answer).len()
}

/// Positions of one longest strictly increasing subsequence (patience sort,
/// O(n log n)). Ties go to the earliest run, so the result is stable.
fn longest_increasing(values: &[usize]) -> Vec<usize> {
    // tails[k] = position of the smallest tail of an increasing run of length k + 1.
    let mut tails: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; values.len()];
    for (pos, &v) in values.iter().enumerate() {
        let k = tails.partition_point(|&t| values[t] < v);
        prev[pos] = k.checked_sub(1).map(|k| tails[k]);
        if k == tails.len() {
            tails.push(pos);
        } else {
            tails[k] = pos;
        }
    }
    let mut out = Vec::with_capacity(tails.len());
    let mut at = tails.last().copied();
    while let Some(pos) = at {
        out.push(pos);
        at = prev[pos];
    }
    out.reverse();
    out
}

/// The grade to suggest from [`slips`]; the learner can still pick another.
pub fn suggest(slips: usize) -> Grade {
    match slips {
        0 => Grade::Good,
        1 => Grade::Hard,
        _ => Grade::Again,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_is_stable_complete_and_never_solved() {
        for n in 0..9 {
            let a = shuffled(n, "hb.L01.008");
            assert_eq!(a, shuffled(n, "hb.L01.008"));
            let mut sorted = a.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..n).collect::<Vec<_>>());
            if n > 1 {
                assert!(!misplaced(&a).is_empty(), "n={n} starts solved: {a:?}");
            }
        }
        assert_ne!(shuffled(6, "hb.L01.008"), shuffled(6, "hb.L03.040"));
    }

    #[test]
    fn shuffle_never_starts_nearly_solved() {
        // Many seeds and sizes: at most a third of the steps at their own number.
        for n in 2..10 {
            for k in 0..200 {
                let a = shuffled(n, &format!("hb.L{k:02}.{n:03}"));
                let fixed = a.iter().enumerate().filter(|&(p, &s)| p == s).count();
                assert!(fixed * 3 <= n, "n={n} seed {k}: {fixed} in place in {a:?}");
            }
        }
    }

    #[test]
    fn a_step_is_in_place_only_at_its_own_number() {
        // The case from issue #173: none of the five is at its number, though
        // 2 and 3 are in the right order relative to each other.
        assert_eq!(misplaced(&[3, 2, 4, 1, 0]), vec![0, 1, 2, 3, 4]);
        // The last step put first: every step is off its number...
        assert_eq!(misplaced(&[4, 0, 1, 2, 3]), vec![0, 1, 2, 3, 4]);
        assert_eq!(misplaced(&[0, 2, 1, 3]), vec![1, 2]);
        assert_eq!(misplaced(&[0, 1, 2, 3]), Vec::<usize>::new());
        assert_eq!(misplaced(&[]), Vec::<usize>::new());
    }

    #[test]
    fn slips_count_the_fewest_moves() {
        // ...but moving one step fixes it.
        assert_eq!(slips(&[4, 0, 1, 2, 3]), 1);
        assert_eq!(slips(&[0, 2, 1, 3]), 1);
        assert_eq!(slips(&[0, 1, 2, 3]), 0);
        assert_eq!(slips(&[3, 2, 1, 0]), 3);
        assert_eq!(slips(&[3, 2, 4, 1, 0]), 3);
        assert_eq!(slips(&[]), 0);
    }

    #[test]
    fn suggestion_follows_the_number_of_slips() {
        assert_eq!(suggest(0), Grade::Good);
        assert_eq!(suggest(1), Grade::Hard);
        assert_eq!(suggest(3), Grade::Again);
    }
}
