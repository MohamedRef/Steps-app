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
