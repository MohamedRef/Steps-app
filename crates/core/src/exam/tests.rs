use super::*;
use crate::content::{Item, Source};

fn mcq(id: &str, topic: &str, tags: &[&str], options: &[&str], answer: usize) -> Item {
    Item {
        id: id.into(),
        body: Body::Mcq {
            stem: format!("stem {id}"),
            options: options.iter().map(|s| s.to_string()).collect(),
            answer,
            why: options.iter().map(|_| "because".to_string()).collect(),
        },
        topic: Some(topic.into()),
        src: Source { book: None, page: 1, quote: None },
        status: Status::Unverified,
        update: None,
        tags: tags.iter().map(|s| s.to_string()).collect(),
    }
}

fn lecture(id: &str, items: Vec<Item>) -> Lecture {
    Lecture {
        schema: crate::content::SCHEMA.into(),
        lecture: id.into(),
        title: id.into(),
        author: None,
        book: crate::content::BookRef { chapter: 1, pages: [1, 99] },
        topics: vec![],
        items,
    }
}

const ABCD: &[&str] = &["Class A", "Class B", "Class C", "Cannot be scored"];

fn library() -> Vec<Lecture> {
    let mut l01: Vec<Item> = (0..30)
        .map(|i| {
            let topic = if i % 3 == 0 { "shock" } else { "pericardium" };
            let tags: &[&str] = if i % 5 == 0 { &["high-yield"] } else { &[] };
            mcq(&format!("cvs.L04.{i:03}"), topic, tags, ABCD, 1)
        })
        .collect();
    l01.push(Item {
        body: Body::Card { front: "f".into(), back: "b".into() },
        ..mcq("cvs.L04.900", "pericardium", &[], ABCD, 0)
    });
    l01.push(Item { status: Status::Rejected, ..mcq("cvs.L04.901", "pericardium", &[], ABCD, 0) });
    let l09 = (0..10).map(|i| mcq(&format!("cvs.L09.{i:03}"), "ie", &[], ABCD, 2)).collect();
    vec![lecture("cvs.L04", l01), lecture("cvs.L09", l09)]
}

#[test]
fn pool_skips_other_kinds_and_rejected_items() {
    let lib = library();
    let pool = mcqs(&lib);
    assert_eq!(pool.len(), 40);
    assert!(pool.iter().all(|q| q.id != "cvs.L04.900" && q.id != "cvs.L04.901"));
}

#[test]
fn filters_combine() {
    let lib = library();
    let pool = mcqs(&lib);
    let last = LastResults::new();
    let mut f = Filter::default();
    assert_eq!(f.select(&pool, &last).len(), 40);
    f.lectures.insert("cvs.L09".into());
    assert_eq!(f.select(&pool, &last).len(), 10);
    f.lectures.clear();
    f.topics.insert(("cvs.L04".into(), "shock".into()));
    assert_eq!(f.select(&pool, &last).len(), 10);
    f.tags.insert("high-yield".into());
    // i % 3 == 0 and i % 5 == 0: 0, 15.
    assert_eq!(f.select(&pool, &last).len(), 2);
}

#[test]
fn wrong_or_unseen_drops_only_last_right() {
    let lib = library();
    let pool = mcqs(&lib);
    let mut last = LastResults::new();
    last.insert("cvs.L09.000".into(), true);
    last.insert("cvs.L09.001".into(), false);
    let f =
        Filter { lectures: ["cvs.L09".to_string()].into(), only: Only::WrongOrUnseen, ..Filter::default() };
    let ids: Vec<&str> = f.select(&pool, &last).iter().map(|q| q.id).collect();
    assert_eq!(ids.len(), 9);
    assert!(!ids.contains(&"cvs.L09.000"));
    assert!(ids.contains(&"cvs.L09.001"));
}

#[test]
fn practice_is_seeded_and_capped() {
    let lib = library();
    let pool = mcqs(&lib);
    let a = practice(&pool, 12, 7);
    assert_eq!(a.len(), 12);
    assert_eq!(a, practice(&pool, 12, 7));
    assert_ne!(a, practice(&pool, 12, 8));
    let ids: BTreeSet<&str> = a.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(ids.len(), 12);
    assert_eq!(practice(&pool, 500, 1).len(), 40);
}

#[test]
fn mock_spreads_by_lecture_and_topic() {
    let lib = library();
    let pool = mcqs(&lib);
    let paper = mock(&pool, 20, 42);
    assert_eq!(paper.len(), 20);
    let l01 = paper.iter().filter(|q| q.lecture == "cvs.L04").count();
    // 30 of 40 MCQs are L01, so 15 of 20.
    assert_eq!(l01, 15);
    let cp = paper.iter().filter(|q| q.topic.as_deref() == Some("shock")).count();
    assert!((7..=8).contains(&cp), "shock got {cp} of 15");
    let ids: BTreeSet<&str> = paper.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(ids.len(), 20);
    assert_eq!(mock(&pool, 99, 1).len(), 40);
    assert!(mock(&[], 40, 1).is_empty());
}

#[test]
fn largest_remainder_sums_and_respects_sizes() {
    assert_eq!(largest_remainder(&[30, 10], 20), vec![15, 5]);
    assert_eq!(largest_remainder(&[1, 1, 1], 2).iter().sum::<usize>(), 2);
    assert_eq!(largest_remainder(&[2, 50], 40), vec![2, 38]);
    assert_eq!(largest_remainder(&[], 5), Vec::<usize>::new());
}

#[test]
fn shuffled_options_keep_the_answer() {
    let lib = library();
    let pool = mcqs(&lib);
    for q in practice(&pool, 40, 3) {
        let mut sorted = q.order.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3]);
        let src = pool.iter().find(|p| p.id == q.id).unwrap();
        assert_eq!(src.options[q.answer], src.options[src.answer]);
    }
}

#[test]
fn options_that_name_others_stay_in_order() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert!(shufflable(&s(&["Metoprolol", "Carvedilol", "Ivabradine", "Digoxin"])));
    assert!(!shufflable(&s(&["a", "b", "c", "All of the above"])));
    assert!(!shufflable(&s(&["a", "b", "Both a and b", "d"])));
    assert!(!shufflable(&s(&["x", "y", "z", "A and C"])));
    assert!(!shufflable(&s(&["x", "y", "z", "All of these"])));
    assert!(!shufflable(&s(&["HCM", "DCM", "Both", "Neither"])));
    assert!(!shufflable(&s(&["x", "y", "None"])));
    assert!(shufflable(&s(&["None while compensated", "x", "y"])));
    assert!(shufflable(&s(&["Vitamin A and D deficiency", "x", "y", "z"])));
}

#[test]
fn numeric_options_go_smallest_first() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(numeric_order(&s(&["500/mm³", "100/mm³", "≥ 250/mm³", "1,000/mm³"])), Some(vec![1, 2, 0, 3]));
    assert_eq!(numeric_order(&s(&["1.1 g/dL", "0.5 g/dL"])), Some(vec![1, 0]));
    assert_eq!(numeric_order(&s(&["Class A", "5"])), None);
    assert_eq!(numeric_order(&s(&["Above 20 ng/ml", "above 10 ng/ml", "Less than 5"])), Some(vec![2, 1, 0]));
}

#[test]
fn timing() {
    assert_eq!(time_limit_ms(40), 60 * 60_000);
    assert_eq!(remaining_ms(1, 1_000, 1_000), 90_000);
    // A full block is Step 1's 30 minutes.
    assert_eq!(time_limit_ms(MOCK_SIZE), 1_800_000);
    assert_eq!(remaining_ms(1, 1_000, 61_000), 30_000);
    assert_eq!(remaining_ms(1, 1_000, 999_999), 0);
}

fn q(id: &str, lecture: &str, topic: &str, answer: usize) -> Question {
    Question {
        id: id.into(),
        lecture: lecture.into(),
        topic: Some(topic.into()),
        answer,
        order: vec![0, 1, 2, 3],
    }
}

fn a(chosen: Option<usize>) -> Answer {
    Answer { chosen, took_ms: 5_000, ..Answer::default() }
}

#[test]
fn scoring_counts_blanks_as_wrong() {
    let qs = [q("1", "cvs.L04", "cp", 1), q("2", "cvs.L04", "mgmt", 0), q("3", "cvs.L09", "ie", 2)];
    let ans = [a(Some(1)), a(None), a(Some(0))];
    let o = score(&qs, &ans, Mode::Mock, 0, 60_000);
    assert_eq!(o.total, Score { right: 1, total: 3 });
    assert_eq!(o.unanswered, 1);
    assert_eq!(o.by_lecture["cvs.L04"], Score { right: 1, total: 2 });
    assert_eq!(o.by_topic["cvs.L09/ie"], Score { right: 0, total: 1 });
    assert_eq!(o.total.percent(), 33);
    assert_eq!(Score { right: 1, total: 2 }.percent(), 50);
    assert_eq!(Score::default().percent(), 0);
    let weak = weak_topics(&o, 60, 1);
    assert_eq!(weak.iter().map(|w| w.0).collect::<Vec<_>>(), ["cvs.L04/mgmt", "cvs.L09/ie"]);
}

#[test]
fn a_thin_mock_still_names_topics_to_revise() {
    // One question per topic, mostly wrong: no topic has two answers.
    let qs = [q("1", "cvs.L04", "cp", 1), q("2", "cvs.L09", "ie", 0), q("3", "cvs.L13", "rf", 2)];
    let o = score(&qs, &[a(Some(0)), a(Some(0)), a(None)], Mode::Mock, 0, 60_000);
    assert!(weak_topics(&o, 60, 2).is_empty());
    let focus: Vec<_> = focus_topics(&o, 60).into_iter().map(|w| w.0).collect();
    assert_eq!(focus, ["cvs.L04/cp", "cvs.L13/rf"]);
    // With two answers in a topic, the stricter list wins.
    let qs = [q("1", "cvs.L04", "cp", 1), q("2", "cvs.L04", "cp", 0), q("3", "cvs.L09", "ie", 2)];
    let o = score(&qs, &[a(Some(0)), a(None), a(Some(0))], Mode::Mock, 0, 60_000);
    assert_eq!(focus_topics(&o, 60).into_iter().map(|w| w.0).collect::<Vec<_>>(), ["cvs.L04/cp"]);
}

#[test]
fn wrong_is_again_right_is_good_blank_sends_nothing() {
    let question = q("cvs.L04.003", "cvs.L04", "cp", 1);
    let right = review_event(&question, &a(Some(1)), Mode::Tutor, 9).unwrap();
    assert_eq!((right.grade, right.origin), (Grade::Good, Origin::Bank));
    let wrong = review_event(&question, &a(Some(0)), Mode::Mock, 9).unwrap();
    assert_eq!((wrong.grade, wrong.origin), (Grade::Again, Origin::Mock));
    assert!(review_event(&question, &a(None), Mode::Timed, 9).is_none());
}

#[test]
fn confidence_and_reason_go_with_the_answer() {
    let question = q("hb.L01.003", "hb.L01", "cp", 1);
    let tagged = |chosen| Answer {
        confidence: Some(Confidence::Sure),
        reason: Some(ErrorReason::Misread),
        ..a(Some(chosen))
    };
    let wrong = review_event(&question, &tagged(0), Mode::Tutor, 9).unwrap();
    assert_eq!((wrong.confidence, wrong.reason), (Some(Confidence::Sure), Some(ErrorReason::Misread)));
    assert!(wrong.sure_miss());
    // A right answer keeps its confidence but never a reason.
    let right = review_event(&question, &tagged(1), Mode::Tutor, 9).unwrap();
    assert_eq!((right.confidence, right.reason), (Some(Confidence::Sure), None));
    assert!(!right.sure_miss());
}

#[test]
fn answers_saved_before_confidence_still_load() {
    let old: Answer = serde_json::from_str(r#"{"chosen":1,"took_ms":5,"flagged":false}"#).unwrap();
    assert_eq!((old.confidence, old.reason), (None, None));
    let json = serde_json::to_string(&old).unwrap();
    assert!(!json.contains("confidence") && !json.contains("reason"));
}

#[test]
fn history_keeps_the_last_result() {
    let qs = [q("1", "cvs.L04", "cp", 1), q("2", "cvs.L04", "cp", 1)];
    let mut h = History::default();
    h.record(&qs, &[a(Some(0)), a(None)]);
    h.record(&qs[..1], &[a(Some(1))]);
    assert_eq!(h.last.get("1"), Some(&true));
    assert_eq!(h.last.get("2"), None);
}
