//! Binds Steps Cloud to the Cloudflare runtime. Kept thin: parse, check the
//! token, call the plain-Rust modules, talk to bindings and upstreams.

mod account;
mod do_sql;
mod login;

use std::cell::{Cell, RefCell};

// adapted from batna-app main: crates/server/src/wasm/mod.rs (sign-in,
// devices and sync only)
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use steps_protocol::PROTOCOL_VERSION;
use steps_protocol::cloud::{
    ApiError, ErrorCode, GoogleSignIn, PasswordLogin, PasswordSignIn, RefreshRequest, RemovePassword,
    Session, SetPassword, normalize_username, valid_username,
};
use wasm_bindgen::JsValue;
use worker::{
    Context, Date, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result, Stub, console_error,
    event,
};

use crate::config::Config;
use crate::google::{self, Jwks};
use crate::internal::{
    AccountLoginState, DEVICE_HEADER, Expiry, LoginCheck, LoginClear, LoginName, LoginSet, Refresh, Register,
    Revoke,
};
use crate::login::{self as pw, Checked};
use crate::tokens::{self, AccessClaims, TokenKey};

pub(crate) fn now_ms() -> u64 {
    Date::now().as_millis()
}

/// `n` (≤ 32) random bytes as hex. Never falls back to a fixed value.
pub(crate) fn random_hex(n: usize) -> std::result::Result<String, getrandom::Error> {
    let bytes = tokens::random_bytes()?;
    Ok(bytes[..n.min(32)].iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn api_error(code: ErrorCode, message: impl Into<String>) -> Result<Response> {
    error_response(ApiError { code, message: message.into(), retry_after_ms: None })
}

fn error_response(e: ApiError) -> Result<Response> {
    let status = status_for(e.code);
    Ok(Response::from_json(&e)?.with_status(status))
}

fn status_for(code: ErrorCode) -> u16 {
    match code {
        ErrorCode::InvalidRequest => 400,
        ErrorCode::Unauthorized | ErrorCode::DeviceRevoked => 401,
        ErrorCode::NotAllowed => 403,
        ErrorCode::NotFound => 404,
        ErrorCode::WrongPassword => 401,
        ErrorCode::UsernameTaken => 409,
        ErrorCode::TooManyTries => 429,
        ErrorCode::Internal | ErrorCode::Other => 500,
    }
}

fn config(env: &Env) -> Config {
    let lookup = |name: &str| {
        env.secret(name).ok().map(|s| s.to_string()).or_else(|| env.var(name).ok().map(|v| v.to_string()))
    };
    match Config::from_lookup(lookup) {
        Ok(c) => c,
        Err(e) => {
            console_error!("config: {e}");
            // Fall back to defaults: auth reports what's missing per request.
            Config::from_lookup(|_| None).unwrap_or_else(|_| unreachable!("defaults parse"))
        }
    }
}

async fn body<T: DeserializeOwned>(req: &mut Request) -> std::result::Result<T, Result<Response>> {
    req.json().await.map_err(|e| api_error(ErrorCode::InvalidRequest, format!("bad JSON body: {e}")))
}

fn bearer(req: &Request) -> Option<String> {
    if let Some(h) = req.headers().get("authorization").ok().flatten() {
        return h.strip_prefix("Bearer ").map(|t| t.trim().to_owned());
    }
    // Browsers can't put headers on a WebSocket: the web app's sync socket
    // brings its access token in the query, and only there (docs/WEB.md).
    if req.path() != "/v1/sync" {
        return None;
    }
    let url = req.url().ok()?;
    url.query_pairs().find(|(k, _)| k == "access_token").map(|(_, v)| v.into_owned())
}

/// The web app's origin, when this request comes from one of its pages.
fn web_origin(req: &Request, cfg: &Config) -> Option<String> {
    let origin = req.headers().get("origin").ok().flatten()?;
    cfg.web_origin_allowed(&origin).then_some(origin)
}

/// CORS for the web app: its own origin only, never `*`. A WebSocket answer
/// (101) passes through untouched.
fn with_cors(res: Response, origin: &str) -> Result<Response> {
    if res.status_code() == 101 {
        return Ok(res);
    }
    // A Durable Object's response has read-only headers: copy them.
    let headers = Headers::new();
    for (k, v) in res.headers().entries() {
        headers.set(&k, &v)?;
    }
    headers.set("access-control-allow-origin", origin)?;
    headers.set("access-control-allow-headers", "authorization, content-type")?;
    headers.set("access-control-allow-methods", "GET, POST, DELETE, OPTIONS")?;
    headers.set("access-control-max-age", "86400")?;
    headers.set("vary", "origin")?;
    Ok(res.with_headers(headers))
}

fn authenticate(req: &Request, key: &TokenKey) -> std::result::Result<AccessClaims, Result<Response>> {
    let token = bearer(req).ok_or_else(|| api_error(ErrorCode::Unauthorized, "missing bearer token"))?;
    tokens::verify_access(&key.public(), &token, now_ms())
        .map_err(|e| api_error(ErrorCode::Unauthorized, format!("access token: {e}")))
}

fn account_stub(env: &Env, account_id: &str) -> Result<Stub> {
    env.durable_object("ACCOUNT")?.id_from_name(account_id)?.get_stub()
}

/// Calls the account's Durable Object.
async fn call_do(
    env: &Env,
    account_id: &str,
    method: Method,
    path: &str,
    device: Option<&str>,
    payload: Option<&impl Serialize>,
) -> Result<Response> {
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    if let Some(d) = device {
        headers.set(DEVICE_HEADER, d)?;
    }
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(p) = payload {
        init.with_body(Some(JsValue::from_str(&serde_json::to_string(p)?)));
    }
    let req = Request::new_with_init(&format!("https://account{path}"), &init)?;
    account_stub(env, account_id)?.fetch_with_request(req).await
}

thread_local! {
    /// Google's signing keys, cached for the isolate's life (or max-age).
    static JWKS: RefCell<Option<(Jwks, u64)>> = const { RefCell::new(None) };
    /// When an unknown `kid` last forced a refetch (`google::may_refetch`).
    static JWKS_FORCED: Cell<Option<u64>> = const { Cell::new(None) };
}

async fn google_jwks(url: &str, force: bool) -> Result<Jwks> {
    let now = now_ms();
    // A forced refetch (unknown kid) only every few minutes; in between the
    // cached keys answer, and an unknown kid fails verification.
    let force = force && google::may_refetch(JWKS_FORCED.get(), now);
    if force {
        JWKS_FORCED.set(Some(now));
    }
    if !force
        && let Some(jwks) =
            JWKS.with_borrow(|c| c.as_ref().filter(|(_, exp)| *exp > now).map(|(j, _)| j.clone()))
    {
        return Ok(jwks);
    }
    let mut resp = Fetch::Url(url.parse()?).send().await?;
    let max_age = google::jwks_max_age_ms(resp.headers().get("cache-control")?.as_deref());
    let jwks: Jwks = resp.json().await?;
    JWKS.with_borrow_mut(|c| *c = Some((jwks.clone(), now + max_age)));
    Ok(jwks)
}

/// Where Google's keys come from. `INSECURE_DEV_JWKS_URL` lets the local
/// end-to-end test sign its own ID tokens; it is ignored unless the request
/// itself is to localhost, so it can't weaken a deployed Worker.
fn jwks_url(env: &Env, req: &Request) -> String {
    let local =
        req.url().ok().and_then(|u| u.host_str().map(|h| h == "localhost" || h == "127.0.0.1")) == Some(true);
    match env.var("INSECURE_DEV_JWKS_URL").map(|v| v.to_string()) {
        Ok(url) if local && !url.is_empty() => url,
        _ => google::JWKS_URL.to_owned(),
    }
}

async fn session_response(
    key: &TokenKey,
    account_id: &str,
    device: String,
    refresh: tokens::NewRefresh,
    expiry: Expiry,
) -> Result<Response> {
    let (access_token, access_expires_at_ms) = tokens::issue_access(key, account_id, &device, now_ms());
    Response::from_json(&Session {
        account_id: account_id.to_owned(),
        device_id: device,
        email: Some(expiry.email),
        access_token,
        access_expires_at_ms,
        refresh_token: refresh.token,
        refresh_expires_at_ms: expiry.refresh_expires_ms,
    })
}

async fn sign_in_google(mut req: Request, env: &Env, cfg: &Config) -> Result<Response> {
    let key = match cfg.auth_ready() {
        Ok(k) => k.clone(),
        Err(e) => return api_error(ErrorCode::Internal, format!("server not set up: {e}")),
    };
    let sign_in: GoogleSignIn = match body(&mut req).await {
        Ok(b) => b,
        Err(r) => return r,
    };
    if !tokens::valid_device_id(&sign_in.device.device_id) {
        return api_error(ErrorCode::InvalidRequest, "device_id must be [A-Za-z0-9_-]{1,64}");
    }
    let now = now_ms();
    let url = jwks_url(env, &req);
    let mut jwks = google_jwks(&url, false).await?;
    let kid = google::token_kid(&sign_in.id_token).unwrap_or_default();
    if !jwks.keys.iter().any(|k| k.kid == kid) {
        jwks = google_jwks(&url, true).await?;
    }
    let identity = match google::verify_id_token(
        &sign_in.id_token,
        &jwks,
        &cfg.google_client_ids,
        sign_in.nonce.as_deref(),
        now,
    ) {
        Ok(id) => id,
        Err(e) => return api_error(ErrorCode::Unauthorized, format!("Google ID token: {e}")),
    };
    if !cfg.email_allowed(&identity.email) {
        return api_error(ErrorCode::NotAllowed, "this Google account is not on Steps's allowlist");
    }
    let account_id = tokens::account_id_for_google_sub(&identity.sub);
    let device = sign_in.device.device_id.clone();
    let refresh = tokens::new_refresh(&account_id, &device, tokens::random_bytes().map_err(js_err)?);
    let register = Register {
        info: sign_in.device,
        email: identity.email,
        refresh_hash: refresh.hash.clone(),
        by_password: false,
    };
    let mut resp = call_do(env, &account_id, Method::Post, "/register", None, Some(&register)).await?;
    if resp.status_code() != 200 {
        return Ok(resp);
    }
    let expiry: Expiry = resp.json().await?;
    session_response(&key, &account_id, device, refresh, expiry).await
}

async fn refresh_session(mut req: Request, env: &Env, cfg: &Config) -> Result<Response> {
    let key = match cfg.session_ready() {
        Ok(k) => k.clone(),
        Err(e) => return api_error(ErrorCode::Internal, format!("server not set up: {e}")),
    };
    let r: RefreshRequest = match body(&mut req).await {
        Ok(b) => b,
        Err(r) => return r,
    };
    let Some(parsed) = tokens::parse_refresh(&r.refresh_token) else {
        return api_error(ErrorCode::Unauthorized, "malformed refresh token");
    };
    let fresh =
        tokens::new_refresh(&parsed.account_id, &parsed.device, tokens::random_bytes().map_err(js_err)?);
    let call =
        Refresh { device: parsed.device.clone(), presented_hash: parsed.hash, new_hash: fresh.hash.clone() };
    let mut resp = call_do(env, &parsed.account_id, Method::Post, "/refresh", None, Some(&call)).await?;
    if resp.status_code() != 200 {
        return Ok(resp);
    }
    let expiry: Expiry = resp.json().await?;
    if !cfg.email_allowed(&expiry.email) {
        let revoke = Revoke { device: parsed.device };
        call_do(env, &parsed.account_id, Method::Post, "/revoke", None, Some(&revoke)).await?;
        return api_error(ErrorCode::NotAllowed, "this Google account is no longer on the allowlist");
    }
    session_response(&key, &parsed.account_id, parsed.device, fresh, expiry).await
}

/// A username's own Durable Object (`crate::login`).
async fn call_login(env: &Env, username: &str, path: &str, payload: &impl Serialize) -> Result<Response> {
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&serde_json::to_string(payload)?)));
    let req = Request::new_with_init(&format!("https://login{path}"), &init)?;
    env.durable_object("LOGIN")?
        .id_from_name(&format!("u:{username}"))?
        .get_stub()?
        .fetch_with_request(req)
        .await
}

/// `POST /v1/auth/password`: the same account as Google, on a device without it.
async fn sign_in_password(mut req: Request, env: &Env, cfg: &Config) -> Result<Response> {
    let key = match cfg.session_ready() {
        Ok(k) => k.clone(),
        Err(e) => return api_error(ErrorCode::Internal, format!("server not set up: {e}")),
    };
    let sign_in: PasswordSignIn = match body(&mut req).await {
        Ok(b) => b,
        Err(r) => return r,
    };
    if !tokens::valid_device_id(&sign_in.device.device_id) {
        return api_error(ErrorCode::InvalidRequest, "device_id must be [A-Za-z0-9_-]{1,64}");
    }
    let username = normalize_username(&sign_in.username);
    if !valid_username(&username) || pw::parse_key(&sign_in.password_key).is_none() {
        return api_error(ErrorCode::WrongPassword, "username or password is wrong");
    }
    let check = LoginCheck { key: sign_in.password_key };
    let mut resp = call_login(env, &username, "/check", &check).await?;
    if resp.status_code() != 200 {
        return Ok(resp);
    }
    let (account_id, email) = match resp.json::<Checked>().await? {
        Checked::Ok { account_id, email } => (account_id, email),
        Checked::Wrong => return api_error(ErrorCode::WrongPassword, "username or password is wrong"),
        Checked::Locked { retry_after_ms } => {
            return error_response(ApiError {
                code: ErrorCode::TooManyTries,
                message: "too many wrong passwords for this username".into(),
                retry_after_ms: Some(retry_after_ms),
            });
        }
    };
    if !cfg.email_allowed(&email) {
        return api_error(ErrorCode::NotAllowed, "this account is no longer on Steps's allowlist");
    }
    let device = sign_in.device.device_id.clone();
    let refresh = tokens::new_refresh(&account_id, &device, tokens::random_bytes().map_err(js_err)?);
    let register =
        Register { info: sign_in.device, email, refresh_hash: refresh.hash.clone(), by_password: true };
    let mut resp = call_do(env, &account_id, Method::Post, "/register", None, Some(&register)).await?;
    if resp.status_code() != 200 {
        return Ok(resp);
    }
    let expiry: Expiry = resp.json().await?;
    session_response(&key, &account_id, device, refresh, expiry).await
}

/// The account's username now, and the caller's email; or the error answer.
async fn login_state(
    env: &Env,
    claims: &AccessClaims,
) -> Result<std::result::Result<AccountLoginState, Response>> {
    let mut resp = call_do(env, &claims.sub, Method::Get, "/login", Some(&claims.dev), None::<&()>).await?;
    if resp.status_code() != 200 {
        return Ok(Err(resp));
    }
    Ok(Ok(resp.json().await?))
}

/// `GET`, `POST` and `DELETE /v1/login`: the username and password of the
/// signed-in account.
async fn manage_login(
    mut req: Request,
    env: &Env,
    claims: &AccessClaims,
    method: Method,
) -> Result<Response> {
    let state = match login_state(env, claims).await? {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    let needs_current = pw::needs_current(state.by_password, state.username.as_deref());
    let username = match method {
        Method::Get => state.username,
        Method::Post => {
            let set: SetPassword = match body(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            if needs_current
                && let Some(refused) =
                    check_current(env, state.username.as_deref(), set.current_key.as_deref(), claims).await?
            {
                return Ok(refused);
            }
            let username = normalize_username(&set.username);
            if !valid_username(&username) {
                return api_error(ErrorCode::InvalidRequest, "username must be 3-32 of a-z 0-9 . _ -");
            }
            let Some(key) = pw::parse_key(&set.password_key) else {
                return api_error(ErrorCode::InvalidRequest, "password_key must be 32 bytes, base64url");
            };
            let salt = random_hex(16).map_err(js_err)?;
            let put = LoginSet {
                account_id: claims.sub.clone(),
                email: state.email,
                verifier: pw::verifier(&salt, &key),
                salt,
            };
            let resp = call_login(env, &username, "/set", &put).await?;
            if resp.status_code() != 200 {
                return Ok(resp);
            }
            if let Some(old) = state.username.filter(|old| *old != username) {
                call_login(env, &old, "/clear", &LoginClear { account_id: claims.sub.clone() }).await?;
            }
            Some(username)
        }
        _ => {
            if needs_current {
                // DELETE may carry a body: the current password.
                let text = req.text().await.unwrap_or_default();
                let remove: RemovePassword = serde_json::from_str(&text).unwrap_or_default();
                if let Some(refused) =
                    check_current(env, state.username.as_deref(), remove.current_key.as_deref(), claims)
                        .await?
                {
                    return Ok(refused);
                }
            }
            if let Some(old) = &state.username {
                call_login(env, old, "/clear", &LoginClear { account_id: claims.sub.clone() }).await?;
            }
            None
        }
    };
    if method != Method::Get {
        let name = LoginName { username: username.clone() };
        let resp = call_do(env, &claims.sub, Method::Post, "/login", Some(&claims.dev), Some(&name)).await?;
        if resp.status_code() != 200 {
            return Ok(resp);
        }
    }
    Response::from_json(&PasswordLogin { username, needs_current })
}

/// Checks the current password against the account's username, through its
/// Login object (so wrong ones count towards its lock). `None` when it is
/// right; otherwise the answer to send.
async fn check_current(
    env: &Env,
    username: Option<&str>,
    current_key: Option<&str>,
    claims: &AccessClaims,
) -> Result<Option<Response>> {
    let (Some(username), Some(key)) = (username, current_key.filter(|k| pw::parse_key(k).is_some())) else {
        return api_error(ErrorCode::WrongPassword, "the current password is needed").map(Some);
    };
    let mut resp = call_login(env, username, "/check", &LoginCheck { key: key.to_owned() }).await?;
    if resp.status_code() != 200 {
        return Ok(Some(resp));
    }
    match resp.json::<Checked>().await? {
        Checked::Ok { account_id, .. } if account_id == claims.sub => Ok(None),
        Checked::Ok { .. } | Checked::Wrong => {
            api_error(ErrorCode::WrongPassword, "the current password is wrong").map(Some)
        }
        Checked::Locked { retry_after_ms } => error_response(ApiError {
            code: ErrorCode::TooManyTries,
            message: "too many wrong passwords for this username".into(),
            retry_after_ms: Some(retry_after_ms),
        })
        .map(Some),
    }
}

fn js_err(e: impl std::fmt::Display) -> worker::Error {
    worker::Error::RustError(e.to_string())
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let cfg = config(&env);
    let path = req.path();
    let method = req.method();
    let origin = web_origin(&req, &cfg);
    if method == Method::Options {
        // A preflight from the web app; anyone else gets no CORS answer.
        return match origin {
            Some(o) => with_cors(Response::empty()?.with_status(204), &o),
            None => Ok(Response::empty()?.with_status(204)),
        };
    }
    let result = match route(req, &env, &cfg, method, &path).await {
        Ok(r) => Ok(r),
        Err(e) => {
            console_error!("{path}: {e}");
            api_error(ErrorCode::Internal, "something went wrong on Steps Cloud")
        }
    };
    match origin {
        Some(o) => with_cors(result?, &o),
        None => result,
    }
}

async fn route(req: Request, env: &Env, cfg: &Config, method: Method, path: &str) -> Result<Response> {
    match (method.clone(), path) {
        (Method::Get, "/" | "/v1/health") => {
            Response::from_json(&json!({ "name": "steps-cloud", "protocol": PROTOCOL_VERSION }))
        }
        (Method::Post, "/v1/auth/google") => sign_in_google(req, env, cfg).await,
        (Method::Post, "/v1/auth/refresh") => refresh_session(req, env, cfg).await,
        (Method::Post, "/v1/auth/password") => sign_in_password(req, env, cfg).await,
        _ => {
            let Some(key) = cfg.token_key.clone() else {
                return api_error(ErrorCode::Internal, "server not set up: TOKEN_SIGNING_KEY is not set");
            };
            let claims = match authenticate(&req, &key) {
                Ok(c) => c,
                Err(r) => return r,
            };
            match (method, path) {
                (Method::Get, "/v1/sync") => {
                    if req.headers().get("upgrade")?.as_deref() != Some("websocket") {
                        return api_error(ErrorCode::InvalidRequest, "expected a WebSocket upgrade");
                    }
                    let headers = Headers::new();
                    headers.set("upgrade", "websocket")?;
                    headers.set(DEVICE_HEADER, &claims.dev)?;
                    let mut init = RequestInit::new();
                    init.with_method(Method::Get).with_headers(headers);
                    let forwarded = Request::new_with_init("https://account/ws", &init)?;
                    account_stub(env, &claims.sub)?.fetch_with_request(forwarded).await
                }
                (Method::Get, "/v1/devices") => {
                    call_do(env, &claims.sub, Method::Get, "/devices", Some(&claims.dev), None::<&()>).await
                }
                (Method::Delete, p) if p.starts_with("/v1/devices/") => {
                    let id = &p["/v1/devices/".len()..];
                    if !tokens::valid_device_id(id) {
                        return api_error(ErrorCode::InvalidRequest, "bad device id");
                    }
                    let revoke = Revoke { device: id.to_owned() };
                    // The hub checks the caller is still signed in (#259).
                    call_do(env, &claims.sub, Method::Post, "/revoke", Some(&claims.dev), Some(&revoke)).await
                }
                (m @ (Method::Get | Method::Post | Method::Delete), "/v1/login") => {
                    manage_login(req, env, &claims, m).await
                }
                (Method::Post, "/v1/auth/signout") => {
                    let revoke = Revoke { device: claims.dev.clone() };
                    call_do(env, &claims.sub, Method::Post, "/revoke", Some(&claims.dev), Some(&revoke)).await
                }
                _ => api_error(ErrorCode::NotFound, "not found"),
            }
        }
    }
}
