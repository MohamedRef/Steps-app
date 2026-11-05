use crate::Hlc;
use crate::sync::OpKind;
use serde_json::json;

use super::*;

fn op(id: &str) -> Op {
    Op {
        id: id.into(),
        table: "item_status".into(),
        row_id: "cvs.L04.001".into(),
        kind: OpKind::Upsert,
        hlc: Hlc { wall_ms: 1, counter: 0, device: "s24u".into() },
        relay: false,
        payload: Some(json!({ "status": "approved" })),
    }
}

fn seq(n: u64) -> SeqOp {
    SeqOp { seq: n, op: op(&format!("o{n}")) }
}

fn hello(last_seq: u64) -> Effect {
    Effect::Send(ClientMsg::Hello { protocol: PROTOCOL_VERSION, last_seq })
}

fn live(last_seq: u64) -> SyncClient {
    let mut c = SyncClient::new(last_seq);
    c.on_open();
    c.on_message(ServerMsg::CaughtUp { head_seq: last_seq });
    c
}

#[test]
fn hello_catch_up_then_live() {
    let mut c = SyncClient::new(4);
    assert_eq!(c.on_open(), vec![hello(4)]);
    assert_eq!(c.phase(), Phase::CatchingUp);
    // Nothing is pushed before the device has the whole log.
    assert_eq!(c.next_push(&[op("a")]), None);

    let fx = c.on_message(ServerMsg::Ops { ops: vec![seq(5), seq(6)], more: false });
    assert_eq!(fx, vec![Effect::Apply { ops: vec![seq(5), seq(6)], last_seq: 6 }]);
    assert_eq!(c.on_message(ServerMsg::CaughtUp { head_seq: 6 }), vec![]);
    assert_eq!(c.phase(), Phase::Live);
    assert!(c.next_push(&[op("a")]).is_some());
}

#[test]
fn a_capped_burst_says_hello_again() {
    let mut c = SyncClient::new(0);
    c.on_open();
    let fx = c.on_message(ServerMsg::Ops { ops: vec![seq(1), seq(2)], more: true });
    assert_eq!(fx[1], hello(2));
    assert_eq!(c.phase(), Phase::CatchingUp);
}

#[test]
fn one_push_in_flight_at_a_time() {
    let mut c = live(0);
    let outbox: Vec<Op> = (0..MAX_OPS_PER_PUSH + 3).map(|i| op(&format!("p{i}"))).collect();
    let Some(ClientMsg::Push { batch: 1, ops }) = c.next_push(&outbox) else { panic!() };
    assert_eq!(ops.len(), MAX_OPS_PER_PUSH);
    assert_eq!(c.next_push(&outbox), None);

    let rejected = vec![Rejected { op_id: "p1".into(), reason: RejectReason::InvalidId }];
    let done: Vec<String> = ops.iter().skip(2).map(|o| o.id.clone()).collect();
    let fx = c.on_message(ServerMsg::Ack {
        batch: 1,
        done: done.clone(),
        rejected: rejected.clone(),
        head_seq: 9,
    });
    assert_eq!(fx, vec![Effect::Acked { done, rejected }, Effect::SaveLastSeq(9)]);
    let Some(ClientMsg::Push { batch: 2, .. }) = c.next_push(&outbox[MAX_OPS_PER_PUSH..]) else { panic!() };
}

#[test]
fn clock_skew_keeps_the_op_for_later() {
    let mut c = live(0);
    let Some(ClientMsg::Push { batch, .. }) = c.next_push(&[op("a"), op("b"), op("c")]) else { panic!() };
    let fx = c.on_message(ServerMsg::Ack {
        batch,
        done: vec!["a".into()],
        rejected: vec![
            Rejected { op_id: "b".into(), reason: RejectReason::ClockSkew },
            Rejected { op_id: "c".into(), reason: RejectReason::UnknownTable },
        ],
        head_seq: 0,
    });
    assert_eq!(
        fx,
        vec![
            Effect::Acked { done: vec!["a".into()], rejected: vec![] },
            Effect::ClockAhead { kept: vec!["b".into()] },
            // A hub older than the app: the op waits for a newer hub.
            Effect::HubBehind { kept: vec!["c".into()] },
        ]
    );
    assert!(c.in_flight().is_empty());
}

#[test]
fn a_failed_merge_asks_for_the_same_ops_again() {
    let mut c = SyncClient::new(4);
    c.on_open();
    c.on_message(ServerMsg::Ops { ops: vec![seq(5), seq(6)], more: false });
    // The store's transaction failed, so it still holds 4.
    c.rewind(4);
    c.on_close();
    assert_eq!(c.on_open(), vec![hello(4)]);
}

#[test]
fn a_dropped_socket_resends_the_push() {
    let mut c = live(3);
    c.next_push(&[op("a")]).unwrap();
    assert_eq!(c.in_flight(), ["a".to_string()]);
    c.on_close();
    assert_eq!(c.phase(), Phase::Offline);
    assert_eq!(c.on_open(), vec![hello(3)]);
    c.on_message(ServerMsg::CaughtUp { head_seq: 3 });
    assert!(c.next_push(&[op("a")]).is_some());
}

#[test]
fn live_ops_move_last_seq() {
    let mut c = live(2);
    let fx = c.on_message(ServerMsg::Ops { ops: vec![seq(3)], more: false });
    assert_eq!(fx, vec![Effect::Apply { ops: vec![seq(3)], last_seq: 3 }]);
    // An old page never moves it back.
    c.on_message(ServerMsg::Ops { ops: vec![seq(1)], more: false });
    assert_eq!(c.last_seq(), 3);
}

#[test]
fn revoked_and_outdated_stop() {
    let mut c = live(0);
    assert_eq!(c.on_message(ServerMsg::Revoked), vec![Effect::Stop]);
    assert_eq!(c.phase(), Phase::Revoked);
    c.on_close();
    assert_eq!(c.phase(), Phase::Revoked);

    let mut c = SyncClient::new(0);
    c.on_open();
    let fx = c.on_message(ServerMsg::Error {
        code: SyncErrorCode::UnsupportedProtocol,
        message: "server speaks 2".into(),
    });
    assert_eq!(fx, vec![Effect::Stop]);
    assert_eq!(c.phase(), Phase::Outdated);

    let mut c = live(0);
    let fx = c.on_message(ServerMsg::Error { code: SyncErrorCode::Internal, message: "db".into() });
    assert!(matches!(fx[0], Effect::Reconnect { .. }));
}

#[test]
fn backoff_doubles_up_to_five_minutes() {
    let mut b = Backoff::default();
    assert_eq!(b.next_delay_ms(1.0), 1000);
    assert_eq!(b.next_delay_ms(1.0), 2000);
    assert_eq!(b.next_delay_ms(0.0), 2000);
    for _ in 0..30 {
        b.next_delay_ms(1.0);
    }
    assert_eq!(b.next_delay_ms(1.0), Backoff::MAX_MS);
    b.reset();
    assert_eq!(b.next_delay_ms(1.0), 1000);
}
