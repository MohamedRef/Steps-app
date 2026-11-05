//! Device registry, kept in the account's Durable Object next to the sync
//! hub, so a remote sign-out and closing that device's socket happen in one
//! place.

use steps_protocol::cloud::{DeviceInfo, DeviceSummary, Platform};

use crate::sql::{Sql, SqlError, Value, col};
use crate::tokens::REFRESH_TTL_MS;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    model TEXT,
    platform TEXT NOT NULL,
    email TEXT,
    signed_in_ms INTEGER NOT NULL,
    last_seen_ms INTEGER NOT NULL,
    last_sync_ms INTEGER,
    refresh_hash TEXT,
    refresh_expires_ms INTEGER,
    prev_hash TEXT,
    rotated_ms INTEGER,
    revoked_ms INTEGER
)";

/// Creates the table. Safe to call on every start. Later schema changes go
/// in as numbered steps through [`crate::sql::migrate`].
pub fn migrate(db: &impl Sql) -> Result<(), SqlError> {
    db.exec_batch(SCHEMA)?;
    crate::sql::migrate(
        db,
        "devices_schema",
        &[
            "ALTER TABLE devices ADD COLUMN retired_hash TEXT",
            // Signed in with the username and password (not Google): such a
            // device must give the current password to change it. Devices
            // from before this column count as password ones, to be safe: a
            // Google one signs in with Google again to be trusted (#541).
            // `register` always sets it for a new sign-in.
            "ALTER TABLE devices ADD COLUMN by_password INTEGER NOT NULL DEFAULT 1",
            // Whether `by_password` was recorded at sign-in (1) or is only
            // the safe guess for a device from before it (0). A guess still
            // asks for the current password, but a password change signs
            // out only devices known to use it, so his Google devices from
            // before the column stay signed in (#608).
            "ALTER TABLE devices ADD COLUMN sign_in_known INTEGER NOT NULL DEFAULT 0",
        ],
    )
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RefreshError {
    #[error("unknown device or token")]
    Unknown,
    #[error("device was signed out")]
    Revoked,
    #[error("refresh token expired")]
    Expired,
    /// A refresh token came back after the device had already used the one
    /// that replaced it: someone may have copied it. The device is signed
    /// out to be safe.
    #[error("refresh token reused")]
    Reused,
    #[error(transparent)]
    Sql(#[from] SqlError),
}

fn platform_str(p: Platform) -> &'static str {
    match p {
        Platform::Android => "android",
        Platform::Macos => "macos",
        Platform::Web => "web",
        Platform::Other => "other",
    }
}

fn platform_from(s: &str) -> Platform {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).unwrap_or(Platform::Other)
}

/// Keeps user-supplied labels short and printable.
fn clean_label(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_owned()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refreshed {
    pub expires_ms: u64,
    pub email: String,
}

pub struct Devices<'a, S: Sql> {
    db: &'a S,
}

impl<'a, S: Sql> Devices<'a, S> {
    /// Creates the tables if needed, then opens.
    pub fn new(db: &'a S) -> Result<Self, SqlError> {
        migrate(db)?;
        Ok(Self::open(db))
    }

    /// Opens without touching the schema (the Durable Object migrates once at start).
    pub fn open(db: &'a S) -> Self {
        Self { db }
    }

    /// Records a sign-in: creates the device or refreshes its details, and
    /// clears an earlier remote sign-out. Returns the refresh token's expiry.
    pub fn register(
        &self,
        info: &DeviceInfo,
        email: &str,
        refresh_hash: &str,
        now_ms: u64,
    ) -> Result<u64, SqlError> {
        let expires = now_ms + REFRESH_TTL_MS;
        let name = clean_label(&info.name, 60);
        let name = if name.is_empty() { info.device_id.clone() } else { name };
        self.db.exec(
            "INSERT INTO devices (device_id, name, model, platform, email, signed_in_ms, last_seen_ms,
                                  refresh_hash, refresh_expires_ms, prev_hash, rotated_ms, revoked_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
             ON CONFLICT (device_id) DO UPDATE SET
                name = excluded.name, model = excluded.model, platform = excluded.platform,
                email = excluded.email, signed_in_ms = excluded.signed_in_ms,
                last_seen_ms = excluded.last_seen_ms, refresh_hash = excluded.refresh_hash,
                refresh_expires_ms = excluded.refresh_expires_ms,
                prev_hash = NULL, retired_hash = NULL, rotated_ms = NULL, revoked_ms = NULL",
            &[
                info.device_id.as_str().into(),
                name.into(),
                info.model.as_deref().map(|m| clean_label(m, 60)).into(),
                platform_str(info.platform).into(),
                email.into(),
                now_ms.into(),
                now_ms.into(),
                refresh_hash.into(),
                expires.into(),
            ],
        )?;
        Ok(expires)
    }

    /// Rotates the refresh token. Returns the new token's expiry and the
    /// email the device signed in with (to re-check the allowlist).
    ///
    /// A rotation commits when the device first presents the new token (#258).
    /// Until then the previous one keeps working, however late, so a lost
    /// response or a token the device couldn't save never signs it out. Once
    /// the new token is used, the old one is retired, and presenting a retired
    /// token signs the device out.
    pub fn refresh(
        &self,
        device: &str,
        presented_hash: &str,
        new_hash: &str,
        now_ms: u64,
    ) -> Result<Refreshed, RefreshError> {
        let rows = self.db.exec(
            "SELECT refresh_hash, refresh_expires_ms, prev_hash, retired_hash, revoked_ms, email
             FROM devices WHERE device_id = ?",
            &[device.into()],
        )?;
        let row = rows.first().ok_or(RefreshError::Unknown)?;
        if !matches!(col(row, 4, "revoked_ms")?, Value::Null) {
            return Err(RefreshError::Revoked);
        }
        let current = col(row, 0, "refresh_hash")?.as_str();
        let expires = col(row, 1, "refresh_expires_ms")?.as_u64().unwrap_or(0);
        let prev = col(row, 2, "prev_hash")?.as_str();
        let retired = col(row, 3, "retired_hash")?.as_str();
        let email = col(row, 5, "email")?.as_str().unwrap_or_default().to_owned();

        // The current token commits the last rotation: the previous one retires.
        // The previous token means the device never got (or kept) the current
        // one: it gets a new one, and the previous stays valid.
        let commit = if current == Some(presented_hash) {
            true
        } else if prev == Some(presented_hash) {
            false
        } else if retired == Some(presented_hash) {
            self.revoke(device, now_ms)?;
            return Err(RefreshError::Reused);
        } else {
            return Err(RefreshError::Unknown);
        };
        if expires <= now_ms {
            return Err(RefreshError::Expired);
        }

        let new_expires = now_ms + REFRESH_TTL_MS;
        let sql = if commit {
            "UPDATE devices SET retired_hash = prev_hash, prev_hash = refresh_hash, refresh_hash = ?,
                                refresh_expires_ms = ?, rotated_ms = ?, last_seen_ms = ?
             WHERE device_id = ?"
        } else {
            "UPDATE devices SET refresh_hash = ?, refresh_expires_ms = ?, rotated_ms = ?, last_seen_ms = ?
             WHERE device_id = ?"
        };
        self.db
            .exec(sql, &[new_hash.into(), new_expires.into(), now_ms.into(), now_ms.into(), device.into()])?;
        Ok(Refreshed { expires_ms: new_expires, email })
    }

    /// Signs a device out. Returns false if it doesn't exist.
    pub fn revoke(&self, device: &str, now_ms: u64) -> Result<bool, SqlError> {
        let rows = self.db.exec(
            "UPDATE devices SET revoked_ms = COALESCE(revoked_ms, ?), refresh_hash = NULL, prev_hash = NULL,
                                retired_hash = NULL
             WHERE device_id = ? RETURNING device_id",
            &[now_ms.into(), device.into()],
        )?;
        Ok(!rows.is_empty())
    }

    pub fn is_active(&self, device: &str) -> Result<bool, SqlError> {
        let rows = self
            .db
            .exec("SELECT 1 FROM devices WHERE device_id = ? AND revoked_ms IS NULL", &[device.into()])?;
        Ok(!rows.is_empty())
    }

    /// Notes how the device signed in: with the username and password, or
    /// with Google. Called after every [`Self::register`].
    pub fn set_by_password(&self, device: &str, by_password: bool) -> Result<(), SqlError> {
        self.db.exec(
            "UPDATE devices SET by_password = ?, sign_in_known = 1 WHERE device_id = ?",
            &[u64::from(by_password).into(), device.into()],
        )?;
        Ok(())
    }

    /// Signs out every device that signed in with the username and password,
    /// except `keep`: after the password changes or goes, the old one opens
    /// nothing (#540). Devices whose sign-in is unknown (from before
    /// `by_password`) stay (#608). Returns the devices signed out.
    pub fn revoke_password_sessions(&self, keep: &str, now_ms: u64) -> Result<Vec<String>, SqlError> {
        let rows = self.db.exec(
            "UPDATE devices SET revoked_ms = ?, refresh_hash = NULL, prev_hash = NULL, retired_hash = NULL
             WHERE by_password = 1 AND sign_in_known = 1 AND revoked_ms IS NULL AND device_id != ?
             RETURNING device_id",
            &[now_ms.into(), keep.into()],
        )?;
        Ok(rows.into_iter().filter_map(|r| r.into_iter().next()).filter_map(Value::into_string).collect())
    }

    /// Whether the device signed in with the username and password.
    pub fn by_password(&self, device: &str) -> Result<bool, SqlError> {
        let rows = self.db.exec("SELECT by_password FROM devices WHERE device_id = ?", &[device.into()])?;
        Ok(rows.first().and_then(|r| r.first()).and_then(Value::as_u64).unwrap_or(0) != 0)
    }

    /// The email a device signed in with, while it is signed in.
    pub fn email(&self, device: &str) -> Result<Option<String>, SqlError> {
        let rows = self
            .db
            .exec("SELECT email FROM devices WHERE device_id = ? AND revoked_ms IS NULL", &[device.into()])?;
        Ok(rows.into_iter().next().and_then(|r| r.into_iter().next()).and_then(Value::into_string))
    }

    /// Notes activity; `synced` when ops moved in either direction.
    pub fn touch(&self, device: &str, now_ms: u64, synced: bool) -> Result<(), SqlError> {
        let id: Value = device.into();
        if synced {
            self.db.exec(
                "UPDATE devices SET last_seen_ms = ?, last_sync_ms = ? WHERE device_id = ?",
                &[now_ms.into(), now_ms.into(), id],
            )?;
        } else {
            self.db.exec("UPDATE devices SET last_seen_ms = ? WHERE device_id = ?", &[now_ms.into(), id])?;
        }
        Ok(())
    }

    /// Everything the Devices list shows, most recently seen first.
    pub fn list(&self, current: &str) -> Result<Vec<DeviceSummary>, SqlError> {
        let rows = self.db.exec(
            "SELECT device_id, name, model, platform, signed_in_ms, last_seen_ms, last_sync_ms, revoked_ms
             FROM devices ORDER BY last_seen_ms DESC",
            &[],
        )?;
        rows.into_iter()
            .map(|row| {
                let id = col(&row, 0, "device_id")?.as_str().unwrap_or_default().to_owned();
                Ok(DeviceSummary {
                    current: id == current,
                    device_id: id,
                    name: col(&row, 1, "name")?.as_str().unwrap_or_default().to_owned(),
                    model: col(&row, 2, "model")?.as_str().map(str::to_owned),
                    platform: platform_from(col(&row, 3, "platform")?.as_str().unwrap_or_default()),
                    signed_in_at_ms: col(&row, 4, "signed_in_ms")?.as_u64().unwrap_or(0),
                    last_seen_ms: col(&row, 5, "last_seen_ms")?.as_u64().unwrap_or(0),
                    last_sync_ms: col(&row, 6, "last_sync_ms")?.as_u64(),
                    revoked: !matches!(col(&row, 7, "revoked_ms")?, Value::Null),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::testing::MemSql;

    const NOW: u64 = 1_793_011_856_000;

    fn info(id: &str, name: &str, platform: Platform) -> DeviceInfo {
        DeviceInfo { device_id: id.into(), name: name.into(), model: Some("SM-S928B".into()), platform }
    }

    fn id(s: &str) -> &str {
        s
    }

    #[test]
    fn sign_in_list_and_remote_sign_out() {
        let db = MemSql::new();
        let d = Devices::new(&db).unwrap();
        d.register(&info("s24u", "Galaxy S24 Ultra", Platform::Android), "me@example.com", "h1", NOW)
            .unwrap();
        d.register(&info("tab", "Galaxy Tab S11", Platform::Android), "me@example.com", "h2", NOW + 10)
            .unwrap();
        d.touch(id("s24u"), NOW + 20, true).unwrap();

        let list = d.list(id("tab")).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].device_id, id("s24u"));
        assert_eq!(list[0].last_sync_ms, Some(NOW + 20));
        assert!(!list[0].current && list[1].current);
        assert_eq!(list[1].last_sync_ms, None);

        assert!(d.revoke(id("s24u"), NOW + 30).unwrap());
        assert!(!d.is_active(id("s24u")).unwrap());
        assert!(d.is_active(id("tab")).unwrap());
        assert_eq!(d.refresh(id("s24u"), "h1", "h3", NOW + 40), Err(RefreshError::Revoked));
        assert!(d.list(id("tab")).unwrap()[0].revoked);
        assert!(!d.revoke(id("ghost"), NOW).unwrap());
        assert_eq!(d.email("s24u").unwrap(), None, "signed out");
        assert_eq!(d.email("tab").unwrap().as_deref(), Some("me@example.com"));

        // Signing in again on the phone brings it back.
        d.register(&info("s24u", "Galaxy S24 Ultra", Platform::Android), "me@example.com", "h4", NOW + 50)
            .unwrap();
        assert!(d.is_active(id("s24u")).unwrap());
    }

    #[test]
    fn a_password_change_signs_out_the_other_password_sessions() {
        let db = MemSql::new();
        let d = Devices::new(&db).unwrap();
        for (dev, by_password) in [("phone", false), ("pc", true), ("laptop", true), ("tab", false)] {
            d.register(&info(dev, dev, Platform::Other), "me@example.com", dev, NOW).unwrap();
            d.set_by_password(dev, by_password).unwrap();
        }
        assert_eq!(d.revoke_password_sessions("laptop", NOW + 1).unwrap(), ["pc"]);
        assert!(!d.is_active("pc").unwrap());
        assert_eq!(d.refresh("pc", "pc", "pc2", NOW + 2), Err(RefreshError::Revoked));
        for dev in ["phone", "laptop", "tab"] {
            assert!(d.is_active(dev).unwrap(), "{dev} stays");
        }
        assert_eq!(d.revoke_password_sessions("phone", NOW + 3).unwrap(), ["laptop"]);
        assert!(d.revoke_password_sessions("phone", NOW + 4).unwrap().is_empty());
    }

    #[test]
    fn devices_from_before_the_column_count_as_password_sessions() {
        let db = MemSql::new();
        db.exec_batch(SCHEMA).unwrap();
        crate::sql::migrate(&db, "devices_schema", &["ALTER TABLE devices ADD COLUMN retired_hash TEXT"])
            .unwrap();
        db.exec_batch(
            "INSERT INTO devices (device_id, name, platform, email, signed_in_ms, last_seen_ms)
             VALUES ('old-pc', 'PC', 'web', 'me@example.com', 1, 1)",
        )
        .unwrap();
        let d = Devices::new(&db).unwrap(); // adds by_password
        assert!(d.by_password("old-pc").unwrap(), "how it signed in is unknown: ask for the password");
        // But a password change doesn't sign it out: it may be a Google one (#608).
        d.register(&info("pc", "PC", Platform::Web), "me@example.com", "h3", NOW).unwrap();
        d.set_by_password("pc", true).unwrap();
        assert!(d.revoke_password_sessions("pc", NOW + 1).unwrap().is_empty());
        assert!(d.is_active("old-pc").unwrap());
        d.register(&info("new-phone", "Phone", Platform::Android), "me@example.com", "h2", NOW).unwrap();
        d.set_by_password("new-phone", false).unwrap();
        assert!(!d.by_password("new-phone").unwrap(), "a fresh Google sign-in is trusted");
    }

    #[test]
    fn refresh_rotates_and_detects_reuse() {
        let db = MemSql::new();
        let d = Devices::new(&db).unwrap();
        d.register(&info("mac", "MacBook Air", Platform::Macos), "me@example.com", "h1", NOW).unwrap();

        assert_eq!(d.refresh(id("mac"), "wrong", "x", NOW), Err(RefreshError::Unknown));
        let r = d.refresh(id("mac"), "h1", "h2", NOW + 1000).unwrap();
        assert_eq!(r, Refreshed { expires_ms: NOW + 1000 + REFRESH_TTL_MS, email: "me@example.com".into() });
        // Lost response (or a token the device couldn't save): the old token
        // still works hours later, as long as h2 was never used (#258).
        d.refresh(id("mac"), "h1", "h3", NOW + 3_600_000).unwrap();
        // h2 never reached the device, so it is simply gone.
        assert_eq!(d.refresh(id("mac"), "h2", "x", NOW + 3_600_001), Err(RefreshError::Unknown));
        // Using h3 commits it: h1 retires, and h3 stays valid as the previous token.
        d.refresh(id("mac"), "h3", "h4", NOW + 3_700_000).unwrap();
        d.refresh(id("mac"), "h3", "h5", NOW + 3_800_000).unwrap();
        // A retired token coming back signs the device out.
        assert_eq!(d.refresh(id("mac"), "h1", "h6", NOW + 3_900_000), Err(RefreshError::Reused));
        assert!(!d.is_active(id("mac")).unwrap());
    }

    #[test]
    fn refresh_tokens_expire() {
        let db = MemSql::new();
        let d = Devices::new(&db).unwrap();
        d.register(&info("mac", "MacBook Air", Platform::Macos), "me@example.com", "h1", NOW).unwrap();
        assert_eq!(d.refresh(id("mac"), "h1", "h2", NOW + REFRESH_TTL_MS), Err(RefreshError::Expired));
    }

    #[test]
    fn labels_are_cleaned() {
        let db = MemSql::new();
        let d = Devices::new(&db).unwrap();
        d.register(&info("x", "  \u{7}Tab\n ", Platform::Other), "me@example.com", "h", NOW).unwrap();
        d.register(&info("y", "", Platform::Other), "me@example.com", "h", NOW).unwrap();
        let list = d.list(id("x")).unwrap();
        let name = |dev: &str| list.iter().find(|s| s.device_id == dev).unwrap().name.clone();
        assert_eq!(name("x"), "Tab");
        assert_eq!(name("y"), "y");
    }
}
