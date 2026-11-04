//! The store's side of Steps Cloud sync: every local write to a synced table
//! also queues an op in `sync_outbox`; ops from other devices merge in by the
//! same rules as an import (append-only rows dedupe by id, LWW rows keep the
//! newest HLC), and `last_seq` remembers how far into the account's log this
//! device has read.
//!
//! The outbox fills whether or not Mohamed is signed in, so signing in later
//! uploads everything studied before. Rows written before the outbox existed
//! are queued once, on the first open after the upgrade.
// adapted from kabar-app main: crates/app/src/store.rs (outbox, apply_remote)

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};
use steps_protocol::Hlc;
use steps_protocol::sync::{Op, OpKind, SeqOp, SyncKind, TABLES, table_kind};

use super::export::cell;
use super::import::{Target, key_and_hlc, merge_row};
use super::{Result, Store};

/// `meta` keys.
const LAST_SEQ: &str = "sync.last_seq";
const LAST_SYNC_MS: &str = "sync.last_sync_ms";
const SEEDED: &str = "sync.seeded";
/// The synced tables this device last read the log with.
const TABLES_READ: &str = "sync.tables";

/// Columns that stay on the device: the account a row belongs to is the
/// account the hub serves.
const LOCAL_COLUMNS: &[&str] = &["account_id"];

/// One synced row as JSON, the way an op carries it.
fn row_json(
    conn: &Connection,
    table: &str,
    key_col: &str,
    key: &str,
) -> rusqlite::Result<Option<Map<String, Value>>> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM \"{table}\" WHERE \"{key_col}\" = ?1"))?;
    let columns: Vec<String> = stmt.column_names().map(String::from).collect();
    stmt.query_row([key], |row| {
        let mut obj = Map::new();
        for (i, col) in columns.iter().enumerate() {
            if !LOCAL_COLUMNS.contains(&col.as_str()) {
                obj.insert(col.clone(), cell(row.get_ref(i)?));
            }
        }
        Ok(obj)
    })
    .optional()
}

/// The op for a row as it stands now. `device` is this device: a row another
/// device wrote goes as a relay.
fn op_for(table: &str, key_col: &str, key: &str, row: Map<String, Value>, device: &str) -> Option<Op> {
    let kind = table_kind(table)?;
    let value = Value::Object(row);
    let (_, hlc) = key_and_hlc(&value, key_col)?;
    let deleted = value["deleted"].as_i64().unwrap_or(0) != 0;
    let (kind, payload) = match kind {
        SyncKind::Lww => (OpKind::Upsert, Some(value)),
        SyncKind::AppendOnly => (OpKind::Append, Some(value)),
        SyncKind::Immutable if deleted => (OpKind::Delete, None),
        SyncKind::Immutable => (OpKind::Append, Some(value)),
    };
    // Unique per write: every write takes a fresh HLC. A relayed row keeps
    // the id its own device gives it, so the hub takes it only once.
    let id = format!(
        "{}-{}-{}{}",
        hlc.device,
        hlc.wall_ms,
        hlc.counter,
        if kind == OpKind::Delete { "-x" } else { "" }
    );
    let relay = hlc.device != device;
    Some(Op { id, table: table.into(), row_id: key.into(), kind, hlc, payload, relay })
}

/// Queues the current version of a row: one this device just wrote, or one a
/// restored backup brought in. Tables that don't sync are skipped.
pub(super) fn enqueue(
    conn: &Connection,
    table: &str,
    key_col: &str,
    key: &str,
    device: &str,
) -> rusqlite::Result<()> {
    if table_kind(table).is_none() {
        return Ok(());
    }
    let Some(row) = row_json(conn, table, key_col, key)? else { return Ok(()) };
    let Some(op) = op_for(table, key_col, key, row, device) else { return Ok(()) };
    conn.execute(
        "INSERT OR IGNORE INTO sync_outbox (op_id, op) VALUES (?1, ?2)",
        params![op.id, serde_json::to_string(&op).unwrap_or_default()],
    )?;
    Ok(())
}

/// When this build syncs a table the last one didn't, reads the account's
/// log again from the start (#21): an older build skipped those ops but
/// still moved `last_seq` past them. Re-reading is harmless, since merging
/// ignores what is already here.
pub(super) fn check_tables(conn: &Connection) -> rusqlite::Result<()> {
    let now: Vec<&str> = TABLES.iter().map(|(t, _)| *t).collect();
    let before = meta_get(conn, TABLES_READ)?;
    let grew = match &before {
        None => true,
        Some(b) => {
            let b: Vec<&str> = b.split(',').collect();
            now.iter().any(|t| !b.contains(t))
        }
    };
    if grew {
        conn.execute("DELETE FROM meta WHERE key = ?1", [LAST_SEQ])?;
    }
    if before.as_deref() != Some(now.join(",").as_str()) {
        meta_set(conn, TABLES_READ, &now.join(","))?;
    }
    Ok(())
}

/// Queues, once, every row this device wrote before the outbox existed.
pub(super) fn seed(conn: &Connection, device: &str) -> rusqlite::Result<()> {
    let done: Option<String> =
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [SEEDED], |r| r.get(0)).optional()?;
    if done.is_some() {
        return Ok(());
    }
    for (table, _) in TABLES {
        let Some(key_col) = key_column(conn, table)? else { continue };
        let keys: Vec<String> = {
            let mut stmt = conn.prepare(&format!(
                "SELECT \"{key_col}\" FROM \"{table}\" WHERE device_id = ?1
                 ORDER BY hlc_wall, hlc_counter"
            ))?;
            stmt.query_map([device], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for key in keys {
            enqueue(conn, table, &key_col, &key, device)?;
        }
    }
    conn.execute("INSERT INTO meta (key, value) VALUES (?1, '1')", [SEEDED])?;
    Ok(())
}

/// The single-column primary key of a local table, if it has one.
fn key_column(conn: &Connection, table: &str) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1) WHERE pk > 0")?;
    let keys: Vec<String> = stmt.query_map([table], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok((keys.len() == 1).then(|| keys[0].clone()))
}

fn meta_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()
}

fn meta_set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

impl Store {
    /// Merges ops from Steps Cloud and saves `last_seq`, in one transaction.
    /// Returns the tables that changed.
    pub fn apply_remote(&self, ops: &[SeqOp], last_seq: u64) -> Result<BTreeSet<String>> {
        let mut targets = std::collections::BTreeMap::<String, Option<Target>>::new();
        for SeqOp { op, .. } in ops {
            if !targets.contains_key(&op.table) {
                let t = if table_kind(&op.table).is_some() { self.target(&op.table)? } else { None };
                targets.insert(op.table.clone(), t);
            }
        }
        let mut changed = BTreeSet::new();
        let mut newest: Option<Hlc> = None;
        {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn.transaction()?;
            for SeqOp { op, .. } in ops {
                // A table this build doesn't keep is skipped; the build that
                // adds it reads the log again (`check_tables`).
                let Some(Some(target)) = targets.get(&op.table) else { continue };
                let applied = match (op.kind, &op.payload) {
                    (OpKind::Delete, _) => {
                        tx.execute(
                            &format!(
                                "UPDATE \"{}\" SET deleted = 1 WHERE \"{}\" = ?1 AND deleted = 0",
                                op.table, target.key
                            ),
                            [&op.row_id],
                        )? > 0
                    }
                    // A row that won't merge stops the page: `last_seq` stays put
                    // and the page is fetched again on the next connection (#21).
                    (_, Some(Value::Object(row))) => {
                        merge_row(&tx, &op.table, target, row).inspect_err(|e| {
                            crate::diag::report!("sync: op {} on {} did not merge: {e}", op.id, op.table)
                        })?
                    }
                    _ => false,
                };
                if applied {
                    changed.insert(op.table.clone());
                }
                if newest.as_ref().is_none_or(|n| op.hlc > *n) {
                    newest = Some(op.hlc.clone());
                }
            }
            meta_set(&tx, LAST_SEQ, &last_seq.to_string())?;
            tx.commit()?;
        }
        // Local stamps from now on sort after what the other devices wrote.
        if let Some(h) = newest {
            self.clock.lock().unwrap().observe(&h);
        }
        Ok(changed)
    }

    pub fn last_seq(&self) -> u64 {
        let conn = self.conn.lock().unwrap();
        meta_get(&conn, LAST_SEQ).ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn set_last_seq(&self, last_seq: u64) -> Result<()> {
        meta_set(&self.conn.lock().unwrap(), LAST_SEQ, &last_seq.to_string())?;
        Ok(())
    }

    pub fn last_sync_ms(&self) -> Option<u64> {
        let conn = self.conn.lock().unwrap();
        meta_get(&conn, LAST_SYNC_MS).ok().flatten().and_then(|v| v.parse().ok())
    }

    pub fn set_last_sync_ms(&self, ms: u64) -> Result<()> {
        meta_set(&self.conn.lock().unwrap(), LAST_SYNC_MS, &ms.to_string())?;
        Ok(())
    }

    /// The oldest `limit` ops waiting for the hub.
    pub fn outbox(&self, limit: usize) -> Result<Vec<Op>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT op FROM sync_outbox ORDER BY seq LIMIT ?1")?;
        let ops = stmt
            .query_map([limit as i64], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok().and_then(|json| serde_json::from_str(&json).ok()))
            .collect();
        Ok(ops)
    }

    pub fn outbox_len(&self) -> u64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64
    }

    /// Drops acked (or rejected) ops.
    pub fn remove_from_outbox<'a>(&self, op_ids: impl IntoIterator<Item = &'a str>) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for id in op_ids {
            tx.execute("DELETE FROM sync_outbox WHERE op_id = ?1", [id])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Forgets the account's log position: the next sign-in reads it all
    /// again (merging is idempotent, so nothing doubles).
    pub fn reset_sync(&self) -> Result<()> {
        forget_position(&self.conn.lock().unwrap())?;
        Ok(())
    }
}

/// Forgets how far this device has read the log, so the next hello asks for
/// all of it.
pub(super) fn forget_position(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM meta WHERE key IN (?1, ?2)", [LAST_SEQ, LAST_SYNC_MS])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use steps_core::review::verify::{Edit, Judgement, Verdict};
    use steps_core::review::{Grade, Origin, ReviewEvent};
    use steps_protocol::sync::SeqOp;

    use super::*;

    fn ev(item: &str, at_ms: i64) -> ReviewEvent {
        ReviewEvent { item_id: item.into(), grade: Grade::Good, origin: Origin::Study, at_ms, took_ms: 3000 }
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
}
