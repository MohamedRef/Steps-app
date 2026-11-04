//! Update orchestration behind the existing Updater API.

use super::*;

/// Whether a feed pack with digest `sha` is exactly the content this app
/// has built in: CI builds the pack before the APK and bakes its digest in
/// as `STEPS_BUILTIN_CONTENT`. Only while no downloaded pack is in use, so a
/// feed going back to this content still replaces a newer one. Adapted from
/// kabar-app main: `crates/app/src/update/content.rs`.
pub(super) fn built_in(builtin: Option<&str>, pack_in_use: bool, sha: &str) -> bool {
    !pack_in_use && builtin.is_some_and(|b| b.trim().eq_ignore_ascii_case(sha))
}

impl Updater {
    pub(super) fn new(
        dir: PathBuf,
        info: BuildInfo,
        platform: Option<UpdatePlatform>,
        feed_override: Option<String>,
        off: Option<Off>,
    ) -> Self {
        let mut prefs = load_prefs(&dir);
        let off = off.or(if platform.is_none() {
            Some(Off::NoPlatform)
        } else if info.key.is_none() {
            Some(Off::NoKey)
        } else {
            None
        });
        let pack = pack_in_use(&dir, &info);
        let content_build = pack.as_ref().map_or(0, |p| p.build);
        let content_at = pack.and_then(|p| p.published_at);
        let this = format!("{}+{}", info.version, info.build);
        let content = (content_build > 0).then(|| format!("{}+{content_build}", info.version));
        if off.is_none()
            && (prefs.last_run.as_deref() != Some(this.as_str()) || prefs.last_content != content)
        {
            // Updated (not a first install): say what's new once.
            if prefs.last_run.as_deref() != Some(this.as_str())
                && let Some(prev) = prefs.last_run.replace(this.clone())
            {
                prefs.whats_new_from = Some(prev);
            }
            // A content-only update taken while Steps was closed: its notes
            // count from the content seen before, else from the app build.
            if prefs.last_content != content {
                if content.is_some() {
                    prefs.content_from = prefs.last_content.clone().or(Some(this.clone()));
                }
                prefs.last_content = content;
            }
            if let Err(e) = save_prefs(&dir, &prefs) {
                report!("update prefs: {e}");
            }
        }
        let history = changelog::load(&dir);
        let installed = info.installed(prefs.channel);
        let mut phase = Phase::Idle;
        if off.is_none()
            && let Some(offer) = load_staged(&dir)
            && is_newer(&installed, &offer)
            && dir.join(&offer.asset.file).metadata().is_ok_and(|m| m.len() == offer.asset.size)
        {
            phase = Phase::Ready(offer);
        }
        let status = Status {
            off,
            phase,
            channel: installed.channel,
            preview: info.preview,
            prefs,
            content_waiting: false,
            version: info.version.to_string(),
            build: info.build,
            content_build,
            content_at,
            content_check: None,
            history,
        };
        let (tx, _) = watch::channel(status);
        Self {
            dir,
            info,
            platform,
            feed_override,
            retry_after: Duration::from_secs(20),
            tx,
            job: Mutex::new(()),
        }
    }

    /// What's installed, on the channel chosen now.
    pub(super) fn installed(&self) -> Installed {
        self.info.installed(self.tx.borrow().prefs.channel)
    }

    /// The feed folder: the local override in development, else the
    /// channel's GitHub release.
    pub(super) fn feed_url(&self) -> String {
        match &self.feed_override {
            Some(url) => url.trim_end_matches('/').to_owned(),
            None => {
                let url = self.installed().channel.feed_url();
                url.trim_end_matches(MANIFEST_FILE).trim_end_matches('/').to_owned()
            }
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Status> {
        self.tx.subscribe()
    }

    pub fn status(&self) -> Status {
        self.tx.borrow().clone()
    }

    pub(super) fn set_phase(&self, phase: Phase) {
        self.tx.send_modify(|s| s.phase = phase);
    }

    pub(super) fn update_prefs(&self, f: impl FnOnce(&mut Prefs)) {
        self.tx.send_modify(|s| {
            f(&mut s.prefs);
            s.channel = self.info.channel(s.prefs.channel);
            if let Err(e) = save_prefs(&self.dir, &s.prefs) {
                report!("update prefs: {e}");
            }
        });
    }

    /// The background loop's step: check when due, download in Auto mode,
    /// then announce what it found.
    pub(super) fn tick(&'static self) {
        self.step(CHECK_EVERY, crate::platform::network::metered());
        self.send_notice(crate::platform::update_notice::post);
    }

    /// Checks when the last check is `every` old, or starts the download an
    /// earlier check left waiting for Wi-Fi.
    pub(super) fn step(&'static self, every: Duration, metered: bool) {
        let s = self.status();
        if s.off.is_some() || s.prefs.mode == Mode::Off {
            return;
        }
        let due =
            s.prefs.last_check_ms.is_none_or(|t| now_ms().saturating_sub(t) >= every.as_millis() as u64);
        // Automatic downloads wait for Wi-Fi (an APK on mobile data, up to
        // every push on the preview channel); the offer still shows.
        let wifi = !metered;
        let auto = s.prefs.mode == Mode::Auto && wifi;
        if due {
            self.check_with(auto, wifi);
        } else if auto && let Phase::Available(_) = s.phase {
            self.download();
        }
    }

    /// Sends the notification due now through `post` (false: not shown, try
    /// again at the next check) and remembers it once shown.
    pub(super) fn send_notice(&self, post: impl FnOnce(&Notice) -> bool) -> Option<Notice> {
        let n = notice(&self.status())?;
        if !post(&n) {
            return None;
        }
        self.update_prefs(|p| p.notified = Some(n.key.clone()));
        Some(n)
    }

    /// Android's background job: the app loop's step, then the notification
    /// to post if `can_notify` (notifications allowed, Steps not in front).
    /// Not marked as sent: a notification Android doesn't show is tried
    /// again at the next run ([`Self::shown`] marks it).
    pub(super) fn background(&'static self, metered: bool, can_notify: bool) -> Option<Notice> {
        self.step(BACKGROUND_EVERY, metered);
        if !can_notify {
            return None;
        }
        notice(&self.status())
    }

    /// The notification `key` was shown.
    pub(super) fn shown(&self, key: &str) {
        if self.status().prefs.notified.as_deref() != Some(key) {
            self.update_prefs(|p| p.notified = Some(key.to_owned()));
        }
    }

    /// Whether Android's background job should run: updates work here and
    /// aren't switched off.
    pub fn background_checks(&self) -> bool {
        let s = self.status();
        s.off.is_none() && s.prefs.mode != Mode::Off
    }

    pub fn set_notify(&self, on: bool) {
        self.update_prefs(|p| p.notify = on);
        if !on {
            crate::platform::update_notice::clear();
        }
    }

    /// "Check now": looks at the feed, and fetches a newer content pack.
    /// With `download`, fetches a newer build right away too.
    pub fn check(&'static self, download: bool) {
        self.check_with(download, true);
    }

    pub(super) fn check_with(&self, download: bool, content: bool) {
        let Ok(_job) = self.job.try_lock() else { return };
        if self.status().off.is_some() {
            return;
        }
        let keep = match self.status().phase {
            // A finished download for the same offer stays; it's re-checked anyway.
            Phase::Ready(o) | Phase::Installed(o) | Phase::NeedsPermission(o) => Some(o),
            _ => None,
        };
        self.set_phase(Phase::Checking);
        let manifest = self.fetch_manifest();
        self.update_prefs(|p| p.last_check_ms = Some(now_ms()));
        let m = match manifest {
            Ok(m) => m,
            Err(why) => {
                if content {
                    self.tx.send_modify(|s| s.content_check = Some(ContentCheck::Failed));
                }
                return self.set_phase(Phase::Failed { offer: None, why });
            }
        };
        self.remember_history(&m.history);
        if content {
            let found = self.refresh_content(&m);
            self.tx.send_modify(|s| s.content_check = Some(found));
        }
        match self.offer(&m) {
            Ok(Some(offer)) => {
                if let Some(k) = keep.filter(|k| k.asset == offer.asset) {
                    self.set_phase(match self.status().phase {
                        Phase::Checking => Phase::Ready(k),
                        other => other,
                    });
                    return;
                }
                self.set_phase(Phase::Available(offer.clone()));
                if download {
                    self.download_locked(offer);
                }
            }
            Ok(None) => {}
            Err(why) => self.set_phase(Phase::Failed { offer: None, why }),
        }
    }

    /// The channel's `update.json`, signature checked.
    pub(super) fn fetch_manifest(&self) -> Result<UpdateManifest, Failure> {
        let key = self.info.key.ok_or(Failure::Verify("no key".into()))?;
        let base = self.feed_url();
        let fetch = || -> Result<_, Failure> {
            let json = get_small(&format!("{base}/{MANIFEST_FILE}"))?;
            let sig = get_small(&format!("{base}/{SIGNATURE_FILE}"))?;
            let sig = String::from_utf8(sig).map_err(|_| Failure::Verify("signature".into()))?;
            Ok(verify_manifest(&json, &sig, &key))
        };
        // A feed caught mid-upload pairs a new signature with the old
        // manifest: look once more before calling it a failure.
        let m = match fetch()? {
            Err(UpdateError::NotSigned) => {
                std::thread::sleep(self.retry_after);
                fetch()?
            }
            first => first,
        };
        m.map_err(|e| match e {
            // A preview whose CI key was renewed can't verify any newer feed.
            UpdateError::NotSigned if self.info.preview => Failure::KeyChanged,
            other => update_failure(other),
        })
    }

    /// `Ok(None)` after setting UpToDate / NotForThisDevice.
    pub(super) fn offer(&self, m: &UpdateManifest) -> Result<Option<Offer>, Failure> {
        let platform = self.platform.ok_or(Failure::Verify("no platform".into()))?;
        match decide(&self.installed(), m, platform).map_err(update_failure)? {
            Decision::UpToDate => {
                self.set_phase(Phase::UpToDate);
                self.clear_downloads(None);
                Ok(None)
            }
            Decision::NotForThisPlatform => {
                self.set_phase(Phase::NotForThisDevice);
                Ok(None)
            }
            Decision::Available(asset) => {
                if asset.size > MAX_BUILD_BYTES {
                    return Err(Failure::Verify("build too large".into()));
                }
                Ok(Some(Offer::from_manifest(m, asset, &self.installed())))
            }
        }
    }

    /// Downloads the feed's content pack when it differs from the one in
    /// use, checks it decodes, and keeps it for the next start. Failures
    /// are only reported: the pack built into the app keeps working.
    pub(super) fn refresh_content(&self, m: &UpdateManifest) -> ContentCheck {
        let current = load_pack_info(&self.dir).and_then(|i| i.origin());
        let asset = match content_update(&self.installed(), m, current.as_ref()) {
            Ok(Some(a)) => a.clone(),
            Ok(None) => return ContentCheck::Current,
            Err(e) => {
                report!("content pack: {e}");
                return ContentCheck::Failed;
            }
        };
        // Steps runs on its built-in content and the feed's pack is that same
        // content (the build that made this app): nothing to download.
        if built_in(
            option_env!("STEPS_BUILTIN_CONTENT"),
            pack_in_use(&self.dir, &self.info).is_some(),
            &asset.sha256,
        ) {
            return ContentCheck::Current;
        }
        if asset.size > MAX_PACK_BYTES {
            report!("content pack: {} bytes is too large", asset.size);
            return ContentCheck::Failed;
        }
        // A pack refused before stays refused: what is in use is the newest
        // there is for this device. One this app build couldn't take waits
        // for an app update.
        if is_rejected(&self.dir, &asset.sha256) || is_unreadable(&self.dir, &self.info, &asset.sha256) {
            return ContentCheck::Current;
        }
        // Err((true, _)): this build can't take the file, and it is
        // remembered so it isn't fetched again; Err((false, _)): try again
        // next time (a cut or mismatched download included).
        let result = (|| -> Result<(), (bool, String)> {
            let bytes = match self.held(&asset) {
                Some(bytes) => bytes,
                None => {
                    let url = format!("{}/{}", self.feed_url(), asset.file);
                    let bytes = get_limited(&url, asset.size, Duration::from_secs(10 * 60))
                        .map_err(|f| (false, format!("{f:?}")))?;
                    check_download(&asset, bytes.len() as u64, &sha256_hex(&bytes))
                        .map_err(|e| (false, e.to_string()))?;
                    bytes
                }
            };
            let pack = Pack::decode(&bytes).map_err(|e| (true, e.to_string()))?;
            if let Some(first) = pack.problems().first() {
                return Err((true, format!("content problem: {first}")));
            }
            if let Some(bad) = crate::look::unsafe_sheet(&pack.look) {
                return Err((true, format!("stylesheet {bad} isn't allowed")));
            }
            let io = |e: std::io::Error| (false, e.to_string());
            fs::create_dir_all(&self.dir).map_err(io)?;
            // The old pack's record goes first: stopped half-way, Steps finds
            // no record and uses its built-in pack, never a mismatched pair.
            match fs::remove_file(self.dir.join(PACK_INFO)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io(e)),
                _ => {}
            }
            let tmp = self.dir.join(format!("{PACK_FILE}.part"));
            fs::write(&tmp, &bytes).map_err(io)?;
            fs::rename(&tmp, self.dir.join(PACK_FILE)).map_err(io)?;
            let _ = fs::remove_file(self.dir.join(PACK_STARTS));
            let info = PackInfo {
                version: m.version.clone(),
                build: m.build,
                sha256: asset.sha256.clone(),
                published_at: published_secs(&m.published_at),
                ui_build: m.ui_build,
            };
            write_json(&self.dir, PACK_INFO, &info).map_err(|e| (false, e))
        })();
        match result {
            Ok(()) => {
                self.tx.send_modify(|s| s.content_waiting = true);
                ContentCheck::New
            }
            Err((bad, e)) => {
                if bad {
                    mark_unreadable(&self.dir, &self.info, &asset.sha256);
                }
                report!("content pack {} not taken: {e}", asset.file);
                ContentCheck::Failed
            }
        }
    }

    /// The feed's pack when this device already keeps exactly those bytes
    /// as its last good pack (a feed that went back to it). A feed naming
    /// the pack in use is never fetched at all (`content_update`).
    fn held(&self, asset: &UpdateAsset) -> Option<Vec<u8>> {
        let bytes = fs::read(recovery::good_file(&self.dir, &asset.sha256)?).ok()?;
        check_download(asset, bytes.len() as u64, &sha256_hex(&bytes)).is_ok().then_some(bytes)
    }

    /// Keeps a verified feed's history, and lets a pending "What's new" go
    /// when the feed has nothing to say about this build.
    pub(super) fn remember_history(&self, fresh: &[BuildChanges]) {
        let known = self.status().history;
        if let Some(all) = changelog::merge(&known, fresh) {
            if let Err(e) = write_json(&self.dir, changelog::FILE, &all) {
                report!("update history: {e}");
            }
            self.tx.send_modify(|s| s.history = all);
        }
        let s = self.status();
        if s.news_pending() && s.whats_new().is_empty() {
            self.close_whats_new();
        }
    }

    /// "What's new" was seen (closed).
    pub fn close_whats_new(&self) {
        if self.status().news_pending() {
            self.update_prefs(|p| {
                p.whats_new_from = None;
                p.content_from = None;
            });
        }
    }

    /// Opens a waiting content pack now (he is between sessions): the
    /// library and the stylesheets switch over, and "What's new" says what
    /// came. Returns whether anything changed; screens must then be shown
    /// again.
    pub fn apply_content(&self) -> bool {
        if !self.status().content_waiting {
            return false;
        }
        let Some(pack) = pack_in_use(&self.dir, &self.info) else { return false };
        crate::content::Library::reload();
        let s = self.status();
        let before = s.content_key().unwrap_or_else(|| s.app_key());
        self.tx.send_modify(|s| {
            s.content_waiting = false;
            s.content_build = pack.build.max(s.content_build);
            s.content_at = pack.published_at.or(s.content_at);
        });
        let now = self.status().content_key();
        if now.as_ref() != Some(&before) {
            self.update_prefs(|p| {
                p.content_from = Some(before);
                p.last_content = now;
            });
        }
        true
    }

    /// Downloads the offered build (a tap in "tell me first" mode).
    pub fn download(&'static self) {
        let Ok(_job) = self.job.try_lock() else { return };
        if let Phase::Available(offer) | Phase::Failed { offer: Some(offer), .. } = self.status().phase {
            self.download_locked(offer);
        }
    }

    pub(super) fn download_locked(&self, offer: Offer) {
        let base = self.feed_url();
        let _ = fs::create_dir_all(&self.dir);
        self.clear_downloads(Some(&offer.asset.file));
        let path = self.dir.join(&offer.asset.file);
        self.set_phase(Phase::Downloading { offer: offer.clone(), done: 0 });
        match self.fetch_build(&format!("{base}/{}", offer.asset.file), &offer, &path) {
            Ok(()) => {
                if let Err(e) = save_staged(&self.dir, &offer) {
                    report!("update staged.json: {e}");
                }
                self.set_phase(Phase::Ready(offer));
            }
            Err(why) => {
                // The `.part` stays, so the next try picks up from there.
                let _ = fs::remove_file(&path);
                self.set_phase(Phase::Failed { offer: Some(offer), why });
            }
        }
    }

    /// Streams the build to `<file>.part` while hashing it, checks size and
    /// SHA-256 against the signed manifest, then renames it into place.
    ///
    /// A download that stops (the network drops, Steps closes) keeps its
    /// `.part`, and the next try asks only for the rest (an HTTP `Range`),
    /// within this call (a few tries, a little apart) and on the next check
    /// or tap. The whole file is still checked at the end; a resumed file
    /// that doesn't match is thrown away and fetched once more from the
    /// start.
    pub(super) fn fetch_build(&self, url: &str, offer: &Offer, path: &Path) -> Result<(), Failure> {
        let part = part_of(path);
        let meta = meta_of(path);
        let expected = PartMeta { size: offer.asset.size, sha256: offer.asset.sha256.clone() };
        // A .part from another signed build must never be resumed, even if
        // that build used the same filename and byte count.
        if read_part_meta(&meta).as_ref() != Some(&expected) {
            let _ = fs::remove_file(&part);
        }
        fs::write(&meta, serde_json::to_vec(&expected).map_err(|e| Failure::Install(e.to_string()))?)
            .map_err(|e| Failure::Install(e.to_string()))?;
        let mut fresh_after_bad = false;
        let mut tries = 0;
        loop {
            let before = fs::metadata(&part).map_or(0, |m| m.len());
            match self.fetch_some(url, offer, &part) {
                Ok(()) => {}
                // It got somewhere before the line dropped: go on from there.
                Err(Failure::Network)
                    if tries < RESUME_TRIES && fs::metadata(&part).map_or(0, |m| m.len()) > before =>
                {
                    tries += 1;
                    if !cfg!(test) {
                        std::thread::sleep(Duration::from_secs(2u64 << tries));
                    }
                    continue;
                }
                Err(e) => return Err(e),
            }
            let (size, sha) = hash_file(&part).map_err(|e| Failure::Install(e.to_string()))?;
            match check_download(&offer.asset, size, &sha) {
                Ok(()) => {
                    fs::rename(&part, path).map_err(|e| Failure::Install(e.to_string()))?;
                    let _ = fs::remove_file(&meta);
                    return Ok(());
                }
                Err(e) => {
                    let _ = fs::remove_file(&part);
                    if fresh_after_bad {
                        return Err(update_failure(e));
                    }
                    fresh_after_bad = true;
                }
            }
        }
    }

    /// The web build never downloads builds (each deploy is the newest).
    #[cfg(target_arch = "wasm32")]
    pub(super) fn fetch_some(&self, _url: &str, _offer: &Offer, _part: &Path) -> Result<(), Failure> {
        Err(Failure::Network)
    }

    /// Appends what `part` still lacks (all of it when it's empty).
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn fetch_some(&self, url: &str, offer: &Offer, part: &Path) -> Result<(), Failure> {
        let size = offer.asset.size;
        let mut have = fs::metadata(part).map_or(0, |m| m.len());
        if have > size {
            let _ = fs::remove_file(part);
            have = 0;
        }
        if have == size {
            return Ok(());
        }
        let mut req = agent(url, Duration::from_secs(30 * 60)).get(url);
        if have > 0 {
            req = req.header("Range", format!("bytes={have}-"));
        }
        let mut res = req.call().map_err(|_| Failure::Network)?;
        let mut file = match res.status().as_u16() {
            206 if have > 0 && range_start(&res) == Some(have) => {
                fs::OpenOptions::new().append(true).open(part)
            }
            // The server sent all of it: start over.
            200 => fs::File::create(part),
            // The part is no use to the server (changed file): start over.
            416 if have > 0 => {
                let _ = fs::remove_file(part);
                return self.fetch_some(url, offer, part);
            }
            _ => return Err(Failure::Network),
        }
        .map_err(|e| Failure::Install(e.to_string()))?;
        if res.status().as_u16() == 200 {
            have = 0;
        }
        let mut reader = res.body_mut().as_reader().take(size - have);
        let (mut done, mut shown) = (have, have);
        self.set_phase(Phase::Downloading { offer: offer.clone(), done });
        let mut buf = vec![0u8; 64 * 1024];
        let result = loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break Ok(()),
                Ok(n) => n,
                Err(_) => break Err(Failure::Network),
            };
            if let Err(e) = file.write_all(&buf[..n]) {
                break Err(Failure::Install(e.to_string()));
            }
            done += n as u64;
            if done - shown > size / 100 {
                shown = done;
                self.set_phase(Phase::Downloading { offer: offer.clone(), done });
            }
        };
        // What came is kept for the next try, even when the line dropped.
        file.sync_all().map_err(|e| Failure::Install(e.to_string()))?;
        result?;
        if done < size { Err(Failure::Network) } else { Ok(()) }
    }

    /// Re-hashes the staged file: whatever sat on disk since the download
    /// is checked again before it's installed.
    pub(super) fn verified_staged(&self, offer: &Offer) -> Result<PathBuf, Failure> {
        let path = self.dir.join(&offer.asset.file);
        let mut file = fs::File::open(&path).map_err(|_| Failure::Verify("download missing".into()))?;
        let mut hash = Sha256::new();
        let size = std::io::copy(&mut file, &mut hash).map_err(|e| Failure::Verify(e.to_string()))?;
        check_download(&offer.asset, size, &hex(&hash.finalize())).map_err(update_failure)?;
        Ok(path)
    }

    /// Installs the ready download (a tap, or a calm moment in Auto mode).
    pub fn install(&'static self) {
        let Ok(_job) = self.job.try_lock() else { return };
        let offer = match self.status().phase {
            Phase::Ready(o)
            | Phase::NeedsPermission(o)
            | Phase::Failed { offer: Some(o), why: Failure::Install(_) } => o,
            _ => return,
        };
        let path = match self.verified_staged(&offer) {
            Ok(p) => p,
            Err(why) => {
                self.clear_downloads(None);
                self.set_phase(Phase::Failed { offer: Some(offer), why });
                return;
            }
        };
        self.set_phase(Phase::Installing(offer.clone()));
        let next = install_build(&path, &offer);
        if let Phase::Installed(_) = next {
            self.clear_downloads(None);
        }
        self.set_phase(next);
    }

    pub fn set_mode(&self, mode: Mode) {
        self.update_prefs(|p| p.mode = mode);
        crate::platform::update_notice::schedule(self.background_checks());
    }

    /// Switches channel (preview, alpha, beta, stable) and looks again.
    pub fn set_channel(&'static self, channel: Channel) {
        if self.status().channel == channel {
            return;
        }
        self.update_prefs(|p| p.channel = Some(channel));
        self.clear_downloads(None);
        self.set_phase(Phase::Idle);
        let auto = self.status().prefs.mode == Mode::Auto;
        std::thread::spawn(move || self.check(auto));
    }

    pub fn dismiss(&self) {
        let key = self.status().banner().map(Offer::key);
        self.update_prefs(|p| p.dismissed = key);
        crate::platform::update_notice::clear();
    }

    /// Android: polls PackageInstaller after an install started.
    #[cfg(target_os = "android")]
    pub fn poll_install(&self) {
        use crate::platform::install::{InstallState, install_state};
        let Phase::Installing(offer) = self.status().phase else { return };
        match install_state() {
            // A success ends this process: the new Steps starts in its place.
            Ok(InstallState::Working | InstallState::WaitingUser) => {}
            Ok(InstallState::Idle | InstallState::Cancelled) => self.set_phase(Phase::Ready(offer)),
            Ok(InstallState::Failed(why)) | Err(why) => {
                self.update_prefs(|p| p.auto_failed = Some(offer.key()));
                self.set_phase(Phase::Failed { offer: Some(offer), why: install_failure(why) });
            }
        }
    }

    /// Removes every app download except `keep`, and the staged record
    /// unless kept. Prefs, the changelog and every content file stay: the
    /// pack in use, its record, the crash-loop guard and the last good pack
    /// (`recovery`), all named `content.*`.
    pub(super) fn clear_downloads(&self, keep: Option<&str>) {
        let Ok(entries) = fs::read_dir(&self.dir) else { return };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), "prefs.json" | changelog::FILE)
                || name.starts_with("content.")
                || Some(name.as_str()) == keep
                || keep.is_some_and(|k| name == format!("{k}.part") || name == format!("{k}.part.json"))
            {
                continue;
            }
            if name == "staged.json" && keep.is_some() {
                continue;
            }
            let _ = fs::remove_file(e.path());
        }
    }
}
