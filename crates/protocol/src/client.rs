//! The device side of live sync, as a state machine with no I/O.
//!
//! The app's transport feeds it socket events and does what it returns:
//! send a message, store remote ops, drop acked ops from the outbox. Keeping
//! it pure means every ordering of messages can be tested without a socket.

use crate::PROTOCOL_VERSION;
use crate::sync::{ClientMsg, MAX_OPS_PER_PUSH, Op, RejectReason, Rejected, SeqOp, ServerMsg, SyncErrorCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No socket.
    Offline,
    /// Hello sent; receiving the ops this device missed.
    CatchingUp,
    /// Has the whole log; pushes the outbox and receives live ops.
    Live,
    /// Signed out from another device. Nothing more until sign-in.
    Revoked,
    /// The server speaks a newer protocol. Nothing more until an update.
    Outdated,
}

/// What the transport must do, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Send(ClientMsg),
    /// Merge these ops into the store and save `last_seq`, in one transaction.
    Apply {
        ops: Vec<SeqOp>,
        last_seq: u64,
    },
    /// Save `last_seq` (nothing to merge).
    SaveLastSeq(u64),
    /// Remove these ops from the outbox. `rejected` ops will never be
    /// accepted, so they leave too; the reasons are for the log.
    Acked {
        done: Vec<String>,
        rejected: Vec<Rejected>,
    },
    /// The hub found this device's clock too far ahead. These ops stay in the
    /// outbox; hold pushes for a while and tell the user to fix the clock.
    ClockAhead {
        kept: Vec<String>,
    },
    /// The hub is older than this device: it doesn't know these ops' tables
    /// (or their kind on that table).
    /// They stay in the outbox; skip their tables until the next connection
    /// (the hub may have been updated by then) so newer ops aren't blocked.
    HubBehind {
        kept: Vec<String>,
    },
    /// Close the socket and reconnect after a backoff.
    Reconnect {
        reason: String,
    },
    /// Close the socket and stop: the user has to act (sign in, update).
    Stop,
}

#[derive(Clone, Debug)]
struct InFlight {
    batch: u64,
    op_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SyncClient {
    phase: Phase,
    last_seq: u64,
    next_batch: u64,
    in_flight: Option<InFlight>,
}

impl SyncClient {
    /// `last_seq` is the highest server seq this device has stored.
    pub fn new(last_seq: u64) -> Self {
        Self { phase: Phase::Offline, last_seq, next_batch: 1, in_flight: None }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// The store didn't take the ops it was given: go back to what it holds,
    /// so the next hello asks for them again instead of skipping past them.
    pub fn rewind(&mut self, stored_seq: u64) {
        self.last_seq = stored_seq;
    }

    /// The socket is open.
    pub fn on_open(&mut self) -> Vec<Effect> {
        self.phase = Phase::CatchingUp;
        self.in_flight = None;
        vec![self.hello()]
    }

    /// The socket closed or failed. A push in flight is resent next time
    /// (the hub ignores ops it already has).
    pub fn on_close(&mut self) {
        if matches!(self.phase, Phase::CatchingUp | Phase::Live) {
            self.phase = Phase::Offline;
        }
        self.in_flight = None;
    }

    /// One message from the hub.
    pub fn on_message(&mut self, msg: ServerMsg) -> Vec<Effect> {
        match msg {
            ServerMsg::Ops { ops, more } => {
                let mut out = Vec::new();
                if let Some(top) = ops.iter().map(|o| o.seq).max() {
                    self.last_seq = self.last_seq.max(top);
                    out.push(Effect::Apply { ops, last_seq: self.last_seq });
                }
                if more {
                    out.push(self.hello());
                }
                out
            }
            ServerMsg::CaughtUp { head_seq } => {
                self.phase = Phase::Live;
                if head_seq > self.last_seq {
                    // Only seqs without a stored op (none today) could be missing.
                    self.last_seq = head_seq;
                    vec![Effect::SaveLastSeq(head_seq)]
                } else {
                    vec![]
                }
            }
            ServerMsg::Ack { batch, done, rejected, head_seq } => {
                if self.in_flight.as_ref().is_some_and(|f| f.batch == batch) {
                    self.in_flight = None;
                }
                let (rejected, kept): (Vec<_>, Vec<_>) =
                    rejected.into_iter().partition(|r| r.reason.is_final());
                let (behind, skewed): (Vec<_>, Vec<_>) =
                    kept.into_iter().partition(|r| r.reason != RejectReason::ClockSkew);
                let mut out = vec![Effect::Acked { done, rejected }];
                if !skewed.is_empty() {
                    out.push(Effect::ClockAhead { kept: skewed.into_iter().map(|r| r.op_id).collect() });
                }
                if !behind.is_empty() {
                    out.push(Effect::HubBehind { kept: behind.into_iter().map(|r| r.op_id).collect() });
                }
                // Live sockets get every other op before the ack of a later
                // push, so everything up to head_seq is here now.
                if self.phase == Phase::Live && head_seq > self.last_seq {
                    self.last_seq = head_seq;
                    out.push(Effect::SaveLastSeq(head_seq));
                }
                out
            }
            ServerMsg::Pong => vec![],
            ServerMsg::Revoked => {
                self.phase = Phase::Revoked;
                self.in_flight = None;
                vec![Effect::Stop]
            }
            ServerMsg::Error { code, message } => {
                self.in_flight = None;
                if code == SyncErrorCode::UnsupportedProtocol {
                    self.phase = Phase::Outdated;
                    vec![Effect::Stop]
                } else {
                    vec![Effect::Reconnect { reason: format!("{code:?}: {message}") }]
                }
            }
        }
    }

    /// The next push, if the socket is live and nothing is in flight.
    /// `outbox` is the oldest pending ops first; at most
    /// [`MAX_OPS_PER_PUSH`] are taken.
    pub fn next_push(&mut self, outbox: &[Op]) -> Option<ClientMsg> {
        if self.phase != Phase::Live || self.in_flight.is_some() || outbox.is_empty() {
            return None;
        }
        let ops: Vec<Op> = outbox.iter().take(MAX_OPS_PER_PUSH).cloned().collect();
        let batch = self.next_batch;
        self.next_batch += 1;
        self.in_flight = Some(InFlight { batch, op_ids: ops.iter().map(|o| o.id.clone()).collect() });
        Some(ClientMsg::Push { batch, ops })
    }

    /// Op ids of the push waiting for its ack.
    pub fn in_flight(&self) -> &[String] {
        self.in_flight.as_ref().map(|f| f.op_ids.as_slice()).unwrap_or_default()
    }

    fn hello(&self) -> Effect {
        Effect::Send(ClientMsg::Hello { protocol: PROTOCOL_VERSION, last_seq: self.last_seq })
    }
}

/// Reconnect delays: 1 s, 2 s, 4 s … up to 5 minutes, with jitter so two
/// devices don't retry in lockstep.
#[derive(Clone, Debug, Default)]
pub struct Backoff {
    attempt: u32,
}

impl Backoff {
    pub const MAX_MS: u64 = 5 * 60 * 1000;

    /// Delay before the next attempt. `jitter` is uniform in `0.0..1.0`.
    pub fn next_delay_ms(&mut self, jitter: f64) -> u64 {
        let base = 1000u64.saturating_mul(1 << self.attempt.min(20)).min(Self::MAX_MS);
        self.attempt += 1;
        // 50–100 % of the base.
        (base as f64 * (0.5 + jitter.clamp(0.0, 1.0) / 2.0)) as u64
    }

    /// A connection reached `Live`.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

#[cfg(test)]
mod tests;
