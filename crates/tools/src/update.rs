//! `steps-tools update …`: update feeds (`docs/RELEASE.md` §5), the signing
//! key, `update.json` and its signature. The checks the app runs live in
//! `steps_core::update`.
//!
//! ```text
//! update keygen [--public-out FILE] [--force]
//! update manifest --channel C [--version V] [--build N] [--commit SHA]
//!                 --asset PLATFORM=PATH … [--notes-en T] [--notes-ar-eg T] [--out DIR]
//!                 [--previous FEED/update.json] [--changes] [--carry PLATFORM …]
//! update changes [--since COMMIT]          what the next build's "what's new" says
//! update changes --gate BASE               fails unless HEAD adds a line (CI, #320)
//! update channels [--version V] [--channel C] the feeds a release goes to
//! update guard [--version V] [--previous FEED/update.json …]
//!                                           refuses a version that can't be released
//! update sign DIR [--public-key FILE]       secret from STEPS_UPDATE_SECRET
//! update verify DIR [--public-key FILE] [--carried PLATFORM …]
//! ```
//!
//! - `keygen` makes the Ed25519 key pair. The public half goes into the app
//!   (`crates/app/update-key.pub`, committed); the secret half is printed
//!   once for the Keychain (`just update-keygen`) and never written to a file.
//! - `manifest` hashes the builds and writes `update.json` next to copies of
//!   them, ready to upload. PLATFORM is `android-arm64`, `macos-arm64`,
//!   `macos-x86_64` or `content` (the pack from `steps-tools packs build`).
//! - `sign` signs `update.json` and checks the result against the public key.
//! - `verify` runs the app's checks on a feed folder. `--carried PLATFORM`
//!   names a row `--carry` kept: its file is on the release already, not in
//!   the folder, so only its build is checked (older than the feed's).
//! - `channels` prints one channel per line: the feeds a release on
//!   `--channel` (default `alpha`) reaches (alpha → alpha; beta → alpha and
//!   beta; stable → all three), for the Release workflow and
//!   `scripts/update-publish.sh`. An old-style `-alpha.N`/`-beta.N` version
//!   keeps the channels its stage names; asking it for another one fails.
//! - `guard` fails on the `-alpha.0` placeholder and on a version that is
//!   not newer than every `--previous` feed's (a missing file is fine), so
//!   the Release workflow never publishes over a newer or equal build (#104).
//! - `--previous` keeps the channel's current history (a missing file is
//!   fine); with `--changes`, `changes/` since that feed's commit becomes
//!   this build's entry, and the notes when none are given (`crate::changes`).
//! - `--carry PLATFORM` keeps that platform's file from `--previous` as it
//!   is (same file, same build): a content-only feed carries the app over,
//!   so devices get the new pack without installing anything. Its history
//!   entry is marked `content_only`.
//!
//! Adapted from batna-app main: `crates/tools/src/update.rs` (clap and
//! anyhow swapped for this crate's plain argument handling).

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use steps_core::update::{
    BuildChanges, Channel, MANIFEST_FILE, SIGNATURE_FILE, Stage, UPDATE_FORMAT, UpdateAsset, UpdateManifest,
    UpdateNotes, UpdatePlatform, Version, check_download, key_line, parse_public_key, verify_manifest,
    version_code,
};

use crate::changes;
use crate::flag;

/// The release public key the app builds in.
pub const RELEASE_KEY_FILE: &str = "crates/app/update-key.pub";
/// The secret, base64, for `update sign`.
const SECRET_ENV: &str = "STEPS_UPDATE_SECRET";

const USAGE: &str = "usage: steps-tools update <keygen | manifest | sign DIR | verify DIR | changes | channels | guard> (see crates/tools/src/update.rs)";

type Res<T> = Result<T, String>;

pub fn run(root: &Path, args: &[String]) -> Result<String, Vec<String>> {
    run_inner(root, args).map_err(|e| vec![e])
}

fn run_inner(root: &Path, args: &[String]) -> Res<String> {
    let public_key = |args: &[String]| -> Res<String> {
        let path = flag(args, "--public-key").map_or_else(|| root.join(RELEASE_KEY_FILE), PathBuf::from);
        read_text(&path)
    };
    match args.first().map(String::as_str) {
        Some("keygen") => {
            let out = flag(args, "--public-out").map_or_else(|| root.join(RELEASE_KEY_FILE), PathBuf::from);
            let (public, secret) = keygen(&out, args.iter().any(|a| a == "--force"))?;
            eprintln!("public key {public} → {}", out.display());
            // stdout carries only the secret, for the Keychain.
            Ok(secret)
        }
        Some("manifest") => {
            let channel = flag(args, "--channel").ok_or("--channel is required")?;
            let channel = Channel::parse(channel)
                .ok_or_else(|| format!("unknown channel {channel:?} (preview, alpha, beta, stable)"))?;
            let version = match flag(args, "--version") {
                Some(v) => v.to_owned(),
                None => workspace_version(root)?,
            };
            let build = match flag(args, "--build") {
                Some(b) => b.parse().map_err(|_| format!("--build {b:?} isn't a number"))?,
                None => 0,
            };
            let assets = values(args, "--asset").map(parse_asset).collect::<Res<Vec<_>>>()?;
            if assets.is_empty() {
                return Err("give at least one --asset PLATFORM=PATH".into());
            }
            let carry = values(args, "--carry")
                .map(|p| UpdatePlatform::parse(p).ok_or_else(|| format!("unknown platform {p:?}")))
                .collect::<Res<Vec<_>>>()?;
            let mut notes = UpdateNotes {
                en: flag(args, "--notes-en").unwrap_or_default().to_owned(),
                ar_eg: flag(args, "--notes-ar-eg").unwrap_or_default().to_owned(),
            };
            let out = flag(args, "--out").map_or_else(|| root.join("dist/update"), PathBuf::from);
            let commit = flag(args, "--commit").map(str::to_owned);
            let previous: Option<UpdateManifest> = match flag(args, "--previous").map(Path::new) {
                Some(p) if p.exists() => {
                    let m: UpdateManifest = serde_json::from_str(&read_text(p)?)
                        .map_err(|e| format!("previous feed {}: {e}", p.display()))?;
                    if m.channel != channel {
                        return Err(format!("the previous feed is for {}", m.channel.as_str()));
                    }
                    Some(m)
                }
                _ => None,
            };
            let carried = carry
                .iter()
                .map(|p| {
                    let m = previous.as_ref().ok_or("--carry needs a --previous feed")?;
                    if m.version != version {
                        return Err(format!("--carry: the previous feed is {}, not {version}", m.version));
                    }
                    let a = m
                        .asset(*p)
                        .ok_or_else(|| format!("--carry: no {} in the previous feed", p.as_str()))?;
                    Ok(UpdateAsset { build: Some(m.build_of(a)), ..a.clone() })
                })
                .collect::<Res<Vec<_>>>()?;
            let mut entry = None;
            if args.iter().any(|a| a == "--changes") {
                let since = previous.as_ref().and_then(|m| m.commit.as_deref());
                let found = changes::collect(root, since)?;
                eprintln!("{} change(s) since {}", found.len(), since.unwrap_or("HEAD^"));
                if notes == UpdateNotes::default() {
                    notes = changes::notes_from(&found);
                }
                entry = (!found.is_empty()).then(|| BuildChanges {
                    version: version.clone(),
                    build,
                    published_at: now_rfc3339(),
                    changes: found,
                    content_only: !carried.is_empty(),
                });
            }
            let history = changes::history(previous.as_ref(), entry, &changes::retro(root)?);
            let m = write_manifest(
                &out,
                channel,
                &version,
                build,
                commit,
                &assets,
                &carried,
                WhatsNew { notes, history },
            )?;
            Ok(format!(
                "{} {} build {} → {} ({} file(s))",
                m.channel.as_str(),
                m.version,
                m.build,
                out.join(MANIFEST_FILE).display(),
                m.assets.len()
            ))
        }
        Some("sign") => {
            let dir = args.get(1).filter(|a| !a.starts_with("--")).ok_or(USAGE)?;
            let secret = std::env::var(SECRET_ENV).map_err(|_| format!("{SECRET_ENV} is not set"))?;
            sign(Path::new(dir), &secret, &public_key(args)?)?;
            Ok(format!("signed {}", Path::new(dir).join(SIGNATURE_FILE).display()))
        }
        Some("changes") => {
            let n = changes::check_all(root)?;
            let mut out = format!("{n} file(s) in {}/ check out", changes::DIR);
            let found = match flag(args, "--gate") {
                Some(base) => changes::gate(root, base)?,
                None => changes::collect(root, flag(args, "--since"))?,
            };
            for c in found {
                out.push_str(&format!("\n{:?}\t{}\t{}", c.kind, c.text.ar_eg, c.text.en));
            }
            Ok(out)
        }
        Some("channels") => {
            let v = match flag(args, "--version") {
                Some(v) => v.to_owned(),
                None => workspace_version(root)?,
            };
            let parsed = Version::parse(&v).ok_or_else(|| format!("{v:?} isn't a version"))?;
            let reach = match flag(args, "--channel") {
                Some(c) => {
                    let c = Channel::parse(c)
                        .filter(|c| *c != Channel::Preview)
                        .ok_or_else(|| format!("--channel {c:?}: alpha, beta or stable"))?;
                    let reach = c.reach();
                    if !reach.iter().all(|r| parsed.channels().contains(r)) {
                        return Err(format!("{v} doesn't belong on the {} channel", c.as_str()));
                    }
                    reach
                }
                // A calendar version goes to alpha unless asked; an
                // old-style one where its stage says.
                None if parsed.stage == Stage::Final => Channel::Alpha.reach(),
                None => parsed.channels(),
            };
            Ok(reach.iter().map(|c| c.as_str()).collect::<Vec<_>>().join("\n"))
        }
        Some("guard") => {
            let v = match flag(args, "--version") {
                Some(v) => v.to_owned(),
                None => workspace_version(root)?,
            };
            let parsed = Version::parse(&v).ok_or_else(|| format!("{v:?} isn't a version"))?;
            if parsed.stage == Stage::Alpha(0) {
                return Err(format!(
                    "{v} is the placeholder before the first alpha: raise `version` in Cargo.toml"
                ));
            }
            for p in values(args, "--previous").map(Path::new).filter(|p| p.exists()) {
                let m: UpdateManifest = serde_json::from_str(&read_text(p)?)
                    .map_err(|e| format!("previous feed {}: {e}", p.display()))?;
                let live = Version::parse(&m.version)
                    .ok_or_else(|| format!("{}: {:?} isn't a version", p.display(), m.version))?;
                if parsed <= live {
                    return Err(format!(
                        "{v} is not newer than {live} on the {} feed: raise `version` in Cargo.toml",
                        m.channel.as_str()
                    ));
                }
            }
            Ok(format!("{v} can be released"))
        }
        Some("verify") => {
            let dir = args.get(1).filter(|a| !a.starts_with("--")).ok_or(USAGE)?;
            let carried = values(args, "--carried")
                .map(|p| UpdatePlatform::parse(p).ok_or_else(|| format!("unknown platform {p:?}")))
                .collect::<Res<Vec<_>>>()?;
            let m = verify(Path::new(dir), &public_key(args)?, &carried)?;
            Ok(format!(
                "ok: {} {} build {}, {} file(s) match",
                m.channel.as_str(),
                m.version,
                m.build,
                m.assets.len()
            ))
        }
        _ => Err(USAGE.into()),
    }
}

/// Every value given after `name` (a flag that may repeat).
fn values<'a>(args: &'a [String], name: &'a str) -> impl Iterator<Item = &'a str> {
    args.windows(2).filter(move |w| w[0] == name).map(|w| w[1].as_str())
}

fn parse_asset(s: &str) -> Res<(UpdatePlatform, PathBuf)> {
    let (p, path) = s.split_once('=').ok_or_else(|| format!("--asset {s:?}: expected PLATFORM=PATH"))?;
    let platform = UpdatePlatform::parse(p).ok_or_else(|| format!("unknown platform {p:?}"))?;
    Ok((platform, PathBuf::from(path)))
}

fn read_text(path: &Path) -> Res<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn io<T>(path: &Path, r: std::io::Result<T>) -> Res<T> {
    r.map_err(|e| format!("{}: {e}", path.display()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn workspace_version(root: &Path) -> Res<String> {
    read_text(&root.join("Cargo.toml"))?
        .lines()
        .find_map(|l| l.strip_prefix("version = \"")?.strip_suffix('"').map(str::to_owned))
        .ok_or_else(|| "no version in Cargo.toml".into())
}

/// Writes the public key file and returns (public, secret), both base64.
pub fn keygen(public_out: &Path, force: bool) -> Res<(String, String)> {
    if !force && fs::read_to_string(public_out).is_ok_and(|t| key_line(&t).is_some()) {
        return Err(format!(
            "{} already holds a key; installed builds trust it. Pass --force only if that key is lost.",
            public_out.display()
        ));
    }
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| format!("no randomness: {e}"))?;
    let key = SigningKey::from_bytes(&seed);
    let public = B64.encode(key.verifying_key().as_bytes());
    let text = format!(
        "# Steps's update key (Ed25519, public half). Builds trust update feeds\n\
         # signed with its secret half, which lives only in the Mac's Keychain\n\
         # (`just update-keygen`). Replacing it strands every installed build.\n\
         {public}\n"
    );
    if let Some(dir) = public_out.parent() {
        io(dir, fs::create_dir_all(dir))?;
    }
    io(public_out, fs::write(public_out, text))?;
    Ok((public, B64.encode(seed)))
}

/// The manifest's notes and history (`crate::changes`).
#[derive(Default)]
pub struct WhatsNew {
    pub notes: UpdateNotes,
    pub history: Vec<BuildChanges>,
}

#[allow(clippy::too_many_arguments)] // each one is a part of the feed
pub fn write_manifest(
    out: &Path,
    channel: Channel,
    version: &str,
    build: u64,
    commit: Option<String>,
    files: &[(UpdatePlatform, PathBuf)],
    carried: &[UpdateAsset],
    whats_new: WhatsNew,
) -> Res<UpdateManifest> {
    let WhatsNew { notes, history } = whats_new;
    let parsed = Version::parse(version).ok_or_else(|| format!("version {version:?}"))?;
    if channel != Channel::Preview && !parsed.channels().contains(&channel) {
        return Err(format!("{version} doesn't belong on the {} channel", channel.as_str()));
    }
    if out.exists() {
        io(out, fs::remove_dir_all(out))?;
    }
    io(out, fs::create_dir_all(out))?;
    let mut assets: Vec<UpdateAsset> = Vec::new();
    for (platform, path) in files {
        if assets.iter().any(|a| a.platform == *platform) {
            return Err(format!("two files for {}", platform.as_str()));
        }
        let bytes = io(path, fs::read(path))?;
        let stem = match platform {
            UpdatePlatform::Content => "steps-content",
            UpdatePlatform::MacosArm64 | UpdatePlatform::MacosX86_64 => "Steps",
            UpdatePlatform::AndroidArm64 => "steps",
        };
        // Preview builds all share one version, so the build number keeps
        // each file name unique: replacing a live file mid-upload would hand
        // devices still holding the old feed a 404 or a hash mismatch.
        let file = match channel {
            Channel::Preview => {
                format!("{stem}-{version}-b{build}-{}.{}", platform.as_str(), platform.extension())
            }
            _ => format!("{stem}-{version}-{}.{}", platform.as_str(), platform.extension()),
        };
        let dest = out.join(&file);
        io(&dest, fs::write(&dest, &bytes))?;
        assets.push(UpdateAsset {
            platform: *platform,
            file,
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            version_code: (*platform == UpdatePlatform::AndroidArm64).then(|| version_code(build)),
            build: None,
        });
    }
    for a in carried {
        if assets.iter().any(|b| b.platform == a.platform) {
            return Err(format!("{} is both carried and given", a.platform.as_str()));
        }
        assets.push(a.clone());
    }
    // The look in the pack fits the app this feed carries: a carried app is
    // older than the feed, and its screens are what the stylesheets style.
    let ui_build = assets.iter().any(|a| a.platform == UpdatePlatform::Content).then(|| {
        let app = assets.iter().filter(|a| a.platform != UpdatePlatform::Content);
        app.map(|a| a.build.unwrap_or(build)).min().unwrap_or(build)
    });
    let m = UpdateManifest {
        format: UPDATE_FORMAT,
        channel,
        version: version.to_owned(),
        build,
        commit,
        published_at: now_rfc3339(),
        notes,
        assets,
        history,
        ui_build,
    };
    let path = out.join(MANIFEST_FILE);
    let json = serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?;
    io(&path, fs::write(&path, json))?;
    Ok(m)
}

/// Signs `update.json` and checks the signature against `public` before
/// writing it, so a feed is never signed with a key the app doesn't trust.
pub fn sign(dir: &Path, secret_b64: &str, public: &str) -> Res<()> {
    let seed: [u8; 32] = B64
        .decode(secret_b64.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| format!("{SECRET_ENV} isn't a base64 32-byte key"))?;
    let key = SigningKey::from_bytes(&seed);
    let path = dir.join(MANIFEST_FILE);
    let json = fs::read(&path).map_err(|_| format!("no {}; run `update manifest` first", path.display()))?;
    let sig = B64.encode(key.sign(&json).to_bytes());
    let trusted = parse_public_key(public).map_err(|e| e.to_string())?;
    if trusted != key.verifying_key() {
        return Err(format!(
            "this secret doesn't belong to the public key the app trusts ({RELEASE_KEY_FILE})"
        ));
    }
    verify_manifest(&json, &sig, &trusted).map_err(|e| e.to_string())?;
    let out = dir.join(SIGNATURE_FILE);
    io(&out, fs::write(&out, format!("{sig}\n")))
}

/// Checks a feed folder as the app would. A `carried` platform's file stays
/// on the release (`--carry`), so it is not in `dir`; its row must still be
/// from an older build, or it was not carried at all.
pub fn verify(dir: &Path, public: &str, carried: &[UpdatePlatform]) -> Res<UpdateManifest> {
    let key = parse_public_key(public).map_err(|e| e.to_string())?;
    let json = io(&dir.join(MANIFEST_FILE), fs::read(dir.join(MANIFEST_FILE)))?;
    let sig = read_text(&dir.join(SIGNATURE_FILE))?;
    let m = verify_manifest(&json, &sig, &key).map_err(|e| e.to_string())?;
    for p in carried {
        let a = m.asset(*p).ok_or_else(|| format!("--carried: no {} in the feed", p.as_str()))?;
        if m.build_of(a) >= m.build {
            return Err(format!("--carried: {} is from this build, not carried", a.file));
        }
    }
    for a in m.assets.iter().filter(|a| !carried.contains(&a.platform)) {
        let path = dir.join(&a.file);
        let bytes = io(&path, fs::read(&path))?;
        check_download(a, bytes.len() as u64, &sha256_hex(&bytes)).map_err(|e| format!("{}: {e}", a.file))?;
        // What a device checks next: a pack devices would refuse is broken.
        if a.platform == UpdatePlatform::Content {
            let pack = steps_core::pack::Pack::decode(&bytes).map_err(|e| format!("{}: {e}", a.file))?;
            if let Some(first) = pack.problems().first() {
                return Err(format!("{}: devices would refuse it: {first}", a.file));
            }
        }
    }
    Ok(m)
}

fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests;
