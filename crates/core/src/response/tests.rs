use super::*;
use crate::content::CaseStep;
use crate::lesson::{MnemonicPart, SortCard, Step};

fn step(body: StepBody) -> Step {
    Step { body, page: None, quote: None }
}

fn check() -> StepBody {
    StepBody::Check {
        question: "Which marker is most specific for myocardial injury?".into(),
        options: vec!["CK-MB".into(), "Troponin I".into(), "LDH".into()],
        answer: 1,
        why: vec!["".into(), "".into(), "".into()],
    }
}

fn order() -> StepBody {
    StepBody::Order {
        prompt: "Atherosclerosis".into(),
        steps: vec!["Endothelial injury".into(), "Foam cell formation".into(), "Fibrous plaque".into()],
        why: None,
    }
}

fn matching() -> StepBody {
    StepBody::Match {
        prompt: "Match".into(),
        pairs: vec![
            ["Coxsackie B".into(), "Myocarditis".into()],
            ["S. aureus".into(), "Acute endocarditis".into()],
        ],
        why: None,
    }
}

fn sort() -> StepBody {
    StepBody::Sort {
        prompt: "Sort".into(),
        bins: vec!["Cyanotic".into(), "Acyanotic".into()],
        cards: vec![
            SortCard { text: "Tetralogy of Fallot".into(), bin: 0 },
            SortCard { text: "Ventricular septal defect".into(), bin: 1 },
        ],
        why: None,
    }
}

fn case() -> StepBody {
    StepBody::Case {
        stem: "A 50-year-old man with chest pain.".into(),
        steps: vec![
            CaseStep {
                reveal: None,
                question: "Next test?".into(),
                options: vec!["ECG".into(), "CT".into()],
                answer: Some(0),
                why: "".into(),
            },
            CaseStep {
                reveal: None,
                question: "Troponin?".into(),
                options: vec![],
                answer: None,
                why: "".into(),
            },
        ],
    }
}

fn reveal() -> StepBody {
    StepBody::Reveal { prompt: "Why?".into(), answer: "Because.".into() }
}

fn lesson() -> Lesson {
    let bodies = vec![
        StepBody::Explain { title: None, text: "Intro".into() },
        check(),
        order(),
        matching(),
        sort(),
        case(),
        reveal(),
        StepBody::Mnemonic {
            word: "AB".into(),
            parts: vec![
                MnemonicPart { key: "A".into(), text: "a".into() },
                MnemonicPart { key: "B".into(), text: "b".into() },
            ],
            hook: None,
        },
    ];
    Lesson {
        id: "cvs.L04.lesson1".into(),
        title: "t".into(),
        hook: "h".into(),
        minutes: 5,
        pages: [1, 2],
        topics: vec![],
        goals: vec![],
        steps: bodies.into_iter().map(step).collect(),
        recap: vec![],
        practice: vec![],
        status: crate::content::Status::Unverified,
    }
}

/// Saves, serializes, and restores the attempt, as a reopen does.
fn reopen(l: &Lesson, a: &Attempt) -> Attempt {
    let json = serde_json::to_string(a).unwrap();
    Attempt::restore(l, Some(serde_json::from_str(&json).unwrap()), None, 99)
}

#[test]
fn every_kind_restores_its_response_and_gating() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    let drafts = [
        (1, Draft::Check { pick: Some(2), checked: true }, Some(false)),
        (2, Draft::Order { placed: vec![0, 1, 2], checked: true }, Some(true)),
        (3, Draft::Match { done: vec![1, 0], misses: 1 }, Some(false)),
        (4, Draft::Sort { put: vec![Some(0), Some(1)], checked: true }, Some(true)),
        (5, Draft::Case { picks: vec![Some(1), Some(0)], at: 1, pick: None, first: vec![] }, Some(false)),
        (6, Draft::Reveal { open: true }, Some(true)),
    ];
    for (k, d, r) in &drafts {
        a.put(*k, sig(&l.steps[*k].body), d.clone());
        a.answer(*k, r.unwrap());
    }
    a.put(7, sig(&l.steps[7].body), Draft::Mnemonic { shown: vec![true, false] });
    let b = reopen(&l, &a);
    assert_eq!(b.drafts, a.drafts);
    assert_eq!(b.answers, a.answers);
    for (k, d, r) in &drafts {
        assert_eq!(b.drafts[*k].draft.result(&l.steps[*k].body), *r, "step {k}");
        assert_eq!(b.state(&l, *k), StepState::Done, "{d:?}");
        assert!(b.state(&l, *k).can_continue());
    }
    assert_eq!(b.state(&l, 0), StepState::Plain);
    assert_eq!(b.state(&l, 7), StepState::Plain);
}

#[test]
fn a_re_solved_case_part_keeps_the_first_pick_for_the_score() {
    let l = lesson();
    let body = &l.steps[5].body;
    // Part 1 first answered wrong (1), then re-solved right (0).
    let d = Draft::Case { picks: vec![Some(0), Some(0)], at: 1, pick: None, first: vec![Some(1), Some(0)] };
    assert!(d.fits(body));
    assert_eq!(d.result(body), Some(false));
    // First right, re-solved wrong: still right.
    let d = Draft::Case { picks: vec![Some(1), Some(0)], at: 1, pick: None, first: vec![Some(0), Some(0)] };
    assert_eq!(d.result(body), Some(true));
    // Old drafts without `first` still read, and serialize as before.
    let old: Draft = serde_json::from_str(r#"{"kind":"case","picks":[0,0],"at":1,"pick":null}"#).unwrap();
    assert_eq!(old.result(body), Some(true));
    let bad = Draft::Case { picks: vec![Some(0), Some(0)], at: 1, pick: None, first: vec![Some(5), None] };
    assert!(!bad.fits(body));
    assert!(!serde_json::to_string(&old).unwrap().contains("first"));
}

#[test]
fn unchecked_drafts_restore_as_drafts_and_keep_continue_off() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    a.put(1, sig(&l.steps[1].body), Draft::Check { pick: Some(0), checked: false });
    a.put(2, sig(&l.steps[2].body), Draft::Order { placed: vec![2], checked: false });
    a.put(3, sig(&l.steps[3].body), Draft::Match { done: vec![0], misses: 0 });
    a.put(4, sig(&l.steps[4].body), Draft::Sort { put: vec![Some(1), None], checked: false });
    a.put(
        5,
        sig(&l.steps[5].body),
        Draft::Case { picks: vec![None, None], at: 0, pick: Some(1), first: vec![] },
    );
    let b = reopen(&l, &a);
    for k in 1..=5 {
        assert_eq!(b.state(&l, k), StepState::Draft, "step {k}");
        assert!(!b.state(&l, k).can_continue());
        assert_eq!(b.answers[k], None);
    }
    assert_eq!(b.drafts[2].draft, Draft::Order { placed: vec![2], checked: false });
    assert_eq!(b.state(&l, 6), StepState::Open);
}

#[test]
fn a_full_but_unchecked_order_is_not_graded() {
    let d = Draft::Order { placed: vec![0, 1, 2], checked: false };
    assert_eq!(d.result(&order()), None);
    assert_eq!(state(&order(), None, false, Some(&d)), StepState::Draft);
}

#[test]
fn old_records_show_answered_before_not_an_empty_task() {
    let l = lesson();
    // A v0 record: results, no responses.
    let json = r#"{"at":3,"answers":[null,true,false,null,null,null,true,null],
            "skipped":[false,false,false,false,false,false,false,false],"tries":[]}"#;
    let old: Attempt = serde_json::from_str(json).unwrap();
    let b = Attempt::restore(&l, Some(old), None, 99);
    assert_eq!(b.at, 3);
    assert_eq!(b.state(&l, 1), StepState::AnsweredBefore);
    assert_eq!(b.state(&l, 2), StepState::AnsweredBefore);
    assert_eq!(b.state(&l, 3), StepState::Open);
    assert!(!b.state(&l, 3).can_continue());
    // A reveal with a result was opened: it shows open.
    assert_eq!(b.drafts[6].draft, Draft::Reveal { open: true });
    assert_eq!(b.state(&l, 6), StepState::Done);
    assert_eq!(b.score(&l), (1, 2));
}

#[test]
fn a_response_to_an_edited_step_is_dropped() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    a.put(1, sig(&l.steps[1].body), Draft::Check { pick: Some(2), checked: true });
    a.answer(1, false);
    let mut edited = l.clone();
    if let StepBody::Check { options, .. } = &mut edited.steps[1].body {
        options[0] = "Myoglobin".into();
    }
    let b = reopen(&edited, &a);
    assert_eq!(b.drafts[1].draft, Draft::None);
    assert_eq!(b.state(&edited, 1), StepState::AnsweredBefore);
}

#[test]
fn out_of_range_responses_are_dropped() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    a.put(2, sig(&l.steps[2].body), Draft::Order { placed: vec![0, 0], checked: false });
    a.put(1, sig(&l.steps[1].body), Draft::Check { pick: Some(7), checked: false });
    a.put(4, sig(&l.steps[4].body), Draft::Check { pick: Some(0), checked: false });
    let b = reopen(&l, &a);
    assert!(b.drafts[1..=4].iter().all(|s| s.draft == Draft::None));
}

#[test]
fn a_checked_response_without_its_result_is_graded_once() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    a.put(1, sig(&l.steps[1].body), Draft::Check { pick: Some(1), checked: true });
    let b = reopen(&l, &a);
    assert_eq!(b.answers[1], Some(true));
    let mut c = reopen(&l, &b);
    assert_eq!(c.answers[1], Some(true));
    // A second check of the same step never replaces the first result.
    assert!(!c.answer(1, false));
    assert_eq!(c.answers[1], Some(true));
}

#[test]
fn skipped_steps_stay_answerable_and_count_once_answered() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 1);
    a.skipped[1] = true;
    assert_eq!(a.state(&l, 1), StepState::Skipped);
    assert!(a.state(&l, 1).can_continue());
    assert_eq!(a.score(&l), (0, 0));
    assert!(a.answer(1, true));
    assert_eq!(a.score(&l), (1, 1));
}

#[test]
fn a_retake_survives_a_reopen_but_a_leftover_does_not() {
    let l = lesson();
    let mut a = Attempt::new(l.steps.len(), 500);
    a.at = 2;
    let json = serde_json::to_string(&a).unwrap();
    let back = || serde_json::from_str::<Attempt>(&json).ok();
    // Finished at 400, retake began at 500: keep it.
    assert_eq!(Attempt::restore(&l, back(), Some(400), 900).at, 2);
    // Finished at 600, after this attempt began: start over.
    let fresh = Attempt::restore(&l, back(), Some(600), 900);
    assert_eq!((fresh.at, fresh.started_ms), (0, 900));
}

#[test]
fn unknown_kinds_from_a_newer_app_load() {
    let s: Saved = serde_json::from_str(r#"{"sig":"x","draft":{"kind":"hotspot","x":1}}"#).unwrap();
    assert_eq!(s.draft, Draft::Unknown);
}

#[test]
fn quote_waits_for_the_answer() {
    assert!(quote_held(&check(), StepState::Open, None));
    assert!(quote_held(&check(), StepState::Skipped, None));
    assert!(!quote_held(&check(), StepState::Done, None));
    assert!(!quote_held(&check(), StepState::AnsweredBefore, None));
    let table = StepBody::Table {
        title: "Shock".into(),
        columns: vec!["Type".into(), "CVP".into()],
        rows: vec![vec!["Hypovolemic".into(), "↓".into()]],
        hide: vec![[0, 1]],
    };
    assert!(quote_held(&table, StepState::Plain, None));
    assert!(!quote_held(&table, StepState::Plain, Some(&Draft::Table { open: vec![[0, 1]] })));
    let StepBody::Table { title, columns, rows, .. } = table else { unreachable!() };
    let plain = StepBody::Table { title, columns, rows, hide: vec![] };
    assert!(!quote_held(&plain, StepState::Plain, None));
}
