use steps_core::review::verify::{Edit, Judgement, Verdict};
use steps_core::review::{Confidence, ErrorReason, Grade, Origin, ReviewEvent};
use steps_protocol::sync::SeqOp;

use super::*;

fn ev(item: &str, at_ms: i64) -> ReviewEvent {
    ReviewEvent::new(item, Grade::Good, Origin::Study, at_ms, 3000)
}

fn judged(verdict: Verdict) -> Judgement {
    Judgement { verdict, edit: Edit::new(), note: None, fingerprint: "f".into(), at_ms: 1 }
}

/// What the hub would send on: `from`'s outbox, numbered from `seq`.
fn relay(from: &Store, seq: u64) -> Vec<SeqOp> {
    let ops = from.outbox(500).unwrap();
    from.remove_from_outbox(ops.iter().map(|o| o.id.as_str())).unwrap();
    ops.into_iter().enumerate().map(|(i, op)| SeqOp { seq: seq + i as u64, op }).collect()
}

#[test]
fn a_build_that_syncs_a_new_table_reads_the_log_again() {
    let s = Store::open_in_memory().unwrap();
    s.set_last_seq(50).unwrap();
    let conn = s.conn.lock().unwrap();
    // Same tables as last time: the position stays.
    check_tables(&conn).unwrap();
    assert_eq!(meta_get(&conn, LAST_SEQ).unwrap().as_deref(), Some("50"));
    // The previous build had no own_cards: it skipped those ops, so start over.
    meta_set(&conn, TABLES_READ, "review_events,item_status,settings").unwrap();
    check_tables(&conn).unwrap();
    assert_eq!(meta_get(&conn, LAST_SEQ).unwrap(), None);
    assert!(meta_get(&conn, TABLES_READ).unwrap().unwrap().contains("own_cards"));
}

#[test]
fn a_row_that_does_not_merge_keeps_last_seq() {
    let tablet = Store::open_in_memory().unwrap();
    tablet.set_last_seq(7).unwrap();
    let phone = Store::open_in_memory().unwrap();
    phone.put_setting("sound", &2u32).unwrap();
    let mut ops = relay(&phone, 8);
    // A payload this build can't store (`value` is NOT NULL).
    if let Some(Value::Object(row)) = &mut ops[0].op.payload {
        row.insert("value".into(), Value::Null);
    }
    assert!(tablet.apply_remote(&ops, 8).is_err());
    assert_eq!(tablet.last_seq(), 7);
}

fn tagged(item: &str, at_ms: i64) -> ReviewEvent {
    ReviewEvent {
        confidence: Some(Confidence::Sure),
        reason: Some(ErrorReason::Misread),
        ..ReviewEvent::new(item, Grade::Again, Origin::Bank, at_ms, 3000)
    }
}

#[test]
fn confidence_and_reason_sync_and_older_builds_still_merge() {
    let phone = Store::open_in_memory().unwrap();
    phone.append_review(tagged("hb.L01.001", 10)).unwrap();
    phone.append_review(ev("hb.L01.002", 20)).unwrap();
    let ops = relay(&phone, 1);
    let payload = ops[0].op.payload.as_ref().unwrap();
    assert_eq!((payload["confidence"].as_str(), payload["reason"].as_str()), (Some("sure"), Some("misread")));
    assert!(ops[1].op.payload.as_ref().unwrap()["reason"].is_null());

    let tablet = Store::open_in_memory().unwrap();
    tablet.apply_remote(&ops, 2).unwrap();
    assert_eq!(tablet.reviews().unwrap(), phone.reviews().unwrap());
    assert_eq!(tablet.reviews().unwrap()[0].event, tagged("hb.L01.001", 10));

    // A build from before these columns keeps the answer and drops the tags.
    let old = Store::open_in_memory().unwrap();
    {
        let conn = old.conn.lock().unwrap();
        conn.execute_batch(
            "ALTER TABLE review_events DROP COLUMN confidence; ALTER TABLE review_events DROP COLUMN reason;",
        )
        .unwrap();
    }
    assert!(old.apply_remote(&ops, 2).unwrap().contains("review_events"));
    let n: i64 =
        old.conn.lock().unwrap().query_row("SELECT COUNT(*) FROM review_events", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 2);
}

#[test]
fn a_phone_answer_and_verdict_reach_the_tablet() {
    let phone = Store::open_in_memory().unwrap();
    let tablet = Store::open_in_memory().unwrap();
    phone.append_review(ev("cvs.L04.001", 10)).unwrap();
    phone.set_judgement("cvs.L04.001", Some(&judged(Verdict::Approved))).unwrap();
    phone.put_setting("sound", &2u32).unwrap();
    let ops = relay(&phone, 1);
    assert_eq!(ops.len(), 3);
    assert_eq!(ops[0].op.kind, OpKind::Append);
    assert_eq!(ops[1].op.kind, OpKind::Upsert);
    assert!(ops.iter().all(|o| o.op.hlc.device == phone.device()));
    assert!(ops[0].op.payload.as_ref().unwrap().get("account_id").is_none());

    let changed = tablet.apply_remote(&ops, 3).unwrap();
    assert_eq!(changed.into_iter().collect::<Vec<_>>(), ["item_status", "review_events", "settings"]);
    assert_eq!(tablet.reviews().unwrap(), phone.reviews().unwrap());
    assert_eq!(tablet.judgements().unwrap(), phone.judgements().unwrap());
    assert_eq!(tablet.setting::<u32>("sound"), Some(2));
    assert_eq!(tablet.last_seq(), 3);
    // Remote rows never go back out from the tablet.
    assert_eq!(tablet.outbox_len(), 0);
    // A replay changes nothing.
    assert!(tablet.apply_remote(&ops, 3).unwrap().is_empty());
    // The tablet's next stamp sorts after the phone's.
    let mine = tablet.append_review(ev("cvs.L04.002", 0)).unwrap();
    assert!(mine.hlc > ops[2].op.hlc);
}

#[test]
fn writes_from_a_clock_that_was_ahead_go_up_once_it_is_right() {
    // #539: an answer saved with the date a day ahead, then the clock is fixed.
    const DAY: i64 = 86_400_000;
    let now = crate::store::now_ms();
    let phone = Store::open_in_memory().unwrap();
    let ahead = phone.append_review(ev("cvs.L04.001", now + DAY)).unwrap();
    assert!(ahead.hlc.wall_ms >= now + DAY);
    assert_eq!(phone.restamp_future(now).unwrap(), 1);
    let ops = phone.outbox(10).unwrap();
    assert_eq!(ops.len(), 1);
    let limit = now + steps_protocol::sync::MAX_CLOCK_SKEW_MS;
    assert!(ops[0].hlc.wall_ms <= limit, "{:?}", ops[0].hlc);
    // The row carries the new stamp too, and nothing else is touched.
    let row = phone.reviews().unwrap().into_iter().find(|r| r.id == ahead.id).unwrap();
    assert_eq!(row.hlc, ops[0].hlc);
    assert_eq!(phone.restamp_future(now).unwrap(), 0);
    // The next write is not pinned to the wrong date either.
    let next = phone.append_review(ev("cvs.L04.002", now)).unwrap();
    assert!(next.hlc.wall_ms <= limit && next.hlc > row.hlc);
}

#[test]
fn a_setting_saved_while_the_clock_was_ahead_can_be_changed_once_it_is_right() {
    // #606: the row kept its future stamp, so the next write lost to it.
    const DAY: i64 = 86_400_000;
    let now = crate::store::now_ms();
    let phone = Store::open_in_memory().unwrap();
    let ahead = Hlc { wall_ms: now + DAY, counter: 0, device: phone.device() };
    phone.put_lww("settings", "key", "sound", None, "1", false, &ahead).unwrap();
    // The clock that wrote it was ahead too.
    phone.clock.lock().unwrap().observe(&ahead);
    // Signed out, so no sync ever restamps it: only the store can.
    phone.conn.lock().unwrap().execute("DELETE FROM sync_outbox", []).unwrap();
    phone.put_setting("sound", &2u32).unwrap();
    assert_eq!(phone.setting::<u32>("sound"), Some(2));

    // And on opening: a copy with the future row, reopened.
    let fresh = Store::open_in_memory().unwrap();
    let ahead = Hlc { wall_ms: now + DAY, counter: 0, device: fresh.device() };
    fresh.put_lww("settings", "key", "theme", None, "\"calm\"", false, &ahead).unwrap();
    let (_, bytes) = fresh.snapshot().unwrap();
    let reopened = Store::from_snapshot(&bytes).unwrap();
    let limit = now + steps_protocol::sync::MAX_CLOCK_SKEW_MS;
    assert!(reopened.outbox(10).unwrap().iter().all(|op| op.hlc.wall_ms <= limit));
    reopened.put_setting("theme", &"glossy").unwrap();
    assert_eq!(reopened.setting::<String>("theme").as_deref(), Some("glossy"));
}

#[test]
fn an_undone_answer_is_taken_back_on_the_other_device_once() {
    let phone = Store::open_in_memory().unwrap();
    let tablet = Store::open_in_memory().unwrap();
    let kept = phone.append_review(ev("cvs.L04.001", 10)).unwrap();
    let undone = phone.append_review(ev("cvs.L04.002", 20)).unwrap();
    tablet.apply_remote(&relay(&phone, 1), 2).unwrap();
    assert_eq!(tablet.reviews().unwrap().len(), 2);
    // Undo on the phone: one tombstone op, never a second one.
    assert!(phone.retract_review(&undone.id).unwrap());
    assert!(!phone.retract_review(&undone.id).unwrap());
    let ops = relay(&phone, 3);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].op.kind, OpKind::Delete);
    tablet.apply_remote(&ops, 3).unwrap();
    let left: Vec<String> = tablet.reviews().unwrap().into_iter().map(|l| l.id).collect();
    assert_eq!(left, std::slice::from_ref(&kept.id));
    assert_eq!(phone.reviews().unwrap().into_iter().map(|l| l.id).collect::<Vec<_>>(), [kept.id]);
}

#[test]
fn the_newest_verdict_wins_on_both_devices() {
    let phone = Store::open_in_memory().unwrap();
    let tablet = Store::open_in_memory().unwrap();
    phone.set_judgement("cvs.L04.001", Some(&judged(Verdict::Wrong))).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));
    tablet.set_judgement("cvs.L04.001", Some(&judged(Verdict::Approved))).unwrap();
    let from_phone = relay(&phone, 1);
    let from_tablet = relay(&tablet, 2);
    phone.apply_remote(&from_tablet, 2).unwrap();
    tablet.apply_remote(&from_phone, 2).unwrap();
    assert_eq!(phone.judgements().unwrap()["cvs.L04.001"].verdict, Verdict::Approved);
    assert_eq!(tablet.judgements().unwrap()["cvs.L04.001"].verdict, Verdict::Approved);
    // Undo travels as a tombstone.
    tablet.set_judgement("cvs.L04.001", None).unwrap();
    phone.apply_remote(&relay(&tablet, 3), 3).unwrap();
    assert!(phone.judgements().unwrap().is_empty());
}

#[test]
fn studying_before_sign_in_is_queued_once() {
    let dir = std::env::temp_dir().join(format!("steps-sync-{}", super::super::new_device_id()));
    let path = dir.join("steps.db");
    {
        let s = Store::open(&path).unwrap();
        s.append_review(ev("cvs.L04.001", 10)).unwrap();
        // As if the outbox came with an upgrade: empty it and forget the seed.
        let conn = s.conn.lock().unwrap();
        conn.execute("DELETE FROM sync_outbox", []).unwrap();
        conn.execute("DELETE FROM meta WHERE key = ?1", [SEEDED]).unwrap();
    }
    let s = Store::open(&path).unwrap();
    assert_eq!(s.outbox_len(), 1);
    drop(s);
    assert_eq!(Store::open(&path).unwrap().outbox_len(), 1, "only once");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn ops_for_a_table_the_hub_lacks_wait_without_blocking_the_rest() {
    let s = Store::open_in_memory().unwrap();
    s.append_review(ev("cvs.L04.001", 10)).unwrap();
    s.set_judgement("cvs.L04.001", Some(&judged(Verdict::Wrong))).unwrap();
    let all = s.outbox(10).unwrap();
    assert_eq!(all.len(), 2);
    let first = &all[0];
    let tables = s.outbox_tables([first.id.as_str(), "nope"]);
    assert_eq!(tables, std::slice::from_ref(&first.table));
    let skip: std::collections::HashSet<String> = tables.into_iter().collect();
    let rest = s.outbox_skipping(10, &skip).unwrap();
    assert_eq!(rest.iter().map(|o| &o.id).collect::<Vec<_>>(), [&all[1].id]);
    assert_eq!(s.outbox_skipping(10, &Default::default()).unwrap().len(), 2);
    // Still in the outbox, for a newer hub.
    assert_eq!(s.outbox_len(), 2);
}

#[test]
fn unknown_tables_and_kinds_are_skipped() {
    let s = Store::open_in_memory().unwrap();
    let op = Op {
        id: "x".into(),
        table: "meta".into(),
        row_id: "device_id".into(),
        kind: OpKind::Upsert,
        hlc: Hlc { wall_ms: 1, counter: 0, device: "evil".into() },
        relay: false,
        payload: Some(serde_json::json!({ "key": "device_id", "value": "stolen" })),
    };
    assert!(s.apply_remote(&[SeqOp { seq: 1, op }], 1).unwrap().is_empty());
    assert_ne!(s.device(), "stolen");
}
