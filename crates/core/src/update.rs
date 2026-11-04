//! Updates over the air (`docs/RELEASE.md` §5): what a feed publishes, the
//! order of versions, Android's `versionCode`, checking a feed's Ed25519
//! signature, and deciding whether a signed manifest is an update for this
//! build. Pure logic: the app downloads and installs, `steps-tools update`
//! signs.
//!
//! Adapted from kabar-app main: `crates/protocol/src/update.rs` (the feed
//! types) and `crates/core/src/update.rs` (the checks), merged into one
//! module. Steps has its own key, its own channels (`steps-*` release tags)
//! and one more kind of asset, the content pack, so new lectures can arrive
//! without a new app build.
//!
//! A feed is a few files on one GitHub release of the repository per channel:
//!
//! ```text
//! update.json        an UpdateManifest
//! update.json.sig    Ed25519 signature of update.json's exact bytes (base64)
//! steps-….apk        the Android build        ┐
//! Steps-….app.zip    the Mac build (ditto)    ├ named by UpdateAsset::file,
//! steps-content-….pack  the content pack      ┘ checked by UpdateAsset::sha256
//! ```
//!
//! The signing key never leaves the Mac's Keychain (or, for `Preview`, the
//! CI cache). The app carries only the public key. A device refuses a
//! manifest whose signature, channel or version doesn't check out, and a
//! download whose size or SHA-256 differs from the signed manifest.

use std::cmp::Ordering;
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
pub use ed25519_dalek::VerifyingKey;
use ed25519_dalek::{Signature, Verifier};
use serde::{Deserialize, Serialize};

/// Bumped when the manifest changes incompatibly. A device ignores feeds with
/// a newer format and says Steps itself needs a manual update.
pub const UPDATE_FORMAT: u32 = 1;
/// The manifest's file name in a feed.
pub const MANIFEST_FILE: &str = "update.json";
/// The detached signature's file name in a feed.
pub const SIGNATURE_FILE: &str = "update.json.sig";
/// Where the feeds live: `<FEED_BASE>/<release tag>/update.json`.
pub const FEED_BASE: &str = "https://github.com/MohamedRef/Steps-app/releases/download";

/// Where a build gets its updates from. A build signed with the release key
/// follows `Alpha`, `Beta` or `Stable`; a CI preview build is signed with the
/// preview key and can only follow `Preview` (Android refuses an APK signed
/// with another key anyway).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Every push to `steps`, built and signed by CI.
    Preview,
    Alpha,
    Beta,
    Stable,
}

impl Channel {
    pub const ALL: [Self; 4] = [Self::Preview, Self::Alpha, Self::Beta, Self::Stable];
    pub const RELEASE: [Self; 3] = [Self::Alpha, Self::Beta, Self::Stable];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s.trim())
    }

    /// The GitHub release tag that carries this channel's feed. All start
    /// with `steps-` but never `steps-v`, the prefix of release-day version tags.
    pub fn release_tag(self) -> &'static str {
        match self {
            Self::Preview => "steps-preview",
            Self::Alpha => "steps-alpha",
            Self::Beta => "steps-beta",
            Self::Stable => "steps-stable",
        }
    }

    /// `update.json`'s URL for this channel.
    pub fn feed_url(self) -> String {
        format!("{FEED_BASE}/{}/{MANIFEST_FILE}", self.release_tag())
    }
}

/// What an asset is: a build for one platform, or the content pack every
/// platform shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdatePlatform {
    /// The S24 Ultra and the Tab S11.
    AndroidArm64,
    /// Steps.app for Apple silicon.
    MacosArm64,
    /// Steps.app for an Intel Mac.
    #[serde(rename = "macos-x86_64")]
    MacosX86_64,
    /// `steps_core::pack` bytes: the lectures, for any platform.
    Content,
}

impl UpdatePlatform {
    pub const ALL: [Self; 4] = [Self::AndroidArm64, Self::MacosArm64, Self::MacosX86_64, Self::Content];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AndroidArm64 => "android-arm64",
            Self::MacosArm64 => "macos-arm64",
            Self::MacosX86_64 => "macos-x86_64",
            Self::Content => "content",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == s.trim())
    }

    /// The extension of this asset's file in a feed.
    pub fn extension(self) -> &'static str {
        match self {
            Self::AndroidArm64 => "apk",
            Self::MacosArm64 | Self::MacosX86_64 => "app.zip",
            Self::Content => "pack",
        }
    }
}

/// `update.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// [`UPDATE_FORMAT`] when written.
    pub format: u32,
    /// The channel this feed is for. A device checks it matches the one it
    /// asked, so a stable feed can't be swapped for an older alpha one.
    pub channel: Channel,
    /// The workspace version, e.g. `1.0.0-alpha.2`.
    pub version: String,
    /// Orders builds of the same version (the CI run number for previews, 0
    /// for releases, which always bump `version`).
    #[serde(default)]
    pub build: u64,
    /// The commit it was built from (shown in Settings › About).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// RFC 3339, when the feed was signed.
    pub published_at: String,
    /// What's new, a line or two per language. Shown before installing.
    #[serde(default)]
    pub notes: UpdateNotes,
    /// Assets for platforms this build doesn't know are skipped, so a feed
    /// can add a platform without older apps refusing the whole manifest.
    #[serde(deserialize_with = "known_only")]
    pub assets: Vec<UpdateAsset>,
    /// What each recent build changed, newest first (this build, when it
    /// changed anything visible, then up to [`HISTORY_BUILDS`] before it), so
    /// the app can say what's new before installing and keep a history.
    /// Older apps ignore it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<BuildChanges>,
    /// The app build the content pack's look (its stylesheets) was made
    /// with: only a running build of this version and exactly this number
    /// uses them, so a newer look never lands on older screens. Absent: the
    /// pack carries content only for every build. Older apps ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_build: Option<u64>,
}

impl UpdateManifest {
    pub fn asset(&self, platform: UpdatePlatform) -> Option<&UpdateAsset> {
        self.assets.iter().find(|a| a.platform == platform)
    }

    /// The build that made `asset`: an earlier one when a content-only feed
    /// carries the app over unchanged, else this feed's.
    pub fn build_of(&self, asset: &UpdateAsset) -> u64 {
        asset.build.unwrap_or(self.build)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateNotes {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub en: String,
    /// Egyptian Arabic (the app's language).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ar_eg: String,
}

/// How many builds [`UpdateManifest::history`] keeps.
pub const HISTORY_BUILDS: usize = 40;

/// One build's changes, as Mohamed reads them: written per pull request in
/// `changes/*.yaml` (`docs/RELEASE.md` §5) and gathered when a feed is built.
///
/// Adapted from kabar-app main: `crates/protocol/src/update.rs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildChanges {
    pub version: String,
    #[serde(default)]
    pub build: u64,
    /// RFC 3339, when the build was published.
    pub published_at: String,
    /// Lines of a kind this build doesn't know are skipped, like assets.
    #[serde(deserialize_with = "known_only")]
    pub changes: Vec<Change>,
    /// Lectures, lessons or the look only: it arrived without installing
    /// the app (a content-only feed).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub content_only: bool,
}

impl BuildChanges {
    /// `version+build`, the same key the app uses for an offer.
    pub fn key(&self) -> String {
        format!("{}+{}", self.version, self.build)
    }
}

/// One line of "what's new".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub kind: ChangeKind,
    /// The line itself; Egyptian Arabic is required.
    #[serde(flatten)]
    pub text: UpdateNotes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Something he couldn't do before.
    New,
    /// Something that works or looks better.
    Improved,
    /// Something that was broken.
    Fixed,
}

impl ChangeKind {
    pub const ALL: [Self; 3] = [Self::New, Self::Improved, Self::Fixed];
}

/// A feed list whose entries name something by a string field: the
/// platform of an asset, the kind of a change line.
trait Named: serde::de::DeserializeOwned {
    const FIELD: &'static str;
    fn known(name: &str) -> bool;
}

impl Named for UpdateAsset {
    const FIELD: &'static str = "platform";
    fn known(name: &str) -> bool {
        UpdatePlatform::parse(name).is_some()
    }
}

impl Named for Change {
    const FIELD: &'static str = "kind";
    fn known(name: &str) -> bool {
        ChangeKind::ALL.iter().any(|k| serde_json::to_value(k).is_ok_and(|v| v == name))
    }
}

/// Entries naming a platform or change kind this build doesn't know (one
/// added after it was built) are skipped instead of failing the whole feed;
/// the signature covers the exact bytes, so nothing is re-signed, and those
/// entries were never for this build. Every other entry must parse: a
/// damaged known entry still fails the manifest.
///
/// Adapted from kabar-app main: `crates/protocol/src/update.rs`.
fn known_only<'de, D, T>(de: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Named,
{
    use serde::de::Error as _;
    let mut out = Vec::new();
    for v in Vec::<serde_json::Value>::deserialize(de)? {
        if let Some(name) = v.get(T::FIELD).and_then(|n| n.as_str())
            && !T::known(name)
        {
            continue;
        }
        out.push(serde_json::from_value(v).map_err(D::Error::custom)?);
    }
    Ok(out)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateAsset {
    pub platform: UpdatePlatform,
    /// File name next to `update.json` in the same release. Never a URL, so
    /// a signed manifest can only point inside its own feed.
    pub file: String,
    pub size: u64,
    /// SHA-256 of the file, lowercase hex.
    pub sha256: String,
    /// Android's `versionCode` of the APK, which must grow for an install
    /// over the old app to succeed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_code: Option<u64>,
    /// The build that made this file when it is carried over from an
    /// earlier feed (a content-only feed keeps the app it had); absent, the
    /// manifest's own build. Apps compare this, not the feed's build, so a
    /// content-only feed never offers the app they already run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<u64>,
}

/// A workspace version: `1.0.0`, `1.0.0-alpha.2`, `1.0.0-beta.1`, `1.0.0-rc.1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub stage: Stage,
}

/// Pre-releases sort before their release: alpha < beta < rc < final.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Alpha(u32),
    Beta(u32),
    Rc(u32),
    Final,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (s, None),
        };
        let mut parts = core.split('.').map(|p| p.parse::<u32>().ok());
        let (major, minor, patch) = (parts.next()??, parts.next()??, parts.next()??);
        if parts.next().is_some() {
            return None;
        }
        let stage = match pre {
            None => Stage::Final,
            Some(p) => {
                let (kind, n) = p.split_once('.')?;
                let n = n.parse().ok()?;
                match kind {
                    "alpha" => Stage::Alpha(n),
                    "beta" => Stage::Beta(n),
                    "rc" => Stage::Rc(n),
                    _ => return None,
                }
            }
        };
        Some(Self { major, minor, patch, stage })
    }

    /// Android's `versionCode`, the same recipe as
    /// `scripts/android-release-apk.sh`: `1.2.3-alpha.4` → 10203104. Major,
    /// minor and patch, then the stage (alpha 100+N, beta 500+N, rc 800+N, a
    /// release 999), so each update installs over the one before.
    pub fn version_code(self) -> u64 {
        let stage = match self.stage {
            Stage::Alpha(n) => 100 + u64::from(n),
            Stage::Beta(n) => 500 + u64::from(n),
            Stage::Rc(n) => 800 + u64::from(n),
            Stage::Final => 999,
        };
        (u64::from(self.major) * 10_000 + u64::from(self.minor) * 100 + u64::from(self.patch)) * 1000 + stage
    }

    /// The release channels that should offer this version: a beta also
    /// reaches everyone on alpha, a release everyone.
    pub fn channels(self) -> &'static [Channel] {
        match self.stage {
            Stage::Alpha(_) => &[Channel::Alpha],
            Stage::Beta(_) | Stage::Rc(_) => &[Channel::Alpha, Channel::Beta],
            Stage::Final => &[Channel::Alpha, Channel::Beta, Channel::Stable],
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch, self.stage).cmp(&(
            other.major,
            other.minor,
            other.patch,
            other.stage,
        ))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        match self.stage {
            Stage::Alpha(n) => write!(f, "-alpha.{n}"),
            Stage::Beta(n) => write!(f, "-beta.{n}"),
            Stage::Rc(n) => write!(f, "-rc.{n}"),
            Stage::Final => Ok(()),
        }
    }
}

/// What this build is: its version, its build number and the channel it
/// follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Installed {
    pub version: Version,
    pub build: u64,
    pub channel: Channel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// The public key built into the app isn't a valid Ed25519 key.
    BadKey,
    /// The signature file isn't a base64 Ed25519 signature.
    BadSignature,
    /// The signature doesn't match: not signed with Steps's key, or changed.
    NotSigned,
    /// The manifest isn't valid JSON for this format.
    BadManifest(String),
    /// Signed, but for another channel than the one asked.
    WrongChannel { asked: Channel, got: Channel },
    /// The feed is newer than this app understands: update by hand once.
    TooNew,
    /// A file name in the manifest that isn't a plain file name.
    BadFileName(String),
    /// The download's size or SHA-256 differs from the signed manifest.
    Mismatch,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadKey => write!(f, "the update key in this build is invalid"),
            Self::BadSignature => write!(f, "the update signature is unreadable"),
            Self::NotSigned => write!(f, "the update isn't signed with Steps's key"),
            Self::BadManifest(e) => write!(f, "the update manifest is invalid: {e}"),
            Self::WrongChannel { asked, got } => {
                write!(f, "asked the {} channel, got {}", asked.as_str(), got.as_str())
            }
            Self::TooNew => write!(f, "this update needs a newer Steps; install it by hand once"),
            Self::BadFileName(n) => write!(f, "unsafe file name in the manifest: {n}"),
            Self::Mismatch => write!(f, "the download doesn't match the signed manifest"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// Parses a base64 Ed25519 public key (32 bytes). Lines starting with `#`
/// and blank lines are skipped, so a key file can carry a comment.
pub fn parse_public_key(text: &str) -> Result<VerifyingKey, UpdateError> {
    let line = key_line(text).ok_or(UpdateError::BadKey)?;
    let bytes: [u8; 32] = B64.decode(line).ok().and_then(|b| b.try_into().ok()).ok_or(UpdateError::BadKey)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| UpdateError::BadKey)
}

/// The first line that isn't blank or a `#` comment.
pub fn key_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))
}

/// Checks `sig` (base64) over the exact bytes of `update.json`, then parses
/// it. Nothing in an unsigned or altered manifest is ever looked at.
pub fn verify_manifest(json: &[u8], sig: &str, key: &VerifyingKey) -> Result<UpdateManifest, UpdateError> {
    let sig: [u8; 64] =
        B64.decode(sig.trim()).ok().and_then(|b| b.try_into().ok()).ok_or(UpdateError::BadSignature)?;
    key.verify(json, &Signature::from_bytes(&sig)).map_err(|_| UpdateError::NotSigned)?;
    // The format is read first, so a newer feed says "too new" rather than
    // failing on a field this build doesn't know.
    #[derive(Deserialize)]
    struct Head {
        format: u32,
    }
    let head: Head = serde_json::from_slice(json).map_err(|e| UpdateError::BadManifest(e.to_string()))?;
    if head.format > UPDATE_FORMAT {
        return Err(UpdateError::TooNew);
    }
    let m: UpdateManifest =
        serde_json::from_slice(json).map_err(|e| UpdateError::BadManifest(e.to_string()))?;
    if Version::parse(&m.version).is_none() {
        return Err(UpdateError::BadManifest(format!("version {:?}", m.version)));
    }
    for a in &m.assets {
        if !is_plain_file_name(&a.file) {
            return Err(UpdateError::BadFileName(a.file.clone()));
        }
    }
    Ok(m)
}

/// A name with no directory part, so it can only resolve next to `update.json`.
pub fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

/// What a verified manifest means for this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    UpToDate,
    /// A newer build for this platform.
    Available(UpdateAsset),
    /// Newer, but published without a build for this platform (yet).
    NotForThisPlatform,
}

/// Is `m` (already verified) newer than `me`, and is there a build for
/// `platform`? A manifest for another channel is refused, so an old feed
/// can't be passed off as the one asked for.
pub fn decide(me: &Installed, m: &UpdateManifest, platform: UpdatePlatform) -> Result<Decision, UpdateError> {
    check_channel(me, m)?;
    let version = Version::parse(&m.version).ok_or_else(|| UpdateError::BadManifest(m.version.clone()))?;
    let asset = m.asset(platform);
    let build = asset.map_or(m.build, |a| m.build_of(a));
    if (version, build) <= (me.version, me.build) {
        return Ok(Decision::UpToDate);
    }
    Ok(match asset {
        Some(a) => Decision::Available(a.clone()),
        None => Decision::NotForThisPlatform,
    })
}

/// Where the content pack in use came from: the feed that delivered it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackOrigin {
    pub version: Version,
    pub build: u64,
    /// The feed's `published_at`, in seconds since 1970 (`None` for a pack
    /// taken before this was recorded).
    pub published_at: Option<i64>,
    /// SHA-256 of the pack, hex.
    pub sha256: String,
}

/// The feed's content pack, when it is newer than the one in use (`current`
/// is where that came from, or `None` for the pack built into the app).
///
/// Content moves on its own: a pack is offered whenever its hash changes,
/// even with the app build unchanged (the learner may keep updates of the
/// app itself off). The pack must still come from a build of the same
/// version or a newer one, since an older build's pack may lack lectures
/// this one already has. Among feeds of one build, only a later
/// `published_at` wins, so a stale cached or replayed feed (still validly
/// signed) never rolls the lectures back. The app decodes it with
/// `steps_core::pack` before switching, so a pack this build can't read is
/// never used.
pub fn content_update<'m>(
    me: &Installed,
    m: &'m UpdateManifest,
    current: Option<&PackOrigin>,
) -> Result<Option<&'m UpdateAsset>, UpdateError> {
    check_channel(me, m)?;
    let version = Version::parse(&m.version).ok_or_else(|| UpdateError::BadManifest(m.version.clone()))?;
    if (version, m.build) < (me.version, me.build) {
        return Ok(None);
    }
    let published = published_secs(&m.published_at)
        .ok_or_else(|| UpdateError::BadManifest(format!("published_at {}", m.published_at)))?;
    Ok(m.asset(UpdatePlatform::Content).filter(|a| {
        current.is_none_or(|c| {
            !a.sha256.eq_ignore_ascii_case(&c.sha256)
                && (version, m.build, Some(published)) > (c.version, c.build, c.published_at)
        })
    }))
}

/// `2026-10-27T08:00:00Z` (fractions and a `+00:00` offset allowed) →
/// seconds since 1970. Feeds are always signed in UTC.
pub fn published_secs(s: &str) -> Option<i64> {
    let s = s.trim();
    let s = s.strip_suffix('Z').or_else(|| s.strip_suffix("+00:00"))?;
    let s = s.split_once('.').map_or(
        s,
        |(whole, frac)| {
            if frac.bytes().all(|b| b.is_ascii_digit()) { whole } else { s }
        },
    );
    let (date, time) = s.split_once('T')?;
    let num = |t: &str, len: usize| {
        (t.len() == len && t.bytes().all(|b| b.is_ascii_digit())).then(|| t.parse::<i64>().ok()).flatten()
    };
    let mut d = date.split('-');
    let (y, mo, day) = (num(d.next()?, 4)?, num(d.next()?, 2)?, num(d.next()?, 2)?);
    let mut t = time.split(':');
    let (h, mi, sec) = (num(t.next()?, 2)?, num(t.next()?, 2)?, num(t.next()?, 2)?);
    if d.next().is_some() || t.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&mo) || !(1..=31).contains(&day) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    // Days from civil, proleptic Gregorian (Howard Hinnant's algorithm).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + sec)
}

fn check_channel(me: &Installed, m: &UpdateManifest) -> Result<(), UpdateError> {
    if m.channel == me.channel {
        Ok(())
    } else {
        Err(UpdateError::WrongChannel { asked: me.channel, got: m.channel })
    }
}

/// Checks a finished download against its signed manifest row.
pub fn check_download(asset: &UpdateAsset, size: u64, sha256_hex: &str) -> Result<(), UpdateError> {
    if asset.size == size && asset.sha256.eq_ignore_ascii_case(sha256_hex) {
        Ok(())
    } else {
        Err(UpdateError::Mismatch)
    }
}

/// The platform this binary was built for, if app updates exist for it.
pub fn this_platform() -> Option<UpdatePlatform> {
    if cfg!(all(target_os = "android", target_arch = "aarch64")) {
        Some(UpdatePlatform::AndroidArm64)
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(UpdatePlatform::MacosArm64)
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(UpdatePlatform::MacosX86_64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    /// The key that signed `fixtures/update/update.json` (test only; seed
    /// 0x42 repeated, written by `steps-tools`' `write_update_fixture` test).
    const FIXTURE_PUBLIC_KEY: &str = include_str!("../../../fixtures/update/test-key.pub");

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn asset(platform: UpdatePlatform, file: &str, sha: &str) -> UpdateAsset {
        UpdateAsset {
            platform,
            file: file.into(),
            size: 3,
            sha256: sha.into(),
            version_code: None,
            build: None,
        }
    }

    fn manifest(channel: Channel, version: &str, build: u64) -> UpdateManifest {
        UpdateManifest {
            format: UPDATE_FORMAT,
            channel,
            version: version.into(),
            build,
            commit: None,
            published_at: "2026-10-27T00:00:00Z".into(),
            notes: UpdateNotes::default(),
            assets: vec![
                UpdateAsset {
                    version_code: Some(v(version).version_code()),
                    ..asset(UpdatePlatform::AndroidArm64, &format!("steps-{version}.apk"), &"ab".repeat(32))
                },
                asset(UpdatePlatform::Content, &format!("steps-content-{version}.pack"), &"cd".repeat(32)),
            ],
            history: Vec::new(),
            ui_build: None,
        }
    }

    #[test]
    fn a_content_only_feed_never_offers_the_app_again() {
        let me = Installed { version: v("0.1.1-alpha.0"), build: 40, channel: Channel::Preview };
        let mut m = manifest(Channel::Preview, "0.1.1-alpha.0", 45);
        m.assets[0].build = Some(40);
        let android = UpdatePlatform::AndroidArm64;
        assert_eq!(decide(&me, &m, android), Ok(Decision::UpToDate), "the app it carries is the one running");
        assert!(content_update(&me, &m, None).unwrap().is_some(), "the new pack still comes");
        let older = Installed { build: 38, ..me };
        assert!(matches!(decide(&older, &m, android), Ok(Decision::Available(_))));
    }

    #[test]
    fn channel_and_platform_names_round_trip() {
        for c in Channel::ALL {
            assert_eq!(Channel::parse(c.as_str()), Some(c));
            assert_eq!(serde_json::to_string(&c).unwrap(), format!("\"{}\"", c.as_str()));
            assert!(c.release_tag().starts_with("steps-"), "feed tags all start with steps-");
        }
        for p in UpdatePlatform::ALL {
            assert_eq!(UpdatePlatform::parse(p.as_str()), Some(p));
            assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{}\"", p.as_str()));
        }
        assert_eq!(Channel::parse("nightly"), None);
        assert_eq!(
            Channel::Preview.feed_url(),
            "https://github.com/MohamedRef/Steps-app/releases/download/steps-preview/update.json"
        );
    }

    #[test]
    fn versions_order_alpha_beta_rc_release() {
        let order = [
            "0.1.0",
            "1.0.0-alpha.1",
            "1.0.0-alpha.2",
            "1.0.0-alpha.10",
            "1.0.0-beta.1",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1-alpha.1",
            "1.1.0",
        ];
        for pair in order.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        for s in order {
            assert_eq!(v(s).to_string(), s);
        }
        assert_eq!(v("v1.2.3"), v("1.2.3"));
        for bad in ["1.0", "1.0.0.0", "1.0.0-nightly.1", "1.0.0-alpha", "x.1.2", ""] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn version_codes_match_the_release_script() {
        // The examples in scripts/android-release-apk.sh.
        assert_eq!(v("1.2.3-alpha.4").version_code(), 10_203_104);
        assert_eq!(v("1.0.0-alpha.1").version_code(), 10_000_101);
        assert_eq!(v("0.1.0").version_code(), 100_999);
        // The workspace sits at 0.1.1-alpha.0 until the first alpha: above
        // the early previews (0.1.0, stage 999) so they keep updating, and
        // below every real alpha.
        assert!(v("0.1.0").version_code() < v("0.1.1-alpha.0").version_code());
        assert!(v("0.1.0") < v("0.1.1-alpha.0"));
        assert!(v("0.1.1-alpha.0").version_code() < v("0.1.1-alpha.1").version_code());
        assert!(v("0.1.1-alpha.0").version_code() < v("1.0.0-alpha.1").version_code());
        assert!(v("0.1.1-alpha.0") < v("0.1.1-alpha.1"));
        let ladder =
            ["1.0.0-alpha.1", "1.0.0-alpha.2", "1.0.0-beta.1", "1.0.0-rc.1", "1.0.0", "1.0.1-alpha.1"];
        for pair in ladder.windows(2) {
            assert!(v(pair[0]).version_code() < v(pair[1]).version_code());
        }
    }

    #[test]
    fn betas_and_releases_reach_alpha_followers() {
        assert_eq!(v("1.0.0-alpha.3").channels(), [Channel::Alpha]);
        assert_eq!(v("1.0.0-beta.1").channels(), [Channel::Alpha, Channel::Beta]);
        assert_eq!(v("1.0.0").channels(), [Channel::Alpha, Channel::Beta, Channel::Stable]);
    }

    #[test]
    fn a_manifest_without_history_still_parses() {
        let mut v: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../fixtures/update/update.json")).unwrap();
        v.as_object_mut().unwrap().remove("history");
        let m: UpdateManifest = serde_json::from_value(v).unwrap();
        assert!(m.history.is_empty());
        assert!(!serde_json::to_string(&m).unwrap().contains("history"));
    }

    #[test]
    fn the_signed_fixture_verifies_and_any_change_is_refused() {
        let key = parse_public_key(FIXTURE_PUBLIC_KEY).unwrap();
        let json = include_bytes!("../../../fixtures/update/update.json");
        let sig = include_str!("../../../fixtures/update/update.json.sig");
        let m = verify_manifest(json, sig, &key).unwrap();
        assert_eq!(m.channel, Channel::Preview);
        assert_eq!(m.build, 42);
        assert!(m.asset(UpdatePlatform::Content).is_some());
        assert!(m.asset(UpdatePlatform::MacosX86_64).is_none());
        let h = &m.history[0];
        assert_eq!((h.key().as_str(), h.changes[0].kind), ("0.1.0+42", ChangeKind::New));

        // One byte changed anywhere: refused before parsing.
        let mut changed = json.to_vec();
        let at = changed.iter().position(|b| *b == b'4').unwrap();
        changed[at] = b'9';
        assert_eq!(verify_manifest(&changed, sig, &key), Err(UpdateError::NotSigned));
        // Another key's signature: refused.
        let other = SigningKey::from_bytes(&[7; 32]);
        let forged = B64.encode(other.sign(json).to_bytes());
        assert_eq!(verify_manifest(json, &forged, &key), Err(UpdateError::NotSigned));
        assert_eq!(verify_manifest(json, "not base64!", &key), Err(UpdateError::BadSignature));
    }

    #[test]
    fn newer_formats_and_path_tricks_are_refused_even_when_signed() {
        let signer = SigningKey::from_bytes(&[9; 32]);
        let key = signer.verifying_key();
        let sign = |json: &[u8]| B64.encode(signer.sign(json).to_bytes());

        let future = br#"{"format": 99, "something": "new"}"#;
        assert_eq!(verify_manifest(future, &sign(future), &key), Err(UpdateError::TooNew));

        let mut m = manifest(Channel::Alpha, "1.0.0-alpha.2", 0);
        m.assets[0].file = "../../etc/passwd".into();
        let json = serde_json::to_vec(&m).unwrap();
        assert!(matches!(verify_manifest(&json, &sign(&json), &key), Err(UpdateError::BadFileName(_))));
        for name in ["https://evil.example/x.apk", "a/b.apk", "..", ""] {
            assert!(!is_plain_file_name(name), "{name}");
        }
        assert!(is_plain_file_name("Steps-1.0.0-alpha.2-macos-arm64.app.zip"));
    }

    #[test]
    fn decide_wants_newer_same_channel_and_this_platform() {
        let me = Installed { version: v("1.0.0-alpha.1"), build: 0, channel: Channel::Alpha };
        let android = UpdatePlatform::AndroidArm64;
        assert_eq!(
            decide(&me, &manifest(Channel::Alpha, "1.0.0-alpha.1", 0), android),
            Ok(Decision::UpToDate)
        );
        assert_eq!(decide(&me, &manifest(Channel::Alpha, "0.9.0", 0), android), Ok(Decision::UpToDate));
        let newer = manifest(Channel::Alpha, "1.0.0-alpha.2", 0);
        assert!(
            matches!(decide(&me, &newer, android), Ok(Decision::Available(a)) if a.file == "steps-1.0.0-alpha.2.apk")
        );
        assert_eq!(decide(&me, &newer, UpdatePlatform::MacosArm64), Ok(Decision::NotForThisPlatform));
        assert_eq!(
            decide(&me, &manifest(Channel::Stable, "2.0.0", 0), android),
            Err(UpdateError::WrongChannel { asked: Channel::Alpha, got: Channel::Stable })
        );

        // Previews share a version and move by build number.
        let preview = Installed { version: v("0.1.0"), build: 140, channel: Channel::Preview };
        assert_eq!(
            decide(&preview, &manifest(Channel::Preview, "0.1.0", 140), android),
            Ok(Decision::UpToDate)
        );
        assert!(matches!(
            decide(&preview, &manifest(Channel::Preview, "0.1.0", 141), android),
            Ok(Decision::Available(_))
        ));
    }

    #[test]
    fn content_packs_move_by_hash_but_never_backwards() {
        let me = Installed { version: v("0.1.0"), build: 140, channel: Channel::Preview };
        let same_build = manifest(Channel::Preview, "0.1.0", 140);
        let pack = &same_build.assets[1];
        let from = |build, published: &str, sha: &str| PackOrigin {
            version: v("0.1.0"),
            build,
            published_at: published_secs(published),
            sha256: sha.into(),
        };
        // The built-in pack is in use: the feed's is new.
        assert_eq!(content_update(&me, &same_build, None), Ok(Some(pack)));
        // Already using that one (case doesn't matter).
        let using = from(140, &same_build.published_at, &"CD".repeat(32));
        assert_eq!(content_update(&me, &same_build, Some(&using)), Ok(None));
        // A newer build's pack is fine too, even if the app update is off.
        let newer = manifest(Channel::Preview, "0.1.0", 141);
        assert!(
            content_update(&me, &newer, Some(&from(140, &newer.published_at, &"ef".repeat(32))))
                .unwrap()
                .is_some()
        );
        // An older build's pack is never taken.
        assert_eq!(content_update(&me, &manifest(Channel::Preview, "0.1.0", 139), None), Ok(None));
        // Nor another channel's.
        assert!(content_update(&me, &manifest(Channel::Alpha, "1.0.0-alpha.1", 0), None).is_err());
        // A feed without a pack offers none.
        let mut bare = newer.clone();
        bare.assets.truncate(1);
        assert_eq!(content_update(&me, &bare, None), Ok(None));
    }

    #[test]
    fn a_stale_feed_of_the_same_build_never_rolls_content_back() {
        let me = Installed { version: v("1.0.0-alpha.2"), build: 0, channel: Channel::Alpha };
        let mut older = manifest(Channel::Alpha, "1.0.0-alpha.2", 0);
        older.published_at = "2026-11-01T08:00:00Z".into();
        older.assets[1].sha256 = "aa".repeat(32);
        let mut later = older.clone();
        later.published_at = "2026-11-03T08:00:00Z".into();
        later.assets[1].sha256 = "bb".repeat(32);
        let holding = |m: &UpdateManifest| PackOrigin {
            version: v(&m.version),
            build: m.build,
            published_at: published_secs(&m.published_at),
            sha256: m.assets[1].sha256.clone(),
        };
        // Holding the older pack: the later feed's is taken.
        assert!(content_update(&me, &later, Some(&holding(&older))).unwrap().is_some());
        // Holding the later pack: a cached or replayed older feed is ignored.
        assert_eq!(content_update(&me, &older, Some(&holding(&later))), Ok(None));
        // A pack taken before the time was recorded still moves forward.
        let legacy = PackOrigin { published_at: None, ..holding(&older) };
        assert!(content_update(&me, &later, Some(&legacy)).unwrap().is_some());
        // A feed whose time can't be read is refused.
        let mut odd = later.clone();
        odd.published_at = "yesterday".into();
        assert!(content_update(&me, &odd, None).is_err());
    }

    #[test]
    fn published_times_parse_as_utc_seconds() {
        assert_eq!(published_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(published_secs("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(published_secs("2026-10-27T08:00:00Z"), Some(1_793_088_000));
        assert_eq!(published_secs("2026-10-27T08:00:00.250Z"), Some(1_793_088_000));
        assert_eq!(published_secs("2026-10-27T08:00:00+00:00"), Some(1_793_088_000));
        for bad in
            ["", "2026-10-27", "2026-10-27T08:00:00", "2026-10-27T08:00:00+02:00", "2026-13-01T00:00:00Z"]
        {
            assert_eq!(published_secs(bad), None, "{bad}");
        }
    }

    #[test]
    fn downloads_must_match_size_and_hash() {
        let a = &manifest(Channel::Alpha, "1.0.0-alpha.2", 0).assets[0];
        assert_eq!(check_download(a, 3, &"AB".repeat(32)), Ok(()));
        assert_eq!(check_download(a, 4, &"ab".repeat(32)), Err(UpdateError::Mismatch));
        assert_eq!(check_download(a, 3, &"cd".repeat(32)), Err(UpdateError::Mismatch));
    }

    #[test]
    fn key_files_may_carry_comments() {
        let key = SigningKey::from_bytes(&[3; 32]).verifying_key();
        let text = format!("# Steps's update key\n\n{}\n", B64.encode(key.as_bytes()));
        assert_eq!(parse_public_key(&text), Ok(key));
        assert_eq!(parse_public_key("# not generated yet\n"), Err(UpdateError::BadKey));
        assert_eq!(parse_public_key("AAAA"), Err(UpdateError::BadKey));
    }

    #[test]
    fn a_feed_with_a_platform_or_change_kind_from_a_newer_build_still_parses() {
        let mut v: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../fixtures/update/update.json")).unwrap();
        v["assets"].as_array_mut().unwrap().push(serde_json::json!({
            "platform": "windows-x86_64", "file": "steps.msix", "size": 1, "sha256": "00"
        }));
        v["history"][0]["changes"].as_array_mut().unwrap().push(serde_json::json!({
            "kind": "removed", "ar_eg": "حاجة اتشالت"
        }));
        v["a_field_from_later"] = serde_json::json!({"anything": [1, 2]});
        let m: UpdateManifest = serde_json::from_value(v).unwrap();
        assert_eq!(m.assets.len(), 2, "the unknown platform is skipped, the others stay");
        assert!(m.asset(UpdatePlatform::Content).is_some());
        assert_eq!(m.history[0].changes.len(), 3);
    }

    #[test]
    fn a_damaged_entry_of_a_known_platform_still_fails_the_feed() {
        // Skipping is for entries from the future, not for damaged ones.
        let mut v: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../fixtures/update/update.json")).unwrap();
        v["assets"][0].as_object_mut().unwrap().remove("sha256");
        assert!(serde_json::from_value::<UpdateManifest>(v).is_err());
    }
}
