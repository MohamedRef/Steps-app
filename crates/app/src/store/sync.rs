//! The store's side of Steps Cloud sync: every local write to a synced table
//! also queues an op in `sync_outbox`; ops from other devices merge in by the
//! same rules as an import (append-only rows dedupe by id, LWW rows keep the
//! newest HLC), and `last_seq` remembers how far into the account's log this
//! device has read.
//!
//! The outbox fills whether or not Mohamed is signed in, so signing in later
//! uploads everything studied before. Rows written before the outbox existed
//! are queued once, on the first open after the upgrade.
// adapted from batna-app main: crates/app/src/store.rs (outbox, apply_remote)

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
    // A row written again before it went up (a lesson's place, saved on
    // every answer) needs only its newest version: the older ops still
    // waiting carry nothing the newest doesn't, so they go.
    if table_kind(table) == Some(SyncKind::Lww) {
        conn.execute(
            "DELETE FROM sync_outbox
             WHERE json_extract(op, '$.table') = ?1 AND json_extract(op, '$.row_id') = ?2
               AND json_extract(op, '$.kind') = 'upsert' AND json_extract(op, '$.relay') IS NULL",
            params![table, op.row_id],
        )?;
    }
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

    /// Like [`Store::outbox`], leaving out ops for tables the hub doesn't
    /// know yet, so they don't block the ops behind them.
    pub fn outbox_skipping(
        &self,
        limit: usize,
        skip_tables: &std::collections::HashSet<String>,
    ) -> Result<Vec<Op>> {
        if skip_tables.is_empty() {
            return self.outbox(limit);
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT op FROM sync_outbox ORDER BY seq")?;
        let ops = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok().and_then(|json| serde_json::from_str::<Op>(&json).ok()))
            .filter(|op| !skip_tables.contains(&op.table))
            .take(limit)
            .collect();
        Ok(ops)
    }

    /// This device's own ops still waiting with a stamp further ahead than
    /// Steps Cloud accepts (written while the clock was wrong, #539) take a
    /// fresh stamp, row and op both, so they go up once the clock is right
    /// instead of being refused as clock skew until real time catches up.
    /// Returns how many were stamped again.
    pub fn restamp_future(&self, now_ms: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        self.restamp_future_in(&conn, now_ms)
    }

    /// [`Self::restamp_future`] on a connection the caller already holds.
    /// Rows too (#606): a row whose op the hub refused, or that went up
    /// before the clock check, still carries the future stamp and would
    /// beat every later local write.
    pub(super) fn restamp_future_in(&self, conn: &Connection, now_ms: i64) -> Result<usize> {
        let device = self.device();
        let limit = now_ms.saturating_add(steps_protocol::sync::MAX_CLOCK_SKEW_MS);
        let ahead: Vec<(String, Op)> = {
            let mut stmt = conn.prepare(
                "SELECT op_id, op FROM sync_outbox
                 WHERE json_extract(op, '$.hlc.wall_ms') > ?1 AND json_extract(op, '$.relay') IS NULL
                 ORDER BY seq",
            )?;
            stmt.query_map([limit], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .filter_map(|r| r.ok())
                .filter_map(|(id, json)| Some((id, serde_json::from_str::<Op>(&json).ok()?)))
                .filter(|(_, op)| op.hlc.device == device)
                .collect()
        };
        let mut done = 0;
        for (op_id, op) in ahead {
            let Some(key_col) = key_column(conn, &op.table)? else { continue };
            let fresh = self.tick(now_ms);
            conn.execute(
                &format!(
                    "UPDATE \"{}\" SET hlc_wall = ?1, hlc_counter = ?2
                     WHERE \"{key_col}\" = ?3 AND hlc_wall = ?4 AND hlc_counter = ?5 AND device_id = ?6",
                    op.table
                ),
                params![fresh.wall_ms, fresh.counter, op.row_id, op.hlc.wall_ms, op.hlc.counter, device],
            )?;
            conn.execute("DELETE FROM sync_outbox WHERE op_id = ?1", [&op_id])?;
            enqueue(conn, &op.table, &key_col, &op.row_id, &device)?;
            done += 1;
        }
        // Rows still ahead with no op waiting.
        for (table, _) in TABLES {
            let Some(key_col) = key_column(conn, table)? else { continue };
            let keys: Vec<String> = {
                let mut stmt = conn.prepare(&format!(
                    "SELECT CAST(\"{key_col}\" AS TEXT) FROM \"{table}\" WHERE device_id = ?1 AND hlc_wall > ?2"
                ))?;
                stmt.query_map(params![device, limit], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
            };
            for key in keys {
                let fresh = self.tick(now_ms);
                conn.execute(
                    &format!(
                        "UPDATE \"{table}\" SET hlc_wall = ?1, hlc_counter = ?2
                         WHERE CAST(\"{key_col}\" AS TEXT) = ?3 AND device_id = ?4"
                    ),
                    params![fresh.wall_ms, fresh.counter, key, device],
                )?;
                enqueue(conn, table, &key_col, &key, &device)?;
                done += 1;
            }
        }
        Ok(done)
    }

    /// The tables of these outbox ops (unknown ids are left out).
    pub fn outbox_tables<'a>(&self, op_ids: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let Ok(mut stmt) = conn.prepare("SELECT op FROM sync_outbox WHERE op_id = ?1") else {
            return Vec::new();
        };
        let mut tables: Vec<String> = op_ids
            .into_iter()
            .filter_map(|id| stmt.query_row([id], |r| r.get::<_, String>(0)).ok())
            .filter_map(|json| serde_json::from_str::<Op>(&json).ok())
            .map(|op| op.table)
            .collect();
        tables.sort();
        tables.dedup();
        tables
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
mod tests;
