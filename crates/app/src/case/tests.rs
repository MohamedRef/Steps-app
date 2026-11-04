//! Existing behavior regressions for the feature facade.

use super::*;

thread_local! {
    /// Cases a test thread adds beside the bundled content: the case list and
    /// player find them like any other.
    static EXTRA: std::cell::RefCell<Vec<(&'static str, &'static Item)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

thread_local! {
    /// Leaves the bundled cases out of the list for this test thread, so the
    /// empty state can still be seen now that the content has cases.
    static HIDE_BUNDLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Hides the bundled cases from the list on this test thread.
pub(crate) fn hide_bundled() {
    HIDE_BUNDLED.with(|h| h.set(true));
}

pub(super) fn bundled_hidden() -> bool {
    HIDE_BUNDLED.with(std::cell::Cell::get)
}

/// Adds `item` under the lecture title `lecture`, for this test thread.
pub(crate) fn add_case(lecture: &'static str, item: Item) {
    let item: &'static Item = Box::leak(Box::new(item));
    EXTRA.with(|e| e.borrow_mut().push((lecture, item)));
}

pub(super) fn extra() -> impl Iterator<Item = (&'static str, &'static Item)> {
    EXTRA.with(|e| e.borrow().clone()).into_iter()
}

#[test]
fn grades_from_steps_right() {
    assert_eq!(grade_for(3, 3), Grade::Good);
    assert_eq!(grade_for(2, 3), Grade::Hard);
    assert_eq!(grade_for(1, 2), Grade::Hard);
    assert_eq!(grade_for(1, 3), Grade::Again);
    assert_eq!(grade_for(0, 0), Grade::Again);
}

#[test]
fn options_are_marked_after_a_pick() {
    assert_eq!(option_state(0, None, None, Some(1)), "");
    assert_eq!(option_state(0, None, Some(0), Some(1)), "selected");
    assert_eq!(option_state(1, Some(1), None, Some(1)), "correct");
    assert_eq!(option_state(1, Some(0), None, Some(1)), "missed");
    assert_eq!(option_state(0, Some(0), None, Some(1)), "wrong");
    assert_eq!(option_state(2, Some(0), None, Some(1)), "dim");
}

fn opts(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|x| (*x).to_owned()).collect()
}

// Issue #86: the key was "A" in 61% of steps because options were
// never shuffled.
#[test]
fn options_are_shuffled_per_attempt() {
    let o = opts(&["Aspirin", "Alteplase", "PFO closure", "Warfarin", "Heparin"]);
    let a = option_order(&o, 1, 0);
    assert_eq!(a, option_order(&o, 1, 0), "steady while the step is open");
    let mut sorted = a.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, [0, 1, 2, 3, 4]);
    // Across attempts the written first option lands in every position.
    let firsts: std::collections::BTreeSet<usize> =
        (0..200u64).map(|seed| option_order(&o, seed, 0).iter().position(|&i| i == 0).unwrap()).collect();
    assert_eq!(firsts.len(), 5);
    // "All of the above" steps keep their written order.
    let fixed = opts(&["a", "b", "c", "All of the above"]);
    assert_eq!(option_order(&fixed, 7, 0), [0, 1, 2, 3]);
}

#[test]
fn a_step_is_selected_then_checked_once() {
    let mut r = CaseRun::new("cvs.L01.901");
    // Nothing selected: Check does nothing.
    assert!(!r.check(Some(1)));
    r.select(0);
    r.select(1);
    assert_eq!((r.draft, r.results.len()), (Some(1), 0));
    assert!(r.check(Some(1)));
    assert_eq!((r.picked, r.results.clone()), (Some(1), vec![(true, Some(1))]));
    // Locked: a second Check or a new selection changes nothing.
    assert!(!r.check(Some(1)));
    r.select(2);
    assert_eq!((r.picked, r.draft, r.results.len()), (Some(1), None, 1));
    r.next();
    assert_eq!((r.step, r.picked, r.draft), (1, None, None));
    // Next without an answer stays put.
    r.next();
    assert_eq!(r.step, 1);
    r.grade_self(false);
    r.grade_self(true);
    assert_eq!(r.results, vec![(true, Some(1)), (false, None)]);
}

#[test]
fn record_keeps_the_case_as_one_review() {
    record(None, "cvs.L01.901", 2, 3, now_ms() - 1500).unwrap();
    let sink = EVENTS.lock().unwrap();
    let e = sink.events.iter().find(|e| e.item_id == "cvs.L01.901").unwrap();
    assert_eq!(e.grade, Grade::Hard);
    assert_eq!(e.origin, Origin::Case);
    assert!(e.took_ms >= 1500);
}

#[test]
fn failed_finish_keeps_the_answer_and_success_records_once() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use steps_core::review::StudyStore;

    struct Fallible {
        blocked: Rc<Cell<bool>>,
        events: Rc<RefCell<Vec<ReviewEvent>>>,
    }
    impl StudyStore for Fallible {
        fn record(&mut self, _: ReviewEvent) {
            panic!("the case must use the fallible store API");
        }
        fn record_logged(&mut self, event: ReviewEvent) -> Result<Option<String>, String> {
            if self.blocked.get() {
                return Err("read only".into());
            }
            self.events.borrow_mut().push(event);
            Ok(Some("logged-case".into()))
        }
        fn due(&self, _: i64) -> Vec<String> {
            Vec::new()
        }
        fn is_new(&self, _: &str) -> bool {
            true
        }
        fn is_hidden(&self, _: &str) -> bool {
            false
        }
        fn next_due_ms(&self, _: i64) -> Option<i64> {
            None
        }
    }
    let blocked = Rc::new(Cell::new(true));
    let events = Rc::new(RefCell::new(Vec::new()));
    let store = StudyStoreHandle::new(Fallible { blocked: blocked.clone(), events: events.clone() });
    let mut run = CaseRun::new("cvs.L01.901");
    run.grade_self(true);
    run.next_logged(2, Some(&store)).unwrap();
    assert_eq!(run.step, 1);
    assert!(events.borrow().is_empty(), "unfinished cases are not reviews");
    run.select(1);
    assert!(run.check(Some(1)));
    let before = run.clone();
    assert!(run.next_logged(2, Some(&store)).is_err());
    assert_eq!(run, before, "failed persistence retains the stage, result and selected answer");
    assert!(events.borrow().is_empty());

    blocked.set(false);
    run.next_logged(2, Some(&store)).unwrap();
    assert_eq!(run.step, 2);
    run.next_logged(2, Some(&store)).unwrap();
    let events = events.borrow();
    assert_eq!(events.len(), 1, "repeated Finish cannot append a second review");
    assert_eq!(events[0].grade, Grade::Good);
    assert_eq!(events[0].origin, Origin::Case);
}

#[test]
fn items_are_found_in_the_library_by_id() {
    let (lecture, item) = find_item("cvs.L01.001").expect("a written card");
    assert_eq!((lecture, item.id.as_str()), ("Embryology", "cvs.L01.001"));
    assert!(find_item("cvs.L01.999").is_none());
    assert!(find_item("zz.L01.001").is_none());
    assert!(find_item("nodot").is_none());
}

#[test]
fn a_long_stem_is_cut_to_one_line_and_other_kinds_have_none() {
    let long: Item = serde_yaml_ng::from_str(&format!(
        "{{ id: cvs.L01.903, kind: case, stem: '{}', steps: [{{ question: q, why: w }}], src: {{ page: 284 }}, status: unverified }}",
        "A newborn is cyanotic. ".repeat(8)
    ))
    .unwrap();
    let line = first_line(&long);
    assert_eq!(line.chars().count(), 121);
    assert!(line.ends_with('…'));
    let (_, card) = find_item("cvs.L01.001").unwrap();
    assert_eq!(first_line(card), "");
}
