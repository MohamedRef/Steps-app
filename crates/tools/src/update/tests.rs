use super::*;

/// A real (empty) content pack, told apart by `tag`: `verify` decodes
/// and checks the content asset like a device.
fn sample_pack(tag: &str) -> Vec<u8> {
    let index: steps_core::content::Index = serde_yaml_ng::from_str(&format!(
        "schema: steps.content/v1\nlectures:\n  - {{ id: cvs.L04, title: {tag}, pages: [1, 2], chunk: 1 }}\n"
    ))
    .unwrap();
    steps_core::pack::Pack::new(index, Vec::new()).encode()
}

/// Seed of the test key behind `fixtures/update/` (never a real key).
const FIXTURE_SEED: [u8; 32] = [0x42; 32];

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn keygen_manifest_sign_verify_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let pub_file = dir.path().join("update-key.pub");
    let (_, secret) = keygen(&pub_file, false).unwrap();
    assert!(keygen(&pub_file, false).is_err(), "an existing key is never replaced silently");
    let public = fs::read_to_string(&pub_file).unwrap();

    let apk = dir.path().join("steps.apk");
    fs::write(&apk, b"not really an apk").unwrap();
    let pack = dir.path().join("steps-content.pack");
    fs::write(&pack, sample_pack("first")).unwrap();
    let feed = dir.path().join("feed");
    let m = write_manifest(
        &feed,
        Channel::Alpha,
        "1.0.0-alpha.2",
        0,
        Some("abc1234".into()),
        &[(UpdatePlatform::AndroidArm64, apk), (UpdatePlatform::Content, pack)],
        &[],
        WhatsNew::default(),
    )
    .unwrap();
    assert_eq!(m.assets[0].file, "steps-1.0.0-alpha.2-android-arm64.apk");
    assert_eq!(m.assets[0].version_code, Some(20_000_000));
    assert_eq!(m.assets[1].file, "steps-content-1.0.0-alpha.2-content.pack");
    assert_eq!(m.assets[1].version_code, None);

    // Signing with a secret the app doesn't trust is refused.
    let stranger = B64.encode([1u8; 32]);
    assert!(sign(&feed, &stranger, &public).is_err());
    assert!(!feed.join(SIGNATURE_FILE).exists());

    sign(&feed, &secret, &public).unwrap();
    assert_eq!(verify(&feed, &public, &[]).unwrap(), m);

    // A signed feed whose pack devices would refuse fails verification.
    let mut broken = steps_core::pack::Pack::decode(&sample_pack("x")).unwrap();
    broken.index.lectures.push(broken.index.lectures[0].clone());
    let broken_pack = dir.path().join("broken.pack");
    fs::write(&broken_pack, broken.encode()).unwrap();
    let broken_feed = dir.path().join("broken-feed");
    write_manifest(
        &broken_feed,
        Channel::Alpha,
        "1.0.0-alpha.2",
        0,
        None,
        &[(UpdatePlatform::Content, broken_pack)],
        &[],
        WhatsNew::default(),
    )
    .unwrap();
    sign(&broken_feed, &secret, &public).unwrap();
    let err = verify(&broken_feed, &public, &[]).unwrap_err();
    assert!(err.contains("devices would refuse it") && err.contains("listed twice"), "{err}");

    // A swapped file fails verification.
    fs::write(feed.join(&m.assets[1].file), b"something else").unwrap();
    assert!(verify(&feed, &public, &[]).is_err());
}

#[test]
fn versions_stay_on_their_channels() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("x");
    fs::write(&f, b"x").unwrap();
    let assets = [(UpdatePlatform::MacosArm64, f)];
    let out = dir.path().join("feed");
    let none = WhatsNew::default;
    assert!(write_manifest(&out, Channel::Stable, "1.0.0-alpha.1", 0, None, &assets, &[], none()).is_err());
    assert!(write_manifest(&out, Channel::Beta, "1.0.0", 0, None, &assets, &[], none()).is_ok());
    let preview = write_manifest(&out, Channel::Preview, "0.1.0", 7, None, &assets, &[], none()).unwrap();
    assert_eq!(preview.assets[0].file, "Steps-0.1.0-b7-macos-arm64.app.zip", "preview names carry the build");
}

#[test]
fn the_command_line_reads_repeated_assets() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.apk"), dir.path().join("c.pack"));
    fs::write(&a, b"a").unwrap();
    fs::write(&b, b"b").unwrap();
    let out = dir.path().join("feed");
    let args = strings(&[
        "manifest",
        "--channel",
        "preview",
        "--version",
        "0.1.0",
        "--build",
        "12",
        "--asset",
        &format!("android-arm64={}", a.display()),
        "--asset",
        &format!("content={}", b.display()),
        "--out",
        &out.display().to_string(),
    ]);
    let said = run(dir.path(), &args).unwrap();
    assert!(said.starts_with("preview 0.1.0 build 12"), "{said}");
    let m: UpdateManifest = serde_json::from_slice(&fs::read(out.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(m.assets.len(), 2);

    assert!(run(dir.path(), &strings(&["manifest", "--channel", "nightly"])).is_err());
    assert!(run(dir.path(), &strings(&["manifest", "--channel", "alpha", "--version", "1.0.0"])).is_err());
    assert!(run(dir.path(), &strings(&["sign"])).is_err());
}

#[test]
fn a_content_only_feed_carries_the_app_over() {
    let dir = tempfile::tempdir().unwrap();
    let (apk, pack) = (dir.path().join("a.apk"), dir.path().join("c.pack"));
    fs::write(&apk, b"a").unwrap();
    fs::write(&pack, sample_pack("one")).unwrap();
    let feed = |build: &str, out: &str, extra: &[&str]| {
        let mut args =
            strings(&["manifest", "--channel", "preview", "--version", "0.1.1-alpha.0", "--build", build]);
        args.extend(strings(&["--asset", &format!("content={}", pack.display()), "--out", out]));
        args.extend(strings(extra));
        run(dir.path(), &args)
    };
    let first = dir.path().join("first").display().to_string();
    let apk_arg = format!("android-arm64={}", apk.display());
    feed("40", &first, &["--asset", &apk_arg]).unwrap();
    let previous = format!("{first}/{MANIFEST_FILE}");
    assert!(feed("41", &first, &["--carry", "android-arm64"]).is_err(), "carrying needs the previous feed");

    fs::write(&pack, sample_pack("two")).unwrap();
    let second = dir.path().join("second").display().to_string();
    feed("45", &second, &["--previous", &previous, "--carry", "android-arm64"]).unwrap();
    let m: UpdateManifest =
        serde_json::from_slice(&fs::read(format!("{second}/{MANIFEST_FILE}")).unwrap()).unwrap();
    let app = m.asset(UpdatePlatform::AndroidArm64).unwrap();
    assert_eq!((app.file.as_str(), m.build_of(app)), ("steps-0.1.1-alpha.0-b40-android-arm64.apk", 40));
    assert!(!Path::new(&second).join(&app.file).exists(), "the carried file stays where it is");
    assert_eq!(
        m.asset(UpdatePlatform::Content).unwrap().file,
        "steps-content-0.1.1-alpha.0-b45-content.pack"
    );
    assert_eq!((m.build, m.ui_build), (45, Some(40)), "the look fits the app it carries");

    // The preview workflow verifies the feed it uploads: the carried APK
    // is on the release, not in the folder (seen on a preview run of the app Steps came from).
    let (pub_file, second) = (dir.path().join("k.pub"), Path::new(&second));
    let (_, secret) = keygen(&pub_file, false).unwrap();
    let public = fs::read_to_string(&pub_file).unwrap();
    sign(second, &secret, &public).unwrap();
    assert!(verify(second, &public, &[]).is_err(), "unnamed, the missing APK still fails");
    assert_eq!(verify(second, &public, &[UpdatePlatform::AndroidArm64]).unwrap(), m);
    assert!(
        verify(second, &public, &[UpdatePlatform::Content]).is_err(),
        "a row this build wrote is not carried"
    );
}

#[test]
fn channels_follow_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let channels = |v: &str| run(dir.path(), &strings(&["channels", "--version", v]));
    assert_eq!(channels("0.1.1-alpha.1").unwrap(), "alpha");
    assert_eq!(channels("0.2.0-beta.1").unwrap(), "alpha\nbeta");
    // A calendar version (or any without a stage) goes to alpha unless
    // its channel is named.
    assert_eq!(channels("2026.11.1").unwrap(), "alpha");
    assert_eq!(channels("1.0.0").unwrap(), "alpha");
    let on = |v: &str, c: &str| run(dir.path(), &strings(&["channels", "--version", v, "--channel", c]));
    assert_eq!(on("2026.11.1", "beta").unwrap(), "alpha\nbeta");
    assert_eq!(on("2026.11.1", "stable").unwrap(), "alpha\nbeta\nstable");
    assert!(on("2026.11.1", "preview").is_err());
    assert!(on("1.0.0-alpha.3", "beta").is_err(), "an old alpha stays an alpha");
    assert_eq!(on("1.0.0-beta.1", "beta").unwrap(), "alpha\nbeta");
    assert!(channels("one").is_err(), "a bad version fails instead of publishing nowhere");
}

#[test]
fn guard_refuses_the_placeholder_and_older_versions() {
    let dir = tempfile::tempdir().unwrap();
    let feed = dir.path().join("alpha.json");
    let manifest = UpdateManifest {
        format: UPDATE_FORMAT,
        channel: Channel::Alpha,
        version: "1.0.0-alpha.2".into(),
        build: 7,
        commit: None,
        published_at: "2026-11-04T00:00:00Z".into(),
        notes: UpdateNotes::default(),
        assets: Vec::new(),
        history: Vec::new(),
        ui_build: None,
    };
    fs::write(&feed, serde_json::to_string(&manifest).unwrap()).unwrap();
    let feed = feed.to_string_lossy().into_owned();
    let missing = dir.path().join("none.json").to_string_lossy().into_owned();
    let guard = |v: &str| {
        run(dir.path(), &strings(&["guard", "--version", v, "--previous", &feed, "--previous", &missing]))
    };
    assert!(guard("1.0.0-alpha.0").unwrap_err()[0].contains("placeholder"));
    assert!(guard("1.0.0-alpha.2").unwrap_err()[0].contains("not newer"));
    assert!(guard("1.0.0-alpha.1").is_err());
    assert!(guard("1.0.0-alpha.3").is_ok());
    assert!(guard("1.0.0-beta.1").is_ok());
    let fresh = |v: &str| run(dir.path(), &strings(&["guard", "--version", v]));
    assert!(fresh("1.0.0-alpha.1").is_ok(), "no feed yet: the first alpha may go");
    assert!(fresh("x").is_err());
}

#[test]
fn dates_are_rfc3339() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_753), (2026, 10, 27));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
}

#[test]
fn the_release_key_file_holds_no_key_until_keygen() {
    // Until `just update-keygen` runs on the Mac, the app trusts no feed
    // at all; once it has, this file holds a valid public key.
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(RELEASE_KEY_FILE))
        .unwrap();
    if key_line(&text).is_some() {
        parse_public_key(&text).unwrap();
    }
}

/// Regenerates the signed fixture the core and app tests read:
/// `cargo test -p steps-tools write_update_fixture -- --ignored`.
#[test]
#[ignore]
fn write_update_fixture() {
    write_update_fixture_to(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/update"));
}

/// The fixture as the generator writes it, in a temp dir: signed with
/// the test key, and the same feed as the committed one.
#[test]
fn the_update_fixture_generator_matches_the_committed_feed() {
    let dir = tempfile::tempdir().unwrap();
    write_update_fixture_to(dir.path());
    let public = fs::read_to_string(dir.path().join("test-key.pub")).unwrap();
    let json = fs::read(dir.path().join(MANIFEST_FILE)).unwrap();
    let sig = fs::read_to_string(dir.path().join(SIGNATURE_FILE)).unwrap();
    let m = verify_manifest(&json, &sig, &parse_public_key(&public).unwrap()).unwrap();
    let committed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/update");
    assert_eq!(public, fs::read_to_string(committed.join("test-key.pub")).unwrap());
    let old: UpdateManifest =
        serde_json::from_str(&fs::read_to_string(committed.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(
        (&m.channel, &m.version, m.build, &m.notes, &m.history),
        (&old.channel, &old.version, old.build, &old.notes, &old.history)
    );
    let rows = |m: &UpdateManifest| {
        m.assets.iter().map(|a| (a.platform, a.size, a.sha256.clone())).collect::<Vec<_>>()
    };
    assert_eq!(rows(&m), rows(&old));
    assert!(m.assets.iter().all(|a| a.file.starts_with("steps")), "{:?}", m.assets);
}

fn write_update_fixture_to(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let key = SigningKey::from_bytes(&FIXTURE_SEED);
    let public = B64.encode(key.verifying_key().as_bytes());
    fs::write(
        root.join("test-key.pub"),
        format!(
            "# Test key for fixtures/update only (seed 0x42 × 32). Never trusted by a build.\n{public}\n"
        ),
    )
    .unwrap();
    // The app tests write these bytes as the feed's files (no .apk is tracked).
    let apk = b"fixture apk bytes".to_vec();
    let pack = b"fixture pack bytes".to_vec();
    let m = UpdateManifest {
        format: UPDATE_FORMAT,
        channel: Channel::Preview,
        version: "0.1.0".into(),
        build: 42,
        commit: Some("0123abc".into()),
        published_at: "2026-10-27T08:00:00Z".into(),
        notes: UpdateNotes {
            en: "Fixture: new L09 questions.".into(),
            ar_eg: "تجربة: أسئلة جديدة في المحاضرة التاسعة.".into(),
        },
        assets: vec![
            UpdateAsset {
                platform: UpdatePlatform::AndroidArm64,
                file: "steps-0.1.0-b42-android-arm64.apk".into(),
                size: apk.len() as u64,
                sha256: sha256_hex(&apk),
                version_code: Some(100_999),
                build: None,
            },
            UpdateAsset {
                platform: UpdatePlatform::Content,
                file: "steps-content-0.1.0-b42-content.pack".into(),
                size: pack.len() as u64,
                sha256: sha256_hex(&pack),
                version_code: None,
                build: None,
            },
        ],
        history: vec![
            BuildChanges {
                version: "0.1.0".into(),
                build: 42,
                published_at: "2026-10-27T08:00:00Z".into(),
                changes: changes::parse(
                    "- kind: new\n  ar_eg: أسئلة جديدة في المحاضرة التاسعة\n  en: New L09 questions\n\
                         - kind: improved\n  ar_eg: البنك بقى أسرع\n\
                         - kind: fixed\n  ar_eg: التايمر في الامتحان التجريبي مبقاش بيقف\n",
                )
                .unwrap(),
                content_only: false,
            },
            BuildChanges {
                version: "0.1.0".into(),
                build: 40,
                published_at: "2026-10-25T08:00:00Z".into(),
                changes: changes::parse("- kind: new\n  ar_eg: أول نسخة تجريبية\n").unwrap(),
                content_only: false,
            },
        ],
        ui_build: None,
    };
    let json = serde_json::to_vec_pretty(&m).unwrap();
    fs::write(root.join(MANIFEST_FILE), &json).unwrap();
    fs::write(root.join(SIGNATURE_FILE), format!("{}\n", B64.encode(key.sign(&json).to_bytes()))).unwrap();
}

#[test]
fn keygen_from_the_command_line_writes_the_public_half() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("keys/update-key.pub");
    let a = strings(&["keygen", "--public-out", &out.display().to_string()]);
    let secret = run(dir.path(), &a).unwrap();
    assert_eq!(B64.decode(&secret).unwrap().len(), 32, "stdout is only the secret");
    let text = fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("# Steps's update key") && key_line(&text).is_some());
    assert!(run(dir.path(), &a).is_err(), "never replaced without --force");
    let mut forced = a.clone();
    forced.push("--force".into());
    assert_ne!(run(dir.path(), &forced).unwrap(), secret);
}

#[test]
fn manifest_flags_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let apk = d.join("a.apk");
    fs::write(&apk, b"a").unwrap();
    let asset = format!("android-arm64={}", apk.display());
    let out = d.join("feed").display().to_string();
    let err = |a: &[&str]| run_inner(d, &strings(a)).unwrap_err();
    assert_eq!(err(&["manifest"]), "--channel is required");
    assert!(
        err(&["manifest", "--channel", "alpha", "--asset", &asset]).contains("Cargo.toml"),
        "no workspace here"
    );
    assert_eq!(
        err(&["manifest", "--channel", "alpha", "--version", "1.0.0-alpha.1", "--build", "b7"]),
        "--build \"b7\" isn't a number"
    );
    assert_eq!(
        err(&["manifest", "--channel", "alpha", "--version", "1.0.0-alpha.1"]),
        "give at least one --asset PLATFORM=PATH"
    );
    assert_eq!(
        err(&["manifest", "--channel", "alpha", "--version", "1.0.0-alpha.1", "--asset", "apk"]),
        "--asset \"apk\": expected PLATFORM=PATH"
    );
    assert_eq!(
        err(&["manifest", "--channel", "alpha", "--version", "1.0.0-alpha.1", "--asset", "ios=x"]),
        "unknown platform \"ios\""
    );
    assert_eq!(
        err(&[
            "manifest",
            "--channel",
            "alpha",
            "--version",
            "1.0.0-alpha.1",
            "--asset",
            &asset,
            "--carry",
            "ios"
        ]),
        "unknown platform \"ios\""
    );
    let two = [
        "manifest",
        "--channel",
        "alpha",
        "--version",
        "1.0.0-alpha.1",
        "--asset",
        &asset,
        "--asset",
        &asset,
        "--out",
        &out,
    ];
    assert_eq!(err(&two), "two files for android-arm64");
    // A previous feed of another channel, or of another version, can't be carried from.
    let base =
        ["manifest", "--channel", "alpha", "--version", "1.0.0-alpha.1", "--asset", &asset, "--out", &out];
    run_inner(d, &strings(&base)).unwrap();
    let previous = format!("{out}/{MANIFEST_FILE}");
    let feed2 = d.join("feed2").display().to_string();
    assert_eq!(
        err(&[
            "manifest",
            "--channel",
            "beta",
            "--version",
            "1.0.0-beta.1",
            "--asset",
            &asset,
            "--previous",
            &previous,
            "--out",
            &feed2
        ]),
        "the previous feed is for alpha"
    );
    let pack = d.join("c.pack");
    fs::write(&pack, sample_pack("cvs")).unwrap();
    let content = format!("content={}", pack.display());
    assert_eq!(
        err(&[
            "manifest",
            "--channel",
            "alpha",
            "--version",
            "1.0.0-alpha.2",
            "--asset",
            &content,
            "--previous",
            &previous,
            "--carry",
            "android-arm64",
            "--out",
            &feed2
        ]),
        "--carry: the previous feed is 1.0.0-alpha.1, not 1.0.0-alpha.2"
    );
    assert_eq!(
        err(&[
            "manifest",
            "--channel",
            "alpha",
            "--version",
            "1.0.0-alpha.1",
            "--asset",
            &content,
            "--previous",
            &previous,
            "--carry",
            "macos-arm64",
            "--out",
            &feed2
        ]),
        "--carry: no macos-arm64 in the previous feed"
    );
    assert_eq!(
        err(&[
            "manifest",
            "--channel",
            "alpha",
            "--version",
            "1.0.0-alpha.1",
            "--asset",
            &asset,
            "--previous",
            &previous,
            "--carry",
            "android-arm64",
            "--out",
            &feed2
        ]),
        "android-arm64 is both carried and given"
    );
    fs::write(&previous, "{").unwrap();
    assert!(
        err(&[
            "manifest",
            "--channel",
            "alpha",
            "--version",
            "1.0.0-alpha.1",
            "--asset",
            &asset,
            "--previous",
            &previous,
            "--out",
            &feed2
        ])
        .starts_with("previous feed")
    );
    assert_eq!(err(&["version"]), USAGE);
    assert_eq!(err(&["verify"]), USAGE);
    assert!(err(&["verify", &out, "--carried", "ios"]).contains("unknown platform"));
    assert!(
        err(&["verify", &out, "--public-key", &d.join("none.pub").display().to_string()])
            .contains("none.pub")
    );
}

#[test]
fn the_workspace_version_is_the_default() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = workspace_version(&root).unwrap();
    assert!(Version::parse(&v).is_some(), "{v}");
    let channels = run_inner(&root, &strings(&["channels"])).unwrap();
    assert!(channels.starts_with("alpha"), "{channels}");
}

/// A git repository with a workspace version and a `changes/` line.
fn release_repo() -> tempfile::TempDir {
    use crate::testkit::{init_repo, write};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "Cargo.toml", "[workspace.package]\nversion = \"1.0.0-alpha.3\"\n");
    write(
        root,
        "changes/cvs-pathology.yaml",
        "- kind: new\n  ar_eg: محاضرة الباثولوجي في القلب\n  en: Cardiovascular pathology\n",
    );
    init_repo(root);
    dir
}

#[test]
fn changes_become_the_builds_whats_new() {
    let dir = release_repo();
    let root = dir.path();
    let pack = root.join("c.pack");
    fs::write(&pack, sample_pack("cvs")).unwrap();
    let out = root.join("feed").display().to_string();
    let said = run_inner(
        root,
        &strings(&[
            "manifest",
            "--channel",
            "alpha",
            "--build",
            "3",
            "--asset",
            &format!("content={}", pack.display()),
            "--changes",
            "--previous",
            "/nonexistent/update.json",
            "--out",
            &out,
        ]),
    )
    .unwrap();
    assert!(said.starts_with("alpha 1.0.0-alpha.3 build 3"), "{said}");
    let m: UpdateManifest =
        serde_json::from_slice(&fs::read(Path::new(&out).join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(m.notes.ar_eg, "محاضرة الباثولوجي في القلب");
    assert_eq!(m.notes.en, "Cardiovascular pathology");
    assert_eq!(m.history.len(), 1);
    assert_eq!((m.history[0].build, m.history[0].content_only), (3, false));

    let listed = run_inner(root, &strings(&["changes"])).unwrap();
    assert!(
        listed.starts_with(
            "1 file(s) in changes/ check out\nNew\tمحاضرة الباثولوجي في القلب\tCardiovascular pathology"
        ),
        "{listed}"
    );
    let base = crate::testkit::git(root, &["rev-parse", "HEAD"]);
    crate::testkit::write(
        root,
        "changes/cvs-pathology.yaml",
        "- kind: new\n  ar_eg: محاضرة الباثولوجي في القلب\n  en: Cardiovascular pathology\n- kind: fixed\n  ar_eg: صفحة الـ tamponade بقت صح\n",
    );
    crate::testkit::commit(root, "fix a page");
    let gated = run_inner(root, &strings(&["changes", "--gate", &base])).unwrap();
    assert!(gated.ends_with("Fixed\tصفحة الـ tamponade بقت صح\t"), "{gated}");
    let since = run_inner(root, &strings(&["changes", "--since", &base])).unwrap();
    assert_eq!(since, gated);
}

#[test]
fn sign_needs_a_real_key_and_a_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let pub_file = dir.path().join("k.pub");
    let (_, secret) = keygen(&pub_file, false).unwrap();
    let public = fs::read_to_string(&pub_file).unwrap();
    assert_eq!(
        sign(dir.path(), "short", &public).unwrap_err(),
        format!("{SECRET_ENV} isn't a base64 32-byte key")
    );
    assert!(sign(dir.path(), &secret, &public).unwrap_err().contains("run `update manifest` first"));
    fs::write(dir.path().join(MANIFEST_FILE), "{}").unwrap();
    assert!(sign(dir.path(), &secret, "not a key").is_err());
    // Without the secret in the environment, the command says which variable.
    if std::env::var(SECRET_ENV).is_err() {
        let err = run_inner(dir.path(), &strings(&["sign", &dir.path().display().to_string()])).unwrap_err();
        assert_eq!(err, format!("{SECRET_ENV} is not set"));
    }
}

#[test]
fn verify_from_the_command_line_names_carried_rows() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let pub_file = d.join("k.pub");
    let (_, secret) = keygen(&pub_file, false).unwrap();
    let public = fs::read_to_string(&pub_file).unwrap();
    let pack = d.join("c.pack");
    fs::write(&pack, sample_pack("cvs")).unwrap();
    let feed = d.join("feed");
    write_manifest(
        &feed,
        Channel::Alpha,
        "1.0.0-alpha.2",
        5,
        None,
        &[(UpdatePlatform::Content, pack)],
        &[],
        WhatsNew::default(),
    )
    .unwrap();
    sign(&feed, &secret, &public).unwrap();
    let f = feed.display().to_string();
    let k = pub_file.display().to_string();
    let ok = run_inner(d, &strings(&["verify", &f, "--public-key", &k])).unwrap();
    assert_eq!(ok, "ok: alpha 1.0.0-alpha.2 build 5, 1 file(s) match");
    let err = run_inner(d, &strings(&["verify", &f, "--public-key", &k, "--carried", "android-arm64"]))
        .unwrap_err();
    assert_eq!(err, "--carried: no android-arm64 in the feed");
    // A missing signature.
    fs::remove_file(feed.join(SIGNATURE_FILE)).unwrap();
    assert!(run(d, &strings(&["verify", &f, "--public-key", &k])).unwrap_err()[0].contains(SIGNATURE_FILE));
}
