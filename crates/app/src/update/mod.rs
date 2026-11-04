//! Updates over the air (`docs/RELEASE.md` §5): Steps checks its feed on
//! GitHub Releases, verifies the feed's Ed25519 signature and each
//! download's SHA-256, then installs it: PackageInstaller on Android, a
//! swapped Steps.app on the Mac. The content pack rides the same feed, so
//! new lectures arrive even while app updates wait or are off.
//!
//! Device-local, never synced:
//!
//! ```text
//! <data>/updates/prefs.json      automatic / tell me first / off, channel, last check,
//!                                the last "new version" notification sent
//! <data>/updates/staged.json     the verified app download waiting to install
//! <data>/updates/<file>          that download (one at a time)
//! <data>/updates/content.pack    the newest verified content pack
//! <data>/updates/content.json    which feed it came from (version, build, SHA-256)
//! <data>/updates/history.json    what each build changed (changelog.rs)
//! ```
//!
//! The work runs on its own thread; the UI follows [`Status`] through a
//! `tokio::sync::watch` channel.
//!
//! A new version is also announced with a system notification ([`Notice`]):
//! by the running app's own checks, and on Android by a periodic background
//! job (`app.steps.UpdateCheckJob`) that runs [`background_check`] even while
//! Steps is closed.
//!
//! Adapted from kabar-app main: crates/app/src/update/mod.rs. Changes for
//! Steps: `STEPS_*` build variables, the content pack (in place of Kabar's
//! offline packs), and one language.

mod recovery;
mod service;

mod model;
pub(crate) use model::notes_of;
use model::*;
pub use model::{
    ContentCheck, Failure, Mode, Notice, Off, Offer, Phase, Prefs, Status, build_number, notice,
};

pub mod bundle;
pub mod changelog;
pub mod sheet;
pub mod view;

use std::fs;
#[cfg_attr(target_arch = "wasm32", allow(unused_imports))] // the downloads (native only)
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use steps_core::pack::Pack;
use steps_core::update::{
    BuildChanges, Channel, Decision, Installed, MANIFEST_FILE, PackOrigin, SIGNATURE_FILE, Stage,
    UpdateAsset, UpdateError, UpdateManifest, UpdateNotes, UpdatePlatform, VerifyingKey, Version,
    check_download, content_update, decide, parse_public_key, published_secs, this_platform, verify_manifest,
};
use tokio::sync::watch;

use crate::diag::report;

/// The release public key (`just update-keygen` fills it once).
const RELEASE_KEY: &str = include_str!("../../update-key.pub");
/// How often a running Steps looks for an update by itself.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);
/// First automatic check after launch: late enough not to slow the start.
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(20);
/// Android's background job runs about every four hours
/// (`UpdateNotice.PERIOD_MS`); it looks at the feed when the last look, its
/// own or the app's, is at least this old.
const BACKGROUND_EVERY: Duration = Duration::from_secs(3 * 3600);
/// `update.json` and its signature are small; anything bigger is refused.
const MAX_FEED_BYTES: u64 = 256 * 1024;
/// Largest build accepted (the APK is a few tens of MB, Steps.app less).
const MAX_BUILD_BYTES: u64 = 400 * 1024 * 1024;
/// Largest content pack accepted (the whole chapter packs to a few MB).
const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;

const PACK_FILE: &str = "content.pack";
const PACK_INFO: &str = "content.json";
/// The sha256 of the last pack that failed its checks, so the same file
/// isn't downloaded again (#344).
const PACK_REJECTED: &str = "content.rejected";
/// How many times a pack was opened without Steps running calmly after it
/// (the crash-loop guard, #344).
const PACK_STARTS: &str = "content.starts";
/// Packs that downloaded whole but this app build couldn't take, with the
/// build that refused them: not fetched again until the app changes.
const PACK_UNREADABLE: &str = "content.unreadable.json";
/// Opens in a row that never got a couple of seconds past the first screen
/// before the pack is dropped.
const MAX_PACK_STARTS: u8 = 3;

/// The commit this build was made from (CI and `just apk` set it).
pub fn commit() -> Option<&'static str> {
    option_env!("STEPS_COMMIT").filter(|c| !c.is_empty())
}

/// Run-time overrides for trying updates from a development build against a
/// local feed (`docs/RELEASE.md` §5). Release builds ignore them.
fn dev_override(name: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(name).ok().filter(|v| !v.trim().is_empty()) } else { None }
}

pub struct Updater {
    dir: PathBuf,
    info: BuildInfo,
    platform: Option<UpdatePlatform>,
    /// A local feed folder instead of GitHub (development only).
    feed_override: Option<String>,
    /// Pause before re-reading a feed whose signature didn't match.
    retry_after: Duration,
    tx: watch::Sender<Status>,
    /// One check, download or install at a time.
    job: Mutex<()>,
}

/// The one updater of this process. On Android the background job may open
/// it before the activity starts ([`job_updater`]); [`init`] then adopts it,
/// so two updaters never share the `updates/` folder.
static UPDATER: OnceLock<Updater> = OnceLock::new();
/// Set once [`init`] has run (the UI is up), so [`get`] stays `None` for a
/// job-only process.
static STARTED: OnceLock<()> = OnceLock::new();

/// Opens the updater under the app's data folder and starts the background
/// checks. Called once at start.
pub fn init() {
    let Some(data) = crate::prefs::data_dir() else { return };
    let feed_override = dev_override("STEPS_UPDATE_URL");
    let platform =
        dev_override("STEPS_UPDATE_PLATFORM").and_then(|p| UpdatePlatform::parse(&p)).or_else(this_platform);
    let off = if cfg!(debug_assertions) && feed_override.is_none() {
        Some(Off::DevBuild)
    } else if cfg!(target_os = "macos") && bundle::running_bundle().is_none() && feed_override.is_none() {
        Some(Off::NotInBundle)
    } else {
        None
    };
    if STARTED.set(()).is_err() {
        return;
    }
    let updater = UPDATER.get_or_init(|| {
        Updater::new(data.join("updates"), BuildInfo::from_env(), platform, feed_override, off)
    });
    let start = updater.status().off.is_none();
    let background = updater.background_checks();
    crate::platform::update_notice::schedule(background);
    if !start {
        return;
    }
    let _ = std::thread::Builder::new().name("steps-update".into()).spawn(|| {
        std::thread::sleep(FIRST_CHECK_AFTER);
        loop {
            if let Some(u) = UPDATER.get() {
                u.tick();
            }
            std::thread::sleep(Duration::from_secs(30 * 60));
        }
    });
}

pub fn get() -> Option<&'static Updater> {
    STARTED.get().and(UPDATER.get())
}

/// The updater Android's background job (`UpdateCheckJob`) works with, in a
/// process that may have no UI: the app's own if it's running here, else
/// one opened under `data_dir` (Context.getFilesDir) that the app adopts if
/// it starts later in the same process.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn job_updater(data_dir: &Path) -> &'static Updater {
    UPDATER.get_or_init(|| {
        let off = cfg!(debug_assertions).then_some(Off::DevBuild);
        Updater::new(data_dir.join("updates"), BuildInfo::from_env(), this_platform(), None, off)
    })
}

/// Android's background job: checks when due and returns the notification
/// to post, if any. It isn't marked as sent yet: the job calls
/// [`background_shown`] once Android has shown it.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn background_check(data_dir: &Path, metered: bool, can_notify: bool) -> Option<Notice> {
    job_updater(data_dir).background(metered, can_notify)
}

/// The job's notification `key` is up: it doesn't go out again.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn background_shown(data_dir: &Path, key: &str) {
    job_updater(data_dir).shown(key);
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn is_newer(me: &Installed, offer: &Offer) -> bool {
    Version::parse(&offer.version).is_some_and(|v| (v, offer.build) > (me.version, me.build))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(target_os = "android")]
fn install_build(path: &Path, offer: &Offer) -> Phase {
    use crate::i18n::update as tx;
    use crate::platform::install;
    if !install::can_request_installs().unwrap_or(false) {
        if let Err(e) = install::open_install_permission() {
            report!("{e}");
        }
        return Phase::NeedsPermission(offer.clone());
    }
    let path = path.to_string_lossy();
    match install::install_apk(
        &path,
        offer.asset.version_code.unwrap_or(0),
        tx::DONE_TITLE,
        // "Steps was updated" says what's new when the feed says it.
        &Some(notes_of(offer)).filter(|n| !n.is_empty()).unwrap_or_else(|| tx::DONE_BODY.to_owned()),
        tx::READY_TITLE,
        (tx::INSTALL_CHANNEL, tx::INSTALL_CHANNEL_SUB),
    ) {
        Ok(()) => Phase::Installing(offer.clone()),
        Err(e) => Phase::Failed { offer: Some(offer.clone()), why: install_failure(e) },
    }
}

/// What `UpdateInstaller.check` says when the download is signed with
/// another key (a renewed preview key): kept in step with the Kotlin text.
const KEY_MISMATCH: &str = "the APK isn't signed with this Steps's key";

/// An install failure from Android; a signing-key mismatch is the renewed
/// key case, which no retry fixes (one reinstall by hand does).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn install_failure(why: String) -> Failure {
    if why.contains(KEY_MISMATCH) { Failure::KeyChanged } else { Failure::Install(why) }
}

#[cfg(target_os = "macos")]
fn install_build(path: &Path, offer: &Offer) -> Phase {
    let Some(app) = bundle::running_bundle() else {
        return Phase::Failed {
            offer: Some(offer.clone()),
            why: Failure::Install("not running from Steps.app".into()),
        };
    };
    match bundle::install_zip(path, &app, bundle::ditto_extract, |new| bundle::check_signature(new, &app)) {
        Ok(()) => Phase::Installed(offer.clone()),
        Err(e) => Phase::Failed { offer: Some(offer.clone()), why: Failure::Install(e) },
    }
}

#[cfg(not(any(target_os = "android", target_os = "macos")))]
fn install_build(_path: &Path, offer: &Offer) -> Phase {
    Phase::Failed { offer: Some(offer.clone()), why: Failure::Install("no installer on this system".into()) }
}

/// Mac, automatic mode: a verified download staged by the last session
/// goes in before the window opens, and Steps reopens as the new version.
/// Called first thing in `main`.
#[cfg(target_os = "macos")]
pub fn apply_staged_on_launch() {
    let Some(data) = crate::prefs::data_dir() else { return };
    let dir = data.join("updates");
    let prefs = load_prefs(&dir);
    if prefs.mode != Mode::Auto {
        return;
    }
    let Some(offer) = load_staged(&dir) else { return };
    let info = BuildInfo::from_env();
    if info.key.is_none()
        || !is_newer(&info.installed(prefs.channel), &offer)
        || prefs.auto_failed.as_deref() == Some(&offer.key())
    {
        return;
    }
    let Some(app) = bundle::running_bundle() else { return };
    let path = dir.join(&offer.asset.file);
    let verified = fs::read(&path)
        .is_ok_and(|bytes| check_download(&offer.asset, bytes.len() as u64, &sha256_hex(&bytes)).is_ok());
    let result = if verified {
        bundle::install_zip(&path, &app, bundle::ditto_extract, |new| bundle::check_signature(new, &app))
    } else {
        Err("changed on disk".into())
    };
    match result {
        Ok(()) => {
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(dir.join("staged.json"));
            bundle::relaunch(&app);
        }
        Err(e) => {
            report!("update {} not applied: {e}", offer.label());
            let mut prefs = prefs;
            prefs.auto_failed = Some(offer.key());
            let _ = save_prefs(&dir, &prefs);
        }
    }
}

/// Mac: quits and reopens the swapped-in Steps.app.
#[cfg(target_os = "macos")]
pub fn restart() {
    if let Some(app) = bundle::running_bundle() {
        bundle::relaunch(&app);
    }
}

/// The content pack a feed delivered, when it is at least as new as this
/// build and still decodes; `content::Library` uses it instead of the
/// embedded one. A pack from an older build (the app was updated since) is
/// removed.
pub fn installed_pack() -> Option<Pack> {
    installed_pack_in(&crate::prefs::data_dir()?.join("updates"), &BuildInfo::from_env())
}

fn installed_pack_in(dir: &Path, info: &BuildInfo) -> Option<Pack> {
    let Some(meta) = pack_in_use(dir, info) else {
        let _ = fs::remove_file(dir.join(PACK_FILE));
        let _ = fs::remove_file(dir.join(PACK_INFO));
        return recovery::restore_last_good(dir, info);
    };
    let drop = |why: &str| {
        report!("content pack {why}; using the built-in one");
        reject(dir, &meta.sha256);
        let _ = fs::remove_file(dir.join(PACK_FILE));
        let _ = fs::remove_file(dir.join(PACK_INFO));
        let _ = fs::remove_file(dir.join(PACK_STARTS));
        recovery::restore_last_good(dir, info)
    };
    // Opened before without Steps running calmly after it: the pack may be
    // what stops it, so it goes (#344).
    let starts =
        fs::read_to_string(dir.join(PACK_STARTS)).ok().and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
    if starts >= MAX_PACK_STARTS {
        return drop("didn't let Steps start");
    }
    let _ = fs::write(dir.join(PACK_STARTS), (starts + 1).to_string());
    let fits = Version::parse(&meta.version) == Some(info.version) && meta.ui_build == Some(info.build);
    let bytes = match fs::read(dir.join(PACK_FILE)) {
        Ok(bytes) => bytes,
        Err(_) => return drop("missing on disk"),
    };
    if !sha256_hex(&bytes).eq_ignore_ascii_case(&meta.sha256) {
        return drop("changed on disk");
    }
    match Pack::decode(&bytes) {
        Err(e) => drop(&format!("doesn't open ({e})")),
        Ok(p) if !p.problems().is_empty() => drop(&format!("isn't sound ({})", p.problems()[0])),
        Ok(mut p) => {
            // The stylesheets style this build's screens only.
            if !fits {
                p.look.clear();
            } else if let Some(bad) = crate::look::unsafe_sheet(&p.look) {
                return drop(&format!("stylesheet {bad} isn't allowed"));
            }
            Some(p)
        }
    }
}

/// Steps has run calmly for a while on the pack in use: the crash-loop
/// guard starts counting again.
pub fn pack_settled() {
    if let Some(dir) = crate::prefs::data_dir() {
        let dir = dir.join("updates");
        if let Err(e) = recovery::save_settled(&dir, &BuildInfo::from_env()) {
            report!("saving last good content pack: {e}");
        }
        let _ = fs::remove_file(dir.join(PACK_STARTS));
    }
}

fn is_rejected(dir: &Path, sha256: &str) -> bool {
    fs::read_to_string(dir.join(PACK_REJECTED)).is_ok_and(|s| s.trim().eq_ignore_ascii_case(sha256))
}

fn reject(dir: &Path, sha256: &str) {
    let _ = fs::create_dir_all(dir);
    let _ = fs::write(dir.join(PACK_REJECTED), sha256);
}

/// A pack that failed this build's checks. Unlike a rejection (it stopped
/// Steps starting) it is tied to the app build, so an app update that can
/// read it tries again. Adapted from kabar-app main:
/// `crates/app/src/update/content.rs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Unreadable {
    sha256: String,
    /// `version+build` of the app that refused it.
    app: String,
}

fn app_key(info: &BuildInfo) -> String {
    format!("{}+{}", info.version, info.build)
}

fn unreadable(dir: &Path) -> Vec<Unreadable> {
    fs::read(dir.join(PACK_UNREADABLE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn is_unreadable(dir: &Path, info: &BuildInfo, sha256: &str) -> bool {
    let app = app_key(info);
    unreadable(dir).iter().any(|u| u.app == app && u.sha256.eq_ignore_ascii_case(sha256))
}

fn mark_unreadable(dir: &Path, info: &BuildInfo, sha256: &str) {
    let app = app_key(info);
    // Entries from earlier app builds no longer apply.
    let mut list: Vec<_> = unreadable(dir).into_iter().filter(|u| u.app == app).collect();
    if list.iter().any(|u| u.sha256.eq_ignore_ascii_case(sha256)) {
        return;
    }
    list.push(Unreadable { sha256: sha256.to_ascii_lowercase(), app });
    let excess = list.len().saturating_sub(16);
    list.drain(..excess);
    if let Err(e) = write_json(dir, PACK_UNREADABLE, &list) {
        report!("unreadable content pack not remembered: {e}");
    }
}

fn update_failure(e: UpdateError) -> Failure {
    match e {
        UpdateError::TooNew => Failure::TooNew,
        other => Failure::Verify(other.to_string()),
    }
}

/// The first byte of a partial HTTP response, if it supplies a valid range.
#[cfg(not(target_arch = "wasm32"))]
fn range_start<B>(res: &ureq::http::Response<B>) -> Option<u64> {
    let value = res.headers().get("content-range")?.to_str().ok()?;
    value.strip_prefix("bytes ")?.split('-').next()?.trim().parse().ok()
}

/// The app's HTTP agent (updates, Steps Cloud, Google sign-in); local
/// development servers skip any proxy in the environment. Unit tests reach
/// only local servers: anything else goes to a closed port and fails at once.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn agent(url: &str, timeout: Duration) -> ureq::Agent {
    let local = url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost");
    let mut config = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(timeout));
    if local {
        config = config.proxy(None);
    } else if cfg!(test) {
        config = config.proxy(ureq::Proxy::new("http://127.0.0.1:9").ok());
    }
    config.build().into()
}

/// Tries within one download before it waits for the next check or tap.
const RESUME_TRIES: u32 = 3;

/// Where a build downloads to until it is whole: `<file>.part`.
fn part_of(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

/// Signed build identity for a resumable download.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PartMeta {
    size: u64,
    sha256: String,
}

fn meta_of(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part.json");
    path.with_file_name(name)
}

fn read_part_meta(path: &Path) -> Option<PartMeta> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Size and SHA-256 (hex) of a file on disk.
fn hash_file(path: &Path) -> std::io::Result<(u64, String)> {
    let mut hash = Sha256::new();
    let size = std::io::copy(&mut fs::File::open(path)?, &mut hash)?;
    Ok((size, hex(&hash.finalize())))
}

fn get_small(url: &str) -> Result<Vec<u8>, Failure> {
    let bytes = get_limited(url, MAX_FEED_BYTES, Duration::from_secs(30))?;
    if bytes.len() as u64 > MAX_FEED_BYTES {
        return Err(Failure::Verify("feed too large".into()));
    }
    Ok(bytes)
}

/// The body, cut one byte past `limit` (so a caller can tell it was longer).
#[cfg(not(target_arch = "wasm32"))]
fn get_limited(url: &str, limit: u64, timeout: Duration) -> Result<Vec<u8>, Failure> {
    let mut res = agent(url, timeout).get(url).call().map_err(|_| Failure::Network)?;
    if res.status().as_u16() != 200 {
        return Err(Failure::Network);
    }
    let mut bytes = Vec::new();
    res.body_mut().as_reader().take(limit + 1).read_to_end(&mut bytes).map_err(|_| Failure::Network)?;
    Ok(bytes)
}

/// The web build never updates itself: each deploy is the newest build.
#[cfg(target_arch = "wasm32")]
fn get_limited(_url: &str, _limit: u64, _timeout: Duration) -> Result<Vec<u8>, Failure> {
    Err(Failure::Network)
}

fn load_prefs(dir: &Path) -> Prefs {
    fs::read(dir.join("prefs.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_prefs(dir: &Path, prefs: &Prefs) -> Result<(), String> {
    write_json(dir, "prefs.json", prefs)
}

fn load_staged(dir: &Path) -> Option<Offer> {
    serde_json::from_slice(&fs::read(dir.join("staged.json")).ok()?).ok()
}

fn save_staged(dir: &Path, offer: &Offer) -> Result<(), String> {
    write_json(dir, "staged.json", offer)
}

/// The feed's pack, when it is at least as new as this build (a pack from an
/// older build is stale: the app was updated since).
fn pack_in_use(dir: &Path, info: &BuildInfo) -> Option<PackInfo> {
    let meta = load_pack_info(dir)?;
    let from = Version::parse(&meta.version).map(|v| (v, meta.build))?;
    (from >= (info.version, info.build)).then_some(meta)
}

fn load_pack_info(dir: &Path) -> Option<PackInfo> {
    serde_json::from_slice(&fs::read(dir.join(PACK_INFO)).ok()?).ok()
}

fn write_json(dir: &Path, name: &str, value: &impl Serialize) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{name}.part"));
    fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    fs::rename(&tmp, dir.join(name)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
