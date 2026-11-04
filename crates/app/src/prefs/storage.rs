//! Atomic publication of the existing device preference file.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::FILE;

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn read(dir: &Path) -> io::Result<String> {
    fs::read_to_string(dir.join(FILE))
}

pub(super) fn write(dir: &Path, text: &str) -> io::Result<()> {
    publish(dir, text, |from, to| fs::rename(from, to))
}

fn publish(dir: &Path, text: &str, rename: impl FnOnce(&Path, &Path) -> io::Result<()>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let temporary =
        dir.join(format!("{FILE}.{}.{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    // Only clean up a temporary file we actually created.
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        rename(&temporary, &dir.join(FILE))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_publication_preserves_the_previous_file_and_removes_temporary() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "text_size=normal").unwrap();
        let result = publish(dir.path(), "text_size=large", |_, _| Err(io::Error::other("publish blocked")));
        assert!(result.is_err());
        assert_eq!(read(dir.path()).unwrap(), "text_size=normal");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        write(dir.path(), "text_size=larger").unwrap();
        assert_eq!(read(dir.path()).unwrap(), "text_size=larger");
    }

    #[test]
    fn a_blocked_folder_can_recover_without_a_partial_preference_file() {
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("device");
        fs::write(&dir, "blocked").unwrap();
        assert!(write(&dir, "theme=paper").is_err());
        fs::remove_file(&dir).unwrap();
        write(&dir, "theme=black").unwrap();
        assert_eq!(read(&dir).unwrap(), "theme=black");
    }
}
