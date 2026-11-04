//! A single last-known-good content pack, promoted only after Steps has
//! successfully run on it. Backups are never created just after a download.
//!
//! The backup file is addressed by its SHA-256. Publishing the small JSON
//! pointer last preserves the previous backup if Steps closes mid-copy.

use super::*;

const GOOD_INFO: &str = "content.good.json";

pub(super) fn good_file(dir: &Path, sha: &str) -> Option<PathBuf> {
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(dir.join(format!("content.good.{}.pack", sha.to_ascii_lowercase())))
}

fn good_meta(dir: &Path) -> Option<PackInfo> {
    serde_json::from_slice(&fs::read(dir.join(GOOD_INFO)).ok()?).ok()
}

fn fits_app(meta: &PackInfo, app: &BuildInfo) -> bool {
    Version::parse(&meta.version).is_some_and(|v| (v, meta.build) >= (app.version, app.build))
}

/// Save the pack that the running UI has already opened and survived. The
/// content.starts marker is cleared by the caller only after this attempt.
/// Backups are not used when a newer APK made their content obsolete.
pub(super) fn save_settled(dir: &Path, app: &BuildInfo) -> Result<(), String> {
    if !dir.join(PACK_STARTS).exists() {
        return Ok(());
    }
    let Some(meta) = pack_in_use(dir, app) else {
        return Ok(());
    };
    if is_rejected(dir, &meta.sha256) {
        return Ok(());
    }
    let file = good_file(dir, &meta.sha256).ok_or("invalid content digest")?;
    let bytes = fs::read(dir.join(PACK_FILE)).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_PACK_BYTES || sha256_hex(&bytes) != meta.sha256.to_ascii_lowercase() {
        return Err("the settled content pack is not intact".into());
    }
    let pack = Pack::decode(&bytes).map_err(|e| e.to_string())?;
    if let Some(first) = pack.problems().first() {
        return Err(format!("the settled content pack isn't sound: {first}"));
    }
    if let Some(bad) = crate::look::unsafe_sheet(&pack.look) {
        return Err(format!("settled stylesheet {bad} isn't allowed"));
    }
    let previous = good_meta(dir);
    let intact = fs::read(&file).is_ok_and(|old| sha256_hex(&old).eq_ignore_ascii_case(&meta.sha256));
    if !intact {
        let tmp = dir.join(format!("content.good.{}.part", meta.sha256.to_ascii_lowercase()));
        fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    }
    // Commit the pointer only after the content-addressed file is ready.
    // If killed mid-copy, the old pointer and file remain usable.
    write_json(dir, GOOD_INFO, &meta)?;
    if let Some(previous) = previous
        && !previous.sha256.eq_ignore_ascii_case(&meta.sha256)
        && let Some(old) = good_file(dir, &previous.sha256)
    {
        let _ = fs::remove_file(old);
    }
    Ok(())
}

/// Restore the last pack known to have survived startup, but only if its
/// version, checksum, schema and stylesheet security still fit this app.
/// Failure (including disk-full) leaves the embedded pack available.
pub(super) fn restore_last_good(dir: &Path, app: &BuildInfo) -> Option<Pack> {
    let meta = good_meta(dir)?;
    if !fits_app(&meta, app) || is_rejected(dir, &meta.sha256) {
        return None;
    }
    let file = good_file(dir, &meta.sha256)?;
    let bytes = fs::read(file).ok()?;
    if bytes.len() as u64 > MAX_PACK_BYTES || !sha256_hex(&bytes).eq_ignore_ascii_case(&meta.sha256) {
        report!("last good content pack changed on disk; using built-in content");
        return None;
    }
    let mut pack = Pack::decode(&bytes).ok()?;
    if let Some(first) = pack.problems().first() {
        report!("last good content pack isn't sound ({first}); using built-in content");
        return None;
    }
    // Stylesheets are tied to an exact app build; the content itself can
    // remain compatible with a different app build.
    let fits_look = Version::parse(&meta.version) == Some(app.version) && meta.ui_build == Some(app.build);
    if let Some(bad) = crate::look::unsafe_sheet(&pack.look) {
        report!("last good content pack has unsafe stylesheet {bad}; using built-in content");
        return None;
    }
    if !fits_look {
        pack.look.clear();
    }

    let tmp = dir.join(format!("{PACK_FILE}.part"));
    let installed = fs::write(&tmp, &bytes)
        .and_then(|()| fs::rename(&tmp, dir.join(PACK_FILE)))
        .map_err(|e| e.to_string())
        .and_then(|()| write_json(dir, PACK_INFO, &meta));
    if let Err(error) = installed {
        // The in-memory verified backup can still be studied this session.
        report!("last good content restored for this session only: {error}");
    } else {
        report!("restored the last good content pack ({})", meta.build);
    }
    Some(pack)
}
