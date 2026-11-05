use serde_json::json;

use super::*;
use crate::sql::testing::MemSql;

const NOW: u64 = 1_793_011_856_000;
const NOW_I: i64 = NOW as i64;

fn conn(device: &str) -> Conn {
    Conn { device: device.into(), conn_id: format!("c-{device}"), ready: false }
}

/// A verdict on an item, as `item_status` syncs it.
fn item_op(id: &str, device: &str, wall_ms: i64, item: &str, status: &str) -> Op {
    Op {
        id: id.into(),
        table: "item_status".into(),
        row_id: item.into(),
        kind: OpKind::Upsert,
        hlc: Hlc { wall_ms, counter: 0, device: device.into() },
        relay: false,
        payload: Some(json!({ "item_id": item, "status": status })),
    }
}

fn send(hub: &Hub<'_, MemSql>, c: &mut Conn, msg: &ClientMsg) -> Outcome {
    hub.handle(c, &serde_json::to_string(msg).unwrap(), NOW).unwrap()
}

fn hello(last_seq: u64) -> ClientMsg {
    ClientMsg::Hello { protocol: PROTOCOL_VERSION, last_seq }
}

/// M2's acceptance test: a word saved on the phone reaches the tablet.
#[test]
fn word_saved_on_phone_appears_on_tablet() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    let mut tablet = conn("tab-s11");
    assert_eq!(send(&hub, &mut phone, &hello(0)).replies, vec![ServerMsg::CaughtUp { head_seq: 0 }]);
    send(&hub, &mut tablet, &hello(0));

    let op = item_op("op-1", "s24u", NOW_I, "cvs.L04.002", "learning");
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: vec![op.clone()] });
    assert_eq!(
        out.replies,
        vec![ServerMsg::Ack { batch: 1, done: vec!["op-1".into()], rejected: vec![], head_seq: 1 }]
    );
    // The Durable Object forwards this to the tablet's socket right away.
    assert_eq!(out.broadcast, vec![SeqOp { seq: 1, op: op.clone() }]);

    // A third device that was offline catches up on connect.
    let mut mac = conn("mac");
    let out = send(&hub, &mut mac, &hello(0));
    assert_eq!(
        out.replies,
        vec![
            ServerMsg::Ops { ops: vec![SeqOp { seq: 1, op }], more: false },
            ServerMsg::CaughtUp { head_seq: 1 }
        ]
    );
    assert!(out.synced);
}

#[test]
fn replays_are_idempotent() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));
    let op = item_op("op-1", "s24u", NOW_I, "cvs.L04.002", "learning");
    send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: vec![op.clone()] });
    // Lost ack → the outbox sends it again.
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 2, ops: vec![op.clone(), op] });
    assert_eq!(
        out.replies,
        vec![ServerMsg::Ack {
            batch: 2,
            done: vec!["op-1".into(), "op-1".into()],
            rejected: vec![],
            head_seq: 1
        }]
    );
    assert!(out.broadcast.is_empty());
}

#[test]
fn concurrent_edits_converge_on_the_newest() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    let mut tablet = conn("tab-s11");
    send(&hub, &mut phone, &hello(0));
    send(&hub, &mut tablet, &hello(0));
    // The tablet's later edit arrives first (the phone was offline).
    let newer = item_op("t-1", "tab-s11", NOW_I + 5, "sega", "known");
    let older = item_op("p-1", "s24u", NOW_I, "sega", "learning");
    assert_eq!(send(&hub, &mut tablet, &ClientMsg::Push { batch: 1, ops: vec![newer] }).broadcast.len(), 1);
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: vec![older] });
    // Done (drop it from the outbox) but not stored: it lost.
    assert!(matches!(&out.replies[0], ServerMsg::Ack { done, .. } if done == &["p-1".to_owned()]));
    assert!(out.broadcast.is_empty());
    assert_eq!(hub.head_seq().unwrap(), 1);
}

#[test]
fn bad_ops_are_rejected_with_a_reason() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));

    let spoofed = item_op("a", "tab-s11", NOW_I, "cvs.L04.002", "known");
    let future = item_op("b", "s24u", NOW_I + MAX_CLOCK_SKEW_MS + 1, "cvs.L04.002", "known");
    let mut unknown = item_op("c", "s24u", NOW_I, "cvs.L04.002", "known");
    unknown.table = "secrets".into();
    let mut review_edit = item_op("d", "s24u", NOW_I, "r1", "known");
    review_edit.table = "review_events".into();
    let bad_id = item_op("e f", "s24u", NOW_I, "cvs.L04.002", "known");
    let out = send(
        &hub,
        &mut phone,
        &ClientMsg::Push { batch: 9, ops: vec![spoofed, future, unknown, review_edit, bad_id] },
    );
    let ServerMsg::Ack { done, rejected, .. } = &out.replies[0] else { panic!("{out:?}") };
    assert!(done.is_empty());
    let reasons: Vec<_> = rejected.iter().map(|r| r.reason).collect();
    assert_eq!(
        reasons,
        vec![
            RejectReason::WrongDevice,
            RejectReason::ClockSkew,
            RejectReason::UnknownTable,
            RejectReason::KindNotAllowed,
            RejectReason::InvalidId
        ]
    );
}

#[test]
fn protocol_errors() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    let push = ClientMsg::Push { batch: 1, ops: vec![] };
    assert!(matches!(
        send(&hub, &mut phone, &push).replies[0],
        ServerMsg::Error { code: SyncErrorCode::HelloFirst, .. }
    ));
    assert!(matches!(
        send(&hub, &mut phone, &ClientMsg::Hello { protocol: 99, last_seq: 0 }).replies[0],
        ServerMsg::Error { code: SyncErrorCode::UnsupportedProtocol, .. }
    ));
    assert!(matches!(
        hub.handle(&mut phone, "{not json", NOW).unwrap().replies[0],
        ServerMsg::Error { code: SyncErrorCode::BadMessage, .. }
    ));
    send(&hub, &mut phone, &hello(0));
    let many =
        (0..=MAX_OPS_PER_PUSH).map(|i| item_op(&format!("o{i}"), "s24u", NOW_I, "x", "known")).collect();
    assert!(matches!(
        send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: many }).replies[0],
        ServerMsg::Error { code: SyncErrorCode::TooManyOps, .. }
    ));
    assert_eq!(send(&hub, &mut phone, &ClientMsg::Ping).replies, vec![ServerMsg::Pong]);
}

#[test]
fn catch_up_pages_and_resumes() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));
    let total = OPS_PAGE_SIZE * MAX_PAGES_PER_HELLO + 10;
    for chunk in (0..total).collect::<Vec<_>>().chunks(MAX_OPS_PER_PUSH) {
        let ops = chunk
            .iter()
            .map(|i| item_op(&format!("o{i}"), "s24u", NOW_I, &format!("w{i}"), "known"))
            .collect();
        send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops });
    }
    let replies = hub.catch_up(0).unwrap();
    assert_eq!(replies.len(), MAX_PAGES_PER_HELLO);
    assert!(matches!(&replies[0], ServerMsg::Ops { more: false, .. }));
    let Some(ServerMsg::Ops { ops, more: true }) = replies.last() else { panic!() };
    let resume = ops.last().unwrap().seq;
    // Not live until it has everything.
    let mut tablet = conn("tab");
    send(&hub, &mut tablet, &hello(0));
    assert!(!tablet.ready);
    send(&hub, &mut tablet, &hello(resume));
    assert!(tablet.ready);
    let rest = hub.catch_up(resume).unwrap();
    assert!(matches!(&rest[0], ServerMsg::Ops { ops, more: false } if ops.len() == 10));
    assert_eq!(rest[1], ServerMsg::CaughtUp { head_seq: total as u64 });
}

#[test]
fn devices_cannot_write_as_the_server() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));
    let mut op = item_op("x", "server", NOW_I, "cvs.L04.002", "known");
    op.hlc.device = "server".into();
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: vec![op] });
    let ServerMsg::Ack { rejected, .. } = &out.replies[0] else { panic!() };
    assert_eq!(rejected[0].reason, RejectReason::WrongDevice);
    let mut relayed = item_op("y", "server", NOW_I, "cvs.L04.002", "known");
    relayed.relay = true;
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 2, ops: vec![relayed] });
    let ServerMsg::Ack { rejected, .. } = &out.replies[0] else { panic!() };
    assert_eq!(rejected[0].reason, RejectReason::WrongDevice);
}

#[test]
fn a_restored_backup_relays_another_devices_rows_once() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut tab = conn("tab");
    send(&hub, &mut tab, &hello(0));
    // The old phone's verdict, restored on the tablet and passed on.
    let mut old = item_op("oldphone-5-0", "oldphone", NOW_I - 1000, "cvs.L04.002", "known");
    old.relay = true;
    let out = send(&hub, &mut tab, &ClientMsg::Push { batch: 1, ops: vec![old.clone()] });
    let ServerMsg::Ack { done, rejected, .. } = &out.replies[0] else { panic!("{out:?}") };
    assert_eq!((done.len(), rejected.len()), (1, 0));
    assert_eq!(out.broadcast[0].op.hlc.device, "oldphone");
    // The old phone signs in later and sends the same write: nothing new.
    let mut phone = conn("oldphone");
    send(&hub, &mut phone, &hello(1));
    old.relay = false;
    let out = send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops: vec![old] });
    assert!(out.broadcast.is_empty());
    // A relay never beats a newer write.
    let newer = item_op("tab-9", "tab", NOW_I, "cvs.L04.002", "unknown");
    send(&hub, &mut tab, &ClientMsg::Push { batch: 2, ops: vec![newer] });
    let mut stale = item_op("oldphone-1-0", "oldphone", NOW_I - 5000, "cvs.L04.002", "known");
    stale.relay = true;
    let out = send(&hub, &mut tab, &ClientMsg::Push { batch: 3, ops: vec![stale] });
    assert!(out.broadcast.is_empty());
}

#[test]
fn answers_and_tombstones_round_trip() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut tab = conn("tab");
    send(&hub, &mut tab, &hello(0));
    let stroke = Op {
        id: "s1".into(),
        table: "review_events".into(),
        row_id: "tab-1-0".into(),
        kind: OpKind::Append,
        hlc: Hlc { wall_ms: NOW_I, counter: 0, device: "tab".into() },
        relay: false,
        payload: Some(json!({ "item_id": "cvs.L04.001", "took_ms": 0.5 })),
    };
    let erase = Op { id: "s2".into(), kind: OpKind::Delete, payload: None, ..stroke.clone() };
    let out = send(&hub, &mut tab, &ClientMsg::Push { batch: 1, ops: vec![stroke, erase] });
    assert_eq!(out.broadcast.len(), 2);
    assert_eq!(out.broadcast[0].op.payload.as_ref().unwrap()["took_ms"], 0.5);
    // A taken-back answer reaches a new device as its tombstone alone.
    let ServerMsg::Ops { ops, .. } = &hub.catch_up(0).unwrap()[0] else { panic!() };
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].op.kind, OpKind::Delete);
    assert_eq!(ops[0].op.payload, None);
}

/// Replays every op of a catch-up into (table, row) → (kind, payload),
/// the way a device's store merges them.
fn replay(
    msgs: &[ServerMsg],
) -> std::collections::BTreeMap<(String, String), (OpKind, Option<serde_json::Value>)> {
    let mut rows = std::collections::BTreeMap::new();
    for m in msgs {
        if let ServerMsg::Ops { ops, .. } = m {
            for SeqOp { op, .. } in ops {
                rows.insert((op.table.clone(), op.row_id.clone()), (op.kind, op.payload.clone()));
            }
        }
    }
    rows
}

#[test]
fn superseded_versions_leave_the_log() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));
    let statuses = ["new", "learning", "known"];
    for (i, status) in statuses.iter().enumerate() {
        let op = item_op(&format!("r{i}"), "s24u", NOW_I + i as i64, "cvs.L04.002", status);
        send(&hub, &mut phone, &ClientMsg::Push { batch: i as u64, ops: vec![op] });
    }
    let mid = hub.head_seq().unwrap();
    send(
        &hub,
        &mut phone,
        &ClientMsg::Push { batch: 9, ops: vec![item_op("k1", "s24u", NOW_I, "cvs.L04.001", "new")] },
    );

    // A new tablet gets one op per row: the current one.
    let fresh = hub.catch_up(0).unwrap();
    let ServerMsg::Ops { ops, .. } = &fresh[0] else { panic!() };
    assert_eq!(ops.iter().map(|o| o.op.id.as_str()).collect::<Vec<_>>(), ["r2", "k1"]);
    assert_eq!(fresh.last(), Some(&ServerMsg::CaughtUp { head_seq: 4 }));
    // A device part-way through still gets every row that changed since.
    let ServerMsg::Ops { ops, .. } = &hub.catch_up(mid - 1).unwrap()[0] else { panic!() };
    assert_eq!(ops.iter().map(|o| o.op.id.as_str()).collect::<Vec<_>>(), ["r2", "k1"]);
    // Replaying a dropped version (a lost ack) is still a no-op.
    let out = send(
        &hub,
        &mut phone,
        &ClientMsg::Push { batch: 10, ops: vec![item_op("r0", "s24u", NOW_I, "cvs.L04.002", "new")] },
    );
    assert!(out.broadcast.is_empty());
    assert_eq!(hub.current_payload("item_status", "cvs.L04.002").unwrap().unwrap()["status"], "known");
}

#[test]
fn the_old_log_is_swept_once_to_the_same_state() {
    let db = MemSql::new();
    let hub = Hub::new(&db).unwrap();
    let mut phone = conn("s24u");
    send(&hub, &mut phone, &hello(0));
    let ops = vec![
        item_op("a", "s24u", NOW_I, "cvs.L04.002", "known"),
        item_op("b", "s24u", NOW_I, "cvs.L04.001", "learning"),
    ];
    send(&hub, &mut phone, &ClientMsg::Push { batch: 1, ops });
    let before = replay(&hub.catch_up(0).unwrap());
    // A version a server from before compaction kept.
    db.exec(
            "INSERT INTO ops (op_id, tbl, row_id, kind, wall_ms, counter, device, payload, received_ms)
             VALUES ('old', 'item_status', 'cvs.L04.002', 'upsert', 1, 0, 's24u', '{\"item_id\":\"cvs.L04.002\",\"status\":\"new\"}', 1)",
            &[],
        )
        .unwrap();
    assert_ne!(replay(&hub.catch_up(0).unwrap()), before, "the stale version replays last");
    assert_eq!(hub.compact_once().unwrap(), 1);
    assert_eq!(replay(&hub.catch_up(0).unwrap()), before);
    assert_eq!(hub.compact_once().unwrap(), 0, "only once");
}
