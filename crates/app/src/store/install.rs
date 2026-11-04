//! Spots a database that was copied onto another device: Android Auto
//! Backup, Smart Switch or a device-to-device transfer can clone `steps.db`,
//! and two devices with one `device_id` would stamp the same HLCs and look
//! like one device to Steps Cloud (#228).
//!
//! The device id is also written to a marker in a folder Android never backs
//! up or transfers (`no_backup`). A database that says it was marked but has
//! no matching marker beside it came from somewhere else, so it takes a new
//! device id and starts its sync over. Its rows keep the old id: they were
//! written on the other device.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use super::{Result, new_device_id, sync};

const MARKED: &str = "device.marked";

/// The device id to use: `device`, or a fresh one when this database was
/// cloned from another install.
pub(super) fn check(conn: &Connection, marker: &Path, device: &str) -> Result<String> {
    let marked: Option<String> =
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [MARKED], |r| r.get(0)).optional()?;
    let on_disk = std::fs::read_to_string(marker).ok().map(|s| s.trim().to_string());
    let device = match (on_disk.as_deref(), marked.is_some()) {
        (Some(id), _) if id == device => return Ok(device.to_string()),
        // First run with markers (a new install, or one from before them).
        (None, false) => device.to_string(),
        // Marked elsewhere: a copy.
        _ => {
            let fresh = new_device_id();
            crate::diag::report!("database copied from device {device}; this install is now {fresh}");
            conn.execute("UPDATE meta SET value = ?1 WHERE key = 'device_id'", [&fresh])?;
            // The other device sends its own changes; this one asks for the
            // whole log again under its new name.
            conn.execute("DELETE FROM sync_outbox", [])?;
            sync::forget_position(conn)?;
            // Device-only settings (`local.*`: which device rings reminders,
            // theme, an open paper) belong to the other device (#284).
            conn.execute("DELETE FROM meta WHERE key LIKE 'local.%'", [])?;
            fresh
        }
    };
    if let Some(dir) = marker.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(marker, &device) {
        // Without a marker the next start treats this as a copy again, so
        // leave the database unmarked and try next time.
        crate::diag::report!("install marker {}: {e}", marker.display());
        return Ok(device);
    }
    conn.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, '1')", params![MARKED])?;
    Ok(device)
}

#[cfg(test)]
mod tests {
    use super::super::Store;

    fn dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("steps-install-{}", super::new_device_id()))
    }

    #[test]
    fn a_copied_database_takes_a_new_device_id() {
        let phone = dir();
        let db = phone.join("files/steps.db");
        let marker = phone.join("no_backup/steps-install");
        let id = {
            let s = Store::open_marked(&db, Some(&marker)).unwrap();
            s.put_local_setting("remind_here", &true).unwrap();
            s.append_review(steps_core::review::ReviewEvent {
                item_id: "hb.L01.001".into(),
                grade: steps_core::review::Grade::Good,
                origin: steps_core::review::Origin::Study,
                at_ms: 1,
                took_ms: 1,
                confidence: None,
                reason: None,
            })
            .unwrap();
            s.device()
        };
        // Reopening on the same install keeps the id.
        assert_eq!(Store::open_marked(&db, Some(&marker)).unwrap().device(), id);

        // The backup lands on a new tablet: same database, no marker.
        let tablet = dir();
        let copy = tablet.join("files/steps.db");
        std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
        std::fs::copy(&db, &copy).unwrap();
        let s = Store::open_marked(&copy, Some(&tablet.join("no_backup/steps-install"))).unwrap();
        assert_ne!(s.device(), id);
        assert_eq!(s.reviews().unwrap().len(), 1, "his answers come along");
        assert!(s.outbox(10).unwrap().is_empty(), "the phone sends its own answers");
        assert_eq!(s.last_seq(), 0);
        assert_eq!(s.local_setting::<bool>("remind_here"), None, "the phone keeps ringing, not both");
        // And it stays that new id.
        let again = s.device();
        drop(s);
        assert_eq!(
            Store::open_marked(&copy, Some(&tablet.join("no_backup/steps-install"))).unwrap().device(),
            again
        );
        let _ = std::fs::remove_dir_all(phone);
        let _ = std::fs::remove_dir_all(tablet);
    }

    #[test]
    fn an_install_from_before_markers_keeps_its_id() {
        let d = dir();
        let db = d.join("steps.db");
        let id = Store::open(&db).unwrap().device();
        assert_eq!(Store::open_marked(&db, Some(&d.join("no_backup/m"))).unwrap().device(), id);
        let _ = std::fs::remove_dir_all(d);
    }
}
