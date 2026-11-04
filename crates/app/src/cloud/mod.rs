//! Steps Cloud from the app's side: Google sign-in, live sync and the
//! Devices list. A background thread runs the [`engine::Engine`] on its own
//! tokio runtime and database connection; the UI talks to it through
//! channels and shows it in Settings › Sync (`ui.rs`).
//!
//! Steps Cloud is its own free Cloudflare Worker (`crates/server`): nothing
//! is shared with Kabar Cloud. See docs/CLOUD.md.
// adapted from kabar-app main: crates/app/src/cloud/mod.rs

pub mod api;
pub mod engine;
// The Mac and Android sign in through a loopback listener; the web app
// leaves the page for Google and comes back (docs/WEB.md).
#[cfg(test)]
mod fake;
#[cfg(not(target_arch = "wasm32"))]
pub mod oauth;
#[cfg(target_arch = "wasm32")]
#[path = "oauth_web.rs"]
pub mod oauth;
pub mod session;
mod socket;
#[cfg(test)]
mod tests;
mod ui;

use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use steps_protocol::cloud::{DeviceInfo, DeviceSummary, Platform};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[cfg(not(target_arch = "wasm32"))]
pub use self::oauth::GoogleClient;

/// The web app's Google client: a "Web application" one, no secret.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
pub struct GoogleClient {
    pub client_id: String,
}

/// Steps Cloud, for builds that don't say otherwise (the web app).
#[cfg(target_arch = "wasm32")]
const DEFAULT_CLOUD_URL: &str = "https://steps-cloud.hamorefaat85.workers.dev";
#[cfg(test)]
pub use ui::Cloud;
pub use ui::{SyncSection, ago, use_provide_cloud};

/// Where Steps Cloud is and which Google client signs in. Set at build time
/// from `.env` (`STEPS_CLOUD_URL`, `STEPS_GOOGLE_CLIENT_ID`,
/// `STEPS_GOOGLE_CLIENT_SECRET`); a run-time `STEPS_CLOUD_URL` overrides the
/// URL for local development.
#[derive(Clone, Debug)]
pub struct CloudConfig {
    pub base_url: String,
    pub google: Option<GoogleClient>,
}

impl CloudConfig {
    /// The web app: Steps Cloud's URL and the Web Google client come from the
    /// build (`STEPS_CLOUD_URL`, `STEPS_WEB_GOOGLE_CLIENT_ID`).
    #[cfg(target_arch = "wasm32")]
    pub fn from_env() -> Option<Self> {
        let base_url =
            option_env!("STEPS_CLOUD_URL").filter(|v| !v.trim().is_empty()).unwrap_or(DEFAULT_CLOUD_URL);
        let google = option_env!("STEPS_WEB_GOOGLE_CLIENT_ID")
            .filter(|v| !v.trim().is_empty())
            .or(WEB_GOOGLE_CLIENT_ID)
            .map(|id| GoogleClient { client_id: id.to_string() });
        Some(Self { base_url: base_url.to_string(), google })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_env() -> Option<Self> {
        let var = |name: &str, built: Option<&'static str>| {
            std::env::var(name).ok().or(built.map(String::from)).filter(|v| !v.trim().is_empty())
        };
        let base_url = var("STEPS_CLOUD_URL", option_env!("STEPS_CLOUD_URL"))?;
        let google = var("STEPS_GOOGLE_CLIENT_ID", option_env!("STEPS_GOOGLE_CLIENT_ID")).map(|client_id| {
            GoogleClient {
                client_id,
                client_secret: var("STEPS_GOOGLE_CLIENT_SECRET", option_env!("STEPS_GOOGLE_CLIENT_SECRET")),
            }
        });
        Some(Self { base_url, google })
    }
}

#[derive(Debug)]
pub enum Command {
    SignInGoogle,
    SignOut,
    LoadDevices,
    RevokeDevice(String),
    /// A local write landed in the outbox: push it now rather than on the
    /// next two-second tick.
    Kick,
    #[allow(dead_code)] // the app runs until the process ends
    Shutdown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Status(CloudStatus),
    Devices(Result<Vec<DeviceSummary>, Problem>),
    /// Ops from another device changed these tables.
    Changed(BTreeSet<String>),
    /// The browser is open for Google sign-in.
    SigningIn(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    SignedOut,
    Connecting,
    CatchingUp,
    Live,
    /// Waiting to reconnect.
    Offline,
    /// Needs an app update or a new sign-in.
    Stopped,
}

/// Something Mohamed should hear about, in Arabic (`ui.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NotConfigured,
    NotAllowed,
    SignInCancelled,
    SignInFailed,
    SignedOutRemotely,
    Network,
    /// Google approved the sign-in; the network half is waiting to retry.
    SignInWaiting,
    Server,
    Outdated,
    /// The hub turned changes away because this device's clock is ahead.
    ClockOff,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloudStatus {
    /// This build knows a Steps Cloud URL.
    pub configured: bool,
    pub email: Option<String>,
    pub device_id: String,
    pub sync: SyncState,
    /// Local changes not yet acked.
    pub pending: u64,
    pub last_sync_ms: Option<u64>,
    pub problem: Option<Problem>,
    /// Google approved a sign-in that is still reaching the server.
    pub finishing_sign_in: bool,
}

impl CloudStatus {
    pub fn not_configured(device_id: String) -> Self {
        Self {
            configured: false,
            email: None,
            device_id,
            sync: SyncState::SignedOut,
            pending: 0,
            last_sync_ms: None,
            problem: None,
            finishing_sign_in: false,
        }
    }
}

/// The web app's own Google client id ("Steps web", public, not a secret;
/// docs/WEB.md). None until Steps' own Google project exists;
/// `STEPS_WEB_GOOGLE_CLIENT_ID` sets it at build time.
#[cfg(target_arch = "wasm32")]
const WEB_GOOGLE_CLIENT_ID: Option<&str> = None;

pub struct CloudHandle {
    pub commands: UnboundedSender<Command>,
    pub events: UnboundedReceiver<Event>,
}

/// Starts the engine on its own thread. `db_path` is opened a second time
/// (SQLite WAL), so the UI's connection never waits on the network.
#[cfg(not(target_arch = "wasm32"))]
pub fn start(
    config: CloudConfig,
    db_path: PathBuf,
    device: DeviceInfo,
    vault: Box<dyn session::SessionVault>,
) -> Result<CloudHandle, String> {
    // Lives as long as the app (the sync thread never ends).
    let store: &'static crate::store::Store =
        Box::leak(Box::new(crate::store::Store::open(&db_path).map_err(|e| e.to_string())?));
    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (ev_tx, ev_rx) = unbounded_channel();
    std::thread::Builder::new()
        .name("steps-cloud".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(r) => r,
                Err(e) => {
                    crate::diag::report!("no runtime for sync: {e}");
                    return;
                }
            };
            let api = api::Api::new(&config.base_url);
            let engine = engine::Engine::new(store, api, config.google, device, vault, ev_tx);
            runtime.block_on(engine.run(cmd_rx));
        })
        .map_err(|e| e.to_string())?;
    Ok(CloudHandle { commands: cmd_tx, events: ev_rx })
}

/// The web app: the engine runs on the page's own event loop, on the store
/// the screens use (the browser has one thread and one database).
#[cfg(target_arch = "wasm32")]
pub fn start(config: CloudConfig, device: DeviceInfo) -> CloudHandle {
    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (ev_tx, ev_rx) = unbounded_channel();
    let api = api::Api::new(&config.base_url);
    let vault = Box::new(session::BrowserVault);
    let engine = engine::Engine::new(crate::store::shared(), api, config.google, device, vault, ev_tx);
    wasm_bindgen_futures::spawn_local(engine.run(cmd_rx));
    CloudHandle { commands: cmd_tx, events: ev_rx }
}

/// What this device calls itself on the Devices list.
pub fn device_info(device_id: String) -> DeviceInfo {
    let model = device_model();
    let (name, platform) = if cfg!(target_os = "android") {
        (model.clone().unwrap_or_else(|| "Android".into()), Platform::Android)
    } else if cfg!(target_os = "macos") {
        (mac_name().unwrap_or_else(|| "Mac".into()), Platform::Macos)
    } else if cfg!(target_arch = "wasm32") {
        (browser_name(), Platform::Web)
    } else {
        ("Steps (dev)".into(), Platform::Other)
    };
    DeviceInfo { device_id, name, model, platform }
}

#[cfg(target_arch = "wasm32")]
fn browser_name() -> String {
    crate::platform::web::browser_name()
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_name() -> String {
    "Browser".into()
}

fn mac_name() -> Option<String> {
    let out = std::process::Command::new("scutil").args(["--get", "ComputerName"]).output().ok()?;
    let name = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "android")]
fn device_model() -> Option<String> {
    use jni::{jni_sig, jni_str};
    crate::platform::tts_android::with_activity(|env, _| {
        let model = env
            .get_static_field(
                jni_str!("android/os/Build"),
                jni_str!("MODEL"),
                jni_sig!("Ljava/lang/String;"),
            )?
            .l()?;
        let model = env.cast_local::<jni::objects::JString>(model)?;
        model.try_to_string(env)
    })
    .ok()
}

#[cfg(not(target_os = "android"))]
fn device_model() -> Option<String> {
    None
}

/// What `open_url` was asked to open (tests).
#[cfg(test)]
pub static OPENED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Opens `url` in the default browser (Google sign-in).
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android_open_url(url)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().ok_or("no window")?;
        window.open_with_url_and_target(url, "_blank").map(drop).map_err(|e| format!("{e:?}"))
    }
    // Unit tests open nothing on the machine running them: the address is
    // only noted, so a test can play the browser.
    #[cfg(test)]
    {
        OPENED.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(url.to_owned());
        Ok(())
    }
    #[cfg(not(any(target_os = "android", target_arch = "wasm32", test)))]
    {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        std::process::Command::new(opener).arg(url).spawn().map(drop).map_err(|e| format!("{opener}: {e}"))
    }
}

// adapted from kabar-app main: crates/app/src/platform/android.rs (open_url)
#[cfg(target_os = "android")]
fn android_open_url(url: &str) -> Result<(), String> {
    use jni::{jni_sig, jni_str};
    crate::platform::tts_android::with_activity(|env, activity| {
        let action = env
            .get_static_field(
                jni_str!("android/content/Intent"),
                jni_str!("ACTION_VIEW"),
                jni_sig!("Ljava/lang/String;"),
            )?
            .l()?;
        let url = env.new_string(url)?;
        let uri = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("parse"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[(&url).into()],
            )?
            .l()?;
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
            &[(&action).into(), (&uri).into()],
        )?;
        // FLAG_ACTIVITY_NEW_TASK
        env.call_method(
            &intent,
            jni_str!("addFlags"),
            jni_sig!("(I)Landroid/content/Intent;"),
            &[0x1000_0000.into()],
        )?;
        env.call_method(
            activity,
            jni_str!("startActivity"),
            jni_sig!("(Landroid/content/Intent;)V"),
            &[(&intent).into()],
        )?;
        Ok(())
    })
}
