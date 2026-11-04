//! Where Steps keeps its database and writes exports.
// adapted from kabar-app main: crates/app/src/platform/mod.rs (no JNI here:
// Android paths come from the package name)

use std::path::PathBuf;

/// The app's private data directory. `STEPS_DATA_DIR` overrides it
/// (previews, screenshots, a fresh profile).
pub fn data_dir() -> Result<PathBuf, String> {
    crate::prefs::data_dir().ok_or_else(|| "no home directory".to_string())
}

/// Where an export goes: `~/Downloads` on the Mac; on Android the app's
/// folder in shared storage (`Android/data/app.steps/files`, reachable over
/// USB and from the Files app), else the private data directory.
pub fn export_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        let shared = PathBuf::from(format!("/storage/emulated/0/Android/data/{}/files", android_package()?));
        if std::fs::create_dir_all(&shared).is_ok() {
            return Ok(shared);
        }
        data_dir()
    }
    #[cfg(not(target_os = "android"))]
    {
        // Unit tests export into their own scratch folder.
        if cfg!(test) {
            return data_dir();
        }
        match directories::UserDirs::new() {
            Some(u) => Ok(u.download_dir().map_or_else(|| u.home_dir().to_path_buf(), PathBuf::from)),
            None => data_dir(),
        }
    }
}

#[cfg_attr(test, allow(dead_code))]
/// Android only: a file in the app's `no_backup` folder, which Auto Backup
/// and device-to-device transfers never copy (`store::install`).
pub fn install_marker() -> Option<PathBuf> {
    if !cfg!(target_os = "android") || std::env::var_os("STEPS_DATA_DIR").is_some() {
        return None;
    }
    let files = data_dir().ok()?;
    Some(files.parent()?.join("no_backup").join("steps-install"))
}

#[cfg(target_os = "android")]
fn android_package() -> Result<String, String> {
    let cmdline = std::fs::read("/proc/self/cmdline").map_err(|e| e.to_string())?;
    let name = cmdline.split(|b| *b == 0).next().unwrap_or_default();
    let name = String::from_utf8_lossy(name).split(':').next().unwrap_or_default().to_string();
    if name.is_empty() { Err("unknown Android package".into()) } else { Ok(name) }
}
