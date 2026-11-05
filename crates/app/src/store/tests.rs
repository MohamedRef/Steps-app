use steps_core::review::verify::{Edit, FieldValue, Verdict};
use steps_core::srs::Retention;

use super::*;

fn ev(item: &str, grade: Grade, at_ms: i64) -> ReviewEvent {
    ReviewEvent::new(item, grade, Origin::Study, at_ms, 3000)
}

fn judgement(verdict: Verdict) -> Judgement {
    Judgement { verdict, edit: Edit::new(), note: None, fingerprint: "f".into(), at_ms: 1 }
}

#[test]
fn answers_logged_before_the_tags_read_without_them() {
    // A database from the build before confidence and reasons.
    let conn = Connection::open_in_memory().unwrap();
    for sql in &MIGRATIONS[..4] {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 4).unwrap();
    conn.execute(
            "INSERT INTO review_events (id, item_id, grade, origin, at_ms, took_ms, device_id, hlc_wall, hlc_counter)
             VALUES ('old-1', 'hb.L01.001', 'again', 'bank', 5, 900, 'old', 5, 0)",
            [],
        )
        .unwrap();
    let s = Store::init(conn, None, None).unwrap();
    let log = s.reviews().unwrap();
    assert_eq!(log[0].event, ReviewEvent::new("hb.L01.001", Grade::Again, Origin::Bank, 5, 900));
    // And a new answer keeps its tags.
    let tagged = ReviewEvent {
        confidence: Some(Confidence::Guess),
        reason: Some(ErrorReason::Timing),
        ..ev("hb.L01.002", Grade::Again, 10)
    };
    s.append_review(tagged.clone()).unwrap();
    assert_eq!(s.reviews().unwrap()[1].event, tagged);
    // A value from a newer build reads as unset, and the answer still counts.
    s.conn.lock().unwrap().execute("UPDATE review_events SET reason = 'future'", []).unwrap();
    assert_eq!(s.reviews().unwrap()[1].event.reason, None);
    assert_eq!(s.reviews().unwrap().len(), 2);
}

#[test]
fn settings_by_prefix() {
    let s = Store::open_in_memory().unwrap();
    s.put_setting("bank.paper.a.1", &1).unwrap();
    s.put_setting("bank.paper.b.2", &2).unwrap();
    s.put_setting("bank.papers", &vec![9]).unwrap();
    s.put_setting("other", &3).unwrap();
    let got: Vec<(String, i32)> = s.settings_with_prefix("bank.paper.");
    assert_eq!(got, vec![("bank.paper.a.1".into(), 1), ("bank.paper.b.2".into(), 2)]);
}

#[test]
fn a_write_and_its_outbox_entry_land_together() {
    let s = Store::open_in_memory().unwrap();
    s.conn.lock().unwrap().execute("DROP TABLE sync_outbox", []).unwrap();
    assert!(s.append_review(ev("cvs.L04.001", Grade::Good, 10)).is_err());
    assert!(s.reviews().unwrap().is_empty(), "the answer rolls back with its outbox entry");
    assert!(s.set_judgement("cvs.L04.001", Some(&judgement(Verdict::Approved))).is_err());
    assert!(s.judgements().unwrap().is_empty());
}

#[test]
fn zone_matches_the_system_offset_now() {
    assert_eq!(zone().offset_at(now_ms()), utc_offset_minutes());
    assert!(zone().changes.windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn an_own_card_from_a_clock_ahead_moves_this_clock_past_it() {
    let dir = std::env::temp_dir().join(format!("steps-store-{}", new_device_id()));
    let path = dir.join("steps.db");
    let card = |front: &str| steps_core::own::OwnCard {
        id: "own.t".into(),
        front: front.into(),
        back: "A".into(),
        lecture: None,
        page: None,
        source: None,
        created_ms: 1,
    };
    {
        // The tablet, two minutes ahead, wrote the card; it synced here.
        let s = Store::open(&path).unwrap();
        let ahead = Hlc { wall_ms: now_ms() + 120_000, counter: 0, device: "tablet".into() };
        let value = serde_json::to_string(&card("Q")).unwrap();
        s.put_lww("own_cards", "id", "own.t", None, &value, false, &ahead).unwrap();
    }
    // Edited here right after a restart, the edit still wins (#257).
    let s = Store::open(&path).unwrap();
    s.put_own_card(&card("Q edited")).unwrap();
    assert_eq!(s.own_cards()[0].front, "Q edited");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reviews_are_appended_in_order_and_survive_reopening() {
    let dir = std::env::temp_dir().join(format!("steps-store-{}", new_device_id()));
    let path = dir.join("steps.db");
    let device;
    {
        let s = Store::open(&path).unwrap();
        device = s.device();
        s.append_review(ev("cvs.L04.001", Grade::Good, 10)).unwrap();
        s.append_review(ev("cvs.L04.002", Grade::Again, 10)).unwrap();
    }
    let s = Store::open(&path).unwrap();
    assert_eq!(s.device(), device);
    let log = s.reviews().unwrap();
    assert_eq!(log.len(), 2);
    assert!(log[0].hlc < log[1].hlc);
    assert_eq!(log[1].event.grade, Grade::Again);
    // A re-inserted event (sync echo) is not duplicated.
    insert_review(&s.conn.lock().unwrap(), &log[0]).unwrap();
    assert_eq!(s.reviews().unwrap().len(), 2);
    // New stamps sort after the stored ones.
    let next = s.append_review(ev("cvs.L04.003", Grade::Good, 0)).unwrap();
    assert!(next.hlc > log[1].hlc);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn verdicts_are_last_writer_wins_and_can_be_undone() {
    let s = Store::open_in_memory().unwrap();
    s.set_judgement("cvs.L04.001", Some(&judgement(Verdict::Wrong))).unwrap();
    let mut edited = judgement(Verdict::Edited);
    edited.edit.insert("back".into(), FieldValue::Text("mine".into()));
    s.set_judgement("cvs.L04.001", Some(&edited)).unwrap();
    assert_eq!(s.judgements().unwrap()["cvs.L04.001"], edited);
    // An older write (a late sync row) loses.
    let old = Hlc { wall_ms: 0, counter: 0, device: "tab".into() };
    s.put_lww("item_status", "item_id", "cvs.L04.001", Some("approved"), "{}", false, &old).unwrap();
    assert_eq!(s.judgements().unwrap()["cvs.L04.001"].verdict, Verdict::Edited);
    // Undo leaves a tombstone: unverified again.
    s.set_judgement("cvs.L04.001", None).unwrap();
    assert!(s.judgements().unwrap().is_empty());
}

#[test]
fn generic_and_device_only_settings() {
    let s = Store::open_in_memory().unwrap();
    assert_eq!(s.setting::<u32>("sound"), None);
    s.put_setting("sound", &2u32).unwrap();
    s.put_setting("sound", &3u32).unwrap();
    assert_eq!(s.setting::<u32>("sound"), Some(3));
    // Saved on every answer, a lesson's place still sends one op.
    let before = s.outbox_len();
    for at in 0..5 {
        s.put_place("lessonpos.y", &at).unwrap();
    }
    assert_eq!(s.outbox_len(), before + 1);
    assert_eq!(s.outbox(100).unwrap().last().unwrap().payload.as_ref().unwrap()["value"], "4");
    // A place syncs and replaces this device's old local copy.
    s.put_local_setting("lessonpos.x", &1).unwrap();
    s.put_place("lessonpos.x", &2).unwrap();
    assert_eq!(s.place::<i32>("lessonpos.x"), Some(2));
    assert_eq!(s.setting::<i32>("lessonpos.x"), Some(2), "synced");
    assert_eq!(s.local_setting::<i32>("lessonpos.x"), None);
    // One too big for a sync op stays here, and wins here.
    let big = "x".repeat(steps_protocol::sync::MAX_PAYLOAD_BYTES);
    s.put_place("lessonpos.x", &big).unwrap();
    assert_eq!(s.place::<String>("lessonpos.x").as_deref(), Some(big.as_str()));
    assert_eq!(s.setting::<i32>("lessonpos.x"), Some(2));
    s.put_local_setting("rings", &true).unwrap();
    assert_eq!(s.local_setting::<bool>("rings"), Some(true));
    assert_eq!(s.setting::<bool>("rings"), None);
}

#[test]
fn settings_default_then_persist() {
    let s = Store::open_in_memory().unwrap();
    assert_eq!(s.review_settings(), ReviewSettings::default());
    let mine = ReviewSettings { retention: Retention::R95, new_per_day: 45, fsrs: None };
    s.put_review_settings(&mine).unwrap();
    assert_eq!(s.review_settings(), mine);
}
