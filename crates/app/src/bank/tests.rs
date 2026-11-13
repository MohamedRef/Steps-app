use super::*;

#[test]
fn dates_and_minutes() {
    assert_eq!(date(0), "1970-01-01");
    assert_eq!(date(1_793_059_200_000), "2026-10-27");
    assert_eq!(minutes(48 * 60_000), 48);
    assert_eq!(minutes(1), 1);
}

fn outcome(at_ms: i64, right: u32) -> Outcome {
    Outcome { total: exam::Score { right, total: 4 }, ..exam::score(&[], &[], Mode::Mock, at_ms, 0) }
}

#[test]
fn papers_from_the_legacy_list_and_per_paper_keys_merge_once() {
    let merged = merge_papers(vec![outcome(30, 1), outcome(10, 2), outcome(30, 1), outcome(20, 3)]);
    assert_eq!(merged.iter().map(|o| o.at_ms).collect::<Vec<_>>(), vec![10, 20, 30]);
}

#[test]
fn an_open_paper_round_trips_through_json() {
    let q = Question { id: "x".into(), lecture: "cvs.L04".into(), topic: None, answer: 0, order: vec![1, 0] };
    let mut p = Paper::new(Mode::Mock, vec![q.clone(), q]);
    p.answers[1].chosen = Some(0);
    p.answers[0].flagged = true;
    p.at = 1;
    let json = serde_json::to_string(&Some(p.clone())).unwrap();
    assert_eq!(serde_json::from_str::<Option<Paper>>(&json).unwrap(), Some(p));
    assert_eq!(serde_json::from_str::<Option<Paper>>("null").unwrap(), None);
}

#[test]
fn question_counts_read_naturally() {
    use crate::i18n::bank::questions;
    assert_eq!(questions(1), "سؤال واحد");
    assert_eq!(questions(2), "سؤالين");
    assert_eq!(questions(7), "7 أسئلة");
    assert_eq!(questions(34), "34 سؤال");
}

#[test]
fn paper_time_adds_up_per_question() {
    let q = Question { id: "x".into(), lecture: "cvs.L04".into(), topic: None, answer: 0, order: vec![0, 1] };
    let mut p = Paper::new(Mode::Timed, vec![q.clone(), q]);
    p.shown_ms = 1_000;
    p.leave(4_000);
    p.at = 1;
    p.leave(5_000);
    p.at = 0;
    p.leave(6_000);
    assert_eq!(p.answers[0].took_ms, 4_000);
    assert_eq!(p.answers[1].took_ms, 1_000);
}

fn question(id: &str) -> Question {
    Question { id: id.into(), lecture: "cvs.L04".into(), topic: None, answer: 1, order: vec![0, 1, 2] }
}

#[test]
fn tutor_selects_then_checks_once() {
    let mut p = Paper::new(Mode::Tutor, vec![question("a"), question("b")]);
    p.shown_ms = 0;
    // Nothing selected: nothing to check.
    assert_eq!(p.check(10), None);
    // A selection is a draft he can change; nothing is graded yet.
    p.pick(0);
    p.pick(1);
    assert_eq!((p.draft, p.answers[0].chosen), (Some(1), None));
    let (q, a) = p.check(2_000).expect("the draft is checked");
    assert_eq!((q.id.as_str(), a.chosen, a.took_ms), ("a", Some(1), 2_000));
    // A second Check, and a later pick, change nothing.
    assert_eq!(p.check(3_000), None);
    p.pick(2);
    assert_eq!((p.draft, p.answers[0].chosen), (None, Some(1)));
}

#[test]
fn moving_on_drops_an_unchecked_draft() {
    let mut p = Paper::new(Mode::Tutor, vec![question("a"), question("b")]);
    p.pick(2);
    p.leave(5);
    p.at = 1;
    p.draft = None;
    assert_eq!(p.check(6), None);
    assert!(p.answers.iter().all(|a| a.chosen.is_none()));
}

#[test]
fn timed_papers_take_the_pick_as_the_answer() {
    let mut p = Paper::new(Mode::Mock, vec![question("a")]);
    p.pick(0);
    p.pick(2);
    assert_eq!((p.draft, p.answers[0].chosen), (None, Some(2)));
    assert_eq!(p.check(1), None, "timed papers are never checked one by one");
}

#[test]
fn confidence_goes_on_the_answer_until_it_is_checked() {
    let mut p = Paper::new(Mode::Tutor, vec![question("a")]);
    p.set_confidence(Some(Confidence::Sure));
    p.pick(0);
    let (_, a) = p.check(5).unwrap();
    assert_eq!(a.confidence, Some(Confidence::Sure));
    // Once the answer is out, it stays what he said before.
    p.set_confidence(Some(Confidence::Guess));
    assert_eq!(p.answers[0].confidence, Some(Confidence::Sure));
    // A timed paper takes it while it runs.
    let mut t = Paper::new(Mode::Mock, vec![question("a")]);
    t.pick(2);
    t.set_confidence(Some(Confidence::Unsure));
    assert_eq!(t.answers[0].confidence, Some(Confidence::Unsure));
}

#[test]
fn a_reason_replaces_the_logged_answer_at_its_first_time() {
    let mut p = Paper::new(Mode::Tutor, vec![question("a"), question("b")]);
    p.pick(0);
    p.check(5).unwrap();
    p.logged = Some(Logged { at: 0, event_id: "e1".into(), at_ms: 5 });
    let (event, old) = p.set_reason(ErrorReason::Misread).unwrap();
    assert_eq!((event.grade, event.reason, event.at_ms), (Grade::Again, Some(ErrorReason::Misread), 5));
    assert_eq!(old.map(|l| l.event_id).as_deref(), Some("e1"));
    // The same chip again takes the reason back.
    p.logged = Some(Logged { at: 0, event_id: "e2".into(), at_ms: 5 });
    let (event, _) = p.set_reason(ErrorReason::Misread).unwrap();
    assert_eq!(event.reason, None);
    assert_eq!(p.answers[0].reason, None);
    // No reason on a right answer, or before checking.
    p.at = 1;
    assert!(p.set_reason(ErrorReason::Timing).is_none());
    p.pick(1);
    p.check(9).unwrap();
    assert!(p.set_reason(ErrorReason::Timing).is_none());
}

#[test]
fn papers_saved_before_drafts_still_load() {
    let p = Paper::new(Mode::Timed, vec![question("a")]);
    let mut json: serde_json::Value = serde_json::to_value(&p).unwrap();
    json.as_object_mut().unwrap().remove("draft");
    assert_eq!(serde_json::from_value::<Paper>(json).unwrap(), p);
}

/// Issue #673: written keys lean to "A", so every screen and export that
/// shows an MCQ or a case step must take its order from
/// `steps_core::exam::option_order`. Papers are built by `exam::practice` and
/// `exam::mock` only (no `Question { .. }` written by hand outside tests), the
/// question card follows `q.order`, the case player and the Anki export call
/// `option_order`, and the one other view of an MCQ's options is the ✎ editor,
/// which edits them in written order on purpose.
#[test]
fn every_mcq_screen_takes_the_shuffled_order() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let mut files = Vec::new();
    let mut dirs = vec![root.join("src")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                if name != "render_tests" {
                    dirs.push(p);
                }
            } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
                files.push(p);
            }
        }
    }
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        assert!(
            !text.contains("Question {"),
            "{rel} builds a paper question by hand; use exam::practice or exam::mock"
        );
        let shows_mcq_options =
            text.contains("Body::Mcq {") && text.contains("options") && text.contains("rsx!");
        assert!(
            !shows_mcq_options || ["src/bank/question.rs", "src/verify/mod.rs"].contains(&rel.as_str()),
            "{rel} shows MCQ options; show them in exam::option_order"
        );
    }
    assert!(read("src/bank/question.rs").contains("q.order.iter()"));
    assert!(read("src/case/player.rs").contains("option_order("));
    assert!(read("src/case/choices.rs").contains("exam::option_order("));
    assert!(read("../anki/src/lib.rs").contains("let order = option_order(id, opts);"));
}
