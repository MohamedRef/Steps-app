use super::*;

#[test]
fn arabic_dates() {
    let d = Day::from_ymd(2027, 1, 20).unwrap();
    assert_eq!(long_date(d), "الأربع 20 يناير 2027");
    assert_eq!(short_date(d), "أربع 20 يناير");
    let lib = crate::content::Library::get();
    assert_eq!(
        lecture_list(&["cvs.L04".into(), "cvs.L12".into()]),
        format!("{}, {}", lib.title("cvs.L04"), lib.title("cvs.L12"))
    );
}

#[test]
fn todays_target_holds_still_while_he_studies() {
    use steps_core::review::{Grade, Origin, ReviewEvent};
    use steps_core::stats::Clock;

    let Some(lecture) =
        crate::content::Library::get().lectures.values().find(|l| l.items.len() >= 6).cloned()
    else {
        return; // no content embedded
    };
    const DAY_MS: i64 = 86_400_000;
    let now_ms = 20_400 * DAY_MS + 12 * 3_600_000;
    let clock = Clock { now_ms, zone: steps_core::zone::Zone::fixed(0), rollover_hour: 4 };
    let answer = |i: usize, days_ago: i64| ReviewEvent {
        item_id: lecture.items[i].id.clone(),
        grade: Grade::Good,
        origin: Origin::Review,
        at_ms: now_ms - days_ago * DAY_MS,
        took_ms: 5000,
        confidence: None,
        reason: None,
    };
    let data = |events: Vec<ReviewEvent>| StudyData {
        lectures: vec![lecture.clone()],
        events,
        marks: Default::default(),
        forecast: Some(Vec::new()),
        clock,
        demo: false,
    };
    let settings = PlanSettings { exam: Some(clock.today().plus(60)), ..PlanSettings::default() };
    assert!(daily_target(&data(Vec::new()), &PlanSettings::default()).is_none(), "no exam day, no target");

    let morning = daily_target(&data(vec![answer(0, 1)]), &settings).unwrap();
    let later =
        daily_target(&data(vec![answer(0, 1), answer(0, 0), answer(1, 0), answer(2, 0)]), &settings).unwrap();
    assert_eq!(morning.days_left, 60);
    assert_eq!(later.new_goal(), morning.new_goal(), "today's share doesn't shrink as he works");
    assert_eq!(later.done, stats::DayDone { new: 2, reviews: 1 });
    assert_eq!(later.new_left(), morning.new_goal().saturating_sub(2));
}

#[test]
fn low_nbme_scores_near_dedicated_offer_a_later_exam() {
    use steps_core::selfassess::{Kind, SelfAssessment};
    let exam = Day::parse("2027-09-15").unwrap();
    let settings = PlanSettings { exam: Some(exam), ..PlanSettings::default() };
    let entry = |id: &str, day: &str, kind: Kind, percent: u8| SelfAssessment {
        id: id.into(),
        kind,
        form: None,
        day: Day::parse(day).unwrap(),
        percent,
        pass_probability: None,
        note: None,
        created_ms: 0,
    };
    let low = [entry("a", "2027-07-20", Kind::Nbme, 58), entry("b", "2027-07-27", Kind::Nbme, 57)];
    let near = Day::parse("2027-07-28").unwrap();
    assert_eq!(not_ready_later(&settings, near, &low), Some(exam.plus(28)));
    assert_eq!(
        not_ready_later(&settings, Day::parse("2027-03-01").unwrap(), &low),
        None,
        "too early to judge"
    );
    let ready = [entry("a", "2027-07-20", Kind::Nbme, 70), entry("b", "2027-07-27", Kind::Nbme, 72)];
    assert_eq!(not_ready_later(&settings, near, &ready), None);
    let uwsa = [entry("a", "2027-07-20", Kind::Uwsa, 50), entry("b", "2027-07-27", Kind::Uwsa, 50)];
    assert_eq!(not_ready_later(&settings, near, &uwsa), None, "only NBME counts");
}

#[test]
fn vacations_parse_or_ask_again() {
    let ok = parse_vacations(&[("2027-01-01".into(), "2027-01-07".into())]).unwrap();
    assert_eq!(
        ok,
        vec![Vacation { from: Day::parse("2027-01-01").unwrap(), to: Day::parse("2027-01-07").unwrap() }]
    );
    assert_eq!(parse_vacations(&[]), Some(Vec::new()));
    assert_eq!(parse_vacations(&[("2027-01-07".into(), "2027-01-01".into())]), None, "ends before it starts");
    assert_eq!(parse_vacations(&[("".into(), "2027-01-01".into())]), None, "half typed");
}

#[test]
fn yesterdays_missed_new_items_are_counted_not_blamed() {
    use steps_core::review::{Grade, Origin, ReviewEvent};
    use steps_core::stats::Clock;

    let Some(lecture) =
        crate::content::Library::get().lectures.values().find(|l| l.items.len() >= 20).cloned()
    else {
        return; // no content embedded
    };
    const DAY_MS: i64 = 86_400_000;
    let now_ms = 20_400 * DAY_MS + 12 * 3_600_000;
    let clock = Clock { now_ms, zone: steps_core::zone::Zone::fixed(0), rollover_hour: 4 };
    let data = |events: Vec<ReviewEvent>| StudyData {
        lectures: vec![lecture.clone()],
        events,
        marks: Default::default(),
        forecast: Some(Vec::new()),
        clock,
        demo: false,
    };
    let settings =
        PlanSettings { exam: Some(clock.today().plus(120)), weekdays: [true; 7], ..PlanSettings::default() };
    let idle = daily_target(&data(Vec::new()), &settings).unwrap();
    assert!(idle.missed > 0, "yesterday's share wasn't started");
    assert_eq!(idle.before.len(), idle.plan.days.len());
    // He did yesterday's share: nothing missed.
    let did: Vec<ReviewEvent> = lecture
        .items
        .iter()
        .take(idle.missed as usize)
        .map(|it| ReviewEvent::new(it.id.clone(), Grade::Good, Origin::Review, now_ms - DAY_MS, 5000))
        .collect();
    assert_eq!(daily_target(&data(did), &settings).unwrap().missed, 0);
}
