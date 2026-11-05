use super::*;

fn sample() -> Lecture {
    let text = include_str!("../../../../../fixtures/content/sample.yaml");
    serde_yaml_ng::from_str(text).expect("sample parses")
}

fn pick(size: usize) -> Pick {
    Pick { lecture: None, size, seed: 7, due: BTreeSet::new() }
}

#[test]
fn only_short_answer_kinds_are_asked() {
    let lecture = sample();
    let kinds: Vec<_> = pool([&lecture], None).iter().map(|p| p.kind).collect();
    assert!(kinds.contains(&"card") && kinds.contains(&"enumerate") && kinds.contains(&"essay"));
    assert!(kinds.iter().all(|k| ["card", "enumerate", "essay", "case"].contains(k)));
    assert!(pool([&lecture], Some("cvs.L05")).is_empty());
}

#[test]
fn enumerate_asks_what_else_and_keeps_the_pass_mark() {
    let lecture = sample();
    let p = pool([&lecture], None).into_iter().find(|p| p.kind == "enumerate").unwrap();
    assert!(p.what_else);
    assert_eq!(p.pass, Some((4, 5)));
    assert!(p.spoken_answer().starts_with("Migratory polyarthritis. Pancarditis"));
}

#[test]
fn essay_answer_reads_headings_with_bullets() {
    let lecture = sample();
    let p = pool([&lecture], None).into_iter().find(|p| p.kind == "essay").unwrap();
    assert_eq!(p.answer[0], "Risk factors: Hypertension (strongest); Bicuspid aortic valve; Marfan syndrome");
}

#[test]
fn rejected_items_are_never_asked() {
    let mut lecture = sample();
    for item in &mut lecture.items {
        item.status = Status::Rejected;
    }
    assert!(pool([&lecture], None).is_empty());
}

#[test]
fn symbols_are_spelled_out() {
    assert_eq!(
        speakable("Pulsus paradoxus >10 mm Hg → tamponade"),
        "Pulsus paradoxus more than 10 mm Hg then tamponade"
    );
    assert_eq!(speakable("EF ≥ 50%, ~80%"), "EF at least 50 percent, about 80 percent");
}

fn card(id: &str, topic: &str) -> Prompt {
    Prompt {
        item_id: id.into(),
        kind: "card",
        lecture: "cvs.L04".into(),
        topic: Some(topic.into()),
        page: 1,
        quote: None,
        question: vec![format!("Q {id}")],
        answer: vec![format!("A {id}")],
        pass: None,
        what_else: false,
    }
}

#[test]
fn again_comes_back_three_questions_later() {
    let pool: Vec<_> = (0..6).map(|i| card(&format!("c{i}"), "t")).collect();
    let mut s = Session::new(pool, &pick(6));
    let first = s.advance().unwrap().prompt.item_id.clone();
    s.rate(Some(Grade::Again));
    let mut order = Vec::new();
    while let Some(e) = s.advance() {
        order.push((e.prompt.item_id.clone(), e.again));
        s.rate(Some(Grade::Good));
    }
    assert_eq!(order.len(), 6);
    assert_eq!(order[AGAIN_GAP], (first, true));
    assert_eq!((s.tally.good, s.tally.again, s.tally.missed.len()), (6, 1, 1));
}

#[test]
fn a_double_tap_rates_one_question() {
    let pool: Vec<_> = (0..3).map(|i| card(&format!("c{i}"), "t")).collect();
    let mut s = Session::new(pool, &pick(3));
    let first = s.advance().unwrap().prompt.item_id.clone();
    let turn = s.asked;
    let rated = s.rate_turn(turn, Some(Grade::Good)).map(|p| p.item_id);
    assert_eq!(rated, Some(first));
    // The second tap still carries the old turn: the next question is untouched.
    let next = s.current.clone();
    assert_eq!(s.rate_turn(turn, Some(Grade::Good)), None);
    assert_eq!(s.current, next);
    assert_eq!((s.tally.good, s.left()), (1, 2));
    // The next question's own turn rates it as usual.
    assert!(s.rate_turn(s.asked, Some(Grade::Again)).is_some());
    assert_eq!(s.tally.again, 1);
}

#[test]
fn again_near_the_end_still_comes_back() {
    let mut s = Session::new(vec![card("only", "t")], &pick(5));
    s.advance();
    s.rate(Some(Grade::Again));
    assert_eq!(s.advance().map(|e| e.again), Some(true));
    s.rate(None);
    assert!(s.advance().is_none());
    assert_eq!(s.tally.heard, 1);
}

#[test]
fn every_third_card_leads_into_its_topics_essay() {
    let mut essay = card("essay", "septation");
    essay.kind = "essay";
    let mut pool: Vec<_> = (0..5).map(|i| card(&format!("c{i}"), "septation")).collect();
    pool.push(essay);
    // Find a seed that puts the essay last, so only the follow-up can pull it forward.
    let seed = (0..200)
        .find(|&seed| {
            let mut s =
                Session::new(pool.clone(), &Pick { lecture: None, size: 6, seed, due: BTreeSet::new() });
            (0..5).all(|_| {
                s.advance().is_some_and(|e| e.prompt.kind == "card") && {
                    s.current = None;
                    true
                }
            })
        })
        .expect("some seed puts the essay last");
    let mut s = Session::new(pool, &Pick { lecture: None, size: 6, seed, due: BTreeSet::new() });
    for _ in 0..3 {
        assert_eq!(s.advance().unwrap().prompt.kind, "card");
        s.rate(Some(Grade::Good));
    }
    let next = s.advance().unwrap();
    assert_eq!((next.prompt.kind, next.follow_up), ("essay", true));
}

#[test]
fn due_items_are_asked_first() {
    let pool: Vec<_> = (0..20).map(|i| card(&format!("c{i}"), "t")).collect();
    let due: BTreeSet<String> = ["c17", "c3", "c11"].map(String::from).into();
    let mut s = Session::new(pool, &Pick { lecture: None, size: 5, seed: 3, due: due.clone() });
    let first: BTreeSet<String> = (0..3)
        .map(|_| {
            let id = s.advance().unwrap().prompt.item_id.clone();
            s.rate(Some(Grade::Good));
            id
        })
        .collect();
    assert_eq!(first, due);
}

#[test]
fn prep_hides_and_edits_items() {
    let lecture = sample();
    let all = pool([&lecture], None).len();
    let none = pool_with([&lecture], None, |_| None);
    assert!(none.is_empty());
    let edited = pool_with([&lecture], None, |item| {
        let mut item = item.clone();
        if let Body::Card { back, .. } = &mut item.body {
            *back = "Edited by Mohamed".into();
        }
        Some(item)
    });
    assert_eq!(edited.len(), all);
    assert!(edited.iter().any(|p| p.answer == ["Edited by Mohamed"]));
}

#[test]
fn size_limits_the_session() {
    let pool: Vec<_> = (0..30).map(|i| card(&format!("c{i}"), "t")).collect();
    let s = Session::new(pool, &pick(10));
    assert_eq!((s.total, s.left()), (10, 10));
}
