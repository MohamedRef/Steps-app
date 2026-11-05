use super::*;

fn day(y: i32, m: u32, d: u32) -> Day {
    Day::from_ymd(y, m, d).unwrap()
}

fn lecture(id: &str, total: u32, seen: u32) -> LectureProgress {
    LectureProgress { id: id.into(), total, seen, seen_twice: 0 }
}

fn settings(exam: Day) -> PlanSettings {
    PlanSettings { exam: Some(exam), ..PlanSettings::default() }
}

#[test]
fn civil_dates_round_trip() {
    assert_eq!(day(1970, 1, 1), Day(0));
    assert_eq!(day(2000, 3, 1).ymd(), (2000, 3, 1));
    for n in [-800_000, -1, 0, 1, 10_957, 20_000, 20_454, 30_000] {
        let (y, m, d) = Day(n).ymd();
        assert_eq!(Day::from_ymd(y, m, d), Some(Day(n)));
    }
    assert_eq!(Day::from_ymd(2027, 2, 29), None);
    assert_eq!(Day::from_ymd(2028, 2, 29).map(Day::iso).as_deref(), Some("2028-02-29"));
    assert_eq!(Day::parse("2027-01-20"), Some(day(2027, 1, 20)));
    assert_eq!(Day::parse("2027-13-01"), None);
}

#[test]
fn weekdays_start_on_saturday() {
    assert_eq!(day(2026, 10, 31).weekday(), 0); // a Saturday
    assert_eq!(day(2026, 10, 30).weekday(), 6); // a Friday
    assert_eq!(day(2027, 1, 1).weekday(), 6); // a Friday
}

#[test]
fn study_day_rolls_over_at_four() {
    // 2026-10-27 01:30 in Cairo (UTC+2) still counts for the 26th.
    let ms = (day(2026, 10, 26).0 as i64) * 86_400_000 + 23 * 3_600_000 + 30 * 60_000;
    assert_eq!(Day::from_unix_ms(ms, 120, 4), day(2026, 10, 26));
    let ms_after = ms + 3 * 3_600_000; // 04:30 Cairo
    assert_eq!(Day::from_unix_ms(ms_after, 120, 4), day(2026, 10, 27));
}

#[test]
fn no_date_no_plan() {
    let p = plan(&PlanSettings::default(), PlanInput { today: Day(0), lectures: &[], review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::NoExamDate);
    assert!(p.days.is_empty());
}

#[test]
fn exam_past_or_today() {
    let today = day(2027, 1, 20);
    let p = plan(&settings(today), PlanInput { today, lectures: &[], review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::ExamDone);
}

#[test]
fn no_weekday_ticked() {
    let mut s = settings(day(2027, 1, 20));
    s.weekdays = [false; 7];
    let p = plan(&s, PlanInput { today: day(2026, 10, 27), lectures: &[], review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::NoStudyDays);
}

#[test]
fn spreads_new_items_and_keeps_dedicated_free() {
    let today = day(2026, 10, 27);
    let exam = day(2027, 3, 20);
    let lectures: Vec<_> = (1..=20).map(|n| lecture(&format!("cvs.L{n:02}"), 75, 0)).collect();
    let p = plan(&settings(exam), PlanInput { today, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::OnTrack);
    assert_eq!(p.days.len(), today.until(exam) as usize);
    let dedicated_from = exam.plus(-42);
    assert_eq!(p.dedicated_from, Some(dedicated_from));
    let new: u32 = p.days.iter().map(|d| d.new).sum();
    assert_eq!(new, 1500);
    // No new items in dedicated, the second pass or on self-assessment
    // days, nor on Fridays.
    for d in &p.days {
        if d.day >= dedicated_from || d.day.weekday() == 6 || matches!(d.focus, Focus::Revisit | Focus::Mock)
        {
            assert_eq!(d.new, 0, "{d:?}");
        }
        assert!(d.reviews <= 150, "{d:?}");
    }
    // Dedicated is qbank blocks and weak topics, with weekly self-assessments.
    let dedicated: Vec<&DayPlan> =
        p.days.iter().filter(|d| d.day >= dedicated_from && d.focus != Focus::Rest).collect();
    assert!(dedicated.iter().all(|d| d.phase == Phase::Dedicated));
    assert!(dedicated.iter().all(|d| matches!(d.focus, Focus::WeakTopics | Focus::Mock)), "{dedicated:?}");
    assert!(dedicated.iter().filter(|d| d.focus == Focus::Mock).count() >= 5);
    // Every lecture gets a revisit, after its own first pass.
    for l in &lectures {
        let revisit = p.days.iter().find(|d| d.revisit.contains(&l.id)).expect("revisit");
        let last_new = p.days.iter().rfind(|d| d.new_lectures.contains(&l.id)).expect("learned");
        assert!(revisit.day > last_new.day, "{}", l.id);
    }
    // Lectures are learned in book order.
    let order: Vec<&String> = p.days.iter().flat_map(|d| &d.new_lectures).collect();
    assert_eq!(order.first().map(|s| s.as_str()), Some("cvs.L01"));
    assert_eq!(order.last().map(|s| s.as_str()), Some("cvs.L20"));
}

#[test]
fn started_lectures_come_first() {
    let today = day(2026, 10, 27);
    let lectures = [lecture("cvs.L04", 50, 0), lecture("cvs.L05", 50, 10)];
    let p = plan(&settings(day(2027, 1, 20)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.days.iter().find(|d| d.new > 0).unwrap().new_lectures[0], "cvs.L05");
    assert_eq!(p.seen, 10);
}

#[test]
fn heavy_load_says_tight_and_suggests_days() {
    let today = day(2026, 10, 27);
    let mut s = settings(day(2026, 12, 15));
    s.weekdays = [true, false, true, false, true, false, false];
    let lectures = [lecture("cvs.L04", 3000, 0)];
    let p = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
    let PlanStatus::Tight { left_over, fix, .. } = p.status else { panic!("{:?}", p.status) };
    assert!(left_over > 0);
    if let TightFix::MoreDays(n) = fix {
        assert!((1..=4).contains(&n));
        let mut more = s.clone();
        let free: Vec<usize> = (0..7).filter(|&w| !s.weekdays[w]).collect();
        for w in &free[..n as usize] {
            more.weekdays[*w] = true;
        }
        assert_ne!(
            plan(&more, PlanInput { today, lectures: &lectures, review_forecast: &[] }).status,
            p.status
        );
    }
}

#[test]
fn suggests_a_higher_cap_only_when_it_fits() {
    let today = day(2026, 10, 27);
    let mut s = settings(day(2026, 12, 15));
    s.weekdays = [true; 7];
    s.review_cap = 60;
    let input =
        |l: &[LectureProgress]| plan(&s, PlanInput { today, lectures: l, review_forecast: &[] }).status;
    let some = [lecture("cvs.L04", 600, 0)];
    let PlanStatus::Tight { fix: TightFix::HigherCap(cap), .. } = input(&some) else {
        panic!("{:?}", input(&some))
    };
    assert!(cap > 60 && cap <= MAX_REVIEW_CAP && cap % 50 == 0);
    let mut raised = s.clone();
    raised.review_cap = cap;
    assert!(!matches!(
        plan(&raised, PlanInput { today, lectures: &some, review_forecast: &[] }).status,
        PlanStatus::Tight { .. }
    ));
    let huge = [lecture("cvs.L04", 100_000, 0)];
    assert!(matches!(input(&huge), PlanStatus::Tight { fix: TightFix::HighYield, .. }));
}

#[test]
fn far_exam_days_are_clamped() {
    let today = day(2026, 10, 27);
    assert!(exam_day_ok(today, day(2027, 1, 20)));
    assert!(!exam_day_ok(today, today));
    assert!(!exam_day_ok(today, day(2207, 1, 20)));
    let lectures = [lecture("cvs.L04", 100, 0)];
    let p = plan(&settings(day(9999, 1, 1)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.days.len(), MAX_DAYS_AHEAD as usize);
}

#[test]
fn forecast_reviews_count_against_the_cap() {
    let today = day(2026, 10, 27);
    let lectures = [lecture("cvs.L04", 300, 0)];
    let s = settings(day(2027, 1, 20));
    let quiet = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
    let busy_forecast = vec![140; 30];
    let busy = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &busy_forecast });
    let first_new = |p: &Plan| p.days.iter().position(|d| d.new > 0).unwrap();
    assert!(first_new(&busy) >= first_new(&quiet));
    for d in &busy.days {
        assert!(d.reviews <= 150 || d.new == 0, "{d:?}");
    }
}

#[test]
fn rest_day_reviews_roll_to_the_next_study_day() {
    let friday = day(2026, 10, 30);
    let s = settings(day(2027, 1, 20));
    let p = plan(&s, PlanInput { today: friday, lectures: &[], review_forecast: &[20, 5] });
    assert_eq!(p.days[0].focus, Focus::Rest);
    assert_eq!(p.days[1].reviews, 25);
    assert_eq!(p.next_study_day().map(|d| d.day), Some(friday.plus(1)));
}

#[test]
fn inside_dedicated_only_self_assessments_and_weak_topics() {
    let exam = day(2027, 1, 20);
    let today = exam.plus(-6);
    let lectures = [lecture("cvs.L04", 40, 10)];
    let p = plan(&settings(exam), PlanInput { today, lectures: &lectures, review_forecast: &[] });
    let later = Some(exam.plus(6 * 7));
    assert_eq!(p.status, PlanStatus::Tight { left_over: 30, fix: TightFix::Dedicated, later_exam: later });
    assert!(p.days.iter().all(|d| matches!(d.focus, Focus::Mock | Focus::WeakTopics | Focus::Rest)));
    let done = [lecture("cvs.L04", 40, 40)];
    let p = plan(&settings(exam), PlanInput { today, lectures: &done, review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::Dedicated);
}

#[test]
fn everything_seen_means_reviews_only() {
    let today = day(2026, 10, 27);
    let lectures = [LectureProgress { id: "cvs.L04".into(), total: 40, seen: 40, seen_twice: 40 }];
    let p = plan(&settings(day(2027, 1, 20)), PlanInput { today, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::OnTrack);
    assert!(p.days.iter().all(|d| d.new == 0));
    assert_eq!(p.seen_twice, 40);
}

/// About First Aid's size: 77 lectures of 60 items.
fn first_aid() -> Vec<LectureProgress> {
    (1..=77).map(|n| lecture(&format!("fa.L{n:02}"), 60, 0)).collect()
}

fn september(today: Day) -> PlanSettings {
    let mut s = settings(day(2027, 9, 15));
    s.started = Some(today);
    s
}

#[test]
fn phases_for_a_september_exam() {
    let today = day(2026, 11, 4);
    let exam = day(2027, 9, 15);
    let s = september(today);
    let lectures = first_aid();
    let p = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.status, PlanStatus::OnTrack);
    assert_eq!(p.dedicated_from, Some(day(2027, 8, 4)), "six weeks before the exam");
    let integration = p.integration_from.expect("a second pass");
    assert!(integration > day(2027, 4, 1) && integration < day(2027, 6, 1), "{}", integration.iso());
    // Phases run in order, each on its own dates.
    assert!(p.days.windows(2).all(|w| w[0].phase <= w[1].phase));
    for d in &p.days {
        let want = if d.day >= day(2027, 8, 4) {
            Phase::Dedicated
        } else if d.day >= integration {
            Phase::Integration
        } else {
            Phase::Systems
        };
        assert_eq!(d.phase, want, "{}", d.day.iso());
    }
    // Qbank questions grow with the phase: 20, 40 and 80 a normal day.
    let qbank = |phase| p.days.iter().find(|d| d.phase == phase && d.focus != Focus::Mock && d.qbank > 0);
    assert_eq!(qbank(Phase::Systems).map(|d| d.qbank), Some(20));
    assert_eq!(qbank(Phase::Integration).map(|d| d.qbank), Some(40));
    assert_eq!(qbank(Phase::Dedicated).map(|d| d.qbank), Some(80));
    assert!(p.days.iter().filter(|d| matches!(d.focus, Focus::Rest | Focus::Mock)).all(|d| d.qbank == 0));
    // Self-assessments: baseline today, two checkpoints, weekly in
    // dedicated, Free 120 four days out (the fifth is a Friday off).
    let marks: Vec<(String, MilestoneKind)> =
        p.days.iter().filter_map(|d| d.milestone.map(|m| (d.day.iso(), m))).collect();
    use MilestoneKind::*;
    let want: Vec<(String, MilestoneKind)> = [
        ("2026-11-04", Baseline),
        ("2027-02-03", Checkpoint),
        ("2027-05-05", Checkpoint),
        ("2027-08-04", Weekly),
        ("2027-08-11", Weekly),
        ("2027-08-18", Weekly),
        ("2027-08-25", Weekly),
        ("2027-09-01", Weekly),
        ("2027-09-11", Free120),
    ]
    .iter()
    .map(|(d, k)| (d.to_string(), *k))
    .collect();
    assert_eq!(marks, want);
    assert_eq!(milestones(&s, today, exam).len(), want.len());
    for d in p.days.iter().filter(|d| d.milestone.is_some()) {
        assert_eq!((d.focus, d.new), (Focus::Mock, 0), "{}", d.day.iso());
    }
    assert_eq!(p.next_milestone(), Some(Milestone { day: today, kind: Baseline }));
    // A month on, the baseline is behind him and the next is the checkpoint.
    let later = today.plus(30);
    let p = plan(&s, PlanInput { today: later, lectures: &lectures, review_forecast: &[] });
    assert_eq!(p.next_milestone().map(|m| (m.day.iso(), m.kind)), Some(("2027-02-03".into(), Checkpoint)));
}

#[test]
fn a_plan_without_a_start_day_keeps_only_dedicated_milestones() {
    let today = day(2026, 11, 4);
    let exam = day(2027, 9, 15);
    let marks = milestones(&settings(exam), today, exam);
    let kinds: Vec<MilestoneKind> = marks.iter().map(|m| m.kind).collect();
    assert_eq!(kinds.first(), Some(&MilestoneKind::Weekly));
    assert_eq!(kinds.last(), Some(&MilestoneKind::Free120));
    assert_eq!(marks.len(), 6);
    // Inside dedicated, too.
    let late = day(2027, 8, 20);
    assert_eq!(milestones(&settings(exam), late, exam).first().map(|m| m.day), Some(day(2027, 8, 4)));
}

#[test]
fn dedicated_length_is_picked_in_weeks() {
    let exam = day(2027, 9, 15);
    let mut s = settings(exam);
    s.dedicated_weeks = 4;
    assert_eq!(s.dedicated_from(exam), exam.plus(-28));
    s.dedicated_weeks = 20; // out of range: clamped to 8
    assert_eq!(s.dedicated_from(exam), exam.plus(-56));
    s.dedicated_weeks = 1;
    assert_eq!(s.dedicated_from(exam), exam.plus(-28));
}

#[test]
fn tight_plans_suggest_a_later_exam_instead_of_a_shorter_dedicated() {
    let today = day(2026, 11, 4);
    let exam = day(2027, 2, 1);
    let s = settings(exam);
    let lectures = first_aid();
    let input = PlanInput { today, lectures: &lectures, review_forecast: &[] };
    let PlanStatus::Tight { later_exam: Some(later), .. } = plan(&s, input).status else {
        panic!("{:?}", plan(&s, input).status)
    };
    assert_eq!(today.until(later) % 7, today.until(exam) % 7, "whole weeks later");
    let mut moved = s.clone();
    moved.exam = Some(later);
    let fits = plan(&moved, input);
    assert!(!matches!(fits.status, PlanStatus::Tight { .. }), "{:?}", fits.status);
    assert_eq!(fits.dedicated_from, Some(later.plus(-42)), "dedicated keeps its six weeks");
    // A week earlier than the suggestion still doesn't fit.
    moved.exam = Some(later.plus(-7));
    assert!(later.plus(-7) == exam || matches!(plan(&moved, input).status, PlanStatus::Tight { .. }));
    // Far too much for any date in range: no suggestion.
    let huge = [lecture("fa.L01", 1_000_000, 0)];
    let p = plan(&s, PlanInput { today, lectures: &huge, review_forecast: &[] });
    assert!(matches!(p.status, PlanStatus::Tight { later_exam: None, .. }));
}

#[test]
fn not_ready_suggests_four_more_weeks_only_near_dedicated() {
    let exam = day(2027, 9, 15);
    let s = settings(exam);
    assert_eq!(later_exam_if_not_ready(&s, day(2027, 3, 1)), None, "early scores are expected to be low");
    assert_eq!(later_exam_if_not_ready(&s, day(2027, 7, 21)), Some(day(2027, 10, 13)));
    assert_eq!(later_exam_if_not_ready(&s, day(2027, 9, 10)), Some(day(2027, 10, 13)));
    assert_eq!(later_exam_if_not_ready(&s, exam), None);
    assert_eq!(later_exam_if_not_ready(&PlanSettings::default(), exam), None);
}

#[test]
fn day_weights_split_new_items_and_qbank() {
    let today = day(2026, 11, 7); // a Saturday
    let mut s = september(today);
    s.review_cap = MAX_REVIEW_CAP;
    s.day_loads[0] = DayLoad::Light; // Saturday
    s.day_loads[1] = DayLoad::Heavy; // Sunday
    let lectures = first_aid();
    let p = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &[] });
    let week: Vec<&DayPlan> = p.days.iter().take(14).filter(|d| d.focus == Focus::Learn).collect();
    let normal = week.iter().find(|d| d.day.weekday() == 2).unwrap();
    for d in &week {
        match d.day.weekday() {
            0 => {
                assert!(d.new * 2 <= normal.new + 1, "light {} vs normal {}", d.new, normal.new);
                assert_eq!(d.qbank, 10);
            }
            1 => {
                assert!(d.new * 2 + 2 >= normal.new * 3, "heavy {} vs normal {}", d.new, normal.new);
                assert_eq!(d.qbank, 30);
            }
            _ => assert_eq!(d.qbank, 20),
        }
    }
    let total: u32 = p.days.iter().map(|d| d.new).sum();
    assert_eq!(total, 77 * 60);
    assert_eq!(DayLoad::Light.scale(25), 13);
    assert_eq!(DayLoad::Heavy.scale(25), 38);
}

#[test]
fn catch_up_days_and_vacations_take_no_new_work() {
    let today = day(2026, 11, 4);
    let mut s = september(today);
    s.catch_up_day = Some(5); // Thursday
    let trip = Vacation { from: day(2026, 12, 20), to: day(2027, 1, 2) };
    s.vacations.push(trip);
    let lectures = first_aid();
    let forecast = vec![10; 400];
    let p = plan(&s, PlanInput { today, lectures: &lectures, review_forecast: &forecast });
    assert!(!matches!(p.status, PlanStatus::Tight { .. }), "{:?}", p.status);
    for d in &p.days {
        if trip.contains(d.day) {
            assert_eq!((d.focus, d.new, d.reviews, d.qbank), (Focus::Rest, 0, 0, 0), "{}", d.day.iso());
        }
        if d.day.weekday() == 5 && d.day < day(2027, 8, 4) && d.milestone.is_none() && !trip.contains(d.day) {
            assert_eq!(d.focus, Focus::CatchUp, "{}", d.day.iso());
            assert_eq!((d.new, d.qbank), (0, 0));
            assert!(d.reviews > 0, "reviews still come on a catch-up day");
        }
    }
    // The vacation's reviews wait for the first study day after it.
    let back = p.days.iter().find(|d| d.day > trip.to && d.focus != Focus::Rest).unwrap();
    assert!(back.reviews >= 14 * 10, "{back:?}");
    // Reminders and the plan agree on what a study day is.
    assert!(!s.is_study_day(day(2026, 12, 25)));
    assert!(s.is_study_day(day(2027, 1, 3)));
    assert!(!s.is_catch_up_day(day(2026, 11, 6)), "Friday is no catch-up day");
}

#[test]
fn missed_items_go_to_the_catch_up_day_and_undo_spreads_them_again() {
    let today = day(2026, 11, 7); // Saturday; the catch-up day is Thursday the 12th
    let mut s = september(today);
    s.catch_up_day = Some(5);
    let lectures = first_aid();
    let input = PlanInput { today, lectures: &lectures, review_forecast: &[] };
    let spread = plan(&s, input);
    let mut moved = s.clone();
    moved.rebalance = Some(Rebalance { on: today, missed: 30, to_catch_up: true });
    let to_catch_up = plan(&moved, input);
    let thursday = |p: &Plan| p.days.iter().find(|d| d.day == day(2026, 11, 12)).unwrap().clone();
    assert_eq!(thursday(&spread).new, 0);
    assert_eq!(thursday(&to_catch_up).new, 30);
    assert!(!thursday(&to_catch_up).new_lectures.is_empty());
    let sum = |p: &Plan| p.days.iter().map(|d| d.new).sum::<u32>();
    assert_eq!(sum(&spread), sum(&to_catch_up));
    // The other days give back exactly what moved.
    let others = |p: &Plan| p.days.iter().filter(|d| d.day != day(2026, 11, 12)).map(|d| d.new).sum::<u32>();
    assert_eq!(others(&spread) - others(&to_catch_up), 30);
    // Lectures stay in book order across the catch-up day.
    let order: Vec<&String> = to_catch_up.days.iter().flat_map(|d| &d.new_lectures).collect();
    assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");
    // Once the catch-up day has passed, the choice no longer applies.
    let next_week = PlanInput { today: day(2026, 11, 14), ..input };
    assert_eq!(plan(&moved, next_week), plan(&s, next_week));
    // Undo: spreading again gives the spread plan back.
    moved.rebalance = Some(Rebalance { on: today, missed: 30, to_catch_up: false });
    assert_eq!(plan(&moved, input), spread);
    moved.rebalance = None;
    assert_eq!(plan(&moved, input), spread);
}

#[test]
fn older_settings_rows_still_load() {
    let old =
        r#"{"exam":20711,"weekdays":[true,true,true,true,true,true,false],"review_cap":150,"final_days":10}"#;
    let s: PlanSettings = serde_json::from_str(old).unwrap();
    assert_eq!(s.exam, Some(Day(20711)));
    assert_eq!(s.dedicated_weeks, DEFAULT_DEDICATED_WEEKS);
    assert_eq!(s.qbank_per_day, DEFAULT_QBANK_PER_DAY);
    assert_eq!(s.day_loads, [DayLoad::Normal; 7]);
    assert_eq!((s.started, s.catch_up_day, s.rebalance), (None, None, None));
    let mut s = s;
    s.set_exam(Day(20800), Day(20500));
    s.set_exam(Day(20900), Day(20600));
    assert_eq!((s.exam, s.started), (Some(Day(20900)), Some(Day(20500))), "the start stays put");
    let round: PlanSettings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(round, s);
}
