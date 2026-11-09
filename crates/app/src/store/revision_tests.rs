//! The review revision behind the Study checkpoint marker follows every
//! change to the review log, made through `Store` or not (a sync merge, an
//! import), and only changes that commit.
//! Adapted from batna-app main: crates/app/src/store/revision_tests.rs.

use super::*;

fn sample(item: &str) -> ReviewEvent {
    ReviewEvent::new(item, Grade::Good, Origin::Study, 100, 20)
}

#[test]
fn the_revision_is_one_row_and_moves_with_every_committed_change() {
    let store = Store::open_in_memory().unwrap();
    let initial = store.study_review_marker().unwrap();
    assert!(initial.starts_with("v2:"));
    assert_eq!(initial.len(), 35, "v2: and a 128-bit random token");
    store.put_local_setting("theme", &"dark").unwrap();
    assert_eq!(store.study_review_marker().unwrap(), initial, "other writes leave it alone");

    let logged = store.append_review(sample("cvs.L04.001")).unwrap();
    let inserted = store.study_review_marker().unwrap();
    assert_ne!(inserted, initial);
    assert_eq!(store.study_review_marker().unwrap(), inserted, "reading it changes nothing");

    {
        let mut conn = store.conn.lock().unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute("UPDATE review_events SET took_ms = 40 WHERE id = ?1", [&logged.id]).unwrap();
        tx.rollback().unwrap();
    }
    assert_eq!(store.study_review_marker().unwrap(), inserted, "a rolled-back change is no change");

    store
        .conn
        .lock()
        .unwrap()
        .execute("UPDATE review_events SET took_ms = 41 WHERE id = ?1", [&logged.id])
        .unwrap();
    let updated = store.study_review_marker().unwrap();
    assert_ne!(updated, inserted, "a synced or imported change outside Store counts too");

    assert!(store.retract_review(&logged.id).unwrap());
    let undone = store.study_review_marker().unwrap();
    assert_ne!(updated, undone);
    assert!(!store.retract_review(&logged.id).unwrap());
    assert_eq!(store.study_review_marker().unwrap(), undone, "a second Undo writes nothing");
    assert!(store.reviews().unwrap().is_empty());

    let reopened = Store::from_snapshot(&store.snapshot().unwrap().1).unwrap();
    assert_eq!(reopened.study_review_marker().unwrap(), undone, "the browser's saved bytes keep it");
    reopened.conn.lock().unwrap().execute("DELETE FROM review_events WHERE id = ?1", [&logged.id]).unwrap();
    assert_ne!(reopened.study_review_marker().unwrap(), undone);
}

#[test]
fn old_and_new_markers_both_see_a_new_answer() {
    let store = Store::open_in_memory().unwrap();
    let old = store.study_review_marker_legacy().unwrap();
    let new = store.study_review_marker().unwrap();
    assert!(old.starts_with("v1:"));
    store.append_review(sample("cvs.L04.001")).unwrap();
    assert_ne!(store.study_review_marker_legacy().unwrap(), old);
    assert_ne!(store.study_review_marker().unwrap(), new);
}

#[test]
fn two_copies_with_as_many_answers_never_share_a_marker() {
    let store = Store::open_in_memory().unwrap();
    let bytes = store.snapshot().unwrap().1;
    let a = Store::from_snapshot(&bytes).unwrap();
    let b = Store::from_snapshot(&bytes).unwrap();
    a.append_review(sample("cvs.L04.001")).unwrap();
    b.append_review(sample("cvs.L04.002")).unwrap();
    assert_ne!(a.study_review_marker().unwrap(), b.study_review_marker().unwrap());
}

#[test]
fn a_database_from_before_the_revision_gets_one() {
    let conn = Connection::open_in_memory().unwrap();
    let before = MIGRATIONS.len() - 1;
    for sql in &MIGRATIONS[..before] {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", before as i64).unwrap();
    conn.execute(
        "INSERT INTO review_events (id, item_id, grade, origin, at_ms, took_ms, device_id, hlc_wall, hlc_counter)
         VALUES ('old-1', 'cvs.L04.001', 'good', 'study', 5, 900, 'old', 5, 0)",
        [],
    )
    .unwrap();
    let s = Store::init(conn, None, None).unwrap();
    let first = s.study_review_marker().unwrap();
    assert!(first.starts_with("v2:"));
    assert_eq!(s.reviews().unwrap().len(), 1, "the old answers stay");
    s.append_review(sample("cvs.L04.002")).unwrap();
    assert_ne!(s.study_review_marker().unwrap(), first);
}
