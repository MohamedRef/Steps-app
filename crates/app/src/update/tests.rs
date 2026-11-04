//! Existing behavior regressions for the feature facade.

use std::collections::BTreeMap;

use super::*;

/// The bytes behind the signed fixture's assets (no .apk is tracked).
const FIXTURE_APK: &[u8] = b"fixture apk bytes";
const FIXTURE_PACK: &[u8] = b"fixture pack bytes";

fn offer(version: &str, build: u64) -> Offer {
    Offer {
        version: version.into(),
        build,
        commit: None,
        notes: UpdateNotes::default(),
        asset: UpdateAsset {
            platform: UpdatePlatform::AndroidArm64,
            file: "steps.apk".into(),
            size: 1,
            sha256: "00".repeat(32),
            version_code: None,
            build: None,
        },
        changes: Vec::new(),
    }
}

fn status(phase: Phase, mode: Mode) -> Status {
    Status {
        off: None,
        phase,
        prefs: Prefs { mode, ..Prefs::default() },
        channel: Channel::Alpha,
        preview: false,
        content_waiting: false,
        version: "0.1.0".into(),
        build: 41,
        content_build: 0,
        content_at: None,
        content_check: None,
        history: Vec::new(),
    }
}

#[test]
fn the_settings_list_says_where_updates_stand() {
    use crate::i18n::update as tx;
    let mut s = status(Phase::UpToDate, Mode::Auto);
    assert_eq!(view::summary(&s), None, "never looked: the section's own line");
    s.prefs.last_check_ms = Some(now_ms());
    assert!(view::summary(&s).unwrap().starts_with(tx::LAST_CHECK));
    s.content_waiting = true;
    assert_eq!(view::summary(&s).as_deref(), Some(tx::SUMMARY_CONTENT_WAITING));
    s.phase = Phase::Checking;
    assert_eq!(view::summary(&s).as_deref(), Some(tx::CHECKING));
    s.off = Some(Off::DevBuild);
    assert_eq!(view::summary(&s), None);
}

fn build(version: &str, build: u64, preview: bool) -> BuildInfo {
    let key = include_str!("../../../../fixtures/update/test-key.pub");
    BuildInfo {
        version: Version::parse(version).unwrap(),
        build,
        preview,
        key: Some(parse_public_key(key).unwrap()),
    }
}

#[test]
fn app_and_content_notes_count_apart() {
    let entry = |build: u64, content_only: bool| BuildChanges {
        version: "0.1.0".into(),
        build,
        published_at: String::new(),
        changes: vec![steps_core::update::Change {
            kind: steps_core::update::ChangeKind::New,
            text: UpdateNotes { en: String::new(), ar_eg: format!("بناء {build}") },
        }],
        content_only,
    };
    // APK 38 runs content 45; the feed's app is 40 (#348).
    let mut s = status(Phase::UpToDate, Mode::Auto);
    s.history = vec![entry(45, true), entry(44, true), entry(40, false), entry(39, false)];
    (s.build, s.content_build) = (38, 45);
    s.prefs.content_from = Some("0.1.0+38".into());
    let keys = |s: &Status| s.whats_new().iter().map(BuildChanges::key).collect::<Vec<_>>();
    assert_eq!(keys(&s), ["0.1.0+45", "0.1.0+44"], "no notes for an app he hasn't installed");
    assert!(s.content_only_news());
    // He installs APK 40 later: its notes open, content seen before stays seen.
    s.prefs = Prefs { whats_new_from: Some("0.1.0+38".into()), ..Prefs::default() };
    s.build = 40;
    assert_eq!(keys(&s), ["0.1.0+40", "0.1.0+39"]);
    assert!(!s.content_only_news());
}

#[test]
fn prefs_round_trip_and_default_to_automatic_alpha() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(load_prefs(dir.path()), Prefs::default());
    assert_eq!(Prefs::default().mode, Mode::Auto);
    let p = Prefs {
        mode: Mode::Notify,
        channel: Some(Channel::Beta),
        last_check_ms: Some(5),
        ..Prefs::default()
    };
    save_prefs(dir.path(), &p).unwrap();
    assert_eq!(load_prefs(dir.path()), p);
    let release = BuildInfo { key: None, ..build("1.0.0", 0, false) };
    assert_eq!(release.channel(None), Channel::Alpha);
    assert_eq!(release.channel(Some(Channel::Beta)), Channel::Beta);
    // Both kinds are signed with the release keys, so a release build may
    // follow the preview feed and a preview build may go back to alpha.
    assert_eq!(release.channel(Some(Channel::Preview)), Channel::Preview);
    let preview = BuildInfo { preview: true, ..release };
    // Every build follows alpha unless Settings picks another channel.
    assert_eq!(preview.channel(None), Channel::Alpha);
    assert_eq!(preview.channel(Some(Channel::Alpha)), Channel::Alpha);
}

#[test]
fn the_banner_waits_for_something_to_do_and_can_be_closed() {
    let o = offer("1.0.0-alpha.2", 0);
    assert!(
        status(Phase::Available(o.clone()), Mode::Auto).banner().is_none(),
        "auto mode is still downloading"
    );
    assert!(status(Phase::Available(o.clone()), Mode::Notify).banner().is_some());
    assert!(status(Phase::Ready(o.clone()), Mode::Auto).banner().is_some());
    let mut s = status(Phase::Ready(o.clone()), Mode::Auto);
    s.prefs.dismissed = Some(o.key());
    assert!(s.banner().is_none());
    // A newer offer shows again.
    s.phase = Phase::Ready(offer("1.0.0-alpha.3", 0));
    assert!(s.banner().is_some());
}

#[test]
fn offers_label_previews_by_build() {
    assert_eq!(offer("1.0.0-alpha.2", 0).label(), "1.0.0-alpha.2");
    assert_eq!(offer("0.1.0", 141).label(), "0.1.0 (141)");
    let me = Installed { version: Version::parse("0.1.0").unwrap(), build: 140, channel: Channel::Preview };
    assert!(is_newer(&me, &offer("0.1.0", 141)));
    assert!(!is_newer(&me, &offer("0.1.0", 140)));
}

#[test]
fn staged_offer_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_staged(dir.path()).is_none());
    let o = offer("1.0.0-alpha.2", 0);
    save_staged(dir.path(), &o).unwrap();
    assert_eq!(load_staged(dir.path()), Some(o));
}

/// Serves `dir` over plain HTTP on localhost, one request per connection.
fn serve(dir: PathBuf) -> String {
    serve_logged(dir, None).0
}

/// Requests seen: (path, Range header).
type Seen = std::sync::Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A tiny file server that honours `Range: bytes=N-`. With `cut`, the
/// first whole-file answer for an `.apk` stops after that many bytes.
fn serve_logged(dir: PathBuf, cut: Option<usize>) -> (String, Seen) {
    use std::io::{BufRead as _, BufReader};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Default::default();
    let log = seen.clone();
    std::thread::spawn(move || {
        let mut cut = cut;
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_owned();
            let mut range = None;
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = h.split_once(':')
                    && k.eq_ignore_ascii_case("range")
                {
                    range = Some(v.trim().to_owned());
                }
            }
            log.lock().unwrap().push((path.clone(), range.clone()));
            let mut stream = reader.into_inner();
            let from = range
                .as_deref()
                .and_then(|r| r.strip_prefix("bytes="))
                .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok());
            let _ = match fs::read(dir.join(&path)) {
                Ok(body) => match from {
                    Some(n) if n >= body.len() => stream.write_all(
                        b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    ),
                    Some(n) => {
                        let head = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {n}-{}/{}\r\nConnection: close\r\n\r\n",
                            body.len() - n,
                            body.len() - 1,
                            body.len()
                        );
                        stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&body[n..]))
                    }
                    None => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let send = match cut.filter(|_| path.ends_with(".apk")) {
                            Some(c) => {
                                cut = None;
                                &body[..c]
                            }
                            None => &body[..],
                        };
                        stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(send))
                    }
                },
                Err(_) => stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
            };
        }
    });
    (url, seen)
}

#[test]
fn a_dropped_download_goes_on_from_where_it_stopped() {
    let feed = fixture_feed();
    let (url, seen) = serve_logged(feed.path().to_path_buf(), Some(8));
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    u.check(true);
    assert!(matches!(u.status().phase, Phase::Ready(_)), "{:?}", u.status().phase);
    let apk: Vec<_> = seen.lock().unwrap().iter().filter(|(p, _)| p.ends_with(".apk")).cloned().collect();
    assert_eq!(apk.iter().map(|(_, r)| r.as_deref()).collect::<Vec<_>>(), [None, Some("bytes=8-")]);
}

#[test]
fn a_part_left_from_before_is_resumed_and_a_bad_one_fetched_again() {
    let feed = fixture_feed();
    let (url, seen) = serve_logged(feed.path().to_path_buf(), None);
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    let file = "batna-0.1.0-b42-android-arm64.apk";
    fs::create_dir_all(&u.dir).unwrap();
    // Steps closed half-way last time.
    fs::write(u.dir.join(format!("{file}.part")), &FIXTURE_APK[..5]).unwrap();
    let expected = PartMeta { size: FIXTURE_APK.len() as u64, sha256: sha256_hex(FIXTURE_APK) };
    fs::write(u.dir.join(format!("{file}.part.json")), serde_json::to_vec(&expected).unwrap()).unwrap();
    u.check(true);
    assert!(matches!(u.status().phase, Phase::Ready(_)), "{:?}", u.status().phase);
    let ranges = |seen: &Seen| -> Vec<Option<String>> {
        seen.lock().unwrap().iter().filter(|(p, _)| p.ends_with(".apk")).map(|(_, r)| r.clone()).collect()
    };
    assert_eq!(ranges(&seen), [Some("bytes=5-".to_owned())]);

    // A part that doesn't match: the whole file is checked, so it is
    // fetched once more from the start.
    let feed = fixture_feed();
    let (url, seen) = serve_logged(feed.path().to_path_buf(), None);
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    fs::create_dir_all(&u.dir).unwrap();
    fs::write(u.dir.join(format!("{file}.part")), b"XXXXX").unwrap();
    fs::write(u.dir.join(format!("{file}.part.json")), serde_json::to_vec(&expected).unwrap()).unwrap();
    u.check(true);
    assert!(matches!(u.status().phase, Phase::Ready(_)), "{:?}", u.status().phase);
    assert_eq!(ranges(&seen), [Some("bytes=5-".to_owned()), None]);
}

fn fixture_feed() -> tempfile::TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/update");
    let feed = tempfile::tempdir().unwrap();
    for name in [MANIFEST_FILE, SIGNATURE_FILE] {
        fs::copy(src.join(name), feed.path().join(name)).unwrap();
    }
    fs::write(feed.path().join("batna-0.1.0-b42-android-arm64.apk"), FIXTURE_APK).unwrap();
    fs::write(feed.path().join("batna-content-0.1.0-b42-content.pack"), FIXTURE_PACK).unwrap();
    feed
}

fn updater(feed: &str, info: BuildInfo) -> (&'static Updater, tempfile::TempDir) {
    let data = tempfile::tempdir().unwrap();
    (updater_in(data.path(), feed, info), data)
}

fn updater_in(data: &Path, feed: &str, info: BuildInfo) -> &'static Updater {
    // The fixture feed is signed for the preview channel. Every build
    // follows alpha until Settings picks another, so a preview build in
    // these tests starts as if he had picked preview.
    let dir = data.join("updates");
    if info.preview && !dir.join("prefs.json").exists() {
        fs::create_dir_all(&dir).unwrap();
        let prefs = Prefs { channel: Some(Channel::Preview), ..Prefs::default() };
        fs::write(dir.join("prefs.json"), serde_json::to_vec(&prefs).unwrap()).unwrap();
    }
    let mut u = Updater::new(
        data.join("updates"),
        info,
        Some(UpdatePlatform::AndroidArm64),
        Some(feed.to_owned()),
        None,
    );
    u.retry_after = Duration::from_millis(10);
    Box::leak(Box::new(u))
}

#[test]
fn a_renewed_signing_key_is_its_own_failure() {
    // UpdateInstaller.check's text (android/…/UpdateInstaller.kt).
    let why = "java.lang.IllegalArgumentException: the APK isn't signed with this Steps's key";
    assert_eq!(install_failure(why.into()), Failure::KeyChanged);
    assert_eq!(install_failure("disk full".into()), Failure::Install("disk full".into()));
}

#[test]
fn whats_new_before_and_after_an_update() {
    use crate::i18n::update as tx;
    let feed = fixture_feed();
    let url = serve(feed.path().to_path_buf());
    let data = tempfile::tempdir().unwrap();

    // A first install says nothing about what's new.
    let u = updater_in(data.path(), &url, build("0.1.0", 39, true));
    assert!(u.status().whats_new().is_empty());
    assert_eq!(u.status().prefs.last_run.as_deref(), Some("0.1.0+39"));

    // Before installing: the offer brings the builds after this one.
    u.check(false);
    let Phase::Available(o) = u.status().phase else { panic!("{:?}", u.status().phase) };
    assert_eq!(o.changes.iter().map(BuildChanges::key).collect::<Vec<_>>(), ["0.1.0+42", "0.1.0+40"]);
    assert_eq!(u.status().history.len(), 2, "the feed's history is kept");
    let mut s = u.status();
    s.prefs.mode = Mode::Notify;
    let n = notice(&s).unwrap();
    assert!(n.body.starts_with("• "), "{}", n.body);
    assert!(!n.body.starts_with(tx::WHATS_NEW), "{}", n.body);

    // The first launch of the new build opens "What's new" once.
    let u = updater_in(data.path(), &url, build("0.1.0", 42, true));
    let w = u.status().whats_new();
    assert_eq!(w.iter().map(BuildChanges::key).collect::<Vec<_>>(), ["0.1.0+42", "0.1.0+40"]);
    u.close_whats_new();
    assert!(u.status().whats_new().is_empty());
    let u = updater_in(data.path(), &url, build("0.1.0", 42, true));
    assert!(u.status().whats_new().is_empty(), "seen stays seen");
}

#[test]
fn checks_downloads_and_verifies_from_a_signed_feed() {
    let feed = fixture_feed();
    let url = serve(feed.path().to_path_buf());

    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    u.check(true);
    let Phase::Ready(offer) = u.status().phase else { panic!("{:?}", u.status().phase) };
    assert_eq!(offer.label(), "0.1.0 (42)");
    assert_eq!(offer.asset.version_code, Some(100_999));
    assert!(u.verified_staged(&offer).is_ok());
    assert!(u.status().prefs.last_check_ms.is_some());
    // A fresh start finds the staged download again.
    let again = Updater::new(u.dir.clone(), u.info.clone(), u.platform, u.feed_override.clone(), None);
    assert_eq!(again.status().phase, Phase::Ready(offer.clone()));
    // Changed on disk after the download: refused at install time.
    fs::write(u.dir.join(&offer.asset.file), b"fixture apk bytez").unwrap();
    assert!(matches!(u.verified_staged(&offer), Err(Failure::Verify(_))));

    // Already on that build: nothing to do, and old downloads go.
    let (u, _data) = updater(&url, build("0.1.0", 42, true));
    u.check(true);
    assert_eq!(u.status().phase, Phase::UpToDate);
}

#[test]
fn a_notice_goes_out_once_per_offer_and_step() {
    use crate::i18n::update as tx;
    let mut o = offer("0.1.0", 42);
    o.notes.ar_eg = "محاضرة الاستسقاء اتضافت".into();
    let mut s = status(Phase::Available(o.clone()), Mode::Notify);
    let n = notice(&s).expect("a new version is announced");
    assert_eq!(n.title, format!("{} · 0.1.0 (42)", tx::NOTICE_AVAILABLE));
    assert!(n.body.starts_with(&format!("{}: محاضرة الاستسقاء اتضافت\n", tx::WHATS_NEW)), "{}", n.body);
    assert_eq!((n.action, n.button), ("download", tx::DOWNLOAD));
    s.prefs.notified = Some(n.key.clone());
    assert!(notice(&s).is_none(), "sent once");
    s.phase = Phase::Ready(o.clone());
    let ready = notice(&s).unwrap();
    assert_eq!(ready.action, "install");
    let mut closed = s.clone();
    closed.prefs.dismissed = Some(o.key());
    assert!(notice(&closed).is_none());
    let mut switch_off = s.clone();
    switch_off.prefs.notify = false;
    assert!(notice(&switch_off).is_none());
    let mut off = s.clone();
    off.prefs.mode = Mode::Off;
    assert!(notice(&off).is_none());
    assert!(notice(&status(Phase::UpToDate, Mode::Auto)).is_none());
    // Old prefs files keep the switch on.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("prefs.json"), r#"{"mode":"notify"}"#).unwrap();
    assert!(load_prefs(dir.path()).notify);
}

#[test]
fn the_background_check_downloads_on_wifi_and_announces_once() {
    let feed = fixture_feed();
    let url = serve(feed.path().to_path_buf());

    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    let n = u.background(false, true).expect("announced");
    assert_eq!(n.action, "install");
    assert!(matches!(u.status().phase, Phase::Ready(_)));
    // Not shown (Android said no): offered again at the next run.
    assert_eq!(u.background(false, true).as_ref(), Some(&n));
    u.shown(&n.key);
    assert!(u.background(false, true).is_none(), "once shown, never again");
    assert!(serde_json::to_string(&n).unwrap().contains(&format!("\"key\":\"{}\"", n.key)));

    // Mobile data: announced, not downloaded.
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    assert_eq!(u.background(true, true).unwrap().action, "download");
    assert!(matches!(u.status().phase, Phase::Available(_)));

    // Steps in front: kept for the next run.
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    assert!(u.background(true, false).is_none());
    assert!(u.background(true, true).is_some());
    assert_eq!(u.status().prefs.notified, None, "marked only once shown");

    let (u, _data) = updater(&url, build("0.1.0", 42, true));
    assert!(u.background(false, true).is_none());
}

#[test]
fn a_swapped_build_or_manifest_is_never_offered() {
    // The build on the server isn't the one the manifest signed.
    let feed = fixture_feed();
    fs::write(feed.path().join("batna-0.1.0-b42-android-arm64.apk"), b"evil apk bytes!!!").unwrap();
    let url = serve(feed.path().to_path_buf());
    let (u, data) = updater(&url, build("0.1.0", 41, true));
    u.check(true);
    assert!(
        matches!(u.status().phase, Phase::Failed { why: Failure::Verify(_), .. }),
        "{:?}",
        u.status().phase
    );
    let left: Vec<_> =
        fs::read_dir(data.path().join("updates")).unwrap().flatten().map(|e| e.file_name()).collect();
    assert!(
        !left.iter().any(|n| n.to_string_lossy().ends_with(".apk")),
        "nothing unverified stays: {left:?}"
    );

    // The manifest changed after signing: a preview calls it a renewed key.
    let feed = fixture_feed();
    let json = fs::read_to_string(feed.path().join(MANIFEST_FILE)).unwrap();
    fs::write(feed.path().join(MANIFEST_FILE), json.replace("\"build\": 42", "\"build\": 99")).unwrap();
    let url = serve(feed.path().to_path_buf());
    let (u, _data) = updater(&url, build("0.1.0", 41, true));
    u.check(true);
    assert!(matches!(u.status().phase, Phase::Failed { why: Failure::KeyChanged, .. }));

    // No connection.
    let (u, _data) = updater("http://127.0.0.1:9", build("0.1.0", 41, true));
    u.check(false);
    assert!(matches!(u.status().phase, Phase::Failed { why: Failure::Network, .. }));
    assert_eq!(u.status().content_check, Some(ContentCheck::Failed), "both halves say so");
}

/// A signed preview feed of `build` with a content pack (and the app,
/// carried over from `app_build`, when given).
fn pack_feed(
    signer: &ed25519_dalek::SigningKey,
    build: u64,
    app_build: Option<u64>,
    pack: &[u8],
    history: Vec<BuildChanges>,
) -> tempfile::TempDir {
    use base64::Engine as _;
    use ed25519_dalek::Signer as _;
    let feed = tempfile::tempdir().unwrap();
    let mut assets = vec![UpdateAsset {
        platform: UpdatePlatform::Content,
        file: "steps-content.pack".into(),
        size: pack.len() as u64,
        sha256: sha256_hex(pack),
        version_code: None,
        build: None,
    }];
    if let Some(b) = app_build {
        assets.push(UpdateAsset {
            platform: UpdatePlatform::AndroidArm64,
            file: format!("steps-0.1.0-b{b}-android-arm64.apk"),
            size: 9,
            sha256: "ab".repeat(32),
            version_code: Some(100_999),
            build: Some(b),
        });
    }
    let manifest = UpdateManifest {
        format: steps_core::update::UPDATE_FORMAT,
        channel: Channel::Preview,
        version: "0.1.0".into(),
        build,
        commit: None,
        published_at: "2026-10-27T08:00:00Z".into(),
        notes: UpdateNotes::default(),
        history,
        assets,
        ui_build: Some(app_build.unwrap_or(build)),
    };
    let json = serde_json::to_vec(&manifest).unwrap();
    fs::write(feed.path().join(MANIFEST_FILE), &json).unwrap();
    let sig = base64::engine::general_purpose::STANDARD.encode(signer.sign(&json).to_bytes());
    fs::write(feed.path().join(SIGNATURE_FILE), sig).unwrap();
    fs::write(feed.path().join("steps-content.pack"), pack).unwrap();
    feed
}

#[test]
fn a_content_only_update_needs_no_install_and_says_what_came() {
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[6; 32]);
    let came = BuildChanges {
        version: "0.1.0".into(),
        build: 45,
        published_at: "2026-10-27T08:00:00Z".into(),
        changes: vec![steps_core::update::Change {
            kind: steps_core::update::ChangeKind::New,
            text: UpdateNotes { en: String::new(), ar_eg: "أسئلة جديدة".into() },
        }],
        content_only: true,
    };
    let feed = pack_feed(&signer, 45, Some(40), &pack, vec![came]);
    let url = serve(feed.path().to_path_buf());
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 40, true) };
    let (u, data) = updater(&url, info.clone());
    assert_eq!(u.status().content_check, None, "nothing to say before a check");
    u.check(true);
    assert_eq!(u.status().phase, Phase::UpToDate, "the feed carries the app he runs");
    assert!(u.status().content_waiting);
    // Settings says the two halves apart: the app is current, content came.
    assert_eq!(u.status().content_check, Some(ContentCheck::New));
    assert!(u.apply_content());
    let s = u.status();
    assert_eq!((s.content_build, s.shown_build()), (45, 45));
    assert!(s.content_at.is_some(), "Settings dates the content in use");
    assert!(s.content_news(), "marked new in Settings until seen");
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::Current));
    let s = u.status();
    assert_eq!(s.whats_new().len(), 1, "what came opens once");
    assert!(s.whats_new()[0].content_only);
    // A quiet note on Today, not the sheet; the app build stays 40 and
    // the content is named apart (#336).
    assert!(s.content_only_news());
    assert_eq!((s.build, s.content_ahead()), (40, Some(45)));
    u.close_whats_new();
    assert!(!u.status().content_only_news());
    assert!(!u.status().content_news());

    // At the next start the pack is still in use: nothing new to say.
    let again = updater_in(data.path(), &url, info);
    assert_eq!(again.status().content_build, 45);
    assert!(again.status().whats_new().is_empty());
}

#[test]
fn a_content_pack_must_verify_and_decode_before_it_is_kept() {
    // The fixture's pack bytes match the manifest but aren't a real pack.
    let feed = fixture_feed();
    let url = serve(feed.path().to_path_buf());
    let (u, data) = updater(&url, build("0.1.0", 42, true));
    u.check(false);
    assert!(!u.status().content_waiting);
    assert_eq!(u.status().content_check, Some(ContentCheck::Failed));
    assert!(!data.path().join("updates").join(PACK_FILE).exists());
    // A bad file is remembered for this app build, so it isn't fetched
    // again (#344) until the app changes.
    let unreadable = fs::read_to_string(data.path().join("updates").join(PACK_UNREADABLE)).unwrap();
    assert!(unreadable.contains("0.1.0+42"));

    // A real pack that matches the signed manifest is kept for the next start.
    let look = BTreeMap::from([("assets/main.css".to_owned(), ".x {}".to_owned())]);
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).with_look(look).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[5; 32]);
    let feed = pack_feed(&signer, 42, None, &pack, Vec::new());
    let url = serve(feed.path().to_path_buf());
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 42, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    assert_eq!(u.status().phase, Phase::UpToDate, "the same build: only the pack is new");
    assert!(u.status().content_waiting);
    let dir = data.path().join("updates");
    let taken = installed_pack_in(&dir, &info).unwrap();
    assert_eq!(taken.lectures.len(), 0);
    assert_eq!(taken.look.len(), 1, "the build the look was made with uses it");
    let older = installed_pack_in(&dir, &BuildInfo { build: 40, ..info.clone() }).unwrap();
    assert!(older.look.is_empty(), "an older build takes the content but keeps its own look");
    assert!(u.apply_content());
    assert!(!u.status().content_waiting);
    assert!(!u.apply_content(), "once");
    // A later build drops a pack from an older one.
    assert!(installed_pack_in(&dir, &BuildInfo { build: 43, ..info }).is_none());
    assert!(!dir.join(PACK_FILE).exists());
}

#[test]
fn a_pack_that_keeps_stopping_steps_goes_and_stays_gone() {
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let feed = pack_feed(&signer, 42, None, &pack, Vec::new());
    let url = serve(feed.path().to_path_buf());
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 42, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    let dir = data.path().join("updates");
    // Three opens with no calm stretch after them (pack_settled)...
    for _ in 0..MAX_PACK_STARTS {
        assert!(installed_pack_in(&dir, &info).is_some());
    }
    // ...and the third start uses the built-in content.
    assert!(installed_pack_in(&dir, &info).is_none());
    assert!(!dir.join(PACK_FILE).exists());
    assert!(is_rejected(&dir, &sha256_hex(&pack)));
    // The feed still names it: it isn't taken again.
    u.check(false);
    assert!(!dir.join(PACK_FILE).exists());
}

#[test]
fn a_pack_with_a_stylesheet_that_reaches_out_is_refused() {
    let look =
        BTreeMap::from([("assets/main.css".to_owned(), "@import 'https://x.example/a.css';".to_owned())]);
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).with_look(look).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let feed = pack_feed(&signer, 42, None, &pack, Vec::new());
    let url = serve(feed.path().to_path_buf());
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 42, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    assert!(!u.status().content_waiting);
    let dir = data.path().join("updates");
    assert!(is_unreadable(&dir, &info, &sha256_hex(&pack)));
    assert!(!is_rejected(&dir, &sha256_hex(&pack)), "not a crash: a later app may take it");
}

#[test]
fn a_crashing_content_update_restores_the_last_settled_pack() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let app = build("0.1.0", 40, true);
    let make = |build: u64| {
        Pack::new(crate::content::Library::get().index.clone(), Vec::new())
            .with_look(BTreeMap::from([("assets/main.css".to_owned(), format!(".v{build} {{}}"))]))
            .encode()
    };
    let old = make(41);
    let newest = make(42);
    let install = |bytes: &[u8], build: u64| {
        fs::write(dir.join(PACK_FILE), bytes).unwrap();
        write_json(
            dir,
            PACK_INFO,
            &PackInfo {
                version: "0.1.0".into(),
                build,
                sha256: sha256_hex(bytes),
                published_at: None,
                ui_build: Some(40),
            },
        )
        .unwrap();
    };
    install(&old, 41);
    assert!(installed_pack_in(dir, &app).is_some());
    recovery::save_settled(dir, &app).unwrap();
    assert!(dir.join("content.good.json").exists());
    install(&newest, 42);
    for _ in 0..MAX_PACK_STARTS {
        assert!(installed_pack_in(dir, &app).is_some());
    }
    let recovered = installed_pack_in(dir, &app).expect("last settled pack");
    assert_eq!(recovered.look.get("assets/main.css").map(String::as_str), Some(".v41 {}"));
    assert!(is_rejected(dir, &sha256_hex(&newest)));
    assert_eq!(load_pack_info(dir).unwrap().build, 41);
    assert_eq!(fs::read(dir.join(PACK_FILE)).unwrap(), old);
}

#[test]
fn a_corrupted_or_incompatible_backup_falls_back_to_embedded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    let app = build("0.1.0", 40, true);
    let old = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    fs::write(path.join(PACK_FILE), &old).unwrap();
    write_json(
        path,
        PACK_INFO,
        &PackInfo {
            version: "0.1.0".into(),
            build: 41,
            sha256: sha256_hex(&old),
            published_at: None,
            ui_build: Some(40),
        },
    )
    .unwrap();
    fs::write(path.join(PACK_STARTS), "1").unwrap();
    recovery::save_settled(path, &app).unwrap();
    fs::write(path.join("content.good.json"), b"bad metadata").unwrap();
    fs::write(path.join(PACK_FILE), b"bad").unwrap();
    assert!(installed_pack_in(path, &app).is_none(), "corrupt metadata cannot restore");
    assert!(!path.join(PACK_FILE).exists());

    // A valid backup for an older app build must not overwrite newer
    // embedded lessons after an APK upgrade.
    write_json(
        path,
        "content.good.json",
        &PackInfo {
            version: "0.1.0".into(),
            build: 41,
            sha256: sha256_hex(&old),
            published_at: None,
            ui_build: Some(40),
        },
    )
    .unwrap();
    assert!(recovery::restore_last_good(path, &build("0.1.0", 42, true)).is_none());
}

#[test]
fn a_pack_with_broken_ids_or_references_is_refused_and_never_opened() {
    // Two lectures under the same id: a device must not take it.
    let index = crate::content::Library::get().index.clone();
    let lecture =
        crate::content::Library::get().lectures.values().next().cloned().expect("a written lecture");
    let pack = Pack::new(index.clone(), vec![lecture.clone(), lecture]).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
    let feed = pack_feed(&signer, 42, None, &pack, Vec::new());
    let url = serve(feed.path().to_path_buf());
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 42, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    assert!(!u.status().content_waiting);
    assert_eq!(u.status().content_check, Some(ContentCheck::Failed));
    let dir = data.path().join("updates");
    assert!(!dir.join(PACK_FILE).exists());
    assert!(is_unreadable(&dir, &info, &sha256_hex(&pack)), "not fetched again");

    // The same pack already on disk (from an older build without the
    // check) is dropped at the next start, and the built-in one is used.
    fs::write(dir.join(PACK_FILE), &pack).unwrap();
    let meta = PackInfo {
        version: "0.1.0".into(),
        build: 42,
        sha256: sha256_hex(&pack),
        published_at: None,
        ui_build: Some(42),
    };
    write_json(&dir, PACK_INFO, &meta).unwrap();
    assert!(installed_pack_in(&dir, &info).is_none());
    assert!(!dir.join(PACK_FILE).exists());
}

#[test]
fn clearing_app_downloads_keeps_the_last_good_content_pack() {
    let (u, data) = updater("http://127.0.0.1:9", build("0.1.0", 40, true));
    let dir = data.path().join("updates");
    fs::create_dir_all(&dir).unwrap();
    let app = build("0.1.0", 40, true);
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    fs::write(dir.join(PACK_FILE), &pack).unwrap();
    let meta = PackInfo {
        version: "0.1.0".into(),
        build: 41,
        sha256: sha256_hex(&pack),
        published_at: None,
        ui_build: Some(40),
    };
    write_json(&dir, PACK_INFO, &meta).unwrap();
    fs::write(dir.join(PACK_STARTS), "1").unwrap();
    recovery::save_settled(&dir, &app).unwrap();
    let good = format!("content.good.{}.pack", meta.sha256);
    assert!(dir.join(&good).exists());
    fs::write(dir.join("steps-0.1.0-b39-android-arm64.apk"), b"old app").unwrap();

    // What an "up to date" check, a finished install or a channel switch do.
    u.clear_downloads(None);

    assert!(!dir.join("steps-0.1.0-b39-android-arm64.apk").exists(), "app downloads go");
    for kept in [PACK_FILE, PACK_INFO, "content.good.json", good.as_str()] {
        assert!(dir.join(kept).exists(), "{kept} stays");
    }
    // So a crash loop on a later pack still has something to fall back to.
    fs::remove_file(dir.join(PACK_FILE)).unwrap();
    fs::remove_file(dir.join(PACK_INFO)).unwrap();
    assert!(recovery::restore_last_good(&dir, &app).is_some());
}

fn pack_requests(seen: &Seen) -> usize {
    seen.lock().unwrap().iter().filter(|(p, _)| p.ends_with(".pack")).count()
}

#[test]
fn a_feed_carrying_the_pack_already_here_downloads_nothing() {
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[11; 32]);
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 40, true) };
    let first = pack_feed(&signer, 45, Some(40), &pack, Vec::new());
    let (u, data) = updater(&serve(first.path().to_path_buf()), info.clone());
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::New));
    assert!(u.apply_content());

    // The next merge changed only the app: same pack, new build.
    let second = pack_feed(&signer, 46, Some(40), &pack, Vec::new());
    let (url, seen) = serve_logged(second.path().to_path_buf(), None);
    let u = updater_in(data.path(), &url, info);
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::Current), "nothing to reopen for");
    assert!(!u.status().content_waiting);
    assert_eq!(pack_requests(&seen), 0, "the pack wasn't downloaded again");
    let dir = data.path().join("updates");
    assert_eq!(fs::read(dir.join(PACK_FILE)).unwrap(), pack);
}

#[test]
fn a_pack_this_build_cannot_take_is_fetched_once_until_the_app_changes() {
    let look =
        BTreeMap::from([("assets/main.css".to_owned(), "@import 'https://x.example/a.css';".to_owned())]);
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).with_look(look).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[12; 32]);
    let feed = pack_feed(&signer, 45, Some(40), &pack, Vec::new());
    let (url, seen) = serve_logged(feed.path().to_path_buf(), None);
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 40, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::Failed));
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::Current));
    assert_eq!(pack_requests(&seen), 1, "remembered for this app build");
    // An app update gets its own try.
    let updated = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 41, true) };
    let u = updater_in(data.path(), &url, updated);
    u.check(false);
    assert_eq!(pack_requests(&seen), 2);
}

#[test]
fn a_mismatched_pack_download_is_tried_again() {
    let pack = Pack::new(crate::content::Library::get().index.clone(), Vec::new()).encode();
    let signer = ed25519_dalek::SigningKey::from_bytes(&[13; 32]);
    let feed = pack_feed(&signer, 45, Some(40), &pack, Vec::new());
    // What arrives isn't what was signed (a cut or swapped download).
    fs::write(feed.path().join("steps-content.pack"), &pack[..pack.len() / 2]).unwrap();
    let (url, seen) = serve_logged(feed.path().to_path_buf(), None);
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 40, true) };
    let (u, data) = updater(&url, info.clone());
    u.check(false);
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::Failed));
    assert_eq!(pack_requests(&seen), 2, "each check tries the network again");
    let dir = data.path().join("updates");
    assert!(!is_rejected(&dir, &sha256_hex(&pack)) && !is_unreadable(&dir, &info, &sha256_hex(&pack)));
}

#[test]
fn a_feed_going_back_to_the_last_good_pack_reuses_it() {
    let make = |build: u64| {
        Pack::new(crate::content::Library::get().index.clone(), Vec::new())
            .with_look(BTreeMap::from([("assets/main.css".to_owned(), format!(".v{build} {{}}"))]))
            .encode()
    };
    let (good, later) = (make(41), make(42));
    let signer = ed25519_dalek::SigningKey::from_bytes(&[14; 32]);
    let info = BuildInfo { key: Some(signer.verifying_key()), ..build("0.1.0", 40, true) };
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().join("updates");
    fs::create_dir_all(&dir).unwrap();
    let install = |bytes: &[u8], build: u64| {
        fs::write(dir.join(PACK_FILE), bytes).unwrap();
        let meta = PackInfo {
            version: "0.1.0".into(),
            build,
            sha256: sha256_hex(bytes),
            published_at: None,
            ui_build: Some(40),
        };
        write_json(&dir, PACK_INFO, &meta).unwrap();
        fs::write(dir.join(PACK_STARTS), "1").unwrap();
    };
    install(&good, 41);
    recovery::save_settled(&dir, &info).unwrap();
    install(&later, 42);

    let feed = pack_feed(&signer, 43, Some(40), &good, Vec::new());
    let (url, seen) = serve_logged(feed.path().to_path_buf(), None);
    let u = updater_in(data.path(), &url, info);
    u.check(false);
    assert_eq!(u.status().content_check, Some(ContentCheck::New));
    assert_eq!(pack_requests(&seen), 0, "taken from the last good copy");
    assert_eq!(fs::read(dir.join(PACK_FILE)).unwrap(), good);
    assert_eq!(load_pack_info(&dir).unwrap().build, 43);
}

#[test]
fn the_feed_pack_of_the_running_build_is_not_downloaded() {
    use super::service::built_in;
    let sha = "ab".repeat(32);
    assert!(built_in(Some(&sha), false, &sha.to_uppercase()), "the same content");
    assert!(!built_in(Some(&sha), true, &sha), "a downloaded pack is in use: the feed may roll it back");
    assert!(!built_in(Some(&"cd".repeat(32)), false, &sha), "other content");
    assert!(!built_in(None, false, &sha), "a local build doesn't know its digest");
}
