//! Everything that differs between the Mac and Android. Android code reaches
//! the framework through JNI, plus one small Kotlin service in `android/`
//! (background oral mode).

pub mod background;
pub mod documents;
pub mod haptics;
#[cfg(target_os = "android")]
pub mod install;
pub mod network;
pub mod reminders;
pub mod system_bars;
pub mod time;
pub mod tts;
#[cfg(target_os = "android")]
pub(crate) mod tts_android;
pub mod update_notice;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod webview_guard;
