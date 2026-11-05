//! Google sign-in in the web app: the page goes to Google and Google sends it
//! back with an ID token in the fragment (OpenID Connect implicit flow,
//! `response_type=id_token`), which Steps Cloud verifies like the apps' one.
//! Needs a Google "Web application" client whose redirect URI is the web
//! app's address (docs/WEB.md). The Mac and Android use `oauth.rs`.

use std::cell::RefCell;

use super::GoogleClient;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
/// `sessionStorage`: the state this tab sent, so only its own answer counts.
const STATE_KEY: &str = "steps:oauth-state";
/// `sessionStorage`: the nonce this tab sent, which Steps Cloud checks
/// against the token's (#542).
const NONCE_KEY: &str = "steps:oauth-nonce";

/// Only the apps exchange codes; the web gets an ID token straight back.
#[derive(Clone, Debug)]
pub struct Authorization;

#[derive(Debug, PartialEq)]
pub enum OAuthError {
    Cancelled,
    Failed(String),
}

thread_local! {
    static RETURNED: RefCell<Option<String>> = const { RefCell::new(None) };
    static NONCE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// 128 random bits as hex; `None` when the browser has no randomness, and
/// then sign-in doesn't start (a fixed state would check nothing, #542).
fn random() -> Option<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The nonce of the sign-in Google just answered, for Steps Cloud.
pub fn sign_in_nonce() -> Option<String> {
    NONCE.with(|n| n.borrow().clone())
}

/// The address Google sends the page back to: the app itself.
fn here() -> Option<String> {
    let location = web_sys::window()?.location();
    Some(format!("{}{}", location.origin().ok()?, location.pathname().ok()?))
}

pub fn auth_url(client_id: &str, redirect_uri: &str, state: &str, nonce: &str) -> String {
    let params = [
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "id_token"),
        ("scope", "openid email"),
        ("state", state),
        ("nonce", nonce),
        ("prompt", "select_account"),
    ];
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{AUTH_URL}?{}", query.join("&"))
}

/// Saves the database, then leaves for Google. Returns only when it couldn't.
pub async fn go_to_google(client: &GoogleClient) -> String {
    let Some(window) = web_sys::window() else { return "no window".into() };
    let Some(redirect) = here() else { return "no address".into() };
    let (Some(state), Some(nonce)) = (random(), random()) else { return "no randomness".into() };
    let Some(storage) = window.session_storage().ok().flatten() else { return "no sessionStorage".into() };
    if storage.set_item(STATE_KEY, &state).is_err() || storage.set_item(NONCE_KEY, &nonce).is_err() {
        return "sessionStorage refused".into();
    }
    crate::platform::web::save_now().await;
    let url = auth_url(&client.client_id, &redirect, &state, &nonce);
    match window.location().assign(&url) {
        Ok(()) => std::future::pending().await,
        Err(e) => format!("{e:?}"),
    }
}

/// At start: takes Google's answer out of the address bar (so a reload or a
/// bookmark never carries the token) and keeps the ID token for the engine.
pub fn catch_redirect() {
    let Some(window) = web_sys::window() else { return };
    let Ok(hash) = window.location().hash() else { return };
    let Some(fragment) = hash.strip_prefix('#') else { return };
    let storage = window.session_storage().ok().flatten();
    let take = |key: &str| {
        storage.as_ref().and_then(|s| {
            let v = s.get_item(key).ok().flatten();
            let _ = s.remove_item(key);
            v
        })
    };
    let expected = take(STATE_KEY);
    let nonce = take(NONCE_KEY);
    let Some(answer) = parse_fragment(fragment, expected.as_deref()) else { return };
    if let Some(path) = here()
        && let Ok(history) = window.history()
    {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
    }
    match answer {
        Ok(token) => {
            RETURNED.with(|r| *r.borrow_mut() = Some(token));
            NONCE.with(|n| *n.borrow_mut() = nonce);
        }
        Err(e) => crate::diag::report!("Google sign-in: {e:?}"),
    }
}

/// The ID token Google brought back, once.
pub fn take_returned_id_token() -> Option<String> {
    RETURNED.with(|r| r.borrow_mut().take())
}

/// `id_token=…&state=…` → the token; `None` when the fragment isn't an
/// answer to this tab's sign-in.
pub fn parse_fragment(fragment: &str, expected_state: Option<&str>) -> Option<Result<String, OAuthError>> {
    let mut token = None;
    let mut state = None;
    let mut error = None;
    for pair in fragment.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v);
        match k {
            "id_token" => token = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            _ => {}
        }
    }
    if expected_state.is_none() || state.as_deref() != expected_state {
        return None;
    }
    Some(match (token, error) {
        (Some(t), None) => Ok(t),
        (_, Some(e)) if e == "access_denied" => Err(OAuthError::Cancelled),
        (_, Some(e)) => Err(OAuthError::Failed(e)),
        (None, None) => Err(OAuthError::Failed("no id_token".into())),
    })
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(s: &str) -> String {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h << 4 | l);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
