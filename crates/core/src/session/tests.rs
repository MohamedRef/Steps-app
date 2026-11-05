use super::*;
use crate::content::Lecture;

fn sample() -> Lecture {
    serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap()
}

const STUDY: &[&str] = &["card", "cloze", "enumerate", "essay"];

fn compose_with(lecture: &Lecture, due: &[&str], seen: &[&str], budget: u32) -> Vec<Planned> {
    let lectures = [lecture];
    let due: Vec<String> = due.iter().map(|s| s.to_string()).collect();
    let seen: HashSet<String> = seen.iter().map(|s| s.to_string()).collect();
    let empty = HashSet::new();
    compose(&Inputs {
        lectures: &lectures,
        topics: &empty,
        kinds: STUDY,
        due: &due,
        seen: &seen,
        hidden: &empty,
        max_new: usize::MAX,
        budget_secs: budget,
        pages: None,
    })
}

fn ids(plan: &[Planned]) -> Vec<&str> {
    plan.iter().map(|p| p.item_id.as_str()).collect()
}

#[test]
fn new_items_follow_book_order_and_only_playable_kinds() {
    let plan = compose_with(&sample(), &[], &[], 600);
    assert_eq!(ids(&plan), ["cvs.L04.001", "cvs.L04.002", "cvs.L04.004", "cvs.L04.005"]);
    assert!(plan.iter().all(|p| p.fresh));
}

#[test]
fn due_reviews_come_first() {
    let plan = compose_with(&sample(), &["cvs.L04.005", "cvs.L04.002"], &["cvs.L04.002", "cvs.L04.005"], 600);
    assert_eq!(&ids(&plan)[..2], ["cvs.L04.005", "cvs.L04.002"]);
    assert!(!plan[0].fresh && !plan[1].fresh);
    assert_eq!(&ids(&plan)[2..], ["cvs.L04.001", "cvs.L04.004"]);
}

#[test]
fn due_reviews_interleave_topics() {
    let mut lecture = sample();
    // Two more shock clozes, so three of the same topic are due together.
    for n in ["101", "102"] {
        let mut extra = lecture.items[1].clone();
        extra.id = format!("cvs.L04.{n}");
        lecture.items.push(extra);
    }
    let due = ["cvs.L04.002", "cvs.L04.101", "cvs.L04.102", "cvs.L04.001"];
    let plan = compose_with(&lecture, &due, &due, 600);
    let topics: Vec<&str> = plan.iter().take(4).map(|p| p.topic.as_str()).collect();
    assert_eq!(topics, ["cvs.L04/shock", "cvs.L04/valves", "cvs.L04/shock", "cvs.L04/shock"]);
}

#[test]
fn new_items_are_blocked_by_topic() {
    let mut lecture = sample();
    let mut late = lecture.items[0].clone();
    late.id = "cvs.L04.200".into();
    lecture.items.push(late); // a "valves" card at the end of the file
    let plan = compose_with(&lecture, &[], &[], 6000);
    assert_eq!(&ids(&plan)[..2], ["cvs.L04.001", "cvs.L04.200"]);
}

#[test]
fn budget_caps_the_session_but_never_empties_it() {
    let plan = compose_with(&sample(), &[], &[], 70);
    let total: u32 = plan.iter().map(|p| p.est_secs).sum();
    assert!(total <= 70, "{total}");
    assert_eq!(ids(&plan), ["cvs.L04.001", "cvs.L04.002"]);
    let tiny = compose_with(&sample(), &[], &[], 1);
    assert_eq!(tiny.len(), 1);
}

#[test]
fn daily_new_cap_holds_back_new_items_not_reviews() {
    let lecture = sample();
    let lectures = [&lecture];
    let due = vec!["cvs.L04.004".to_string()];
    let seen: HashSet<String> = due.iter().cloned().collect();
    let empty = HashSet::new();
    let plan = compose(&Inputs {
        lectures: &lectures,
        topics: &empty,
        kinds: STUDY,
        due: &due,
        seen: &seen,
        hidden: &empty,
        max_new: 1,
        budget_secs: 600,
        pages: None,
    });
    assert_eq!(ids(&plan), ["cvs.L04.004", "cvs.L04.001"]);
}

#[test]
fn page_gate_holds_back_new_items_from_unread_pages_not_reviews() {
    let mut lecture = sample();
    // Another source's item cites its own page: it waits for any read
    // page of the lecture instead.
    let mut pathoma = lecture.items[0].clone();
    pathoma.id = "cvs.L04.300".into();
    pathoma.src.book = Some("pathoma".into());
    pathoma.src.page = 99;
    lecture.items.push(pathoma);
    let mut own = lecture.items[0].clone();
    own.id = format!("{}x", crate::own::PREFIX);
    own.src.page = 500;
    lecture.items.push(own.clone());
    let lectures = [&lecture];
    // 004 (page 319, read or not) is due: reviews are never gated.
    let due = vec!["cvs.L04.004".to_string()];
    let seen: HashSet<String> = due.iter().cloned().collect();
    let empty = HashSet::new();
    let run = |pages: &PagesRead| {
        let plan = compose(&Inputs {
            lectures: &lectures,
            topics: &empty,
            kinds: STUDY,
            due: &due,
            seen: &seen,
            hidden: &empty,
            max_new: usize::MAX,
            budget_secs: 6000,
            pages: Some(pages),
        });
        ids(&plan).into_iter().map(str::to_string).collect::<Vec<_>>()
    };
    let mut read = PagesRead::from(vec![[319, 319], [305, 305]]);
    // The two copies of 001 share its topic, so they come together.
    assert_eq!(run(&read), ["cvs.L04.004", "cvs.L04.001", "cvs.L04.300", own.id.as_str(), "cvs.L04.005"]);
    // Reading page 317 unlocks the cloze on it.
    read.mark(317, 317);
    assert!(run(&read).contains(&"cvs.L04.002".to_string()));
    // Nothing read: only the review and his own card.
    assert_eq!(run(&PagesRead::default()), ["cvs.L04.004", own.id.as_str()]);
}

#[test]
fn rejected_hidden_and_filtered_topics_stay_out() {
    let mut lecture = sample();
    lecture.items[0].status = Status::Rejected;
    let lectures = [&lecture];
    let topics: HashSet<String> = ["cvs.L04/shock".to_string(), "cvs.L04/rheumatic".into()].into();
    let hidden: HashSet<String> = ["cvs.L04.004".to_string()].into();
    let empty = HashSet::new();
    let plan = compose(&Inputs {
        lectures: &lectures,
        topics: &topics,
        kinds: STUDY,
        due: &[],
        seen: &empty,
        hidden: &hidden,
        max_new: usize::MAX,
        budget_secs: 600,
        pages: None,
    });
    assert_eq!(ids(&plan), ["cvs.L04.002"]);
}

#[test]
fn hidden_items_are_skipped_and_never_retried() {
    let plan = compose_with(&sample(), &[], &[], 600);
    let mut s = Session::new(plan);
    // He marks 001 ✗ and grades it Again; 002 arrives ✗ by sync.
    let hidden = |id: &str| matches!(id, "cvs.L04.001" | "cvs.L04.002");
    s.grade_unless(Grade::Again, hidden);
    assert_ne!(s.current().unwrap().item_id, "cvs.L04.002");
    while !s.is_done() {
        assert!(!hidden(&s.current().unwrap().item_id));
        s.grade_unless(Grade::Good, hidden);
    }
    assert_eq!(s.outcomes.iter().filter(|o| o.item_id == "cvs.L04.001").count(), 1);
}

#[test]
fn again_comes_back_once_at_the_end() {
    let plan = compose_with(&sample(), &[], &[], 600);
    let mut s = Session::new(plan);
    s.grade(Grade::Again); // 001
    s.grade(Grade::Good);
    s.grade(Grade::Hard);
    s.grade(Grade::Easy);
    assert_eq!(s.current().unwrap().item_id, "cvs.L04.001");
    assert!(s.is_retry());
    s.grade(Grade::Again); // no second retry
    assert!(s.is_done());
    let sum = s.summary();
    assert_eq!(sum.items, 4);
    assert_eq!(sum.right, 2);
    assert_eq!(sum.fresh, 4);
    assert_eq!(sum.shaky, ["cvs.L04.001", "cvs.L04.004"]);
}

#[test]
fn typed_numbers_ignore_units_and_spacing() {
    assert_eq!(check_typed("5-6", "5 - 6"), Match::Right);
    assert_eq!(check_typed("5-6", "5 to 6"), Match::Right);
    assert_eq!(check_typed("5-6", "٥-٦"), Match::Right);
    assert_eq!(check_typed("≥250 cells/mm³", ">= 250"), Match::Right);
    assert_eq!(check_typed("≥250", "250"), Match::Close);
    assert_eq!(check_typed("≥250", "<250"), Match::Wrong);
    assert_eq!(check_typed("1.1 g/dL", "1,1"), Match::Right);
    assert_eq!(check_typed("1,000", "1000"), Match::Right);
    assert_eq!(check_typed("10-15", "10-16"), Match::Wrong);
    assert_eq!(check_typed("10-15", ""), Match::Wrong);
}

#[test]
fn typed_words_allow_case_and_small_typos() {
    assert_eq!(check_typed("HOCM", "hocm"), Match::Right);
    assert_eq!(check_typed("Eisenmenger syndrome", "eisenmenger-syndrome"), Match::Right);
    assert_eq!(check_typed("Nitroprusside", "nitroprusid"), Match::Close);
    assert_eq!(check_typed("PDA", "PDAS"), Match::Wrong);
    assert_eq!(check_typed("Metoprolol", "Carvedilol"), Match::Wrong);
}

#[test]
fn opposite_prefixes_are_wrong_not_typos() {
    assert_eq!(check_typed("Hypokalaemia", "hyperkalaemia"), Match::Wrong);
    assert_eq!(check_typed("Hypothyroidism", "hyperthyroidism"), Match::Wrong);
    assert_eq!(check_typed("Intracellular", "extracellular"), Match::Wrong);
    assert_eq!(check_typed("Intracellular", "intercellular"), Match::Wrong);
    assert_eq!(check_typed("Bradycardia", "tachycardia"), Match::Wrong);
    assert_eq!(check_typed("Microcytic", "macrocytic"), Match::Wrong);
    assert_eq!(check_typed("Afferent", "efferent"), Match::Wrong);
    assert_eq!(check_typed("Preload", "postload"), Match::Wrong);
    assert_eq!(check_typed("Primary hypoaldosteronism", "primary hyperaldosteronism"), Match::Wrong);
    assert_eq!(check_typed("Abduction", "adduction"), Match::Wrong);
    // British and American spellings are the same answer.
    assert_eq!(check_typed("Hypokalaemia", "hypokalemia"), Match::Right);
    assert_eq!(check_typed("Haemolysis", "hemolysis"), Match::Right);
    assert_eq!(check_typed("Oedematous ankles", "edematous ankles"), Match::Right);
    assert_eq!(check_typed("Ischaemia", "ischemia"), Match::Right);
    assert_eq!(check_typed("Tumour", "tumor"), Match::Right);
    // Shared first letters are not a prefix swap: typos stay close.
    assert_eq!(check_typed("Hemolysis", "hemolisis"), Match::Close);
    assert_eq!(check_typed("Abdominal", "adbominal"), Match::Close);
    assert_eq!(check_typed("Infection", "infaction"), Match::Close);
    assert_eq!(check_typed("Hyponatremia", "hyponatreima"), Match::Close);
}

#[test]
fn numbers_keep_units_signs_and_thousands() {
    assert_eq!(check_typed("7 days", "7 weeks"), Match::Wrong);
    assert_eq!(check_typed("7 days", "7 d"), Match::Right);
    assert_eq!(check_typed("7 days", "7"), Match::Right);
    assert_eq!(check_typed("1.5 mg/dL", "1.5 mmol/L"), Match::Wrong);
    assert_eq!(check_typed("−2.60", "2.60"), Match::Wrong);
    assert_eq!(check_typed("−2.60", "-2.6"), Match::Right);
    assert_eq!(check_typed("≤ −2.60", "<=-2.60"), Match::Right);
    assert_eq!(check_typed("−2.60 to −1.39", "-2.6 to -1.39"), Match::Right);
    assert_eq!(check_typed("5-6", "5 - 6"), Match::Right);
    assert_eq!(check_typed("1,000,000", "1000000"), Match::Right);
    assert_eq!(check_typed("10,000,000 copies/mL", "10000000"), Match::Right);
    assert_eq!(check_typed("1,000.5", "1000.5"), Match::Right);
    assert_eq!(check_typed("1,5", "1.5"), Match::Right);
}

#[test]
fn undo_restores_the_session_exactly() {
    let plan = compose_with(&sample(), &[], &[], 600);
    let mut s = Session::new(plan);
    let start = (s.progress(), s.current().cloned());
    s.grade_logged(Grade::Again, Some("e1".into()), |_| false);
    s.grade_logged(Grade::Good, Some("e2".into()), |_| false);
    let undone = s.undo().unwrap();
    assert_eq!(undone.event.as_deref(), Some("e2"));
    assert_eq!(undone.outcome.grade, Grade::Good);
    let undone = s.undo().unwrap();
    assert_eq!(undone.event.as_deref(), Some("e1"));
    // The Again retry is gone with it, and the first item is current again.
    assert_eq!((s.progress(), s.current().cloned()), start);
    assert!(s.outcomes.is_empty());
    assert!(!s.can_undo());
    assert!(s.undo().is_none());
    // Grading it Again afterwards still earns its one retry.
    s.grade(Grade::Again);
    assert_eq!(s.progress().1, start.0.1 + 1);
}

#[test]
fn undo_after_the_last_item_reopens_it_and_keeps_appended_items() {
    let p = |id: &str| Planned {
        item_id: id.into(),
        lecture: "L".into(),
        topic: "L".into(),
        est_secs: 10,
        fresh: true,
    };
    let mut s = Session::new(vec![p("a"), p("b")]);
    s.grade(Grade::Again); // a: retry queued at the end
    s.append(vec![p("c")]);
    s.grade(Grade::Good); // b
    s.grade(Grade::Good); // a (retry)
    s.grade(Grade::Good); // c
    assert!(s.is_done());
    s.undo();
    assert_eq!(s.current().unwrap().item_id, "c");
    s.undo();
    s.undo();
    s.undo();
    // Back at a, its retry gone, c still queued after b.
    let mut seen = Vec::new();
    while let Some(c) = s.current() {
        seen.push(c.item_id.clone());
        s.grade(Grade::Good);
    }
    assert_eq!(seen, ["a", "b", "c"]);
    assert_eq!(s.summary().items, 3);
}

#[test]
fn undo_puts_back_hidden_items_it_skipped() {
    let p = |id: &str| Planned {
        item_id: id.into(),
        lecture: "L".into(),
        topic: "L".into(),
        est_secs: 10,
        fresh: true,
    };
    let mut s = Session::new(vec![p("a"), p("b"), p("c")]);
    s.grade_unless(Grade::Good, |id| id == "b");
    assert_eq!(s.current().unwrap().item_id, "c");
    s.undo();
    assert_eq!(s.current().unwrap().item_id, "a");
}

#[test]
fn append_queues_after_whats_left_without_repeats() {
    let p = |id: &str| Planned {
        item_id: id.into(),
        lecture: "L".into(),
        topic: "L".into(),
        est_secs: 10,
        fresh: true,
    };
    let mut s = Session::new(vec![p("a"), p("b")]);
    s.grade(Grade::Good);
    assert_eq!(s.append(vec![p("b"), p("c"), p("a")]), 2);
    let mut seen = Vec::new();
    while let Some(c) = s.current() {
        seen.push(c.item_id.clone());
        s.grade(Grade::Good);
    }
    assert_eq!(seen, ["b", "c", "a"]);
}

#[test]
fn part_of_a_list_answer_is_close() {
    let e = "vitamin A and retinoids";
    assert_eq!(check_typed(e, "vitamin A"), Match::Close);
    assert_eq!(check_typed(e, "retinoids"), Match::Close);
    assert_eq!(check_typed(e, "retinoids and vitamin A"), Match::Right);
    assert_eq!(check_typed(e, "vitamin A, iron"), Match::Wrong);
    assert_eq!(check_typed(e, "glycogen"), Match::Wrong);
    assert_eq!(check_typed("CK-MB or troponin", "troponin"), Match::Right);
    assert_eq!(check_typed("hyperkalaemia and acidosis", "hypokalaemia"), Match::Wrong);
}

#[test]
fn recall_needs_more_than_a_shared_stem() {
    let points: Vec<String> =
        ["Cardiac tamponade", "Cardiogenic shock", "Pulmonary hypertension"].map(String::from).to_vec();
    assert_eq!(recalled(&points, "cardiac tamponade\nsystemic hypertension"), [true, false, false]);
    assert_eq!(recalled(&points, "card"), [false, false, false]);
    assert_eq!(recalled(&points, "BCK, CS\npulmonary htn"), [false, false, true]);
}

#[test]
fn grade_suggestions() {
    assert_eq!(suggest_cloze(&[Match::Right, Match::Right]), Grade::Good);
    assert_eq!(suggest_cloze(&[Match::Right, Match::Close]), Grade::Hard);
    assert_eq!(suggest_cloze(&[Match::Close, Match::Wrong]), Grade::Again);
}

#[test]
fn free_recall_pre_ticks_matching_points() {
    let points: Vec<String> = [
        "Migratory polyarthritis",
        "Pancarditis",
        "Subcutaneous nodules",
        "Erythema marginatum",
        "Sydenham chorea",
        "ASO titer",
    ]
    .map(String::from)
    .to_vec();
    let typed = "polyarthritis migratory\nsubcutaneous nodules\nerythema marginatum\nsydenham chorea";
    assert_eq!(recalled(&points, typed), [true, false, true, true, true, false]);
    assert_eq!(recalled(&points, ""), [false; 6]);
    let headings = ["Hypertrophic cardiomyopathy".to_string(), "Pulmonary hypertension".into()];
    assert_eq!(recalled(&headings, "cardiomyopathy"), [true, false]);
}

#[test]
fn outline_sections_flatten_headings() {
    let lecture = sample();
    let Body::Essay { outline, .. } = &lecture.items[4].body else { panic!() };
    let s = outline_sections(outline);
    assert_eq!(s.len(), 4);
    assert_eq!(s[0].0, "Risk factors");
    assert_eq!(s[3].1.len(), 3);
}
