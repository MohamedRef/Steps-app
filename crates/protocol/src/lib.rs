//! Types shared by the Steps app and Steps Cloud (a Cloudflare Worker of its
//! own, separate from Kabar's): the HLC, live-sync messages, the HTTP API,
//! the merge rules both sides follow, and the device-side sync state machine.
//! Everything here is plain serde data and pure functions.
// adapted from kabar-app main: crates/protocol/src/{lib,hlc,sync,cloud}.rs,
// crates/core/src/{sync,sync_client}.rs

pub mod client;
pub mod cloud;
pub mod hlc;
pub mod merge;
pub mod sync;

pub use hlc::Hlc;

/// Bumped whenever a wire type changes incompatibly.
pub const PROTOCOL_VERSION: u32 = 1;
