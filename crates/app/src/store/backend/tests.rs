//! Existing behavior regressions for the feature facade.

use steps_core::content::Lecture;
use steps_core::review::{Grade, Origin};

use super::*;

const T0: i64 = 1_793_610_000_000;

fn backend() -> ReviewBackend {
    let lec: Lecture =
        serde_yaml_ng::from_str(include_str!("../../../../../fixtures/content/sample.yaml")).unwrap();
    let catalog = lec
        .items
        .iter()
        .map(|i| (i.id.clone(), Entry { lecture: lec.lecture.clone(), item: i.clone() }))
        .collect();
    let store: &'static Store = Box::leak(Box::new(Store::open_in_memory().unwrap()));
    ReviewBackend::new(store, catalog)
}

#[test]
fn an_answer_that_isnt_saved_says_so_and_changes_nothing() {
    let mut b = backend();
    answer(&mut b, "hb.L01.001", Grade::Good, T0);
    let before = b.states.get("hb.L01.001").cloned();
    b.store.conn.lock().unwrap().execute("DROP TABLE sync_outbox", []).unwrap();
    let got = StudyStore::record_logged(
        &mut b,
        ReviewEvent {
            item_id: "hb.L01.001".into(),
            grade: Grade::Again,
            origin: Origin::Study,
            at_ms: T0 + 60_000,
            took_ms: 4000,
        },
    );
    assert!(got.is_err());
    assert_eq!(b.reviews_logged(), 1);
    assert_eq!(b.states.get("hb.L01.001").cloned(), before);
}

#[test]
fn rating_again_after_a_failed_save_clears_todays_note() {
    // An id no other test answers: Today's note is one process-wide slot.
    const ID: &str = "hb.L01.retry-note";
    let mut b = backend();
    let event = || ReviewEvent {
        item_id: ID.into(),
        grade: Grade::Good,
        origin: Origin::Study,
        at_ms: T0,
        took_ms: 4000,
    };
    b.store.conn.lock().unwrap().execute("DROP TABLE sync_outbox", []).unwrap();
    assert!(StudyStore::record_logged(&mut b, event()).is_err());
    assert!(super::super::write_error().is_some_and(|e| e.contains(ID)));
    b.store
        .conn
        .lock()
        .unwrap()
        .execute(
            "CREATE TABLE sync_outbox (seq INTEGER PRIMARY KEY AUTOINCREMENT, \
             op_id TEXT NOT NULL UNIQUE, op TEXT NOT NULL) STRICT",
            [],
        )
        .unwrap();
    assert!(StudyStore::record_logged(&mut b, event()).is_ok());
    assert_eq!(b.reviews_logged(), 1);
    assert!(super::super::write_error().is_none_or(|e| !e.contains(ID)));
}

fn answer(b: &mut ReviewBackend, id: &str, grade: Grade, at_ms: i64) {
    StudyStore::record(
        b,
        ReviewEvent { item_id: id.into(), grade, origin: Origin::Study, at_ms, took_ms: 5000 },
    );
}

#[test]
fn own_cards_join_the_catalog_and_never_ask_to_verify() {
    let mut b = backend();
    let before = b.catalog.len();
    let card = steps_core::own::OwnCard {
        id: "own.t1".into(),
        front: "SAAG?".into(),
        back: "≥ 1.1".into(),
        lecture: None,
        page: None,
        source: None,
        created_ms: T0,
    };
    b.save_own(&card).unwrap();
    assert_eq!(b.catalog.len(), before + 1);
    assert_eq!(b.entry("own.t1").unwrap().lecture, steps_core::own::LECTURE);
    assert_eq!(b.check("own.t1"), Check::Settled);
    b.delete_own("own.t1").unwrap();
    assert!(b.entry("own.t1").is_none());
    assert_eq!(b.catalog.len(), before);
}

#[test]
fn a_content_switch_rebuilds_the_catalog_and_keeps_his_cards() {
    let mut b = backend();
    let card = steps_core::own::OwnCard {
        id: "own.t2".into(),
        front: "R factor?".into(),
        back: "(ALT/ULN) / (ALP/ULN)".into(),
        lecture: None,
        page: None,
        source: None,
        created_ms: T0,
    };
    b.save_own(&card).unwrap();
    b.library_replaced();
    assert_eq!(b.catalog.len(), catalog(Library::get()).len() + 1);
    assert!(b.entry("own.t2").is_some());
}

#[test]
fn due_cases_go_to_the_cases_screen_and_mcqs_never_fall_due() {
    let mut b = backend();
    // A card, a case, a calc item and an MCQ, all missed (#242).
    for id in ["hb.L01.001", "hb.L01.006", "hb.L01.009", "hb.L01.003"] {
        answer(&mut b, id, Grade::Again, T0);
    }
    let later = T0 + 6 * 3_600_000;
    let lib = Library::get();
    let o = b.overview(lib, later);
    assert_eq!(o.due, 1, "only the card plays in a session");
    assert_eq!(o.due_cases, vec!["hb.L01.006".to_string(), "hb.L01.009".to_string()]);
    assert_eq!(b.session_due(later), vec!["hb.L01.001".to_string()]);
    // The Cases screen and oral still see the case and calc item.
    let mut all = b.due(later);
    all.sort();
    assert_eq!(all, ["hb.L01.001", "hb.L01.006", "hb.L01.009"]);
    assert_eq!(b.due_forecast(1, later), vec![3]);
}

#[test]
fn a_settings_change_keeps_a_fit_made_meanwhile() {
    let mut b = backend();
    let stale = b.settings.clone();
    // The refit thread stores a personal fit behind the backend's back.
    let mut fitted = b.store.review_settings();
    fitted.fsrs = Some(steps_core::srs::FsrsParams { weights: Vec::new(), reviews: 400, trained_at_ms: T0 });
    b.store.put_review_settings(&fitted).unwrap();
    assert!(b.set_settings(ReviewSettings { new_per_day: 45, ..stale }));
    assert_eq!(b.store.review_settings().fsrs.map(|p| p.reviews), Some(400));
    assert_eq!(b.store.review_settings().new_per_day, 45);
}

#[test]
fn undo_retracts_the_answer_and_restores_the_schedule() {
    let mut b = backend();
    answer(&mut b, "hb.L01.001", Grade::Good, T0);
    let before_state = b.states.get("hb.L01.001").cloned();
    let before_intervals = b.intervals("hb.L01.001", T0 + 60_000);
    let id = StudyStore::record_logged(
        &mut b,
        ReviewEvent {
            item_id: "hb.L01.001".into(),
            grade: Grade::Again,
            origin: Origin::Study,
            at_ms: T0 + 60_000,
            took_ms: 4000,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(b.reviews_logged(), 2);
    assert_ne!(b.states.get("hb.L01.001").cloned(), before_state);

    assert!(b.retract(&id));
    assert_eq!(b.reviews_logged(), 1);
    assert_eq!(b.states.get("hb.L01.001").cloned(), before_state);
    assert_eq!(b.intervals("hb.L01.001", T0 + 60_000), before_intervals);
    // A second Undo (a double tap, a resumed screen) writes nothing.
    let queued = b.store.outbox_len();
    assert!(b.retract(&id));
    assert_eq!(b.reviews_logged(), 1);
    assert_eq!(b.store.outbox_len(), queued);
    // And it stays gone after the store is read again (a restart).
    b.reload();
    assert_eq!(b.reviews_logged(), 1);
    assert_eq!(b.states.get("hb.L01.001").cloned(), before_state);

    // Undoing a first answer makes the item new again.
    let first = StudyStore::record_logged(
        &mut b,
        ReviewEvent {
            item_id: "hb.L01.002".into(),
            grade: Grade::Good,
            origin: Origin::Study,
            at_ms: T0,
            took_ms: 4000,
        },
    )
    .unwrap()
    .unwrap();
    assert!(!b.is_new("hb.L01.002"));
    assert!(b.retract(&first));
    assert!(b.is_new("hb.L01.002"));
}

#[test]
fn answers_schedule_and_wrong_items_hide() {
    let mut b = backend();
    assert!(b.is_new("hb.L01.001"));
    answer(&mut b, "hb.L01.001", Grade::Again, T0);
    answer(&mut b, "hb.L01.002", Grade::Good, T0);
    assert!(!b.is_new("hb.L01.001"));
    // FSRS brings an Again back about five hours later.
    let later = T0 + 6 * 3_600_000;
    assert_eq!(b.due(later), vec!["hb.L01.001".to_string()]);
    assert!(b.next_due_ms(T0).is_some());

    b.judge("hb.L01.001", Verdict::Wrong, Edit::new(), Some("  page 2 says otherwise ".into()));
    assert!(b.is_hidden("hb.L01.001"));
    assert!(b.due(later).is_empty());
    assert_eq!(b.judgement("hb.L01.001").unwrap().note.as_deref(), Some("page 2 says otherwise"));
    b.undo_judgement("hb.L01.001");
    assert_eq!(b.check("hb.L01.001"), Check::Ask);
}

#[test]
fn edits_override_what_he_studies() {
    let mut b = backend();
    let edit = Edit::from([("back".into(), verify::FieldValue::Text("Fibrosis + nodules".into()))]);
    b.judge("hb.L01.001", Verdict::Edited, edit, None);
    let item = b.entry("hb.L01.001").unwrap().item.clone();
    let Body::Card { back, .. } = b.effective(&item).body else { panic!() };
    assert_eq!(back, "Fibrosis + nodules");
    assert_eq!(b.check("hb.L01.001"), Check::Settled);
    assert_eq!(b.verdicts(T0).verdicts.len(), 1);
}

#[test]
fn stats_read_the_log_marks_and_forecast() {
    let mut b = backend();
    answer(&mut b, "hb.L01.001", Grade::Good, T0);
    answer(&mut b, "hb.L01.002", Grade::Again, T0 + 1000);
    let log = StudyStore::log(&b);
    assert_eq!(log.len(), 2);
    assert_eq!(log[1].item_id, "hb.L01.002");

    b.judge("hb.L01.001", Verdict::Edited, Edit::new(), None);
    b.judge("hb.L01.002", Verdict::Wrong, Edit::new(), None);
    let marks = b.marks();
    assert_eq!(marks.get("hb.L01.001"), Some(&Status::Approved));
    assert_eq!(marks.get("hb.L01.002"), Some(&Status::Rejected));
    assert_eq!(marks.len(), 2);

    // The ✗ item is out of the forecast; the Good one is not due today.
    let f = StudyStore::forecast(&b, 30, T0 + 2000);
    assert_eq!(f.len(), 30);
    assert_eq!(f[0], 0);
    assert_eq!(f.iter().sum::<u32>(), 1);
}

#[test]
fn counts_for_reminders_and_recap() {
    let mut b = backend();
    let day = 86_400_000;
    answer(&mut b, "hb.L01.001", Grade::Good, T0 - day);
    answer(&mut b, "hb.L01.002", Grade::Good, T0);
    answer(&mut b, "hb.L01.002", Grade::Good, T0 + 1000);
    assert_eq!(b.answers_today(T0 + 2000), 2);
    assert_eq!(b.learned_per_day(3, T0 + 2000), vec![0, 1, 1]);
    assert_eq!(b.due_forecast(30, T0).iter().sum::<u32>(), 2);
}

#[test]
fn the_daily_new_cap_counts_items_started_today() {
    let mut b = backend();
    b.set_settings(ReviewSettings { new_per_day: 15, ..Default::default() });
    answer(&mut b, "hb.L01.001", Grade::Good, T0);
    answer(&mut b, "hb.L01.001", Grade::Good, T0 + 1000);
    assert_eq!(b.new_left_today(T0 + 2000), 14);
    let i = b.intervals("hb.L01.002", T0).unwrap();
    assert!(i[0] <= i[3]);
}
