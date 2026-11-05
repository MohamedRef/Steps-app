//! The per-account sync hub: an op log with a monotonically increasing `seq`,
//! plus the current version of every row so merges are O(1). Runs inside the
//! account's Durable Object; the WebSocket plumbing lives in `wasm::account`.

use serde::{Deserialize, Serialize};
use steps_protocol::merge::{Merge, RowMeta, merge, validate};
use steps_protocol::sync::{
    ClientMsg, MAX_CLOCK_SKEW_MS, MAX_OPS_PER_PUSH, MAX_PAYLOAD_BYTES, OPS_PAGE_SIZE, Op, OpKind,
    RejectReason, Rejected, SeqOp, ServerMsg, SyncErrorCode,
};
use steps_protocol::{Hlc, PROTOCOL_VERSION};

use crate::sql::{Row, Sql, SqlError, Value, col};
use crate::tokens::SERVER_DEVICE_ID;

/// Pages sent per `hello` before the device has to ask again. Keeps one
/// Durable Object event small even for a fresh install after a long time.
pub const MAX_PAGES_PER_HELLO: usize = 25;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ops (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    op_id TEXT NOT NULL UNIQUE,
    tbl TEXT NOT NULL,
    row_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    wall_ms INTEGER NOT NULL,
    counter INTEGER NOT NULL,
    device TEXT NOT NULL,
    payload TEXT,
    received_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS row_state (
    tbl TEXT NOT NULL,
    row_id TEXT NOT NULL,
    wall_ms INTEGER NOT NULL,
    counter INTEGER NOT NULL,
    device TEXT NOT NULL,
    deleted INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    PRIMARY KEY (tbl, row_id)
)";

/// `meta` key set once the log written before compaction was swept.
const SWEPT_KEY: &str = "hub_swept_superseded";

/// Per-connection state, kept as the WebSocket's attachment so it survives
/// hibernation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conn {
    pub device: String,
    /// Random per connection, to tell two sockets of one device apart.
    pub conn_id: String,
    pub ready: bool,
}

/// What to do after one client message.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// Send to the connection that sent the message, in order.
    pub replies: Vec<ServerMsg>,
    /// Send to every other ready connection of the account.
    pub broadcast: Vec<SeqOp>,
    /// Ops moved (for the Devices screen's "last sync").
    pub synced: bool,
}

fn kind_str(k: OpKind) -> &'static str {
    match k {
        OpKind::Upsert => "upsert",
        OpKind::Append => "append",
        OpKind::Delete => "delete",
    }
}

fn kind_from(s: &str) -> Result<OpKind, SqlError> {
    match s {
        "upsert" => Ok(OpKind::Upsert),
        "append" => Ok(OpKind::Append),
        "delete" => Ok(OpKind::Delete),
        other => Err(SqlError(format!("bad kind {other}"))),
    }
}

fn valid_op_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn valid_row_id(id: &str) -> bool {
    (1..=200).contains(&id.len()) && !id.chars().any(char::is_control)
}

fn error(code: SyncErrorCode, message: impl Into<String>) -> ServerMsg {
    ServerMsg::Error { code, message: message.into() }
}

pub struct Hub<'a, S: Sql> {
    db: &'a S,
}

impl<'a, S: Sql> Hub<'a, S> {
    /// Creates the tables if needed, then opens.
    pub fn new(db: &'a S) -> Result<Self, SqlError> {
        db.exec_batch(SCHEMA)?;
        Ok(Self::open(db))
    }

    /// Opens without touching the schema (the Durable Object migrates once at start).
    pub fn open(db: &'a S) -> Self {
        Self { db }
    }

    /// Drops ops no row points at any more (older versions of LWW rows, the
    /// strokes behind a tombstone), once: [`Self::apply`] keeps the log
    /// compact from then on. Returns how many went.
    ///
    /// Catch-up pages by `seq`, so a compacted log is already a snapshot: a
    /// fresh device gets each row's current op plus every append, and a
    /// device part-way through still gets every row that changed after its
    /// `last_seq`, because a row's current op is never older than the ones it
    /// replaced.
    pub fn compact_once(&self) -> Result<u64, SqlError> {
        self.db.exec_batch(crate::sql::META_SCHEMA)?;
        if crate::sql::meta_get(self.db, SWEPT_KEY)?.is_some() {
            return Ok(0);
        }
        let gone = self
            .db
            .exec("DELETE FROM ops WHERE seq NOT IN (SELECT seq FROM row_state) RETURNING seq", &[])?
            .len() as u64;
        crate::sql::meta_set(self.db, SWEPT_KEY, "1")?;
        Ok(gone)
    }

    pub fn head_seq(&self) -> Result<u64, SqlError> {
        let rows = self.db.exec("SELECT COALESCE(MAX(seq), 0) FROM ops", &[])?;
        Ok(rows.first().and_then(|r| r.first()).and_then(Value::as_u64).unwrap_or(0))
    }

    /// Handles one text frame from a device.
    pub fn handle(&self, conn: &mut Conn, text: &str, now_ms: u64) -> Result<Outcome, SqlError> {
        let msg: ClientMsg = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                return Ok(Outcome {
                    replies: vec![error(SyncErrorCode::BadMessage, e.to_string())],
                    ..Default::default()
                });
            }
        };
        match msg {
            ClientMsg::Ping => Ok(Outcome { replies: vec![ServerMsg::Pong], ..Default::default() }),
            ClientMsg::Hello { protocol, last_seq } => {
                if protocol != PROTOCOL_VERSION {
                    let msg = format!("server speaks protocol {PROTOCOL_VERSION}, device sent {protocol}");
                    return Ok(Outcome {
                        replies: vec![error(SyncErrorCode::UnsupportedProtocol, msg)],
                        ..Default::default()
                    });
                }
                let replies = self.catch_up(last_seq)?;
                // Live ops only after the device has the whole log, so it can
                // keep `last_seq` as the highest seq it has seen.
                conn.ready = matches!(replies.last(), Some(ServerMsg::CaughtUp { .. }));
                let synced =
                    replies.iter().any(|m| matches!(m, ServerMsg::Ops { ops, .. } if !ops.is_empty()));
                Ok(Outcome { replies, broadcast: vec![], synced })
            }
            ClientMsg::Push { batch, ops } => {
                if !conn.ready {
                    return Ok(Outcome {
                        replies: vec![error(SyncErrorCode::HelloFirst, "send hello first")],
                        ..Default::default()
                    });
                }
                if ops.len() > MAX_OPS_PER_PUSH {
                    let msg = format!("at most {MAX_OPS_PER_PUSH} ops per push");
                    return Ok(Outcome {
                        replies: vec![error(SyncErrorCode::TooManyOps, msg)],
                        ..Default::default()
                    });
                }
                let (ack, applied) = self.push(&conn.device, batch, ops, now_ms)?;
                Ok(Outcome { replies: vec![ack], synced: true, broadcast: applied })
            }
        }
    }

    /// Ops after `after_seq`, then `caught_up` (or `more: true` when the
    /// device should send another `hello` to continue).
    pub fn catch_up(&self, after_seq: u64) -> Result<Vec<ServerMsg>, SqlError> {
        let mut out = Vec::new();
        let mut cursor = after_seq;
        for page in 0..MAX_PAGES_PER_HELLO {
            let (ops, more) = self.page(cursor, OPS_PAGE_SIZE)?;
            if let Some(last) = ops.last() {
                cursor = last.seq;
            }
            if !more {
                if !ops.is_empty() {
                    out.push(ServerMsg::Ops { ops, more: false });
                }
                out.push(ServerMsg::CaughtUp { head_seq: cursor.max(self.head_seq()?) });
                return Ok(out);
            }
            // `more` only on the last page of a capped burst: the device sends
            // hello again from there.
            out.push(ServerMsg::Ops { ops, more: page + 1 == MAX_PAGES_PER_HELLO });
        }
        Ok(out)
    }

    fn page(&self, after_seq: u64, limit: usize) -> Result<(Vec<SeqOp>, bool), SqlError> {
        let rows = self.db.exec(
            "SELECT seq, op_id, tbl, row_id, kind, wall_ms, counter, device, payload
             FROM ops WHERE seq > ? ORDER BY seq LIMIT ?",
            &[after_seq.into(), (limit as u64 + 1).into()],
        )?;
        let more = rows.len() > limit;
        let ops = rows.iter().take(limit).map(row_to_op).collect::<Result<_, _>>()?;
        Ok((ops, more))
    }

    /// Applies a batch from `device`. Returns the ack and the ops that changed
    /// state (to broadcast).
    pub fn push(
        &self,
        device: &str,
        batch: u64,
        ops: Vec<Op>,
        now_ms: u64,
    ) -> Result<(ServerMsg, Vec<SeqOp>), SqlError> {
        let mut done = Vec::new();
        let mut rejected = Vec::new();
        let mut applied = Vec::new();
        for op in ops {
            match self.apply(device, op, now_ms)? {
                Ok((id, Some(seq_op))) => {
                    done.push(id);
                    applied.push(seq_op);
                }
                Ok((id, None)) => done.push(id),
                Err(r) => rejected.push(r),
            }
        }
        let ack = ServerMsg::Ack { batch, done, rejected, head_seq: self.head_seq()? };
        Ok((ack, applied))
    }

    /// The payload of a live LWW row (`None` if missing or deleted).
    pub fn current_payload(&self, table: &str, row_id: &str) -> Result<Option<serde_json::Value>, SqlError> {
        let rows = self.db.exec(
            "SELECT o.payload FROM row_state r JOIN ops o ON o.seq = r.seq
             WHERE r.tbl = ? AND r.row_id = ? AND r.deleted = 0",
            &[table.into(), row_id.into()],
        )?;
        parse_payload(rows.first().and_then(|r| r.first()))
    }

    fn row_meta(&self, table: &str, row_id: &str) -> Result<Option<RowMeta>, SqlError> {
        self.db
            .exec(
                "SELECT wall_ms, counter, device, deleted FROM row_state WHERE tbl = ? AND row_id = ?",
                &[table.into(), row_id.into()],
            )?
            .first()
            .map(|r| -> Result<RowMeta, SqlError> {
                Ok(RowMeta {
                    hlc: Hlc {
                        wall_ms: col(r, 0, "wall_ms")?.as_i64().unwrap_or(0),
                        counter: col(r, 1, "counter")?.as_u64().unwrap_or(0) as u32,
                        device: col(r, 2, "device")?.as_str().unwrap_or_default().to_owned(),
                    },
                    deleted: col(r, 3, "deleted")?.as_i64() == Some(1),
                })
            })
            .transpose()
    }

    #[allow(clippy::type_complexity)]
    fn apply(
        &self,
        device: &str,
        op: Op,
        now_ms: u64,
    ) -> Result<Result<(String, Option<SeqOp>), Rejected>, SqlError> {
        let reject = |op: &Op, reason| Ok(Err(Rejected { op_id: op.id.clone(), reason }));
        if !valid_op_id(&op.id) || !valid_row_id(&op.row_id) {
            return reject(&op, RejectReason::InvalidId);
        }
        // A relay passes on rows another device wrote (a restored backup);
        // no device may write as the server.
        let relayed = op.relay && valid_op_id(&op.hlc.device) && op.hlc.device != SERVER_DEVICE_ID;
        if op.hlc.device != device && !relayed {
            return reject(&op, RejectReason::WrongDevice);
        }
        if op.hlc.wall_ms > now_ms as i64 + MAX_CLOCK_SKEW_MS {
            return reject(&op, RejectReason::ClockSkew);
        }
        let sync = match validate(&op, MAX_PAYLOAD_BYTES) {
            Ok(s) => s,
            Err(reason) => return reject(&op, reason),
        };
        if !self.db.exec("SELECT 1 FROM ops WHERE op_id = ?", &[op.id.as_str().into()])?.is_empty() {
            return Ok(Ok((op.id, None)));
        }
        let existing = self.row_meta(&op.table, &op.row_id)?;
        if merge(sync, &op, existing.as_ref()) == Merge::Ignore {
            return Ok(Ok((op.id, None)));
        }

        let payload = op.payload.as_ref().map(|p| p.to_string());
        let seq = self
            .db
            .exec(
                "INSERT INTO ops (op_id, tbl, row_id, kind, wall_ms, counter, device, payload, received_ms)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING seq",
                &[
                    op.id.as_str().into(),
                    op.table.as_str().into(),
                    op.row_id.as_str().into(),
                    kind_str(op.kind).into(),
                    op.hlc.wall_ms.into(),
                    u64::from(op.hlc.counter).into(),
                    op.hlc.device.as_str().into(),
                    payload.into(),
                    now_ms.into(),
                ],
            )?
            .first()
            .and_then(|r| r.first())
            .and_then(Value::as_u64)
            .ok_or_else(|| SqlError("insert returned no seq".into()))?;
        // The version this op replaces is dead weight for every device: those
        // that had it get this op next, the rest never need it. A replay of
        // it still merges as a no-op against the newer row.
        if existing.is_some() {
            self.db.exec(
                "DELETE FROM ops WHERE seq = (SELECT seq FROM row_state WHERE tbl = ? AND row_id = ?)",
                &[op.table.as_str().into(), op.row_id.as_str().into()],
            )?;
        }
        self.db.exec(
            "INSERT INTO row_state (tbl, row_id, wall_ms, counter, device, deleted, seq)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (tbl, row_id) DO UPDATE SET wall_ms = excluded.wall_ms, counter = excluded.counter,
                device = excluded.device, deleted = excluded.deleted, seq = excluded.seq",
            &[
                op.table.as_str().into(),
                op.row_id.as_str().into(),
                op.hlc.wall_ms.into(),
                u64::from(op.hlc.counter).into(),
                op.hlc.device.as_str().into(),
                (op.kind == OpKind::Delete).into(),
                seq.into(),
            ],
        )?;
        let id = op.id.clone();
        Ok(Ok((id, Some(SeqOp { seq, op }))))
    }
}

/// A stored JSON column (`None` for a missing row or NULL).
fn parse_payload(value: Option<&Value>) -> Result<Option<serde_json::Value>, SqlError> {
    match value {
        Some(Value::Text(s)) => serde_json::from_str(s).map(Some).map_err(|e| SqlError(e.to_string())),
        _ => Ok(None),
    }
}

fn row_to_op(r: &Row) -> Result<SeqOp, SqlError> {
    let text = |i, name| col(r, i, name).map(|v| v.as_str().unwrap_or_default().to_owned());
    let payload = match col(r, 8, "payload")? {
        Value::Text(s) => Some(serde_json::from_str(s).map_err(|e| SqlError(e.to_string()))?),
        _ => None,
    };
    Ok(SeqOp {
        seq: col(r, 0, "seq")?.as_u64().unwrap_or(0),
        op: Op {
            id: text(1, "op_id")?,
            table: text(2, "tbl")?,
            row_id: text(3, "row_id")?,
            kind: kind_from(col(r, 4, "kind")?.as_str().unwrap_or_default())?,
            hlc: Hlc {
                wall_ms: col(r, 5, "wall_ms")?.as_i64().unwrap_or(0),
                counter: col(r, 6, "counter")?.as_u64().unwrap_or(0) as u32,
                device: text(7, "device")?,
            },
            payload,
            relay: false,
        },
    })
}

#[cfg(test)]
mod tests;
