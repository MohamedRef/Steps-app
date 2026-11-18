//! Where this device keeps the VQ key. Like the sign-in session
//! (`cloud::session`) it is never in the database, so it is never in a
//! backup, an export or a sync: on the Mac and other desktops a file in the
//! data folder only this user can read, on Android the Keystore through
//! `keyring-core`, in the browser the site's own `localStorage`.
// The vault pattern of crate::cloud::session.

use steps_core::sealed::Key;

/// The key kept here; a stored text that isn't a key reads as none.
pub fn load() -> Option<Key> {
    imp::load().and_then(|text| Key::parse(&text).ok())
}

/// Keeps `key` for the next launch; false when it couldn't be kept.
pub fn save(key: &Key) -> bool {
    imp::save(&key.text())
}

/// Forgets the key on this device.
pub fn clear() {
    imp::clear();
}

/// A file in the data folder, readable by this user only (0600). The Mac's
/// Keychain trusts only the build that saved an item (`cloud::session`).
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
mod imp {
    const FILE: &str = "vq-key.txt";

    fn path() -> Option<std::path::PathBuf> {
        crate::prefs::data_dir().map(|dir| dir.join(FILE))
    }

    pub fn load() -> Option<String> {
        std::fs::read_to_string(path()?).ok()
    }

    pub fn save(text: &str) -> bool {
        let Some(path) = path() else { return false };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        crate::cloud::session::write_private(&path, text.as_bytes())
            .map_err(|e| crate::diag::report!("keeping the VQ key failed: {e}"))
            .is_ok()
    }

    pub fn clear() {
        if let Some(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The Android Keystore, as the session uses it.
#[cfg(target_os = "android")]
mod imp {
    const SERVICE: &str = "app.steps.vq";
    const USER: &str = "key";

    fn entry() -> Option<keyring_core::Entry> {
        crate::cloud::session::init_default_store();
        keyring_core::Entry::new(SERVICE, USER)
            .map_err(|e| crate::diag::report!("no credential store: {e}"))
            .ok()
    }

    pub fn load() -> Option<String> {
        entry()?.get_password().ok()
    }

    pub fn save(text: &str) -> bool {
        let Some(entry) = entry() else { return false };
        entry.set_password(text).map_err(|e| crate::diag::report!("keeping the VQ key failed: {e}")).is_ok()
    }

    pub fn clear() {
        if let Some(entry) = entry() {
            let _ = entry.delete_credential();
        }
    }
}

/// The browser: the site's own storage, which only this origin can read.
#[cfg(target_arch = "wasm32")]
mod imp {
    const KEY: &str = "vq-key";

    pub fn load() -> Option<String> {
        crate::platform::web::read_pref(KEY).filter(|text| !text.is_empty())
    }

    pub fn save(text: &str) -> bool {
        crate::platform::web::write_pref(KEY, text)
    }

    pub fn clear() {
        crate::platform::web::write_pref(KEY, "");
    }
}
