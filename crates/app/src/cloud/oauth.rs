//! Google sign-in for installed apps: OAuth 2.0 with PKCE and a loopback
//! redirect (RFC 8252). The browser signs in, Google redirects to a one-shot
//! listener on 127.0.0.1, and the code is exchanged for an ID token, which
//! Steps Cloud verifies.
//!
//! The same flow on the Mac and on Android (Chrome comes back to the
//! loopback listener the app holds open).
// adapted from kabar-app main: crates/app/src/cloud/oauth.rs

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// How long the browser has to come back.
const WAIT: Duration = Duration::from_secs(5 * 60);

/// A Google "Desktop app" OAuth client. Google treats the secret of an
/// installed app as non-confidential, but it still comes from the build
/// environment (`.env`), not the repo.
#[derive(Clone, Debug)]
pub struct GoogleClient {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum OAuthError {
    Cancelled,
    TimedOut,
    Browser(String),
    /// The request never got an answer (no network, DNS, a dropped socket).
    /// Android cuts a backgrounded app off the network, so this is what a
    /// call made while Chrome is still in front looks like: worth retrying
    /// once Steps is back.
    Network(String),
    Failed(String),
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("sign-in was cancelled"),
            Self::TimedOut => f.write_str("sign-in timed out"),
            Self::Browser(e) => write!(f, "couldn't open the browser: {e}"),
            Self::Network(e) => write!(f, "network: {e}"),
            Self::Failed(e) => f.write_str(e),
        }
    }
}

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("system randomness");
        Self::from_verifier(URL_SAFE_NO_PAD.encode(bytes))
    }

    fn from_verifier(verifier: String) -> Self {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self { verifier, challenge }
    }
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("system randomness");
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn auth_url(client_id: &str, redirect_uri: &str, pkce: &Pkce, state: &str) -> String {
    let params = [
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", "openid email"),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("prompt", "select_account"),
    ];
    format!("{AUTH_URL}?{}", form(&params))
}

/// What the browser hands back: everything [`exchange`] needs to turn the
/// code into an ID token. Google's codes live about ten minutes.
#[derive(Clone)]
pub struct Authorization {
    code: String,
    verifier: String,
    redirect_uri: String,
}

impl std::fmt::Debug for Authorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Authorization").field("redirect_uri", &self.redirect_uri).finish_non_exhaustive()
    }
}

/// Opens the browser and waits for Google's redirect. No network beyond the
/// loopback listener, so it works while Steps is in the background; the
/// caller exchanges the code once the app may use the network again.
pub async fn google_authorization(client: &GoogleClient) -> Result<Authorization, OAuthError> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| OAuthError::Failed(e.to_string()))?;
    let port = listener.local_addr().map_err(|e| OAuthError::Failed(e.to_string()))?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let pkce = Pkce::new();
    let state = random_state();
    super::open_url(&auth_url(&client.client_id, &redirect_uri, &pkce, &state))
        .map_err(OAuthError::Browser)?;

    let code = tokio::time::timeout(WAIT, wait_for_code(&listener, &state))
        .await
        .map_err(|_| OAuthError::TimedOut)??;
    Ok(Authorization { code, verifier: pkce.verifier, redirect_uri })
}

async fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String, OAuthError> {
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|e| OAuthError::Failed(e.to_string()))?;
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let Some(result) = parse_redirect(&request, state) else {
            // favicon.ico and other strays.
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n").await;
            continue;
        };
        use crate::i18n::cloud::{BROWSER_CANCELLED, BROWSER_DONE};
        let page =
            page(if result.is_ok() { BROWSER_DONE } else { BROWSER_CANCELLED }, cfg!(target_os = "android"));
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
            page.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        return result;
    }
}

/// `GET /?code=…&state=… HTTP/1.1` → the code. `None` for requests that
/// aren't the redirect.
pub fn parse_redirect(request: &str, expected_state: &str) -> Option<Result<String, OAuthError>> {
    let target = request.lines().next()?.strip_prefix("GET ")?.split(' ').next()?;
    let query = target.strip_prefix("/?")?;
    let mut code = None;
    let mut state = None;
    let mut error = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v);
        match k {
            "code" => code = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            _ => {}
        }
    }
    // Not our sign-in (another page or process on this port): a stray, so
    // the listener answers 404 and keeps waiting for the real one (#262).
    if state.as_deref() != Some(expected_state) {
        return None;
    }
    Some(match (code, error) {
        (Some(code), None) => Ok(code),
        (_, Some(e)) if e == "access_denied" => Err(OAuthError::Cancelled),
        (_, Some(e)) => Err(OAuthError::Failed(e)),
        (None, None) => Err(OAuthError::Failed("no code".into())),
    })
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    id_token: String,
}

/// Trades the code for an ID token (blocking). A code is single-use: retry
/// only on [`OAuthError::Network`], when Google may never have seen it.
pub fn exchange(client: &GoogleClient, auth: &Authorization) -> Result<String, OAuthError> {
    let (code, verifier, redirect_uri) = (&auth.code, &auth.verifier, &auth.redirect_uri);
    let agent = crate::update::agent(TOKEN_URL, Duration::from_secs(20));
    let mut form = vec![
        ("code", code.as_str()),
        ("client_id", client.client_id.as_str()),
        ("code_verifier", verifier.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri.as_str()),
    ];
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret));
    }
    let mut res = agent.post(TOKEN_URL).send_form(form).map_err(|e| OAuthError::Network(e.to_string()))?;
    if !res.status().is_success() {
        let body = res.body_mut().read_to_string().unwrap_or_default();
        return Err(OAuthError::Failed(format!("token exchange {}: {body}", res.status())));
    }
    let token: TokenResponse = res.body_mut().read_json().map_err(|e| OAuthError::Failed(e.to_string()))?;
    Ok(token.id_token)
}

fn form(params: &[(&str, &str)]) -> String {
    params.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect::<Vec<_>>().join("&")
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

/// Opens Steps from the browser on Android (`SignInReturnActivity` in
/// android/src/main/AndroidManifest.xml brings the running app to the front).
pub const RETURN_URL: &str = "app.steps://signed-in";

/// The page the browser shows once Google sends it back here. On Android it
/// also tries to bring Steps back by itself and offers a big button for when
/// Chrome wants a tap first: sign-in finishes only once Steps is in front,
/// because Android keeps a backgrounded app off the network.
fn page(body: &str, back_button: bool) -> String {
    let back = if back_button {
        format!(
            r#"<p><a href="{RETURN_URL}" style="display:inline-block;padding:1rem 2rem;border-radius:999px;background:#1f6f5c;color:#fff;text-decoration:none;font-size:1.25rem">{}</a></p>
<script>setTimeout(function(){{location.href="{RETURN_URL}"}},300)</script>"#,
            crate::i18n::cloud::BROWSER_BACK
        )
    } else {
        String::new()
    };
    format!(
        r#"<!doctype html><html lang="ar" dir="rtl"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title>
<body style="font-family:system-ui;text-align:center;padding:3rem;font-size:1.2rem">{body}{back}</body></html>"#,
        crate::i18n::app::NAME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc_7636() {
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into());
        assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        assert_eq!(Pkce::new().verifier.len(), 43);
    }

    #[test]
    fn auth_url_is_encoded() {
        let p = Pkce::from_verifier("v".into());
        let url = auth_url("123-desk.apps.googleusercontent.com", "http://127.0.0.1:5000", &p, "s1");
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=123-desk.apps"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5000"));
        assert!(url.contains("scope=openid%20email"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn redirect_parsing() {
        let ok = "GET /?state=s1&code=4%2F0Ab_x&scope=email HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert_eq!(parse_redirect(ok, "s1"), Some(Ok("4/0Ab_x".into())));
        let denied = "GET /?error=access_denied&state=s1 HTTP/1.1\r\n";
        assert_eq!(parse_redirect(denied, "s1"), Some(Err(OAuthError::Cancelled)));
        let forged = "GET /?state=evil&code=x HTTP/1.1\r\n";
        assert_eq!(parse_redirect(forged, "s1"), None);
        assert_eq!(parse_redirect("GET /favicon.ico HTTP/1.1\r\n", "s1"), None);
    }

    #[test]
    fn android_page_links_back_to_steps() {
        let p = page("<p>done</p>", true);
        assert!(p.contains(r#"href="app.steps://signed-in""#));
        assert!(p.contains("viewport"));
        assert!(!page("<p>done</p>", false).contains("app.steps://"));
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(decode("a%20b+c%2F"), "a b c/");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
    }
}
