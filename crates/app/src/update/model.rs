//! Update policy, offers and observable state; public names remain in update.

use super::*;

/// What Mohamed chose in Settings › Updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Check and download by itself; install when it's calm (Mac: next
    /// launch; Android: while Steps is in the background and may update
    /// quietly, otherwise one tap).
    #[default]
    Auto,
    /// Check by itself and say so; download and install on a tap.
    Notify,
    /// Only when "Check now" is pressed.
    Off,
}
impl Mode {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Notify, Self::Off];
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub mode: Mode,
    /// Release builds only (a preview build always follows `preview`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_check_ms: Option<u64>,
    /// The banner for this offer (`version+build`) was closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dismissed: Option<String>,
    /// An automatic install of this download failed once; wait for a tap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_failed: Option<String>,
    /// Announce a new version with a system notification (on by default).
    #[serde(default = "yes")]
    pub notify: bool,
    /// The last notification sent (`version+build:stage`), so each goes out once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notified: Option<String>,
    /// The app build (`version+build`) that ran last, to notice an update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<String>,
    /// Set at the first launch after an update to the build that ran
    /// before: "What's new" opens once for what came since, then clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whats_new_from: Option<String>,
    /// The content-only update (`version+build` of its feed) in use, kept
    /// apart from the app build so neither hides the other's notes (#348).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_content: Option<String>,
    /// Where the content-only notes count from, until seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_from: Option<String>,
}
pub(super) fn yes() -> bool {
    true
}
impl Default for Prefs {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            channel: None,
            last_check_ms: None,
            dismissed: None,
            auto_failed: None,
            notify: true,
            notified: None,
            last_run: None,
            whats_new_from: None,
            last_content: None,
            content_from: None,
        }
    }
}
/// The "new version" notification, as `app.steps.UpdateNotice.show` reads
/// it (JSON).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Notice {
    /// `version+build:stage`: remembered once shown. Android's job hands it
    /// back through `UpdateCheckJob.shown` after the notification is up.
    pub key: String,
    pub title: String,
    pub body: String,
    /// What the notification's button does in the app: `download` or
    /// `install`; empty for no button.
    pub action: &'static str,
    pub button: &'static str,
    /// The Android notification channel's name and description.
    pub channel: &'static str,
    pub channel_description: &'static str,
}
/// What's new, Egyptian Arabic first.
pub(crate) fn notes_of(o: &Offer) -> String {
    [&o.notes.ar_eg, &o.notes.en]
        .into_iter()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .unwrap_or_default()
        .to_owned()
}
/// The notification due for `s`, if any: one per offer and step (out, then
/// downloaded and ready), never for an offer whose banner was closed, and
/// nothing with the switch off or in "only when I check" mode.
pub fn notice(s: &Status) -> Option<Notice> {
    use crate::i18n::update as tx;
    if s.off.is_some() || s.prefs.mode == Mode::Off || !s.prefs.notify {
        return None;
    }
    let (offer, stage, title, hint, action, button) = match &s.phase {
        Phase::Available(o) => {
            (o, "available", tx::NOTICE_AVAILABLE, tx::NOTICE_TAP_AVAILABLE, "download", tx::DOWNLOAD)
        }
        Phase::Ready(o) | Phase::NeedsPermission(o) => {
            (o, "ready", tx::NOTICE_READY, tx::NOTICE_TAP_READY, "install", tx::INSTALL_NOW)
        }
        Phase::Installed(o) => (o, "installed", tx::NOTICE_INSTALLED, tx::NEXT_LAUNCH, "", ""),
        _ => return None,
    };
    if s.prefs.dismissed.as_deref() == Some(offer.key().as_str()) {
        return None;
    }
    let key = format!("{}:{stage}", offer.key());
    if s.prefs.notified.as_deref() == Some(key.as_str()) {
        return None;
    }
    let mut whats_new = if offer.changes.is_empty() {
        notes_of(offer)
    } else {
        changelog::lines_upto(&offer.changes, tx::notice_more, 3)
    };
    if whats_new.chars().count() > 300 {
        whats_new = whats_new.chars().take(299).collect::<String>() + "…";
    }
    let body = if whats_new.is_empty() {
        hint.to_owned()
    } else if offer.changes.is_empty() {
        format!("{}: {whats_new}\n{hint}", tx::WHATS_NEW)
    } else {
        format!("{whats_new}\n{hint}")
    };
    Some(Notice {
        key,
        title: format!("{title} · {}", offer.label()),
        body,
        action,
        button,
        channel: tx::NOTICE_CHANNEL,
        channel_description: tx::NOTICE_CHANNEL_SUB,
    })
}
/// Why this build doesn't update itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Off {
    /// A development build (`dx serve`, `cargo run`).
    DevBuild,
    /// No update key yet: `just update-keygen` hasn't run.
    NoKey,
    /// No builds are published for this system (Linux, x86 Android).
    NoPlatform,
    /// The Mac app runs from a build folder, not from a Steps.app.
    NotInBundle,
}
/// A newer build, from a verified manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub version: String,
    pub build: u64,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub notes: UpdateNotes,
    pub asset: UpdateAsset,
    /// What it brings: the feed's builds after the installed one, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<BuildChanges>,
}
impl Offer {
    pub(super) fn from_manifest(m: &UpdateManifest, asset: UpdateAsset, me: &Installed) -> Self {
        let changes = match changelog::order(&m.version, m.build_of(&asset)) {
            Some(to) => changelog::between(&m.history, Some((me.version, me.build)), to)
                .into_iter()
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        Self {
            version: m.version.clone(),
            // A content-only feed carries an older app; offer that build.
            build: m.build_of(&asset),
            commit: m.commit.clone(),
            notes: m.notes.clone(),
            asset,
            changes,
        }
    }

    /// `1.0.0-alpha.2`, or `0.1.0 (141)` for previews.
    pub fn label(&self) -> String {
        if self.build > 0 { format!("{} ({})", self.version, self.build) } else { self.version.clone() }
    }

    pub(super) fn key(&self) -> String {
        format!("{}+{}", self.version, self.build)
    }
}
/// Where the content pack in `<data>/updates/content.pack` came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PackInfo {
    pub(super) version: String,
    pub(super) build: u64,
    pub(super) sha256: String,
    /// The feed's `published_at` in seconds, so a stale feed of the same
    /// build can't roll the pack back (absent in packs taken before).
    #[serde(default)]
    pub(super) published_at: Option<i64>,
    /// The app build the pack's stylesheets were made with (the feed's
    /// `ui_build`); any other build uses only its content.
    #[serde(default)]
    pub(super) ui_build: Option<u64>,
}
impl PackInfo {
    pub(super) fn origin(&self) -> Option<PackOrigin> {
        Some(PackOrigin {
            version: Version::parse(&self.version)?,
            build: self.build,
            published_at: self.published_at,
            sha256: self.sha256.clone(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// No connection, or GitHub said no; try later.
    Network,
    /// The feed or the download didn't verify. Never installed.
    Verify(String),
    /// The system refused the install.
    Install(String),
    /// The feed needs a newer Steps: install by hand once.
    TooNew,
    /// Preview only: CI's update or APK signing key was renewed; install
    /// the newest preview by hand once.
    KeyChanged,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    UpToDate,
    /// Newer, but not published for this device yet.
    NotForThisDevice,
    Available(Offer),
    Downloading {
        offer: Offer,
        done: u64,
    },
    /// Downloaded and verified.
    Ready(Offer),
    Installing(Offer),
    /// Android: "Install unknown apps" is off for Steps.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    NeedsPermission(Offer),
    /// Mac: the new Steps.app is in place; it starts at the next launch.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Installed(Offer),
    Failed {
        offer: Option<Offer>,
        why: Failure,
    },
}
/// The content half of a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentCheck {
    /// The pack in use (or the built-in one) is the feed's.
    Current,
    /// A newer pack was fetched; it opens between sessions.
    New,
    /// The feed or the pack couldn't be fetched or taken; tried again later.
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub off: Option<Off>,
    pub phase: Phase,
    pub prefs: Prefs,
    /// The channel being followed right now.
    pub channel: Channel,
    /// A preview build: the channel is fixed.
    pub preview: bool,
    /// A newer content pack is downloaded and verified; it opens once he
    /// is between sessions ([`Updater::apply_content`]) or at the next start.
    pub content_waiting: bool,
    /// This build: version and build number.
    pub version: String,
    pub build: u64,
    /// The feed build whose content pack is in use, when newer than this
    /// build (a content-only update), else 0.
    pub content_build: u64,
    /// When that pack's feed was published (seconds), for Settings.
    pub content_at: Option<i64>,
    /// What the last check found for the lessons and questions, apart from
    /// the app's own `phase` (Settings shows both).
    pub content_check: Option<ContentCheck>,
    /// Every build's changes seen so far, newest first (changelog.rs).
    pub history: Vec<BuildChanges>,
}
impl Status {
    /// This build's place among the history's builds.
    pub fn current(&self) -> Option<(Version, u64)> {
        changelog::order(&self.version, self.shown_build())
    }

    /// The build number he is on: this build, or the content-only update
    /// on top of it.
    pub fn shown_build(&self) -> u64 {
        self.build.max(self.content_build)
    }

    /// The "what's new" to tell is only content-only updates: a quiet card
    /// on Today says so instead of the sheet opening by itself (#336).
    pub fn content_only_news(&self) -> bool {
        let news = self.whats_new();
        !news.is_empty() && news.iter().all(|b| b.content_only)
    }

    /// The content-only update on top of this build, when there is one.
    pub fn content_ahead(&self) -> Option<u64> {
        (self.content_build > self.build).then_some(self.content_build)
    }

    /// `version+build` of the app build he runs.
    pub(super) fn app_key(&self) -> String {
        format!("{}+{}", self.version, self.build)
    }

    /// `version+build` of the content-only update in use, if any.
    pub(super) fn content_key(&self) -> Option<String> {
        (self.content_build > 0).then(|| format!("{}+{}", self.version, self.content_build))
    }

    /// "What's new" to open by itself after an update, newest first: the
    /// app builds after the one that ran before, up to the one running, and
    /// the content-only updates after the content seen last, up to the one
    /// in use. App and content count apart, so a content update that is
    /// ahead never tells an app build he hasn't installed, nor swallows its
    /// notes once he does (#348). Empty once seen.
    pub fn whats_new(&self) -> Vec<BuildChanges> {
        let order = |k: &str| k.split_once('+').and_then(|(v, b)| changelog::order(v, b.parse().ok()?));
        let part = |from: Option<&String>, to: u64, content: bool| -> Vec<&BuildChanges> {
            let (Some(from), Some(to)) = (from.and_then(|k| order(k)), changelog::order(&self.version, to))
            else {
                return Vec::new();
            };
            changelog::between(&self.history, Some(from), to)
                .into_iter()
                .filter(|b| b.content_only == content)
                .collect()
        };
        let mut all = part(self.prefs.whats_new_from.as_ref(), self.build, false);
        all.extend(part(self.prefs.content_from.as_ref(), self.content_build, true));
        all.sort_by_key(|b| std::cmp::Reverse(changelog::order(&b.version, b.build)));
        all.dedup_by_key(|b| b.key());
        all.into_iter().cloned().collect()
    }

    /// Whether "What's new" has anything waiting to be seen.
    /// A content-only update came that he hasn't looked at yet: Settings
    /// marks the content row "new" until "What's new" is closed.
    pub fn content_news(&self) -> bool {
        self.whats_new().iter().any(|b| b.content_only)
    }

    pub fn news_pending(&self) -> bool {
        self.prefs.whats_new_from.is_some() || self.prefs.content_from.is_some()
    }

    /// The offer the banner should mention, unless closed for it.
    pub fn banner(&self) -> Option<&Offer> {
        let offer = match &self.phase {
            Phase::Available(o) if self.prefs.mode == Mode::Notify => o,
            Phase::Ready(o) | Phase::NeedsPermission(o) | Phase::Installed(o) => o,
            _ => return None,
        };
        (self.prefs.dismissed.as_deref() != Some(offer.key().as_str())).then_some(offer)
    }
}
/// What this binary is, from the build (CI sets the env vars for previews).
#[derive(Clone, Debug)]
pub(super) struct BuildInfo {
    pub(super) version: Version,
    pub(super) build: u64,
    pub(super) preview: bool,
    pub(super) key: Option<VerifyingKey>,
}
impl BuildInfo {
    pub(super) fn from_env() -> Self {
        let preview = option_env!("STEPS_UPDATE_CHANNEL") == Some("preview");
        let key_text = dev_override("STEPS_UPDATE_KEY").or_else(|| {
            if preview {
                option_env!("STEPS_UPDATE_KEY").map(str::to_owned)
            } else {
                Some(RELEASE_KEY.to_owned())
            }
        });
        let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
            stage: Stage::Alpha(0),
        });
        let build = dev_override("STEPS_BUILD")
            .as_deref()
            .or(option_env!("STEPS_BUILD"))
            .and_then(|b| b.trim().parse().ok())
            .unwrap_or(0);
        Self { version, build, preview, key: key_text.and_then(|k| parse_public_key(&k).ok()) }
    }

    /// The channel picked in Settings, else the build's own: `preview`
    /// for a CI preview build, `alpha` for a release build. Since the first
    /// alpha both are signed with the release keys (Mohamed, 2026-11-02), so
    /// either kind can follow any channel and move between them.
    pub(super) fn channel(&self, chosen: Option<Channel>) -> Channel {
        chosen.unwrap_or(if self.preview { Channel::Preview } else { Channel::Alpha })
    }

    pub(super) fn installed(&self, chosen: Option<Channel>) -> Installed {
        Installed { version: self.version, build: self.build, channel: self.channel(chosen) }
    }
}
/// The build number (CI's run number for previews, 0 otherwise).
pub fn build_number() -> u64 {
    get().map_or_else(|| BuildInfo::from_env().build, |u| u.info.build)
}
