//! Replacing Steps.app on the Mac (`docs/RELEASE.md` §5): unzip the new
//! bundle next to the running one, swap the two with renames (so a failure
//! halfway puts the old one back), and reopen Steps.
//!
//! The running process keeps its already-open binary, so the swap is safe
//! while Steps runs; it only restarts when the learner says so, or at the
//! next launch in automatic mode.
//!
//! Adapted from kabar-app main: crates/app/src/update/bundle.rs.

use std::fs;
use std::path::{Path, PathBuf};

/// The `.app` this binary runs from (`Steps.app/Contents/MacOS/steps`), if any.
pub fn running_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    bundle_of(&exe)
}

fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && app.extension()? == "app")
        .then(|| app.to_path_buf())
}

/// Unzips `zip` with `extract` into a folder beside `bundle`, checks it holds
/// one complete `.app` that passes `check` (the code signature on the Mac),
/// and swaps it in.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn install_zip(
    zip: &Path,
    bundle: &Path,
    extract: impl FnOnce(&Path, &Path) -> Result<(), String>,
    check: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let parent = bundle.parent().ok_or("Steps.app has no folder")?;
    let stage = parent.join(format!(".steps-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).map_err(|e| format!("{}: {e}", stage.display()))?;
    let result = (|| {
        extract(zip, &stage)?;
        let apps: Vec<PathBuf> = fs::read_dir(&stage)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "app"))
            .collect();
        let [new] = apps.as_slice() else {
            return Err(format!("the zip holds {} apps, not one", apps.len()));
        };
        if !new.join("Contents/Info.plist").is_file() || !new.join("Contents/MacOS").is_dir() {
            return Err("the zip's Steps.app is incomplete".into());
        }
        check(new)?;
        swap(bundle, new)
    })();
    let _ = fs::remove_dir_all(&stage);
    result
}

/// Puts `new` where `current` is; on failure the old bundle goes back.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn swap(current: &Path, new: &Path) -> Result<(), String> {
    let parent = current.parent().ok_or("no parent folder")?;
    let old = parent.join(format!(".Steps-old-{}.app", std::process::id()));
    let _ = fs::remove_dir_all(&old);
    fs::rename(current, &old).map_err(|e| format!("{}: {e}", current.display()))?;
    if let Err(e) = fs::rename(new, current) {
        let _ = fs::rename(&old, current);
        return Err(format!("{}: {e}", current.display()));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

/// `ditto -x -k`: the unzip that keeps a bundle's symlinks, modes and
/// signature intact (the feed's zip is made with `ditto -c -k --keepParent`).
#[cfg(target_os = "macos")]
pub fn ditto_extract(zip: &Path, into: &Path) -> Result<(), String> {
    let out = std::process::Command::new("/usr/bin/ditto")
        .arg("-x")
        .arg("-k")
        .arg(zip)
        .arg(into)
        .output()
        .map_err(|e| format!("ditto: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("ditto: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// The new bundle's code signature must be intact, and when the running
/// Steps is signed with a certificate ("Steps Code Signing",
/// `scripts/mac-codesign-cert.sh`), the new one must satisfy the same
/// designated requirement: the same signer, so the Keychain keeps trusting
/// it (the sign-ins it holds) and a second check beyond the hash.
/// Ad-hoc builds (no certificate yet) are only required to be consistent.
#[cfg(target_os = "macos")]
pub fn check_signature(new: &Path, running: &Path) -> Result<(), String> {
    use std::process::Command;
    let codesign = |args: &[&str], path: &Path| {
        Command::new("/usr/bin/codesign").args(args).arg(path).output().map_err(|e| format!("codesign: {e}"))
    };
    let requirement = |path: &Path| -> Option<String> {
        let out = codesign(&["-d", "-r-"], path).ok()?;
        // codesign prints the requirement on stdout or stderr depending on the version.
        let text =
            format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        text.lines().find_map(|l| l.strip_prefix("designated => ")).map(str::to_owned)
    };
    let current = requirement(running).filter(|r| r.contains("certificate"));
    let Some(current) = current else {
        // Not certificate-signed yet: nothing to match against.
        return Ok(());
    };
    let verify = codesign(&["--verify", "--deep", "--strict"], new)?;
    if !verify.status.success() {
        return Err(format!(
            "the new Steps.app's signature is broken: {}",
            String::from_utf8_lossy(&verify.stderr).trim()
        ));
    }
    let req = format!("={current}");
    let same = codesign(&["--verify", "-R", &req], new)?;
    if same.status.success() {
        Ok(())
    } else {
        Err("the new Steps.app isn't signed by the same signer".into())
    }
}

/// Opens `bundle` again once this process has gone, then quits.
#[cfg(target_os = "macos")]
pub fn relaunch(bundle: &Path) -> ! {
    let _ = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("sleep 1; /usr/bin/open -n \"$0\"")
        .arg(bundle)
        .spawn();
    std::process::exit(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_app(dir: &Path, marker: &str) -> PathBuf {
        let app = dir.join("Steps.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/Info.plist"), "<plist/>").unwrap();
        fs::write(app.join("Contents/MacOS/steps"), marker).unwrap();
        app
    }

    #[test]
    fn finds_the_bundle_from_its_binary() {
        let exe = Path::new("/Applications/Steps.app/Contents/MacOS/steps");
        assert_eq!(bundle_of(exe), Some(PathBuf::from("/Applications/Steps.app")));
        assert_eq!(bundle_of(Path::new("/home/me/steps/target/release/steps")), None);
    }

    #[test]
    fn a_test_binary_runs_from_no_bundle() {
        assert_eq!(running_bundle(), None);
    }

    #[test]
    fn a_zip_without_exactly_one_app_or_a_swap_that_fails_keeps_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_app(dir.path(), "old");
        let zip = dir.path().join("x.zip");
        let err = install_zip(&zip, &bundle, |_, _| Ok(()), |_| Ok(())).unwrap_err();
        assert_eq!(err, "the zip holds 0 apps, not one");
        // The new one vanished before the swap: the old one goes back.
        let err = swap(&bundle, &dir.path().join("Gone.app")).unwrap_err();
        assert!(err.contains("Steps.app"), "{err}");
        assert_eq!(fs::read_to_string(bundle.join("Contents/MacOS/steps")).unwrap(), "old");
    }

    #[test]
    fn swaps_in_the_new_app_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("Applications");
        let bundle = fake_app(&apps, "old");
        let staged = tempfile::tempdir().unwrap();
        let zip = dir.path().join("Steps.app.zip");
        fs::write(&zip, b"zip").unwrap();
        install_zip(
            &zip,
            &bundle,
            |z, into| {
                assert_eq!(z, zip);
                let new = fake_app(staged.path(), "new");
                fs::rename(new, into.join("Steps.app")).map_err(|e| e.to_string())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(bundle.join("Contents/MacOS/steps")).unwrap(), "new");
        let left: Vec<_> = fs::read_dir(&apps).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left, ["Steps.app"], "no stage or old copy is left behind");
    }

    #[test]
    fn a_broken_zip_leaves_the_old_app_alone() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_app(dir.path(), "old");
        let zip = dir.path().join("x.zip");
        let err = install_zip(
            &zip,
            &bundle,
            |_, into| fs::create_dir_all(into.join("Steps.app/Contents")).map_err(|e| e.to_string()),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(err.contains("incomplete"), "{err}");
        assert_eq!(fs::read_to_string(bundle.join("Contents/MacOS/steps")).unwrap(), "old");
        assert!(install_zip(&zip, &bundle, |_, _| Err("bad zip".into()), |_| Ok(())).is_err());
        // A bundle that fails the signature check never replaces the old one.
        let staged = tempfile::tempdir().unwrap();
        let err = install_zip(
            &zip,
            &bundle,
            |_, into| {
                let new = fake_app(staged.path(), "new");
                fs::rename(new, into.join("Steps.app")).map_err(|e| e.to_string())
            },
            |_| Err("other signer".into()),
        )
        .unwrap_err();
        assert_eq!(err, "other signer");
        assert_eq!(fs::read_to_string(bundle.join("Contents/MacOS/steps")).unwrap(), "old");
    }

    /// The real unzip, on the Mac CI runner: a bundle zipped the way the
    /// feed is made comes back whole, symlinks included.
    #[cfg(target_os = "macos")]
    #[test]
    fn ditto_round_trip_installs() {
        use std::process::Command;
        let dir = tempfile::tempdir().unwrap();
        let build = tempfile::tempdir().unwrap();
        let new = fake_app(build.path(), "new");
        std::os::unix::fs::symlink("steps", new.join("Contents/MacOS/link")).unwrap();
        let zip = dir.path().join("Steps.app.zip");
        let ok = Command::new("/usr/bin/ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(&new)
            .arg(&zip)
            .status()
            .unwrap();
        assert!(ok.success());
        let bundle = fake_app(&dir.path().join("Applications"), "old");
        install_zip(&zip, &bundle, ditto_extract, |new| check_signature(new, &bundle)).unwrap();
        assert_eq!(fs::read_to_string(bundle.join("Contents/MacOS/steps")).unwrap(), "new");
        assert!(fs::symlink_metadata(bundle.join("Contents/MacOS/link")).unwrap().file_type().is_symlink());
    }
}
