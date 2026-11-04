//! Property tests for the merge rules: a phone and a tablet write to the
//! same rows, and every replica that sees the same ops, in any order and with
//! any duplicates, ends in the same state. Tombstones never come back.
//! Adapted from batna-app main: crates/core/src/sync/props.rs

use std::collections::BTreeMap;

use proptest::prelude::*;
use serde_json::{Value, json};

use super::{Merge, RowMeta, merge, validate};
use crate::Hlc;
use crate::sync::{Op, OpKind, SyncKind};

/// One table of each kind. Steps has no append-only table yet, so that rule
/// runs on a stand-in name without [`validate`].
const TABLES: [(&str, SyncKind); 3] = [
    ("item_status", SyncKind::Lww),
    ("events", SyncKind::AppendOnly),
    ("review_events", SyncKind::Immutable),
];
const DEVICES: [&str; 2] = ["s24u", "tab-s11"];

#[derive(Clone, Debug, PartialEq)]
struct Row {
    meta: RowMeta,
    payload: Option<Value>,
}

type Replica = BTreeMap<(String, String), Row>;

fn kind_of(table: &str) -> SyncKind {
    TABLES.iter().find(|(t, _)| *t == table).unwrap().1
}

/// Applies `op` the way the store and the hub do. An immutable tombstone
/// keeps the event's payload (the answer is retracted, not lost).
fn apply(replica: &mut Replica, op: &Op) -> Merge {
    let sync = kind_of(&op.table);
    if sync != SyncKind::AppendOnly {
        assert_eq!(validate(op, 1024), Ok(sync));
    }
    let key = (op.table.clone(), op.row_id.clone());
    let existing = replica.get(&key);
    let decision = merge(sync, op, existing.map(|r| &r.meta));
    if decision == Merge::Apply {
        let payload = match (&op.payload, existing) {
            (None, Some(row)) if sync == SyncKind::Immutable => row.payload.clone(),
            (p, _) => p.clone(),
        };
        let meta = RowMeta { hlc: op.hlc.clone(), deleted: op.kind == OpKind::Delete };
        replica.insert(key, Row { meta, payload });
    }
    decision
}

/// What a reader sees: live rows with their payloads, and which are gone.
fn visible(replica: &Replica) -> BTreeMap<(String, String), Option<Value>> {
    replica.iter().map(|(k, r)| (k.clone(), (!r.meta.deleted).then(|| r.payload.clone()).flatten())).collect()
}

#[derive(Clone, Debug)]
struct Action {
    device: usize,
    table: usize,
    row: usize,
    delete: bool,
    clock_step: i64,
    synced: bool,
}

fn action() -> impl Strategy<Value = Action> {
    (0..2usize, 0..3usize, 0..3usize, any::<bool>(), -50i64..200, any::<bool>()).prop_map(
        |(device, table, row, delete, clock_step, synced)| Action {
            device,
            table,
            row,
            delete,
            clock_step,
            synced,
        },
    )
}

/// The ops two devices would write, each with its own HLC that never goes
/// back and, after a sync, sorts after everything it has seen.
fn ops_from(actions: &[Action]) -> Vec<Op> {
    let mut last = [(i64::MIN, 0u32); 2];
    let mut walls = [1_000_000i64; 2];
    let mut ops: Vec<Op> = Vec::new();
    for (n, a) in actions.iter().enumerate() {
        walls[a.device] += a.clock_step;
        let now = walls[a.device];
        if a.synced {
            for op in &ops {
                last[a.device] = last[a.device].max((op.hlc.wall_ms, op.hlc.counter));
            }
        }
        let (wall, counter) = last[a.device];
        last[a.device] = if now > wall { (now, 0) } else { (wall, counter + 1) };
        let hlc =
            Hlc { wall_ms: last[a.device].0, counter: last[a.device].1, device: DEVICES[a.device].into() };

        let (table, sync) = TABLES[a.table];
        let row_id = format!("hb.L01.00{}", a.row);
        let appended = ops.iter().any(|o| o.table == table && o.row_id == row_id && o.kind == OpKind::Append);
        let kind = match sync {
            SyncKind::Lww if a.delete => OpKind::Delete,
            SyncKind::Lww => OpKind::Upsert,
            SyncKind::AppendOnly if appended => continue,
            SyncKind::AppendOnly => OpKind::Append,
            SyncKind::Immutable if !appended => OpKind::Append,
            SyncKind::Immutable => OpKind::Delete,
        };
        let payload = match kind {
            OpKind::Delete => None,
            OpKind::Append => Some(json!({ "item": row_id, "grade": "good" })),
            OpKind::Upsert => Some(json!({ "status": n, "by": DEVICES[a.device] })),
        };
        ops.push(Op { id: format!("op-{n}"), table: table.into(), row_id, kind, hlc, relay: false, payload });
    }
    ops
}

fn delivery(len: usize) -> impl Strategy<Value = Vec<usize>> {
    proptest::collection::vec(0..len.max(1), 0..len + 1).prop_flat_map(move |dupes| {
        let mut order: Vec<usize> = (0..len).chain(dupes.into_iter().filter(|_| len > 0)).collect();
        order.sort_unstable();
        Just(order).prop_shuffle()
    })
}

fn scenario() -> impl Strategy<Value = (Vec<Op>, Vec<usize>, Vec<usize>)> {
    proptest::collection::vec(action(), 1..40).prop_flat_map(|actions| {
        let ops = ops_from(&actions);
        let len = ops.len();
        (Just(ops), delivery(len), delivery(len))
    })
}

fn replay(ops: &[Op], order: &[usize]) -> Replica {
    let mut replica = Replica::new();
    for &i in order {
        apply(&mut replica, &ops[i]);
    }
    replica
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn any_delivery_order_converges((ops, phone, tablet) in scenario()) {
        let a = replay(&ops, &phone);
        let b = replay(&ops, &tablet);
        prop_assert_eq!(visible(&a), visible(&b));
        for (key, row) in &a {
            if kind_of(&key.0) == SyncKind::Lww {
                prop_assert_eq!(&row.meta, &b[key].meta);
            }
        }
    }

    #[test]
    fn lww_rows_hold_the_write_with_the_highest_clock((ops, order, _) in scenario()) {
        let replica = replay(&ops, &order);
        let mut newest: BTreeMap<(String, String), &Op> = BTreeMap::new();
        for op in ops.iter().filter(|o| o.table == "item_status") {
            let key = (op.table.clone(), op.row_id.clone());
            if newest.get(&key).is_none_or(|n| op.hlc > n.hlc) {
                newest.insert(key, op);
            }
        }
        for (key, op) in newest {
            let row = &replica[&key];
            prop_assert_eq!(&row.meta.hlc, &op.hlc);
            prop_assert_eq!(row.meta.deleted, op.kind == OpKind::Delete);
            prop_assert_eq!(&row.payload, &op.payload);
        }
    }

    #[test]
    fn retracted_answers_never_come_back((ops, order, _) in scenario()) {
        let mut replica = Replica::new();
        for &i in &order {
            let op = &ops[i];
            let key = (op.table.clone(), op.row_id.clone());
            let was_deleted = replica.get(&key).is_some_and(|r| r.meta.deleted);
            apply(&mut replica, op);
            if kind_of(&op.table) != SyncKind::Lww && was_deleted {
                prop_assert!(replica[&key].meta.deleted, "{key:?} came back after its tombstone");
            }
        }
        for op in ops.iter().filter(|o| o.table == "review_events" && o.kind == OpKind::Delete) {
            prop_assert!(replica[&(op.table.clone(), op.row_id.clone())].meta.deleted);
        }
    }

    #[test]
    fn replaying_the_same_ops_changes_nothing((ops, order, _) in scenario()) {
        let mut replica = replay(&ops, &order);
        let before = replica.clone();
        for op in &ops {
            prop_assert_eq!(apply(&mut replica, op), Merge::Ignore);
        }
        prop_assert_eq!(replica, before);
    }
}
