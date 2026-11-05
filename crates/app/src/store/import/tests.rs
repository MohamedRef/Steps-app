use steps_core::review::verify::{Edit, Judgement, Verdict};
use steps_core::review::{Grade, Origin, ReviewEvent};
use steps_core::srs::{Retention, ReviewSettings};

use super::*;

fn ev(item: &str, at_ms: i64) -> ReviewEvent {
    ReviewEvent::new(item, Grade::Good, Origin::Review, at_ms, 3000)
}

fn judged(verdict: Verdict) -> Judgement {
    Judgement { verdict, edit: Edit::new(), note: None, fingerprint: "f".into(), at_ms: 1 }
}

fn round_trip(from: &Store) -> Value {
    let doc = from.export("0.1.0", 2).unwrap().document;
    parse(&serde_json::to_string(&doc).unwrap()).unwrap()
}

#[test]
fn restores_onto_a_fresh_device() {
    let phone = Store::open_in_memory().unwrap();
    phone.append_review(ev("cvs.L04.001", 10)).unwrap();
    phone.append_review(ev("cvs.L04.002", 20)).unwrap();
    phone.set_judgement("cvs.L04.001", Some(&judged(Verdict::Approved))).unwrap();
    phone.put_review_settings(&ReviewSettings { retention: Retention::R95, ..Default::default() }).unwrap();
    let doc = round_trip(&phone);

    let tablet = Store::open_in_memory().unwrap();
    let plan = tablet.plan_import(&doc).unwrap();
    assert_eq!(plan.device_id, phone.device());
    assert_eq!(plan.table("review_events").unwrap().new, 2);
    assert_eq!(plan.table("item_status").unwrap().new, 1);
    assert!(plan.skipped.contains(&"meta".to_string()));
    tablet.apply_import(&doc).unwrap();

    assert_eq!(tablet.reviews().unwrap(), phone.reviews().unwrap());
    assert_eq!(tablet.judgements().unwrap(), phone.judgements().unwrap());
    assert_eq!(tablet.review_settings().retention, Retention::R95);
    // The tablet keeps its own identity, and its next stamp sorts last.
    assert_ne!(tablet.device(), phone.device());
    let next = tablet.append_review(ev("cvs.L04.003", 0)).unwrap();
    assert!(phone.reviews().unwrap().iter().all(|l| l.hlc < next.hlc));
    // Importing again changes nothing.
    assert_eq!(tablet.plan_import(&doc).unwrap().changes(), 0);
    tablet.apply_import(&doc).unwrap();
    assert_eq!(tablet.reviews().unwrap().len(), 3);
}

#[test]
fn rows_that_fail_are_counted_not_reported_as_merged() {
    let phone = Store::open_in_memory().unwrap();
    phone.append_review(ev("cvs.L04.001", 10)).unwrap();
    phone.append_review(ev("cvs.L04.002", 20)).unwrap();
    phone.append_review(ev("cvs.L04.003", 30)).unwrap();
    let mut doc = round_trip(&phone);
    let rows = doc["tables"]["review_events"]["rows"].as_array_mut().unwrap();
    // One row this build can't take (a NOT NULL column missing), one
    // with no HLC at all.
    rows[0].as_object_mut().unwrap().remove("item_id");
    rows[1].as_object_mut().unwrap().remove("hlc_wall");

    let tablet = Store::open_in_memory().unwrap();
    assert_eq!(tablet.plan_import(&doc).unwrap().table("review_events").unwrap().new, 2);
    let done = tablet.apply_import(&doc).unwrap();
    let t = done.table("review_events").unwrap();
    assert_eq!((t.rows, t.new, t.newer, t.failed), (3, 1, 0, 2));
    assert_eq!(done.changes(), 1);
    assert_eq!(done.failed(), 2);
    assert_eq!(tablet.reviews().unwrap().len(), 1);
}

#[test]
fn a_restore_goes_on_to_steps_cloud() {
    let phone = Store::open_in_memory().unwrap();
    let answer = phone.append_review(ev("cvs.L04.001", 10)).unwrap();
    phone.set_judgement("cvs.L04.001", Some(&judged(Verdict::Approved))).unwrap();
    let doc = round_trip(&phone);

    let tablet = Store::open_in_memory().unwrap();
    let before = tablet.outbox(100).unwrap().len();
    tablet.apply_import(&doc).unwrap();
    let outbox = tablet.outbox(100).unwrap();
    let relayed: Vec<_> = outbox.iter().filter(|o| o.relay).collect();
    assert!(relayed.iter().any(|o| o.table == "review_events" && o.row_id == answer.id));
    assert!(relayed.iter().any(|o| o.table == "item_status" && o.row_id == "cvs.L04.001"));
    assert!(relayed.iter().all(|o| o.hlc.device == phone.device()));
    // The ids match the phone's own ops, so the hub takes each only once.
    let phone_ids: Vec<_> = phone.outbox(100).unwrap().into_iter().map(|o| o.id).collect();
    assert!(relayed.iter().all(|o| phone_ids.contains(&o.id)), "{relayed:?} vs {phone_ids:?}");
    // A second restore queues nothing more.
    let after = outbox.len();
    assert!(after > before);
    tablet.apply_import(&doc).unwrap();
    assert_eq!(tablet.outbox(100).unwrap().len(), after);
}

#[test]
fn merges_by_hlc_and_never_deletes_local_rows() {
    let old = Store::open_in_memory().unwrap();
    old.set_judgement("cvs.L04.001", Some(&judged(Verdict::Wrong))).unwrap();
    old.set_judgement("cvs.L04.002", Some(&judged(Verdict::Wrong))).unwrap();
    old.append_review(ev("cvs.L04.009", 5)).unwrap();
    let backup = round_trip(&old);

    // Later, on another device: a newer verdict for 001 and its own review.
    let now = Store::open_in_memory().unwrap();
    now.apply_import(&backup).unwrap();
    now.set_judgement("cvs.L04.001", Some(&judged(Verdict::Approved))).unwrap();
    now.append_review(ev("cvs.L04.004", 50)).unwrap();

    // Restoring the older backup keeps the newer verdict and the new review.
    let plan = now.plan_import(&backup).unwrap();
    assert_eq!(plan.changes(), 0);
    now.apply_import(&backup).unwrap();
    let j = now.judgements().unwrap();
    assert_eq!(j["cvs.L04.001"].verdict, Verdict::Approved);
    assert_eq!(j["cvs.L04.002"].verdict, Verdict::Wrong);
    assert_eq!(now.reviews().unwrap().len(), 2);

    // And the newer device's export wins over the old one's rows.
    let newer = round_trip(&now);
    assert_eq!(old.plan_import(&newer).unwrap().table("item_status").unwrap().newer, 1);
    old.apply_import(&newer).unwrap();
    assert_eq!(old.judgements().unwrap()["cvs.L04.001"].verdict, Verdict::Approved);
    assert_eq!(old.reviews().unwrap().len(), 2);
}

#[test]
fn rejects_files_that_are_not_steps_exports() {
    assert!(matches!(parse("not json"), Err(ImportError::NotJson(_))));
    assert_eq!(parse(r#"{"format":"kabar-export","tables":{}}"#), Err(ImportError::NotSteps));
    assert_eq!(
        parse(r#"{"format":"steps-export","format_version":9,"tables":{}}"#),
        Err(ImportError::TooNew(9))
    );
}

#[test]
fn reads_the_recorded_fixture() {
    let doc = parse(include_str!("../../../../../fixtures/export/steps-export-v1.json")).unwrap();
    let s = Store::open_in_memory().unwrap();
    let plan = s.apply_import(&doc).unwrap();
    assert_eq!(plan.table("review_events").unwrap().new, 2);
    let j = s.judgements().unwrap();
    assert_eq!(j["cvs.L04.003"].verdict, Verdict::Edited);
    assert_eq!(s.reviews().unwrap()[0].event.item_id, "cvs.L04.003");
    assert_eq!(s.review_settings().new_per_day, 45);
}
