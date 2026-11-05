//! The sync engine: one task that owns the session, the WebSocket and its own
//! database connection. It drives [`SyncClient`] and reports to the UI.
// adapted from batna-app main: crates/app/src/cloud/engine.rs

use std::collections::HashSet;
use std::time::Duration;

use steps_protocol::client::{Backoff, Effect, Phase, SyncClient};
use steps_protocol::cloud::{
    DeviceInfo, DeviceSummary, ErrorCode, GoogleSignIn, PasswordLogin, PasswordSignIn, RemovePassword,
    Session, SetPassword,
};
use steps_protocol::sync::{ClientMsg, MAX_OPS_PER_PUSH, ServerMsg};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use web_time::Instant;

use super::GoogleClient;
use super::api::{Api, CloudError};
use super::oauth::{self, OAuthError};
use super::password::{self, Invalid};
use super::session::{Auth, SessionVault};
use super::socket::{ConnectError, Frame, Socket};
use super::{CloudStatus, Command, Event, LoginStatus, Problem, SyncState};
use crate::store::Store;

type Ws = Socket;

/// Keeps NAT mappings and the Cloudflare edge from dropping an idle socket.
/// Protocol pings are answered by the runtime, so they don't wake the hub.
const PING_EVERY: Duration = Duration::from_secs(30);
/// Writes from screens that don't send [`Command::Kick`] still go up this soon.
const OUTBOX_EVERY: Duration = Duration::from_secs(2);
/// After the hub says this device's clock is ahead, pushes wait this long.
const CLOCK_RETRY: Duration = Duration::from_secs(60);
/// How long a sign-in Google already approved keeps retrying: Google's codes
/// live about ten minutes, its ID tokens an hour.
const CODE_LIFE: Duration = Duration::from_secs(9 * 60);
const ID_TOKEN_LIFE: Duration = Duration::from_secs(50 * 60);

/// A sign-in Google approved that still has to reach Google's token endpoint
/// or Steps Cloud. Android keeps a backgrounded app off the network (DNS
/// answers nothing, open sockets are aborted), and the redirect lands while
/// Chrome is still in front, so the network half waits and retries until
/// Steps is back in front ([`Command::Kick`]) or the grant runs out.
struct PendingSignIn {
    step: SignInStep,
    until: Instant,
    retry_at: Instant,
    attempts: u32,
}

enum SignInStep {
    /// The Mac and Android: a code for Google's token endpoint.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Code(oauth::Authorization),
    IdToken(String),
}

impl PendingSignIn {
    fn new(step: SignInStep, life: Duration) -> Self {
        let now = Instant::now();
        Self { step, until: now + life, retry_at: now, attempts: 0 }
    }

    /// 1, 2, 4, 8, 15, 15… seconds: quick enough to finish right after the
    /// app comes back even if no Kick arrives, cheap while it can't.
    fn schedule_retry(&mut self) {
        const DELAYS: [u64; 5] = [1, 2, 4, 8, 15];
        let secs = DELAYS[(self.attempts as usize).min(DELAYS.len() - 1)];
        self.attempts += 1;
        self.retry_at = Instant::now() + Duration::from_secs(secs);
    }
}

pub struct Engine {
    store: &'static Store,
    api: Api,
    google: Option<GoogleClient>,
    device: DeviceInfo,
    auth: Option<Auth>,
    /// Kept while signed out, for the next sign-in.
    vault: Option<Box<dyn SessionVault>>,
    client: SyncClient,
    ws: Option<Ws>,
    backoff: Backoff,
    retry_at: Instant,
    /// Set after `Revoked`, `Outdated` or sign-out: no reconnects until sign-in.
    stopped: bool,
    /// The last status sent, so an unchanged one isn't sent again.
    last_status: Option<CloudStatus>,
    force_refresh: bool,
    problem: Option<Problem>,
    /// Pushes wait until then (the clock was ahead); pulls go on.
    hold_pushes_until: Option<Instant>,
    /// Tables the hub answered `unknown_table` for (it is older than this
    /// app). Their ops stay in the outbox and are skipped until the next
    /// connection, which tries them again.
    hub_lacks: HashSet<String>,
    pending_sign_in: Option<PendingSignIn>,
    /// The usual vault, set aside while a sign-in on someone else's device
    /// keeps its session in a forgetful one; back after sign-out.
    spare_vault: Option<Box<dyn SessionVault>>,
    login: LoginStatus,
    events: UnboundedSender<Event>,
}

impl Engine {
    pub fn new(
        store: &'static Store,
        api: Api,
        google: Option<GoogleClient>,
        device: DeviceInfo,
        vault: Box<dyn SessionVault>,
        events: UnboundedSender<Event>,
    ) -> Self {
        let client = SyncClient::new(store.last_seq());
        let (auth, vault) = match Auth::restore(vault, now_ms()) {
            Ok(auth) => (Some(auth), None),
            Err(vault) => (None, Some(vault)),
        };
        // The web: Google came back to the page with an ID token (`oauth`).
        #[cfg(target_arch = "wasm32")]
        let pending_sign_in = oauth::take_returned_id_token()
            .map(|t| PendingSignIn::new(SignInStep::IdToken(t), ID_TOKEN_LIFE));
        #[cfg(not(target_arch = "wasm32"))]
        let pending_sign_in = None;
        Self {
            store,
            api,
            google,
            device,
            auth,
            vault,
            client,
            ws: None,
            backoff: Backoff::default(),
            retry_at: Instant::now(),
            stopped: false,
            last_status: None,
            force_refresh: false,
            problem: None,
            hold_pushes_until: None,
            hub_lacks: HashSet::new(),
            pending_sign_in,
            spare_vault: None,
            login: LoginStatus::default(),
            events,
        }
    }

    pub async fn run(mut self, mut commands: UnboundedReceiver<Command>) {
        // Both ticks only run while a socket is open.
        let mut next_ping = Instant::now() + PING_EVERY;
        let mut next_outbox = Instant::now() + OUTBOX_EVERY;
        self.emit_status();
        loop {
            if self.pending_sign_in.as_ref().is_some_and(|p| Instant::now() >= p.retry_at) {
                self.finish_sign_in().await;
                self.emit_status();
            }
            if self.ws.is_none() && self.wants_socket() && Instant::now() >= self.retry_at {
                self.connect().await;
                self.emit_status();
            }
            let mut wait_until = if self.ws.is_none() && self.wants_socket() {
                self.retry_at
            } else {
                Instant::now() + Duration::from_secs(3600)
            };
            if let Some(p) = &self.pending_sign_in {
                wait_until = wait_until.min(p.retry_at);
            }
            if self.ws.is_some() {
                wait_until = wait_until.min(next_ping).min(next_outbox);
            }
            let nap = wait_until.saturating_duration_since(Instant::now());
            tokio::select! {
                cmd = commands.recv() => match cmd {
                    None | Some(Command::Shutdown) => break,
                    Some(cmd) => self.command(cmd).await,
                },
                frame = next_frame(&mut self.ws) => self.frame(frame).await,
                _ = crate::platform::time::sleep(nap) => {}
            }
            let now = Instant::now();
            if self.ws.is_none() {
                next_ping = now + PING_EVERY;
                next_outbox = now + OUTBOX_EVERY;
            }
            if self.ws.is_some() && now >= next_outbox {
                next_outbox = now + OUTBOX_EVERY;
                self.pump().await;
            }
            if now >= next_ping {
                next_ping = now + PING_EVERY;
                if let Some(ws) = &mut self.ws
                    && ws.ping().await.is_err()
                {
                    self.drop_socket();
                }
            }
            self.emit_status();
        }
        if let Some(ws) = self.ws.take() {
            ws.close().await;
        }
    }

    fn wants_socket(&self) -> bool {
        self.auth.is_some() && !self.stopped
    }

    // --- commands --------------------------------------------------------

    async fn command(&mut self, cmd: Command) {
        match cmd {
            Command::SignInGoogle => {
                let Some(google) = self.google.clone() else {
                    self.problem = Some(Problem::NotConfigured);
                    return;
                };
                self.problem = None;
                self.pending_sign_in = None;
                self.events.send(Event::SigningIn(true)).ok();
                self.sign_in_google(google).await;
            }
            Command::SignInPassword { username, password, remember } => {
                self.problem = None;
                self.pending_sign_in = None;
                self.events.send(Event::SigningIn(true)).ok();
                self.sign_in_password(username, password, remember).await;
                self.events.send(Event::SigningIn(false)).ok();
            }
            Command::LoadLogin => {
                let result = self.authed(|api, access| async move { api.login(&access).await }).await;
                self.login_answer(result);
            }
            Command::SetLogin { username, password, current } => {
                self.set_login(username, password, current).await
            }
            Command::RemoveLogin { current } => {
                self.login_busy();
                let Some(current_key) = self.current_key(current).await else { return };
                let body = RemovePassword { current_key };
                let call = move |api: Api, access: String| {
                    let body = body.clone();
                    async move { api.remove_login(&access, &body).await }
                };
                let result = self.authed(call).await;
                self.login_answer(result);
            }
            Command::SignOut => {
                let _ = self.authed(|api, access| async move { api.sign_out(&access).await }).await;
                self.sign_out_locally();
            }
            Command::LoadDevices => self.load_devices().await,
            Command::RevokeDevice(device) => {
                if device == self.device.device_id {
                    return Box::pin(self.command(Command::SignOut)).await;
                }
                let call = move |api: Api, access: String| {
                    let device = device.clone();
                    async move { api.revoke(&access, &device).await }
                };
                if let Some(Err(e)) = self.authed(call).await {
                    self.cloud_error(&e);
                }
                self.load_devices().await;
            }
            Command::Kick => {
                // Also sent when Steps comes back to the front: a waiting
                // sign-in tries again right away.
                if let Some(p) = &mut self.pending_sign_in {
                    p.retry_at = Instant::now();
                }
                self.pump().await
            }
            Command::Shutdown => {}
        }
    }

    /// The web leaves the page for Google and comes back with an ID token
    /// (`Engine::new`); this only returns if it couldn't go.
    #[cfg(target_arch = "wasm32")]
    async fn sign_in_google(&mut self, google: GoogleClient) {
        let error = oauth::go_to_google(&google).await;
        self.events.send(Event::SigningIn(false)).ok();
        crate::diag::report!("Google sign-in: {error}");
        self.problem = Some(Problem::SignInFailed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn sign_in_google(&mut self, google: GoogleClient) {
        let authorization = oauth::google_authorization(&google).await;
        self.events.send(Event::SigningIn(false)).ok();
        match authorization {
            Ok(auth) => {
                self.pending_sign_in = Some(PendingSignIn::new(SignInStep::Code(auth), CODE_LIFE));
                self.finish_sign_in().await;
            }
            Err(OAuthError::Cancelled) => self.problem = Some(Problem::SignInCancelled),
            Err(e) => {
                crate::diag::report!("Google sign-in: {e}");
                self.problem = Some(Problem::SignInFailed);
            }
        }
    }

    /// Runs the network half of a sign-in Google approved: the code
    /// exchange, then Steps Cloud. A network failure keeps it pending and
    /// retries ([`PendingSignIn`]); anything else ends it.
    async fn finish_sign_in(&mut self) {
        let Some(pending) = self.pending_sign_in.take() else { return };
        #[cfg(not(target_arch = "wasm32"))]
        let mut pending = pending;
        if Instant::now() >= pending.until {
            crate::diag::report!("Steps Cloud sign-in: gave up waiting for the network");
            self.problem = Some(Problem::SignInFailed);
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let SignInStep::Code(auth) = &pending.step {
            let Some(google) = self.google.clone() else {
                self.problem = Some(Problem::NotConfigured);
                return;
            };
            let auth = auth.clone();
            match blocking(move || oauth::exchange(&google, &auth)).await {
                Ok(id_token) => pending = PendingSignIn::new(SignInStep::IdToken(id_token), ID_TOKEN_LIFE),
                Err(OAuthError::Network(e)) => {
                    crate::diag::report!("Google sign-in: network: {e}");
                    self.wait_for_network(pending);
                    return;
                }
                Err(e) => {
                    crate::diag::report!("Google sign-in: {e}");
                    self.problem = Some(Problem::SignInFailed);
                    return;
                }
            }
        }
        let SignInStep::IdToken(id_token) = &pending.step else { return };
        let body = GoogleSignIn {
            id_token: id_token.clone(),
            device: self.device.clone(),
            nonce: oauth::sign_in_nonce(),
        };
        match self.api.sign_in_google(&body).await {
            Ok(session) => self.begin_session(session, true),
            Err(CloudError::Network(e)) => {
                crate::diag::report!("Steps Cloud sign-in: network: {e}");
                self.wait_for_network(pending);
            }
            Err(e) => {
                crate::diag::report!("Steps Cloud sign-in: {e}");
                self.problem = Some(match &e {
                    CloudError::Api { code: ErrorCode::NotAllowed, .. } => Problem::NotAllowed,
                    _ => Problem::SignInFailed,
                });
            }
        }
    }

    /// Keeps a session Steps Cloud just gave and starts syncing: the whole
    /// account comes down from the start of its log, so a new device ends
    /// up where the others are. `remember: false` keeps it until the app or
    /// tab closes ([`super::session::not_remembered`]).
    fn begin_session(&mut self, session: Session, remember: bool) {
        let usual = self.vault.take().unwrap_or_else(|| Box::new(super::session::MemoryVault::default()));
        let vault = if remember {
            usual
        } else {
            // An older session for this device mustn't outlive this one.
            usual.clear();
            self.spare_vault = Some(usual);
            super::session::not_remembered()
        };
        match Auth::new(session, vault) {
            Ok(auth) => self.auth = Some(auth),
            Err(vault) => {
                crate::diag::report!("Steps Cloud: sign-in session was not saved");
                self.vault = Some(self.spare_vault.take().unwrap_or(vault));
                self.problem = Some(Problem::SignInFailed);
                return;
            }
        }
        self.problem = None;
        self.stopped = false;
        self.client = SyncClient::new(self.store.last_seq());
        self.backoff.reset();
        self.retry_at = Instant::now();
    }

    async fn sign_in_password(&mut self, username: String, password: String, remember: bool) {
        let key = match stretch(username.clone(), password).await {
            Ok(k) => k,
            Err(e) => {
                crate::diag::report!("password sign-in: {e}");
                self.problem = Some(Problem::SignInFailed);
                return;
            }
        };
        let body = PasswordSignIn { username, password_key: key, device: self.device.clone() };
        match self.api.sign_in_password(&body).await {
            Ok(session) => self.begin_session(session, remember),
            Err(e) => {
                crate::diag::report!("password sign-in: {e}");
                self.problem = Some(password_problem(&e).unwrap_or(match e {
                    CloudError::Network(_) => Problem::Network,
                    CloudError::Api { code: ErrorCode::NotAllowed, .. } => Problem::NotAllowed,
                    _ => Problem::SignInFailed,
                }));
            }
        }
    }

    fn login_busy(&mut self) {
        self.login.saving = true;
        self.login.saved = false;
        self.login.problem = None;
        self.events.send(Event::Login(self.login.clone())).ok();
    }

    /// Settings shows what Steps Cloud said about the username, or why it
    /// couldn't say.
    fn login_answer(&mut self, result: Option<Result<PasswordLogin, CloudError>>) {
        let was_saving = std::mem::take(&mut self.login.saving);
        match result {
            Some(Ok(l)) => {
                self.login.username = Some(l.username);
                self.login.needs_current = l.needs_current;
                self.login.saved = was_saving;
                self.login.problem = None;
            }
            Some(Err(e)) => {
                // Here a wrong password can only be the current one.
                let p = password_problem(&e)
                    .map(|p| if p == Problem::WrongPassword { Problem::CurrentWrong } else { p });
                self.login.problem = Some(p.unwrap_or_else(|| {
                    self.cloud_error(&e);
                    self.problem.unwrap_or(Problem::Server)
                }));
            }
            None => self.login.problem = Some(self.problem.unwrap_or(Problem::Network)),
        }
        self.events.send(Event::Login(self.login.clone())).ok();
    }

    /// The current password's key for the account's username (it may differ
    /// from a new username being set). `Some(None)` when none was given; `None`
    /// when stretching failed (the reason is shown).
    async fn current_key(&mut self, current: Option<String>) -> Option<Option<String>> {
        let Some(current) = current.filter(|c| !c.is_empty()) else { return Some(None) };
        let Some(Some(name)) = self.login.username.clone() else { return Some(None) };
        match stretch(name, current).await {
            Ok(k) => Some(Some(k)),
            Err(e) => {
                crate::diag::report!("checking the current password: {e}");
                self.login.saving = false;
                self.login.problem = Some(Problem::Server);
                self.events.send(Event::Login(self.login.clone())).ok();
                None
            }
        }
    }

    async fn set_login(&mut self, username: String, password: String, current: Option<String>) {
        if let Err(why) = password::check_new(&username, &password) {
            self.login.problem = Some(match why {
                Invalid::Username => Problem::BadUsername,
                Invalid::TooShort => Problem::ShortPassword,
            });
            self.login.saved = false;
            self.events.send(Event::Login(self.login.clone())).ok();
            return;
        }
        self.login_busy();
        let key = match stretch(username.clone(), password).await {
            Ok(k) => k,
            Err(e) => {
                crate::diag::report!("setting the password: {e}");
                self.login.saving = false;
                self.login.problem = Some(Problem::Server);
                self.events.send(Event::Login(self.login.clone())).ok();
                return;
            }
        };
        let Some(current_key) = self.current_key(current).await else { return };
        let body = SetPassword { username, password_key: key, current_key };
        let call = move |api: Api, access: String| {
            let body = body.clone();
            async move { api.set_login(&access, &body).await }
        };
        let result = self.authed(call).await;
        self.login_answer(result);
    }

    fn wait_for_network(&mut self, mut pending: PendingSignIn) {
        pending.schedule_retry();
        self.pending_sign_in = Some(pending);
        self.problem = Some(Problem::SignInWaiting);
    }

    fn sign_out_locally(&mut self) {
        self.drop_socket();
        if let Some(auth) = self.auth.take() {
            let forgetful = auth.forget();
            self.vault = Some(self.spare_vault.take().unwrap_or(forgetful));
        }
        self.login = LoginStatus::default();
        self.events.send(Event::Login(LoginStatus::default())).ok();
        // The next sign-in may be another account: read its log from the start.
        let _ = self.store.reset_sync();
        self.stopped = false;
        self.events.send(Event::Devices(Ok(Vec::new()))).ok();
    }

    async fn load_devices(&mut self) {
        let Some(result) = self.authed(|api, access| async move { api.devices(&access).await }).await else {
            return;
        };
        let devices: Result<Vec<DeviceSummary>, Problem> = match result {
            Ok(list) => Ok(list.devices),
            Err(e) => {
                self.cloud_error(&e);
                Err(self.problem.unwrap_or(Problem::Network))
            }
        };
        self.events.send(Event::Devices(devices)).ok();
    }

    /// Runs a call with an access token. A 401 means the token is older than
    /// this device thinks (its clock runs behind the hub's), so it refreshes
    /// once and retries before anything counts as signed out (#261). `None`
    /// when there is no token (the reason is recorded).
    async fn authed<T, F>(&mut self, call: impl Fn(Api, String) -> F) -> Option<Result<T, CloudError>>
    where
        F: std::future::Future<Output = Result<T, CloudError>>,
    {
        let mut force = false;
        loop {
            let access = self.access(force).await?;
            match call(self.api.clone(), access).await {
                Err(CloudError::Api { code: ErrorCode::Unauthorized, .. }) if !force => force = true,
                result => return Some(result),
            }
        }
    }

    /// A fresh access token, or `None` (with the reason recorded).
    async fn access(&mut self, force: bool) -> Option<String> {
        let mut auth = self.auth.take()?;
        let result = auth.access_token(&self.api, now_ms(), force).await;
        self.auth = Some(auth);
        match result {
            Ok(token) => Some(token),
            Err(e) => {
                self.cloud_error(&e);
                None
            }
        }
    }

    fn cloud_error(&mut self, e: &CloudError) {
        crate::diag::report!("Steps Cloud: {e}");
        if e.is_signed_out() {
            self.problem = Some(Problem::SignedOutRemotely);
            self.sign_out_locally();
        } else {
            self.problem = Some(match e {
                CloudError::Network(_) => Problem::Network,
                _ => Problem::Server,
            });
        }
    }

    // --- socket ----------------------------------------------------------

    async fn connect(&mut self) {
        let force = std::mem::take(&mut self.force_refresh);
        let Some(access) = self.access(force).await else {
            self.schedule_retry();
            return;
        };
        match Socket::connect(&self.api.sync_url(), &access).await {
            Ok(ws) => {
                self.ws = Some(ws);
                self.problem = None;
                // The hub may have been updated since: offer those ops again.
                self.hub_lacks.clear();
                let effects = self.client.on_open();
                self.run_effects(effects).await;
            }
            Err(e) => {
                if matches!(e, ConnectError::Unauthorized) {
                    // Expired or revoked: refreshing tells which.
                    self.force_refresh = true;
                } else {
                    self.problem = Some(Problem::Network);
                    // A browser can't tell a refused upgrade from a dropped
                    // line: refresh first, which also tells if it was revoked.
                    self.force_refresh = cfg!(target_arch = "wasm32");
                }
                crate::diag::report!("sync connect: {e}");
                self.schedule_retry();
            }
        }
    }

    fn schedule_retry(&mut self) {
        let jitter = (now_ms() % 1000) as f64 / 1000.0;
        self.retry_at = Instant::now() + Duration::from_millis(self.backoff.next_delay_ms(jitter));
    }

    fn drop_socket(&mut self) {
        self.ws = None;
        self.client.on_close();
        if self.wants_socket() {
            self.schedule_retry();
        }
    }

    async fn frame(&mut self, frame: Frame) {
        let text = match frame {
            Frame::Text(text) => text,
            Frame::Closed(code) => {
                // 4001: signed out from another device (after `revoked`).
                if code == Some(4001) {
                    self.revoked();
                }
                self.drop_socket();
                return;
            }
            Frame::Other => return,
            Frame::Error(e) => {
                crate::diag::report!("sync socket: {e}");
                self.drop_socket();
                return;
            }
        };
        match serde_json::from_str::<ServerMsg>(&text) {
            Ok(msg) => {
                let effects = self.client.on_message(msg);
                self.run_effects(effects).await;
                if self.client.phase() == Phase::Live {
                    self.backoff.reset();
                }
                self.pump().await;
            }
            Err(e) => crate::diag::report!("unreadable sync message: {e}"),
        }
    }

    async fn run_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Send(msg) => self.send(&msg).await,
                Effect::Apply { ops, last_seq } => match self.store.apply_remote(&ops, last_seq) {
                    Ok(changed) => {
                        self.touch_last_sync();
                        if !changed.is_empty() {
                            self.events.send(Event::Changed(changed)).ok();
                        }
                    }
                    Err(e) => {
                        // Not saved, so the next hello brings them again.
                        crate::diag::report!("applying remote ops: {e}");
                        self.client.rewind(self.store.last_seq());
                        self.drop_socket();
                        return;
                    }
                },
                Effect::SaveLastSeq(seq) => {
                    let _ = self.store.set_last_seq(seq);
                }
                Effect::Acked { done, rejected } => {
                    for r in &rejected {
                        crate::diag::report!("op {} rejected: {:?}", r.op_id, r.reason);
                    }
                    if self.problem == Some(Problem::ClockOff) && !done.is_empty() {
                        self.problem = None;
                    }
                    let ids =
                        done.iter().map(String::as_str).chain(rejected.iter().map(|r| r.op_id.as_str()));
                    if let Err(e) = self.store.remove_from_outbox(ids) {
                        crate::diag::report!("outbox: {e}");
                    }
                    self.touch_last_sync();
                }
                Effect::ClockAhead { kept } => {
                    // Kept in the outbox; they go once the clock is right.
                    crate::diag::report!("clock ahead of the server: {} ops wait", kept.len());
                    self.problem = Some(Problem::ClockOff);
                    self.hold_pushes_until = Some(Instant::now() + CLOCK_RETRY);
                }
                Effect::HubBehind { kept } => {
                    let tables = self.store.outbox_tables(kept.iter().map(String::as_str));
                    crate::diag::report!("hub lacks {tables:?}: {} ops wait for an update", kept.len());
                    self.hub_lacks.extend(tables);
                }
                Effect::Reconnect { reason } => {
                    crate::diag::report!("sync reconnects: {reason}");
                    if let Some(ws) = self.ws.take() {
                        ws.close().await;
                    }
                    self.drop_socket();
                }
                Effect::Stop => {
                    if self.client.phase() == Phase::Revoked {
                        self.revoked();
                    } else {
                        self.problem = Some(Problem::Outdated);
                        self.stopped = true;
                    }
                    if let Some(ws) = self.ws.take() {
                        ws.close().await;
                    }
                    self.client.on_close();
                }
            }
        }
    }

    fn revoked(&mut self) {
        self.problem = Some(Problem::SignedOutRemotely);
        self.sign_out_locally();
    }

    /// Pushes the next outbox batch if the socket can take it.
    async fn pump(&mut self) {
        if self.ws.is_none() || self.hold_pushes_until.is_some_and(|t| Instant::now() < t) {
            return;
        }
        // Ops written while the clock was wrong go up with a fresh stamp (#539).
        if let Err(e) = self.store.restamp_future(crate::store::now_ms()) {
            crate::diag::report!("restamping outbox: {e}");
        }
        let outbox = match self.store.outbox_skipping(MAX_OPS_PER_PUSH, &self.hub_lacks) {
            Ok(o) => o,
            Err(e) => {
                crate::diag::report!("reading outbox: {e}");
                return;
            }
        };
        if let Some(msg) = self.client.next_push(&outbox) {
            self.send(&msg).await;
        }
    }

    async fn send(&mut self, msg: &ClientMsg) {
        let Some(ws) = &mut self.ws else { return };
        let text = serde_json::to_string(msg).unwrap_or_default();
        if let Err(e) = ws.send_text(text).await {
            crate::diag::report!("sync send: {e}");
            self.drop_socket();
        }
    }

    fn touch_last_sync(&mut self) {
        let _ = self.store.set_last_sync_ms(now_ms());
    }

    fn emit_status(&mut self) {
        let sync = match (&self.auth, self.client.phase(), &self.ws) {
            (None, _, _) => SyncState::SignedOut,
            (Some(_), _, _) if self.stopped => SyncState::Stopped,
            (Some(_), Phase::Live, Some(_)) => SyncState::Live,
            (Some(_), Phase::CatchingUp, Some(_)) => SyncState::CatchingUp,
            (Some(_), _, None) if Instant::now() < self.retry_at => SyncState::Offline,
            (Some(_), _, _) => SyncState::Connecting,
        };
        let status = CloudStatus {
            configured: true,
            email: self.auth.as_ref().and_then(|a| a.session().email.clone()),
            device_id: self.device.device_id.clone(),
            sync,
            pending: self.store.outbox_len(),
            last_sync_ms: self.store.last_sync_ms(),
            problem: self.problem,
            finishing_sign_in: self.pending_sign_in.is_some(),
            temporary: self.auth.as_ref().is_some_and(|a| !a.remembered()),
        };
        if self.last_status.as_ref() != Some(&status) {
            self.last_status = Some(status.clone());
            self.events.send(Event::Status(status)).ok();
        }
    }
}

async fn next_frame(ws: &mut Option<Ws>) -> Frame {
    match ws {
        Some(ws) => ws.next().await,
        None => std::future::pending().await,
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.expect("blocking task panicked")
}

pub fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// Stretches the password (`password::password_key`) off the UI thread
/// where there is one. The browser has one thread: a short nap first lets
/// the page show «بيدخل…» before the work starts.
async fn stretch(username: String, password: String) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    return blocking(move || password::password_key(&username, &password)).await;
    #[cfg(target_arch = "wasm32")]
    {
        crate::platform::time::sleep(Duration::from_millis(50)).await;
        password::password_key(&username, &password)
    }
}

/// What a password answer means for Mohamed, when it's about the password.
fn password_problem(e: &CloudError) -> Option<Problem> {
    match e {
        CloudError::Api { code: ErrorCode::WrongPassword, .. } => Some(Problem::WrongPassword),
        CloudError::Api { code: ErrorCode::UsernameTaken, .. } => Some(Problem::UsernameTaken),
        CloudError::Api { code: ErrorCode::TooManyTries, retry_after_ms, .. } => {
            Some(Problem::Locked(retry_after_ms.unwrap_or(60_000).div_ceil(60_000).max(1)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_answers_become_their_own_problems() {
        let api = |status, body: &str| super::super::api::api_error(status, body);
        assert_eq!(
            password_problem(&api(401, r#"{"code":"wrong_password","message":""}"#)),
            Some(Problem::WrongPassword)
        );
        assert_eq!(
            password_problem(&api(409, r#"{"code":"username_taken","message":""}"#)),
            Some(Problem::UsernameTaken)
        );
        // Minutes, rounded up: 61 seconds is "2 minutes", never "0".
        let locked = |ms: u64| {
            password_problem(&api(
                429,
                &format!(r#"{{"code":"too_many_tries","message":"","retry_after_ms":{ms}}}"#),
            ))
        };
        assert_eq!(locked(61_000), Some(Problem::Locked(2)));
        assert_eq!(locked(1), Some(Problem::Locked(1)));
        assert_eq!(locked(3_600_000), Some(Problem::Locked(60)));
        assert_eq!(password_problem(&api(401, r#"{"code":"unauthorized","message":""}"#)), None);
        assert_eq!(password_problem(&CloudError::Network("down".into())), None);
    }

    #[test]
    fn a_waiting_sign_in_retries_quickly_then_every_fifteen_seconds() {
        let mut p = PendingSignIn::new(SignInStep::IdToken("t".into()), ID_TOKEN_LIFE);
        assert!(p.retry_at <= Instant::now(), "the first try is immediate");
        let delays: Vec<u64> = (0..7)
            .map(|_| {
                p.schedule_retry();
                // Rounded: a few microseconds pass between the two clock reads.
                (p.retry_at - Instant::now()).as_secs_f64().round() as u64
            })
            .collect();
        assert_eq!(delays, [1, 2, 4, 8, 15, 15, 15]);
        assert!(p.until - Instant::now() > CODE_LIFE, "an ID token outlives a code");
    }
}
