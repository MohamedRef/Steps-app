//! Choices for clinical-case steps, kept separate from case rendering.

use steps_core::exam::{self, Rng};

/// The order a step's options are shown in, as written indices: the bank's
/// rule (`exam::option_order`), seeded per attempt (`seed`) so the key isn't
/// always "A". Numeric options go smallest first, and steps whose options
/// refer to each other ("All of these", "A and C") keep their order.
pub(super) fn option_order(options: &[String], seed: u64, step: usize) -> Vec<usize> {
    exam::option_order(options, &mut Rng::new(seed ^ (step as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// "A"–"E": the key that picks the option shown at `pos`.
pub(super) fn option_key(pos: usize) -> &'static str {
    ["A", "B", "C", "D", "E", "F"].get(pos).copied().unwrap_or("")
}

/// The shared `button.answer` state (docs/DESIGN.md) of option `i`; "dim"
/// is an option neither picked nor right, once checked.
pub(super) fn option_state(
    i: usize,
    picked: Option<usize>,
    draft: Option<usize>,
    answer: Option<usize>,
) -> &'static str {
    match picked {
        None if draft == Some(i) => "selected",
        None => "",
        Some(p) if answer == Some(i) && p == i => "correct",
        Some(_) if answer == Some(i) => "missed",
        Some(p) if p == i => "wrong",
        Some(_) => "dim",
    }
}
