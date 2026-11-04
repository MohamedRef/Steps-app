//! Messages between the Worker and an account's Durable Object. Both sides
//! ship in the same build, so these don't need versioning; devices never see
//! them. The DO is reachable only through the Worker's binding, so the
//! `x-steps-device` header set by the Worker is trusted there.

use serde::{Deserialize, Serialize};
use steps_protocol::cloud::DeviceInfo;

pub const DEVICE_HEADER: &str = "x-steps-device";

#[derive(Debug, Serialize, Deserialize)]
pub struct Register {
    pub info: DeviceInfo,
    pub email: String,
    pub refresh_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Refresh {
    pub device: String,
    pub presented_hash: String,
    pub new_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Revoke {
    pub device: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expiry {
    pub refresh_expires_ms: u64,
    pub email: String,
}

/// Account object `GET /login`: the caller's email (to put on a new
/// username) and the username the account has now.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountLoginState {
    pub email: String,
    pub username: Option<String>,
}

/// Account object `POST /login`: the username it has from now on.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginName {
    pub username: Option<String>,
}

/// Login object `POST /set`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginSet {
    pub account_id: String,
    pub email: String,
    pub salt: String,
    pub verifier: String,
}

/// Login object `POST /clear`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginClear {
    pub account_id: String,
}

/// Login object `POST /check`: the app's stretched key, base64url.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginCheck {
    pub key: String,
}
