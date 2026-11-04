//! Steps Cloud's HTTP API (docs/CLOUD.md §2). Async on every target: on the
//! Mac and Android a blocking `ureq` call runs on tokio's blocking pool, in
//! the browser it is a `fetch` (docs/WEB.md).
// adapted from kabar-app main: crates/app/src/cloud/api.rs (no AI, TTS or usage)

use serde::Serialize;
use serde::de::DeserializeOwned;
use steps_protocol::cloud::{
    ApiError, DeviceList, ErrorCode, GoogleSignIn, PasswordLogin, PasswordSignIn, RefreshRequest, Session,
    SetPassword,
};

#[derive(Clone, Debug, PartialEq)]
pub enum CloudError {
    Network(String),
    /// `retry_after_ms` comes with [`ErrorCode::TooManyTries`].
    Api {
        status: u16,
        code: ErrorCode,
        message: String,
        retry_after_ms: Option<u64>,
    },
    Decode(String),
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network: {e}"),
            Self::Api { status, code, message, .. } => write!(f, "{status} {code:?}: {message}"),
            Self::Decode(e) => write!(f, "unexpected response: {e}"),
        }
    }
}

impl CloudError {
    /// The session is gone for good: sign in again.
    pub fn is_signed_out(&self) -> bool {
        matches!(self, Self::Api { code: ErrorCode::Unauthorized | ErrorCode::DeviceRevoked, .. })
    }
}

#[derive(Clone)]
pub struct Api {
    #[cfg(not(target_arch = "wasm32"))]
    agent: ureq::Agent,
    base: String,
}

impl std::fmt::Debug for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Api").field("base", &self.base).finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verb {
    Get,
    Post,
    Delete,
}

impl Api {
    pub fn new(base_url: &str) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(std::time::Duration::from_secs(20)))
                .build()
                .into(),
            base: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// `wss://…/v1/sync` (or `ws://` for a local server).
    pub fn sync_url(&self) -> String {
        let rest = self.base.strip_prefix("https://").map(|r| format!("wss://{r}"));
        rest.unwrap_or_else(|| self.base.replacen("http://", "ws://", 1)) + "/v1/sync"
    }

    pub async fn sign_in_google(&self, body: &GoogleSignIn) -> Result<Session, CloudError> {
        read(self.call(Verb::Post, "/v1/auth/google", None, Some(json(body)?)).await)
    }

    pub async fn sign_in_password(&self, body: &PasswordSignIn) -> Result<Session, CloudError> {
        read(self.call(Verb::Post, "/v1/auth/password", None, Some(json(body)?)).await)
    }

    pub async fn login(&self, access: &str) -> Result<PasswordLogin, CloudError> {
        read(self.call(Verb::Get, "/v1/login", Some(access), None).await)
    }

    pub async fn set_login(&self, access: &str, body: &SetPassword) -> Result<PasswordLogin, CloudError> {
        read(self.call(Verb::Post, "/v1/login", Some(access), Some(json(body)?)).await)
    }

    pub async fn remove_login(&self, access: &str) -> Result<PasswordLogin, CloudError> {
        read(self.call(Verb::Delete, "/v1/login", Some(access), None).await)
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Session, CloudError> {
        let body = RefreshRequest { refresh_token: refresh_token.into() };
        read(self.call(Verb::Post, "/v1/auth/refresh", None, Some(json(&body)?)).await)
    }

    pub async fn sign_out(&self, access: &str) -> Result<(), CloudError> {
        self.call(Verb::Post, "/v1/auth/signout", Some(access), None).await.map(drop)
    }

    pub async fn devices(&self, access: &str) -> Result<DeviceList, CloudError> {
        read(self.call(Verb::Get, "/v1/devices", Some(access), None).await)
    }

    pub async fn revoke(&self, access: &str, device: &str) -> Result<(), CloudError> {
        self.call(Verb::Delete, &format!("/v1/devices/{device}"), Some(access), None).await.map(drop)
    }

    /// One request; a 2xx answer's body, or the error it means.
    async fn call(
        &self,
        verb: Verb,
        path: &str,
        access: Option<&str>,
        body: Option<String>,
    ) -> Result<String, CloudError> {
        let url = format!("{}{path}", self.base);
        let bearer = access.map(|a| format!("Bearer {a}"));
        let (status, text) = self.send(verb, url, bearer, body).await.map_err(CloudError::Network)?;
        if (200..300).contains(&status) { Ok(text) } else { Err(api_error(status, &text)) }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn send(
        &self,
        verb: Verb,
        url: String,
        bearer: Option<String>,
        body: Option<String>,
    ) -> Result<(u16, String), String> {
        let agent = self.agent.clone();
        crate::platform::time::blocking(move || {
            let res = match (verb, body) {
                (Verb::Get, _) => with_auth(agent.get(&url), bearer).call(),
                (Verb::Delete, _) => with_auth(agent.delete(&url), bearer).call(),
                (Verb::Post, Some(body)) => with_auth(agent.post(&url), bearer)
                    .header("content-type", "application/json")
                    .send(body.as_bytes()),
                (Verb::Post, None) => with_auth(agent.post(&url), bearer).send_empty(),
            };
            let mut res = res.map_err(|e| e.to_string())?;
            let status = res.status().as_u16();
            let text = res.body_mut().read_to_string().unwrap_or_default();
            Ok((status, text))
        })
        .await?
    }

    #[cfg(target_arch = "wasm32")]
    async fn send(
        &self,
        verb: Verb,
        url: String,
        bearer: Option<String>,
        body: Option<String>,
    ) -> Result<(u16, String), String> {
        let method = match verb {
            Verb::Get => "GET",
            Verb::Post => "POST",
            Verb::Delete => "DELETE",
        };
        crate::platform::web::fetch(method, &url, bearer.as_deref(), body.as_deref()).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn with_auth<B>(req: ureq::RequestBuilder<B>, bearer: Option<String>) -> ureq::RequestBuilder<B> {
    match bearer {
        Some(b) => req.header("authorization", b),
        None => req,
    }
}

fn json<T: Serialize>(body: &T) -> Result<String, CloudError> {
    serde_json::to_string(body).map_err(|e| CloudError::Decode(e.to_string()))
}

fn read<T: DeserializeOwned>(text: Result<String, CloudError>) -> Result<T, CloudError> {
    serde_json::from_str(&text?).map_err(|e| CloudError::Decode(e.to_string()))
}

/// Error bodies are [`ApiError`]; anything else (a proxy page, a 502 from
/// Cloudflare) still gets a code from the status.
pub fn api_error(status: u16, body: &str) -> CloudError {
    match serde_json::from_str::<ApiError>(body) {
        Ok(e) => {
            CloudError::Api { status, code: e.code, message: e.message, retry_after_ms: e.retry_after_ms }
        }
        Err(_) => {
            let code = match status {
                401 => ErrorCode::Unauthorized,
                403 => ErrorCode::NotAllowed,
                404 => ErrorCode::NotFound,
                _ => ErrorCode::Internal,
            };
            CloudError::Api { status, code, message: body.chars().take(200).collect(), retry_after_ms: None }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_url_follows_the_scheme() {
        assert_eq!(
            Api::new("https://steps-cloud.x.workers.dev/").sync_url(),
            "wss://steps-cloud.x.workers.dev/v1/sync"
        );
        assert_eq!(Api::new("http://127.0.0.1:8787").sync_url(), "ws://127.0.0.1:8787/v1/sync");
    }

    #[test]
    fn errors_keep_the_server_code() {
        let e = api_error(401, r#"{"code":"device_revoked","message":"signed out"}"#);
        assert!(e.is_signed_out());
        assert!(matches!(e, CloudError::Api { code: ErrorCode::DeviceRevoked, .. }));
        let e = api_error(403, "<html>blocked</html>");
        assert!(matches!(e, CloudError::Api { code: ErrorCode::NotAllowed, .. }));
        assert!(!e.is_signed_out());
    }

    #[test]
    fn a_lockout_keeps_its_wait_and_a_wrong_password_is_not_a_sign_out() {
        let e = api_error(429, r#"{"code":"too_many_tries","message":"wait","retry_after_ms":120000}"#);
        assert!(matches!(
            e,
            CloudError::Api { code: ErrorCode::TooManyTries, retry_after_ms: Some(120_000), .. }
        ));
        let e = api_error(401, r#"{"code":"wrong_password","message":"no"}"#);
        assert!(!e.is_signed_out(), "a typo must not sign the device out");
    }
}
