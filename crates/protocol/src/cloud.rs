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
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
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
    }
}
