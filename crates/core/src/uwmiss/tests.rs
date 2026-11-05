use super::*;

fn miss(qid: u32, reason: ErrorReason, created_ms: i64) -> Miss {
    Miss { qid, reason, note: None, day: Day::parse("2027-03-01").unwrap(), created_ms }
}

fn lecture_of(qid: u32) -> Option<String> {
    match qid {
        100..=199 => Some("cvs.L04".into()),
        200..=299 => Some("renal.L03".into()),
        _ => None,
    }
}

#[test]
fn a_qid_is_its_digits_and_never_zero() {
    assert_eq!(parse_qid(" #1707 "), Some(1707));
    assert_eq!(parse_qid("QID 12"), Some(12));
    assert_eq!(parse_qid("0"), None);
    assert_eq!(parse_qid("abc"), None);
    assert_eq!(parse_qid("99999999999999"), None);
    assert_eq!(key(1707), "uw.miss.1707");
}

#[test]
fn it_round_trips_as_plain_json() {
    let m = Miss { note: Some("mixed up MEN1 and MEN2".into()), ..miss(1707, ErrorReason::Knowledge, 5) };
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains(r#""reason":"knowledge""#) && json.contains(r#""day":"2027-03-01""#), "{json}");
    assert_eq!(serde_json::from_str::<Miss>(&json).unwrap(), m);
    let bare = serde_json::to_string(&miss(1, ErrorReason::Timing, 1)).unwrap();
    assert!(!bare.contains("note"), "{bare}");
}

#[test]
fn tidy_drops_an_empty_note_and_a_long_one_is_refused() {
    let m = Miss { note: Some("   ".into()), ..miss(1, ErrorReason::Misread, 1) }.tidy();
    assert_eq!(m.note, None);
    assert_eq!(m.problem(), None);
    let long = Miss { note: Some("x".repeat(MAX_NOTE + 1)), ..m };
    assert_eq!(long.problem(), Some(Problem::NoteTooLong));
}

#[test]
fn newest_first_orders_by_day_then_entry() {
    let a = Miss { day: Day::parse("2027-03-02").unwrap(), ..miss(1, ErrorReason::Misread, 1) };
    let b = miss(2, ErrorReason::Misread, 9);
    let c = miss(3, ErrorReason::Misread, 5);
    let order: Vec<u32> = newest_first(&[b.clone(), c.clone(), a.clone()]).iter().map(|m| m.qid).collect();
    assert_eq!(order, [1, 2, 3]);
}

#[test]
fn patterns_rank_reasons_and_lectures() {
    let log = [
        miss(101, ErrorReason::Knowledge, 1),
        miss(202, ErrorReason::Misread, 2),
        miss(103, ErrorReason::Knowledge, 3),
        miss(104, ErrorReason::Reasoning, 4),
        miss(900, ErrorReason::Knowledge, 5),
    ];
    let p = patterns(&log, lecture_of);
    assert_eq!(p.total, 5);
    assert_eq!(
        p.by_reason,
        [(ErrorReason::Knowledge, 3), (ErrorReason::Misread, 1), (ErrorReason::Reasoning, 1)]
    );
    assert_eq!(p.top_reason(), Some((ErrorReason::Knowledge, 3)));
    assert_eq!(p.share(3), 60);
    assert_eq!(p.by_lecture, [("cvs.L04".to_string(), vec![101, 103, 104]), ("renal.L03".into(), vec![202])]);
    assert_eq!(p.unfiled, 1);
}

#[test]
fn no_top_reason_with_too_few_misses_or_a_tie() {
    let few =
        patterns(&[miss(101, ErrorReason::Knowledge, 1), miss(102, ErrorReason::Knowledge, 2)], lecture_of);
    assert_eq!(few.top_reason(), None);
    let tie = patterns(
        &[
            miss(101, ErrorReason::Knowledge, 1),
            miss(102, ErrorReason::Misread, 2),
            miss(103, ErrorReason::Knowledge, 3),
            miss(104, ErrorReason::Misread, 4),
        ],
        lecture_of,
    );
    assert_eq!(tie.top_reason(), None);
    assert_eq!(patterns(&[], lecture_of), Patterns::default());
    assert_eq!(Patterns::default().share(3), 0);
}

#[test]
fn only_the_top_lectures_are_listed() {
    let log: Vec<Miss> = (0..8).map(|i| miss(100 + i * 1000, ErrorReason::Timing, i64::from(i))).collect();
    let p = patterns(&log, |q| Some(format!("l{q}")));
    assert_eq!(p.by_lecture.len(), TOP_LECTURES);
}
