use ed25519_dalek::{Signer, SigningKey};

use super::*;

/// The key that signed `fixtures/update/update.json` (test only; seed
/// 0x42 repeated, written by `steps-tools`' `write_update_fixture` test).
const FIXTURE_PUBLIC_KEY: &str = include_str!("../../../../fixtures/update/test-key.pub");

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

fn asset(platform: UpdatePlatform, file: &str, sha: &str) -> UpdateAsset {
    UpdateAsset { platform, file: file.into(), size: 3, sha256: sha.into(), version_code: None, build: None }
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
                version_code: Some(version_code(build)),
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
fn version_codes_follow_the_build() {
    // The recipe in scripts/android-release-apk.sh.
    assert_eq!(version_code(564), 20_000_564);
    // Above every code the old recipe made (1.0.0-alpha.3 was 10 000 103),
    // so the first build of the new scheme installs over it.
    assert!(version_code(1) > 10_000_103);
    assert!(version_code(2_000_000_000 - VERSION_CODE_BASE) <= 2_100_000_000, "Android's ceiling");
}

#[test]
fn calendar_versions_sort_above_the_old_alphas() {
    assert!(v("2026.11.1") > v("1.0.0-alpha.3"));
    assert!(v("2026.11.0") > v("1.0.0-alpha.3"));
    assert!(v("2026.11.1") > v("1.0.0"));
    assert!(v("2026.11.1") < v("2026.11.2"));
    assert!(v("2026.11.9") < v("2026.12.1"));
    assert!(v("2026.12.1") < v("2027.1.1"));
    assert_eq!(v("2026.11.1").to_string(), "2026.11.1");
    assert_eq!(v("2026.11.1").channels(), Channel::RELEASE);
}

#[test]
fn channels_reach_the_ones_below() {
    assert_eq!(Channel::Alpha.reach(), [Channel::Alpha]);
    assert_eq!(Channel::Beta.reach(), [Channel::Alpha, Channel::Beta]);
    assert_eq!(Channel::Stable.reach(), Channel::RELEASE);
    assert_eq!(Channel::Preview.reach(), [Channel::Preview]);
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
        serde_json::from_slice(include_bytes!("../../../../fixtures/update/update.json")).unwrap();
    v.as_object_mut().unwrap().remove("history");
    let m: UpdateManifest = serde_json::from_value(v).unwrap();
    assert!(m.history.is_empty());
    assert!(!serde_json::to_string(&m).unwrap().contains("history"));
}

#[test]
fn the_signed_fixture_verifies_and_any_change_is_refused() {
    let key = parse_public_key(FIXTURE_PUBLIC_KEY).unwrap();
    let json = include_bytes!("../../../../fixtures/update/update.json");
    let sig = include_str!("../../../../fixtures/update/update.json.sig");
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
    assert_eq!(decide(&me, &manifest(Channel::Alpha, "1.0.0-alpha.1", 0), android), Ok(Decision::UpToDate));
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
    assert_eq!(decide(&preview, &manifest(Channel::Preview, "0.1.0", 140), android), Ok(Decision::UpToDate));
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
    for bad in ["", "2026-10-27", "2026-10-27T08:00:00", "2026-10-27T08:00:00+02:00", "2026-13-01T00:00:00Z"]
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
        serde_json::from_slice(include_bytes!("../../../../fixtures/update/update.json")).unwrap();
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
        serde_json::from_slice(include_bytes!("../../../../fixtures/update/update.json")).unwrap();
    v["assets"][0].as_object_mut().unwrap().remove("sha256");
    assert!(serde_json::from_value::<UpdateManifest>(v).is_err());
}
