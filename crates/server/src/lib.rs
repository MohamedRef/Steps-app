//! Steps Cloud: Google sign-in, the device registry and the per-account sync
//! hub. A separate free Cloudflare Worker with its own storage and secrets;
//! it shares nothing with any other app's cloud.
//!
//! Everything outside `wasm` is plain Rust, tested natively against real
//! SQLite. `wasm` binds it to the Cloudflare runtime (Worker + Durable Object).
// adapted from batna-app main: crates/server (auth, devices and sync only:
// no text-to-speech, push or blobs).

pub mod account;
pub mod config;
pub mod devices;
pub mod google;
pub mod hub;
pub mod internal;
pub mod login;
pub mod sql;
pub mod tokens;

#[cfg(target_arch = "wasm32")]
mod wasm;
