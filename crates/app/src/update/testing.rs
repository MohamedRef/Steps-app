//! Screen tests' updater: one per test thread, over the signed fixture feed
//! served from 127.0.0.1, so Settings › Updates, the banner and the sheet
//! run on a real `Updater` without touching the network.

use std::cell::Cell;

use super::*;

thread_local! {
    pub(super) static TEST_UPDATER: Cell<Option<&'static Updater>> = const { Cell::new(None) };
}

/// The bytes behind the fixture's Android build.
pub const FIXTURE_APK: &[u8] = b"fixture apk bytes";

/// The signed fixture feed (preview channel, 0.1.0 build 42 for Android),
/// served on 127.0.0.1; returns its URL.
pub fn serve_fixture_feed() -> String {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/update");
    let feed = tempfile::tempdir().unwrap().keep();
    for name in [MANIFEST_FILE, SIGNATURE_FILE] {
        fs::copy(src.join(name), feed.join(name)).unwrap();
    }
    fs::write(feed.join("steps-0.1.0-b42-android-arm64.apk"), FIXTURE_APK).unwrap();
    serve(feed)
}

/// A URL where nothing answers.
pub fn dead_feed() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", l.local_addr().unwrap())
}

fn serve(dir: PathBuf) -> String {
    use std::io::{BufRead as _, BufReader};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                    break;
                }
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_owned();
            let mut stream = reader.into_inner();
            let _ = match fs::read(dir.join(&path)) {
                Ok(body) => stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .and_then(|()| stream.write_all(&body)),
                Err(_) => stream
                    .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
            };
        }
    });
    url
}

/// Opens an updater for this test thread, as build `version+build` (a
/// preview build following the preview channel) over `feed`, with `prefs`
/// already saved; [`super::get`] returns it on this thread from now on.
pub fn install(feed: &str, version: &str, build: u64, prefs: Prefs) -> &'static Updater {
    let key = include_str!("../../../../fixtures/update/test-key.pub");
    let info = BuildInfo {
        version: Version::parse(version).unwrap(),
        build,
        preview: true,
        key: Some(parse_public_key(key).unwrap()),
    };
    let dir = tempfile::tempdir().unwrap().keep().join("updates");
    let prefs = Prefs { channel: prefs.channel.or(Some(Channel::Preview)), ..prefs };
    save_prefs(&dir, &prefs).unwrap();
    let mut u = Updater::new(dir, info, Some(UpdatePlatform::AndroidArm64), Some(feed.to_owned()), None);
    u.retry_after = Duration::from_millis(10);
    let u: &'static Updater = Box::leak(Box::new(u));
    TEST_UPDATER.with(|t| t.set(Some(u)));
    u
}

/// An updater that is switched off for `why`.
pub fn install_off(why: Off) -> &'static Updater {
    let dir = tempfile::tempdir().unwrap().keep().join("updates");
    let u = Updater::new(dir, BuildInfo::from_env(), None, None, Some(why));
    let u: &'static Updater = Box::leak(Box::new(u));
    TEST_UPDATER.with(|t| t.set(Some(u)));
    u
}

impl Updater {
    /// Puts the status in any state a screen should draw.
    pub fn set_status_for_tests(&self, f: impl FnOnce(&mut Status)) {
        self.tx.send_modify(f);
    }

    /// The folder the updater keeps its files in.
    pub fn dir_for_tests(&self) -> &Path {
        &self.dir
    }
}
