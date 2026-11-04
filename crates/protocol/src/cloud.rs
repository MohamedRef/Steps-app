//! Steps Cloud HTTP API types (everything under `/v1` except the WebSocket
//! messages, which live in [`crate::sync`]). Authenticated routes take
//! `Authorization: Bearer <access_token>`. See docs/CLOUD.md.

use serde::{Deserialize, Serialize};

/// Machine-readable error. The app turns `code` into a friendly Arabic
/// sentence; `message` is for logs only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    /// Missing, expired or bad token. Refresh, or sign in again.
    Unauthorized,
    /// Signed in with a Google account that is not on the allowlist.
    NotAllowed,
    /// This device was signed out remotely. Sign in again.
    DeviceRevoked,
    NotFound,
    Internal,
    /// Username and password don't match (`POST /v1/auth/password`). The
    /// same answer for a username nobody has.
    WrongPassword,
    /// Too many wrong passwords for this username: wait `retry_after_ms`.
    TooManyTries,
    /// Another account already uses this username.
    UsernameTaken,
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    /// With [`ErrorCode::TooManyTries`]: how long until the next try counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Android,
    Macos,
    /// The web app in a browser (docs/WEB.md). Older apps read it as `Other`.
    Web,
    #[serde(other)]
    Other,
}

/// What a device says about itself when it signs in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Stable per installation: `[A-Za-z0-9_-]{1,64}`.
    pub device_id: String,
    /// Shown on the Devices list, e.g. "Galaxy S24 Ultra".
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
    pub platform: Platform,
}

/// `POST /v1/auth/google`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoogleSignIn {
    /// Google ID token from the browser sign-in (OAuth + PKCE).
    pub id_token: String,
    pub device: DeviceInfo,
}

/// `POST /v1/auth/password`: sign in on a device without Google (a
/// borrowed laptop, a library computer) with the username and password set
/// from a signed-in device. It joins the same account, so everything syncs.
///
/// The password never leaves the device: the app stretches it with
/// Argon2id ([`password_salt`], [`PASSWORD_PARAMS`]) and sends only the
/// 32-byte result, base64url. The server keeps a salted SHA-256 of that,
/// so a copy of its storage is no shortcut to the password, and every guess
/// still costs the Argon2id work (docs/CLOUD.md §4.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PasswordSignIn {
    pub username: String,
    pub password_key: String,
    pub device: DeviceInfo,
}

/// `POST /v1/login` (signed in): set the username and password, or change
/// either. The username must be free; the account's old one is released.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetPassword {
    pub username: String,
    pub password_key: String,
}

/// `GET /v1/login`, and the answer to `POST`/`DELETE /v1/login`: the
/// username this account signs in with, if it has one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PasswordLogin {
    #[serde(default)]
    pub username: Option<String>,
}

/// Argon2id cost for [`PasswordSignIn::password_key`]: memory in KiB,
/// passes, lanes (OWASP's 2025 minimum is 19 MiB, 2 passes). Changing any
/// of these changes every key, so it is part of the protocol.
pub const PASSWORD_PARAMS: (u32, u32, u32) = (19_456, 2, 1);
/// The shortest password the app accepts.
pub const MIN_PASSWORD_CHARS: usize = 10;

/// The username as the server keys it: trimmed and lower-case.
pub fn normalize_username(s: &str) -> String {
    s.trim().to_lowercase()
}

/// 3–32 of `a-z 0-9 . _ -`, starting with a letter or digit. Checked on
/// the normalized form.
pub fn valid_username(s: &str) -> bool {
    (3..=32).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        && s.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
}

/// The Argon2id salt for a username: fixed per username, so any device
/// derives the same key, and unique to Steps, so no other site's table fits.
pub fn password_salt(username: &str) -> Vec<u8> {
    format!("steps-login-v1:{}", normalize_username(username)).into_bytes()
}

/// `POST /v1/auth/refresh`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Returned by sign-in and refresh. The refresh token rotates on every
/// refresh and lives in the platform credential store, never in SQLite.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub account_id: String,
    pub device_id: String,
    pub email: Option<String>,
    pub access_token: String,
    pub access_expires_at_ms: u64,
    pub refresh_token: String,
    pub refresh_expires_at_ms: u64,
}

/// One row of the Devices list. `GET /v1/devices`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub device_id: String,
    pub name: String,
    pub model: Option<String>,
    pub platform: Platform,
    pub signed_in_at_ms: u64,
    pub last_seen_ms: u64,
    /// Last time the device pushed or pulled ops; `None` before its first sync.
    pub last_sync_ms: Option<u64>,
    /// The device making the request.
    pub current: bool,
    pub revoked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceList {
    pub devices: Vec<DeviceSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_from_a_newer_server_still_parse() {
        let p: Platform = serde_json::from_str(r#""ios""#).unwrap();
        assert_eq!(p, Platform::Other);
        let e: ApiError = serde_json::from_str(r#"{"code":"quota_exhausted","message":"x"}"#).unwrap();
        assert_eq!(e.code, ErrorCode::Other);
        assert_eq!(e.retry_after_ms, None);
    }

    #[test]
    fn usernames_are_normalized_and_restricted() {
        assert_eq!(normalize_username("  Mohamed.Refaat "), "mohamed.refaat");
        assert!(valid_username("mohamed.refaat"));
        assert!(valid_username("dr_mo-85"));
        assert!(!valid_username("mo"));
        assert!(!valid_username(".mo"));
        assert!(!valid_username("Mohamed"), "checked after normalizing");
        assert!(!valid_username("محمد"));
        assert!(!valid_username("mo hamed"));
        assert!(!valid_username(&"a".repeat(33)));
        assert_eq!(password_salt(" MO.R "), b"steps-login-v1:mo.r".to_vec());
    }

    #[test]
    fn a_lockout_answer_carries_its_wait() {
        let e =
            ApiError { code: ErrorCode::TooManyTries, message: "wait".into(), retry_after_ms: Some(60_000) };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains(r#""code":"too_many_tries""#), "{json}");
        assert_eq!(serde_json::from_str::<ApiError>(&json).unwrap(), e);
        let plain = ApiError { code: ErrorCode::NotFound, message: "x".into(), retry_after_ms: None };
        assert!(!serde_json::to_string(&plain).unwrap().contains("retry_after_ms"));
    }
}
