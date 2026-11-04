//! The sync engine against a local stand-in for Steps Cloud (`fake.rs`):
//! sign-in, catching up, pushing the outbox, the Devices list and every way
//! the hub can stop or redirect a device.

use std::time::{Duration, Instant};

use steps_protocol::cloud::{DeviceInfo, Platform, Session};
use steps_protocol::sync::{ClientMsg, RejectReason, Rejected, SeqOp, ServerMsg, SyncErrorCode};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::api::{Api, CloudError, api_error};
use super::engine::Engine;
use super::fake::{Fake, Reply, devices, json, session};
use super::session::{Auth, MemoryVault, SessionVault};
use super::{CloudStatus, Command, Event, GoogleClient, Problem, SyncState};
use crate::store::Store;

/// One engine on its own thread, as the app runs it.
struct Run {
    commands: UnboundedSender<Command>,
    events: UnboundedReceiver<Event>,
    store: &'static Store,
    /// Every status it reported, in order.
    statuses: Vec<CloudStatus>,
    others: Vec<Event>,
}

fn store() -> &'static Store {
    Box::leak(Box::new(Store::open_in_memory().unwrap()))
}

fn start(
    fake: &Fake,
    store: &'static Store,
    signed_in: Option<Session>,
    google: Option<GoogleClient>,
) -> Run {
    let vault = MemoryVault::default();
    if let Some(s) = &signed_in {
        assert!(vault.save(s));
    }
    let device = DeviceInfo {
        device_id: store.device(),
        name: "Steps (dev)".into(),
        model: None,
        platform: Platform::Other,
    };
    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (ev_tx, ev_rx) = unbounded_channel();
    let api = Api::new(&fake.url);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(Engine::new(store, api, google, device, Box::new(vault), ev_tx).run(cmd_rx));
    });
    Run { commands: cmd_tx, events: ev_rx, store, statuses: Vec::new(), others: Vec::new() }
}

impl Run {
    fn send(&self, c: Command) {
        self.commands.send(c).unwrap();
    }

    fn pump(&mut self) {
        while let Ok(e) = self.events.try_recv() {
            match e {
                Event::Status(s) => self.statuses.push(s),
                other => self.others.push(other),
            }
        }
    }

    /// The first status from now on that `pred` accepts.
    #[track_caller]
    fn status(&mut self, what: &str, pred: impl Fn(&CloudStatus) -> bool) -> CloudStatus {
        let from = self.statuses.len();
        let end = Instant::now() + Duration::from_secs(8);
        loop {
            self.pump();
            if let Some(s) = self.statuses[from..].iter().find(|s| pred(s)) {
                return s.clone();
            }
            assert!(Instant::now() < end, "no status {what}; saw {:#?}", &self.statuses[from..]);
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The first other event `pred` accepts (taken off the list).
    #[track_caller]
    fn event(&mut self, what: &str, pred: impl Fn(&Event) -> bool) -> Event {
        let end = Instant::now() + Duration::from_secs(8);
        loop {
            self.pump();
            if let Some(i) = self.others.iter().position(&pred) {
                return self.others.remove(i);
            }
            assert!(Instant::now() < end, "no event {what}; saw {:?}", self.others);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
    }
}

const DAY: u64 = 86_400_000;

fn now() -> u64 {
    super::engine::now_ms()
}

/// Ops the Tab S11 wrote: a setting and a verdict, as the hub hands them on.
fn tablet_ops(first_seq: u64) -> Vec<SeqOp> {
    let tab = Store::open_in_memory().unwrap();
    tab.put_setting("plan", &serde_json::json!({ "exam": "2027-06-01" })).unwrap();
    tab.outbox(50)
        .unwrap()
        .into_iter()
        .enumerate()
        .map(|(i, op)| SeqOp { seq: first_seq + i as u64, op })
        .collect()
}

/// A hub that sends the tablet's ops on hello and acks every push.
fn live_hub(fake: &Fake) {
    let ops = tablet_ops(1);
    let head = ops.len() as u64;
    fake.on_message(move |m| match m {
        ClientMsg::Hello { .. } => vec![
            Reply::Msg(ServerMsg::Ops { ops: ops.clone(), more: false }),
            Reply::Msg(ServerMsg::CaughtUp { head_seq: head }),
        ],
        ClientMsg::Push { batch, ops } => vec![
            Reply::Msg(ServerMsg::Pong),
            Reply::Raw("not a sync message".into()),
            Reply::Msg(ServerMsg::Ack {
                batch: *batch,
                done: ops.iter().map(|o| o.id.clone()).collect(),
                rejected: Vec::new(),
                head_seq: head + 1,
            }),
        ],
        ClientMsg::Ping => vec![Reply::Msg(ServerMsg::Pong)],
    });
}

#[test]
fn a_signed_in_device_catches_up_pushes_and_manages_devices() {
    let fake = Fake::start();
    live_hub(&fake);
    let s = store();
    let me = s.device();
    fake.route("GET /v1/devices", 200, devices(&me));
    fake.route("DELETE /v1/devices/tab", 200, "{}");
    fake.route("POST /v1/auth/signout", 200, "{}");
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);

    // The tablet's plan arrives and the screens hear which table changed.
    let Event::Changed(tables) = run.event("changed", |e| matches!(e, Event::Changed(_))) else {
        unreachable!()
    };
    assert!(tables.contains("settings"), "{tables:?}");
    let live = run.status("live", |s| s.sync == SyncState::Live);
    assert_eq!(live.email.as_deref(), Some("mohamed@example.com"));
    assert_eq!(live.device_id, me);
    assert!(live.configured);
    assert_eq!(s.setting::<serde_json::Value>("plan").unwrap()["exam"], "2027-06-01");
    assert_eq!(s.last_seq(), 1);
    assert_eq!(fake.sockets(), [Some("Bearer acc-1".to_owned())]);
    assert_eq!(
        fake.received()[0],
        ClientMsg::Hello { protocol: steps_protocol::PROTOCOL_VERSION, last_seq: 0 }
    );

    // A local write goes up at once on a kick, and leaves the outbox once acked.
    s.put_setting("sound", &serde_json::json!({ "on": false })).unwrap();
    assert_eq!(s.outbox_len(), 1);
    run.send(Command::Kick);
    let done =
        run.status("pushed", |st| st.pending == 0 && st.last_sync_ms.is_some() && st.sync == SyncState::Live);
    assert_eq!(done.problem, None);
    assert_eq!(s.outbox_len(), 0);
    assert!(
        fake.received()
            .iter()
            .any(|m| matches!(m, ClientMsg::Push { ops, .. } if ops[0].table == "settings"))
    );
    assert_eq!(s.last_seq(), 2, "the ack's head is saved while live");

    // Devices: listed, the tablet signed out, the list read again.
    run.send(Command::LoadDevices);
    let Event::Devices(Ok(list)) =
        run.event("devices", |e| matches!(e, Event::Devices(Ok(l)) if !l.is_empty()))
    else {
        unreachable!()
    };
    assert_eq!(list.len(), 2);
    assert!(list[0].current);
    run.send(Command::RevokeDevice("tab".into()));
    run.event("devices again", |e| matches!(e, Event::Devices(Ok(l)) if !l.is_empty()));
    let revoke = fake.requests("DELETE /v1/devices/tab");
    assert_eq!(revoke.len(), 1);
    assert_eq!(revoke[0].auth.as_deref(), Some("Bearer acc-1"));

    // Signing this device out from the list is a sign-out here.
    run.send(Command::RevokeDevice(me.clone()));
    let out = run.status("signed out", |st| st.sync == SyncState::SignedOut);
    assert_eq!(out.email, None);
    run.event("empty list", |e| *e == Event::Devices(Ok(Vec::new())));
    assert_eq!(fake.requests("POST /v1/auth/signout").len(), 1);
    assert_eq!(s.last_seq(), 0, "the next account reads its log from the start");
    assert_eq!(s.last_sync_ms(), None);

    // Signed out, the Devices list and revoking have no token: nothing is sent.
    run.send(Command::LoadDevices);
    run.send(Command::RevokeDevice("tab".into()));
    run.send(Command::Kick);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(fake.requests("GET /v1/devices").len(), 2);
    assert_eq!(fake.requests("DELETE /v1/devices/tab").len(), 1);
}

#[test]
fn an_expired_token_is_refreshed_and_a_refused_socket_retries_with_a_fresh_one() {
    let fake = Fake::start();
    live_hub(&fake);
    let s = store();
    let me = s.device();
    fake.route("POST /v1/auth/refresh", 200, json(&session(&me, "acc-2", now() + DAY)));
    fake.route("POST /v1/auth/refresh", 200, json(&session(&me, "acc-3", now() + DAY)));
    // The hub's clock says acc-2 is already old.
    fake.refuse_socket(401);
    let mut run = start(&fake, s, Some(session(&me, "acc-1", 0)), None);
    run.status("live", |st| st.sync == SyncState::Live);
    let refreshes = fake.requests("POST /v1/auth/refresh");
    assert_eq!(refreshes.len(), 2, "once for the expiry, once after the 401");
    assert!(refreshes[0].body.contains("refresh-acc-1"), "{}", refreshes[0].body);
    assert!(refreshes[1].body.contains("refresh-acc-2"), "the rotated token is used: {}", refreshes[1].body);
    assert_eq!(fake.sockets(), [Some("Bearer acc-3".to_owned())]);
    assert!(run.statuses.iter().any(|st| st.sync == SyncState::Offline), "it waited before retrying");
}

#[test]
fn a_devices_call_answered_401_refreshes_once_then_counts_as_signed_out() {
    let fake = Fake::start();
    let s = store();
    let me = s.device();
    fake.refuse_socket(500).refuse_socket(500).refuse_socket(500);
    fake.route("GET /v1/devices", 401, r#"{"code":"unauthorized","message":"old token"}"#);
    fake.route("POST /v1/auth/refresh", 200, json(&session(&me, "acc-2", now() + DAY)));
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    // The hub is down: the device says it's offline, and why.
    let offline = run.status("network problem", |st| st.problem == Some(Problem::Network));
    assert_eq!(offline.email.as_deref(), Some("mohamed@example.com"));
    run.send(Command::LoadDevices);
    let out = run.status("signed out remotely", |st| st.problem == Some(Problem::SignedOutRemotely));
    assert_eq!(out.sync, SyncState::SignedOut);
    assert_eq!(fake.requests("GET /v1/devices").len(), 2, "retried once with a fresh token");
    assert_eq!(fake.requests("POST /v1/auth/refresh").len(), 1);
    assert_eq!(fake.requests("GET /v1/devices")[1].auth.as_deref(), Some("Bearer acc-2"));
}

#[test]
fn a_server_error_on_the_devices_list_says_so() {
    let fake = Fake::start();
    live_hub(&fake);
    let s = store();
    let me = s.device();
    fake.route("GET /v1/devices", 502, "<html>bad gateway</html>");
    fake.route("DELETE /v1/devices/tab", 500, r#"{"code":"internal","message":"db"}"#);
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    run.status("live", |st| st.sync == SyncState::Live);
    run.send(Command::LoadDevices);
    assert_eq!(
        run.event("error", |e| matches!(e, Event::Devices(Err(_)))),
        Event::Devices(Err(Problem::Server))
    );
    run.send(Command::RevokeDevice("tab".into()));
    run.event("error again", |e| matches!(e, Event::Devices(Err(_))));
    std::thread::sleep(Duration::from_millis(50));
    run.pump();
    let last = run.statuses.last().unwrap();
    assert_eq!(
        (last.problem, last.sync),
        (Some(Problem::Server), SyncState::Live),
        "still live, with the reason"
    );
}

#[test]
fn a_refresh_the_hub_refuses_signs_the_device_out() {
    let fake = Fake::start();
    let s = store();
    let me = s.device();
    fake.route("POST /v1/auth/refresh", 401, r#"{"code":"device_revoked","message":"gone"}"#);
    let mut run = start(&fake, s, Some(session(&me, "acc-1", 0)), None);
    let out = run.status("signed out", |st| st.problem == Some(Problem::SignedOutRemotely));
    assert_eq!(out.sync, SyncState::SignedOut);
    assert!(fake.sockets().is_empty());
}

/// Starts a signed-in engine against a hub that answers hello with `replies`.
fn hub_says(replies: impl Fn() -> Vec<Reply> + Send + 'static) -> (Fake, Run) {
    let fake = Fake::start();
    fake.on_message(move |m| if matches!(m, ClientMsg::Hello { .. }) { replies() } else { Vec::new() });
    let s = store();
    let me = s.device();
    let run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    (fake, run)
}

#[test]
fn signed_out_from_another_device_stops_and_says_so() {
    let (_fake, mut run) = hub_says(|| vec![Reply::Msg(ServerMsg::Revoked), Reply::Close(4001)]);
    let st = run.status("revoked", |st| st.problem == Some(Problem::SignedOutRemotely));
    assert_eq!(st.sync, SyncState::SignedOut);
    // The session is gone from the vault too: a sign-in starts over.
    assert_eq!(run.store.last_seq(), 0);
}

#[test]
fn a_close_with_the_signed_out_code_counts_as_revoked() {
    let (_fake, mut run) = hub_says(|| vec![Reply::Close(4001)]);
    let st = run.status("revoked", |st| st.problem == Some(Problem::SignedOutRemotely));
    assert_eq!(st.sync, SyncState::SignedOut);
}

#[test]
fn a_newer_protocol_stops_sync_until_the_app_updates() {
    let (fake, mut run) = hub_says(|| {
        vec![Reply::Msg(ServerMsg::Error { code: SyncErrorCode::UnsupportedProtocol, message: "v9".into() })]
    });
    let st = run.status("outdated", |st| st.problem == Some(Problem::Outdated));
    assert_eq!(st.sync, SyncState::Stopped);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(fake.sockets().len(), 1, "no reconnects while stopped");
}

#[test]
fn a_hub_error_reconnects_and_a_dropped_socket_comes_back() {
    let fake = Fake::start();
    let mut hellos = 0;
    fake.on_message(move |m| {
        if !matches!(m, ClientMsg::Hello { .. }) {
            return Vec::new();
        }
        hellos += 1;
        match hellos {
            1 => vec![Reply::Msg(ServerMsg::Error { code: SyncErrorCode::Internal, message: "busy".into() })],
            2 => vec![Reply::Close(1011)],
            _ => vec![Reply::Msg(ServerMsg::CaughtUp { head_seq: 7 })],
        }
    });
    let s = store();
    let me = s.device();
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    run.status("live", |st| st.sync == SyncState::Live);
    assert_eq!(fake.sockets().len(), 3);
    assert_eq!(s.last_seq(), 7, "caught up to the head");
    assert!(run.statuses.iter().any(|st| st.sync == SyncState::CatchingUp));
}

#[test]
fn a_clock_ahead_keeps_changes_in_the_outbox_and_says_so() {
    let fake = Fake::start();
    fake.on_message(|m| match m {
        ClientMsg::Hello { .. } => vec![Reply::Msg(ServerMsg::CaughtUp { head_seq: 0 })],
        ClientMsg::Push { batch, ops } => vec![Reply::Msg(ServerMsg::Ack {
            batch: *batch,
            done: Vec::new(),
            rejected: ops
                .iter()
                .map(|o| Rejected { op_id: o.id.clone(), reason: RejectReason::ClockSkew })
                .collect(),
            head_seq: 0,
        })],
        ClientMsg::Ping => Vec::new(),
    });
    let s = store();
    let me = s.device();
    s.put_setting("plan", &serde_json::json!({ "weekdays": [true, true, true, true, true, false, false] }))
        .unwrap();
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    let st = run.status("clock off", |st| st.problem == Some(Problem::ClockOff));
    assert_eq!(st.pending, 1, "kept for later");
    // Held: a kick sends nothing more for now.
    run.send(Command::Kick);
    std::thread::sleep(Duration::from_millis(200));
    let pushes = fake.received().iter().filter(|m| matches!(m, ClientMsg::Push { .. })).count();
    assert_eq!(pushes, 1);
}

#[test]
fn a_final_rejection_drops_the_op_and_an_unreadable_one_is_skipped() {
    let fake = Fake::start();
    fake.on_message(|m| match m {
        ClientMsg::Hello { .. } => vec![Reply::Msg(ServerMsg::CaughtUp { head_seq: 0 })],
        ClientMsg::Push { batch, ops } => vec![Reply::Msg(ServerMsg::Ack {
            batch: *batch,
            done: Vec::new(),
            rejected: ops
                .iter()
                .map(|o| Rejected { op_id: o.id.clone(), reason: RejectReason::UnknownTable })
                .collect(),
            head_seq: 0,
        })],
        ClientMsg::Ping => Vec::new(),
    });
    let s = store();
    let me = s.device();
    s.put_setting("sound", &serde_json::json!({ "on": true })).unwrap();
    let mut run = start(&fake, s, Some(session(&me, "acc-1", now() + DAY)), None);
    run.status("emptied", |st| st.sync == SyncState::Live && st.pending == 0);
    assert_eq!(s.outbox_len(), 0);
}

// --- Google sign-in ---------------------------------------------------

/// Plays the browser: waits for the engine to open Google's page for
/// `client`, then sends the redirect with `query` (`code=…`), carrying the
/// page's state unless `query` has its own.
fn browser_returns(client: &str, query: &str) {
    let end = Instant::now() + Duration::from_secs(5);
    let url = loop {
        let found = super::OPENED
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|u| u.contains(&format!("client_id={client}&")))
            .cloned();
        if let Some(u) = found {
            break u;
        }
        assert!(Instant::now() < end, "Google's page never opened for {client}");
        std::thread::sleep(Duration::from_millis(10));
    };
    let param = |name: &str| {
        url.split(['?', '&']).find_map(|p| p.strip_prefix(&format!("{name}="))).unwrap().to_owned()
    };
    let redirect = param("redirect_uri").replace("%3A", ":").replace("%2F", "/");
    let state = param("state");
    let get = |path: String| {
        let agent = crate::update::agent(&redirect, Duration::from_secs(5));
        let mut res = agent.get(&format!("{redirect}{path}")).call().unwrap();
        (res.status().as_u16(), res.body_mut().read_to_string().unwrap_or_default())
    };
    // A stray request on the port first: it gets a 404 and the wait goes on.
    assert_eq!(get("/favicon.ico".into()).0, 404);
    let (status, page) = get(format!("/?{query}&state={state}"));
    assert_eq!(status, 200);
    assert!(page.contains("lang=\"ar\""), "{page}");
}

fn google(id: &str) -> Option<GoogleClient> {
    Some(GoogleClient { client_id: id.into(), client_secret: Some("not-secret".into()) })
}

fn token_endpoint(client: &str, fake: &Fake) {
    super::oauth::TOKEN_URLS.lock().unwrap().push((client.into(), format!("{}/token", fake.url)));
}

#[test]
fn signing_in_with_google_reaches_steps_cloud_and_goes_live() {
    let fake = Fake::start();
    live_hub(&fake);
    let s = store();
    let me = s.device();
    token_endpoint("steps-test-ok", &fake);
    fake.route("POST /token", 200, r#"{"id_token":"google-id-token"}"#);
    fake.route("POST /v1/auth/google", 200, json(&session(&me, "acc-g", now() + DAY)));
    let mut run = start(&fake, s, None, google("steps-test-ok"));
    assert_eq!(run.status("signed out", |_| true).sync, SyncState::SignedOut);
    run.send(Command::SignInGoogle);
    assert_eq!(run.event("browser open", |e| matches!(e, Event::SigningIn(_))), Event::SigningIn(true));
    browser_returns("steps-test-ok", "code=4%2F0Ab");
    assert_eq!(run.event("browser done", |e| matches!(e, Event::SigningIn(_))), Event::SigningIn(false));
    let live = run.status("live", |st| st.sync == SyncState::Live);
    assert_eq!(live.email.as_deref(), Some("mohamed@example.com"));
    let token = fake.requests("POST /token");
    assert!(token[0].body.contains("code=4%2F0Ab"), "{}", token[0].body);
    assert!(token[0].body.contains("client_secret=not-secret"));
    let sign_in = fake.requests("POST /v1/auth/google");
    assert!(sign_in[0].body.contains("google-id-token"), "{}", sign_in[0].body);
    assert!(sign_in[0].body.contains(&me));
}

#[test]
fn a_cancelled_or_failed_google_page_says_which() {
    let fake = Fake::start();
    let mut run = start(&fake, store(), None, google("steps-test-cancel"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-cancel", "error=access_denied");
    assert_eq!(run.status("cancelled", |st| st.problem.is_some()).problem, Some(Problem::SignInCancelled));

    let mut run = start(&fake, store(), None, google("steps-test-error"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-error", "error=server_error");
    assert_eq!(run.status("failed", |st| st.problem.is_some()).problem, Some(Problem::SignInFailed));
}

#[test]
fn a_refused_code_or_account_ends_the_sign_in() {
    let fake = Fake::start();
    token_endpoint("steps-test-badcode", &fake);
    fake.route("POST /token", 400, r#"{"error":"invalid_grant"}"#);
    let mut run = start(&fake, store(), None, google("steps-test-badcode"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-badcode", "code=used");
    let st = run.status("failed", |st| st.problem.is_some());
    assert_eq!((st.problem, st.finishing_sign_in), (Some(Problem::SignInFailed), false));

    // Another Google account than Mohamed's: Steps Cloud says no.
    let fake = Fake::start();
    token_endpoint("steps-test-other", &fake);
    fake.route("POST /token", 200, r#"{"id_token":"someone-else"}"#);
    fake.route("POST /v1/auth/google", 403, r#"{"code":"not_allowed","message":"not on the list"}"#);
    let mut run = start(&fake, store(), None, google("steps-test-other"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-other", "code=ok");
    assert_eq!(run.status("not allowed", |st| st.problem.is_some()).problem, Some(Problem::NotAllowed));

    // Steps Cloud fails in another way.
    let fake = Fake::start();
    token_endpoint("steps-test-500", &fake);
    fake.route("POST /token", 200, r#"{"id_token":"t"}"#);
    fake.route("POST /v1/auth/google", 500, "oops");
    let mut run = start(&fake, store(), None, google("steps-test-500"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-500", "code=ok");
    assert_eq!(run.status("failed", |st| st.problem.is_some()).problem, Some(Problem::SignInFailed));
}

#[test]
fn an_approved_sign_in_waits_for_the_network_and_retries_on_a_kick() {
    // No token endpoint for this client: Google's real one is out of reach
    // under test, as it is for a backgrounded Android app.
    let fake = Fake::start();
    let mut run = start(&fake, store(), None, google("steps-test-offline"));
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-offline", "code=ok");
    let st = run.status("waiting", |st| st.problem == Some(Problem::SignInWaiting));
    assert!(st.finishing_sign_in);
    assert_eq!(st.sync, SyncState::SignedOut);
    // Back in front: it tries again at once, and keeps waiting.
    let before = run.statuses.len();
    run.send(Command::Kick);
    std::thread::sleep(Duration::from_millis(300));
    run.pump();
    assert!(run.statuses[before..].iter().all(|st| st.finishing_sign_in));
    // A new sign-in replaces the waiting one.
    run.send(Command::SignInGoogle);
    assert_eq!(run.event("browser open", |e| *e == Event::SigningIn(true)), Event::SigningIn(true));
}

#[test]
fn a_sign_in_that_reaches_google_but_not_steps_cloud_waits_too() {
    let fake = Fake::start();
    token_endpoint("steps-test-cloud-down", &fake);
    fake.route("POST /token", 200, r#"{"id_token":"t"}"#);
    // Steps Cloud's address answers nothing.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (ev_tx, ev_rx) = unbounded_channel();
    let s = store();
    let device = DeviceInfo {
        device_id: s.device(),
        name: "Steps (dev)".into(),
        model: None,
        platform: Platform::Other,
    };
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let engine = Engine::new(
            s,
            Api::new(&dead),
            google("steps-test-cloud-down"),
            device,
            Box::new(MemoryVault::default()),
            ev_tx,
        );
        rt.block_on(engine.run(cmd_rx));
    });
    let mut run = Run { commands: cmd_tx, events: ev_rx, store: s, statuses: Vec::new(), others: Vec::new() };
    run.send(Command::SignInGoogle);
    browser_returns("steps-test-cloud-down", "code=ok");
    let st = run.status("waiting", |st| st.problem == Some(Problem::SignInWaiting));
    assert!(st.finishing_sign_in);
    assert_eq!(fake.requests("POST /token").len(), 1, "the code is exchanged once");
}

#[test]
fn without_a_google_client_sign_in_is_not_configured() {
    let fake = Fake::start();
    let mut run = start(&fake, store(), None, None);
    run.send(Command::SignInGoogle);
    assert_eq!(run.status("problem", |st| st.problem.is_some()).problem, Some(Problem::NotConfigured));
    // Signing out while signed out is harmless.
    run.send(Command::SignOut);
    run.event("empty list", |e| *e == Event::Devices(Ok(Vec::new())));
    assert!(fake.requests("POST /v1/auth/signout").is_empty());
}

#[test]
fn the_sync_thread_starts_on_its_own_database() {
    let fake = Fake::start();
    let dir = tempfile::tempdir().unwrap();
    let config = super::CloudConfig { base_url: fake.url.clone(), google: None };
    let device = super::device_info("d-test".into());
    assert_eq!(
        (device.name.as_str(), device.platform, device.model.clone()),
        ("Steps (dev)", Platform::Other, None)
    );
    let mut handle =
        super::start(config, dir.path().join("steps.db"), device, Box::new(MemoryVault::default())).unwrap();
    let first = handle.events.blocking_recv().unwrap();
    let Event::Status(st) = first else { panic!("{first:?}") };
    assert_eq!(st.sync, SyncState::SignedOut);
    assert!(st.configured);
    handle.commands.send(Command::Shutdown).unwrap();
    // A database that can't be opened is an error, not a panic.
    let bad = super::start(
        super::CloudConfig { base_url: fake.url.clone(), google: None },
        dir.path().join("steps.db").join("inside-a-file"),
        super::device_info("d".into()),
        Box::new(MemoryVault::default()),
    );
    assert!(bad.is_err());
}

#[test]
fn not_configured_status_and_browser_name() {
    let st = CloudStatus::not_configured("d1".into());
    assert!(!st.configured);
    assert_eq!(st.sync, SyncState::SignedOut);
    assert_eq!(super::browser_name(), "Browser");
    assert!(super::open_url("https://example.invalid/never-opened").is_ok());
    assert!(super::OPENED.lock().unwrap().iter().any(|u| u == "https://example.invalid/never-opened"));
}

// --- The HTTP API and the session ---------------------------------------

fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
}

#[test]
fn the_api_talks_to_the_hub_and_reads_its_errors() {
    let fake = Fake::start();
    fake.route("POST /v1/auth/refresh", 200, "not json");
    fake.route("GET /v1/devices", 200, devices("d1"));
    fake.route("POST /v1/auth/signout", 204, "");
    let api = Api::new(&format!("{}/", fake.url));
    assert!(format!("{api:?}").contains(&fake.url));
    assert!(matches!(block(api.refresh("r")), Err(CloudError::Decode(_))));
    assert_eq!(block(api.devices("tok")).unwrap().devices.len(), 2);
    assert_eq!(fake.requests("GET /v1/devices")[0].auth.as_deref(), Some("Bearer tok"));
    assert_eq!(block(api.sign_out("tok")), Ok(()));
    let missing = block(api.revoke("tok", "ghost")).unwrap_err();
    assert!(matches!(missing, CloudError::Api { status: 404, .. }), "{missing:?}");
    assert!(missing.to_string().starts_with("404 NotFound"));
    // Nothing listening: a network error.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = Api::new(&format!("http://{}", closed.local_addr().unwrap()));
    drop(closed);
    let e = block(dead.devices("tok")).unwrap_err();
    assert!(matches!(e, CloudError::Network(_)) && e.to_string().starts_with("network: "), "{e}");
    assert_eq!(CloudError::Decode("x".into()).to_string(), "unexpected response: x");
    // Error pages without the API's body still get a code from the status.
    assert!(api_error(401, "").is_signed_out());
    assert!(matches!(
        api_error(404, ""),
        CloudError::Api { code: steps_protocol::cloud::ErrorCode::NotFound, .. }
    ));
    assert!(matches!(
        api_error(500, ""),
        CloudError::Api { code: steps_protocol::cloud::ErrorCode::Internal, .. }
    ));
}

/// A vault that never keeps anything (a locked keychain).
struct Refusing;
impl SessionVault for Refusing {
    fn load(&self) -> Option<Session> {
        None
    }
    fn save(&self, _: &Session) -> bool {
        false
    }
    fn clear(&self) {}
}

#[test]
fn a_session_the_vault_cannot_keep_still_works_on_the_old_refresh_token() {
    let fake = Fake::start();
    fake.route("POST /v1/auth/refresh", 200, json(&session("d1", "acc-2", now() + DAY)));
    let api = Api::new(&fake.url);
    let first = session("d1", "acc-1", 0);
    // Not kept: sign-in fails and the vault comes back.
    assert!(Auth::new(first.clone(), Box::new(Refusing)).is_err());
    // Kept once, then the vault stops taking writes: the new access token is
    // used with the refresh token the vault still holds.
    struct Once(std::sync::atomic::AtomicBool);
    impl SessionVault for Once {
        fn load(&self) -> Option<Session> {
            None
        }
        fn save(&self, _: &Session) -> bool {
            !self.0.swap(true, std::sync::atomic::Ordering::SeqCst)
        }
        fn clear(&self) {}
    }
    let Ok(mut auth) = Auth::new(first, Box::new(Once(Default::default()))) else { panic!("kept once") };
    assert_eq!(block(auth.access_token(&api, now(), false)).unwrap(), "acc-2");
    assert_eq!(auth.session().refresh_token, "refresh-acc-1");
    // A fresh token needs no refresh unless forced.
    assert_eq!(block(auth.access_token(&api, 0, false)).unwrap(), "acc-2");
    assert_eq!(fake.requests("POST /v1/auth/refresh").len(), 1);
    // An expired refresh token restores nothing.
    let vault = MemoryVault::default();
    let mut old = session("d1", "acc-1", 0);
    old.refresh_expires_at_ms = 5;
    vault.save(&old);
    assert!(Auth::restore(Box::new(vault), 10).is_err());
}

#[test]
fn the_keyring_vault_keeps_and_forgets_the_session() {
    let v = super::session::KeyringVault::new();
    let s = session("d1", "acc-1", 1);
    if v.save(&s) {
        assert_eq!(v.load(), Some(s));
        v.clear();
    }
    assert_eq!(v.load(), None);
}
