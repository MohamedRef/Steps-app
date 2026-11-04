//! Live sync wire types: ops, and the WebSocket messages between a device and
//! its account's sync hub (a Durable Object in Steps Cloud).
//!
//! Flow: the device opens `GET /v1/sync` (WebSocket), sends [`ClientMsg::Hello`]
//! with the last server `seq` it has seen, receives the missing ops in
//! [`ServerMsg::Ops`] pages and a [`ServerMsg::CaughtUp`]. From then on it
//! pushes its outbox with [`ClientMsg::Push`] and receives other devices' ops
//! as they arrive.

use serde::{Deserialize, Serialize};

use crate::Hlc;

/// Most ops a device may send in one [`ClientMsg::Push`].
pub const MAX_OPS_PER_PUSH: usize = 50;
/// Largest serialized payload of a single op, in bytes.
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
/// Most ops the hub sends in one [`ServerMsg::Ops`] page.
pub const OPS_PAGE_SIZE: usize = 200;
/// How far ahead of the hub's clock an op's HLC may be. Anything further
/// would win every last-writer-wins race until the real clock catches up.
pub const MAX_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

/// How a table merges concurrent writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncKind {
    /// Last writer wins by HLC: statuses, settings. A tombstone is a newer
    /// write with `deleted: 1` in its payload.
    Lww,
    /// Events that are only ever added.
    AppendOnly,
    /// Written once, deleted only by a tombstone: review events (an answer
    /// can be taken back, never changed).
    Immutable,
}

/// Every synced table and how it merges. A table missing here is rejected by
/// the hub, so an old server never silently drops a new client's data. A
/// chunk that adds a synced table adds it here and redeploys Steps Cloud.
pub const TABLES: &[(&str, SyncKind)] = &[
    // One row per answer (chunk 3).
    ("review_events", SyncKind::Immutable),
    // Mohamed's ✓ / ✎ / ✗ per item (chunk 3).
    ("item_status", SyncKind::Lww),
    // Synced settings by key (every chunk, through `Store::put_setting`).
    ("settings", SyncKind::Lww),
    // Mohamed's own cards, one JSON row each (chunk 21, `steps_core::own`).
    ("own_cards", SyncKind::Lww),
    // Mohamed's notes on items, lesson steps and pages (`steps_core::note`).
    ("notes", SyncKind::Lww),
];

pub fn table_kind(table: &str) -> Option<SyncKind> {
    TABLES.iter().find(|(name, _)| *name == table).map(|(_, kind)| *kind)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    /// Create or replace a row (LWW tables).
    Upsert,
    /// Add a new row that is never changed afterwards.
    Append,
    /// Tombstone a row (immutable tables).
    Delete,
}

/// One local write, as it travels from a device's outbox to the hub and on to
/// the other devices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Op {
    /// Unique per op; makes replays idempotent.
    pub id: String,
    pub table: String,
    pub row_id: String,
    pub kind: OpKind,
    /// When the write happened; `hlc.device` is the writing device.
    pub hlc: Hlc,
    /// The row's columns. `None` for deletes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// A row another device wrote, passed on by this one (from a restored
    /// backup). The hub takes it although `hlc.device` is not the device
    /// that is connected, and merges it by HLC like any other op.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub relay: bool,
}

/// An op as stored by the hub, with its position in the account's log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeqOp {
    pub seq: u64,
    #[serde(flatten)]
    pub op: Op,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    /// First message on every connection.
    Hello {
        protocol: u32,
        /// Highest `seq` this device has applied; 0 for a fresh install.
        last_seq: u64,
    },
    /// Ops from the device's outbox, at most [`MAX_OPS_PER_PUSH`].
    Push {
        batch: u64,
        ops: Vec<Op>,
    },
    Ping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    UnknownTable,
    /// e.g. an `upsert` on an immutable table.
    KindNotAllowed,
    /// `hlc.device` is not the device that is connected.
    WrongDevice,
    MissingPayload,
    PayloadTooLarge,
    /// The op's clock is too far ahead of the hub's. The op is fine; the
    /// device keeps it and sends it again once its clock is right.
    ClockSkew,
    InvalidId,
}

impl RejectReason {
    /// The hub will never take this op, however often it is sent.
    pub fn is_final(self) -> bool {
        self != RejectReason::ClockSkew
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rejected {
    pub op_id: String,
    pub reason: RejectReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncErrorCode {
    BadMessage,
    UnsupportedProtocol,
    TooManyOps,
    HelloFirst,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    /// A page of the account's log, oldest first. Live updates use the same
    /// message.
    Ops {
        ops: Vec<SeqOp>,
        /// The hub stopped sending before the head: send [`ClientMsg::Hello`]
        /// again with the last `seq` applied. No [`ServerMsg::CaughtUp`]
        /// follows this page.
        more: bool,
    },
    /// Everything up to `head_seq` has been sent; live updates follow.
    CaughtUp {
        head_seq: u64,
    },
    /// Reply to a [`ClientMsg::Push`]. Ops in `done` can leave the outbox:
    /// they were stored, were already known, or lost a last-writer-wins race
    /// to a newer write (which the device has or will receive).
    Ack {
        batch: u64,
        done: Vec<String>,
        rejected: Vec<Rejected>,
        head_seq: u64,
    },
    Pong,
    /// This device was signed out from another device. The socket closes next.
    Revoked,
    Error {
        code: SyncErrorCode,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_is_listed_once() {
        for (i, (name, _)) in TABLES.iter().enumerate() {
            assert!(TABLES[i + 1..].iter().all(|(other, _)| other != name), "{name} twice");
        }
        assert_eq!(table_kind("item_status"), Some(SyncKind::Lww));
        assert_eq!(table_kind("review_events"), Some(SyncKind::Immutable));
        assert_eq!(table_kind("meta"), None);
    }

    #[test]
    fn messages_use_a_type_tag() {
        let op = Op {
            id: "op1".into(),
            table: "item_status".into(),
            row_id: "hb.L01.001".into(),
            kind: OpKind::Upsert,
            hlc: Hlc { wall_ms: 5, counter: 0, device: "s24u".into() },
            relay: false,
            payload: Some(serde_json::json!({ "status": "approved" })),
        };
        let json =
            serde_json::to_value(ServerMsg::Ops { ops: vec![SeqOp { seq: 7, op }], more: false }).unwrap();
        assert_eq!(json["type"], "ops");
        assert_eq!(json["ops"][0]["seq"], 7);
        assert_eq!(json["ops"][0]["table"], "item_status");

        let hello: ClientMsg = serde_json::from_str(r#"{"type":"hello","protocol":1,"last_seq":0}"#).unwrap();
        assert_eq!(hello, ClientMsg::Hello { protocol: 1, last_seq: 0 });
    }
}
