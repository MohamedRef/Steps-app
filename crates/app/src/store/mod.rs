//! The local SQLite store: review events, Mohamed's verdicts and review
//! settings. One file on each device, synced through Steps Cloud when
//! Mohamed signs in (`sync.rs`).
//!
//! Every synced table carries `account_id`, `device_id`, an HLC and a
//! tombstone, so sync can be added without a schema change. Review events
//! are append-only; verdicts and settings are last-writer-wins by HLC.
// adapted from kabar-app main: crates/app/src/store.rs

mod backend;
pub mod export;
pub mod import;
mod install;
pub mod own;
pub mod pages;
mod paths;
pub mod sync;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use steps_core::review::verify::Judgement;
use steps_core::review::{Grade, Hlc, HlcClock, LoggedReview, Origin, ReviewEvent};
use steps_core::srs::ReviewSettings;
use steps_core::zone::Zone;

pub use backend::{LectureProgress, Overview, Review, use_provide_review, use_review};

#[derive(Debug)]
pub enum StoreError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    TooNew(u32),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(e) => write!(f, "database: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
            Self::TooNew(v) => write!(f, "the database is from a newer Steps (schema {v})"),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Numbered migrations; the index + 1 is the schema version.
const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) STRICT;
    -- Append-only: one row per answer, never updated (a tombstone only for sync).
    CREATE TABLE review_events (
        id          TEXT PRIMARY KEY,
        item_id     TEXT NOT NULL,
        grade       TEXT NOT NULL,
        origin      TEXT NOT NULL,
        at_ms       INTEGER NOT NULL,
        took_ms     INTEGER NOT NULL,
        account_id  TEXT,
        device_id   TEXT NOT NULL,
        hlc_wall    INTEGER NOT NULL,
        hlc_counter INTEGER NOT NULL,
        deleted     INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    CREATE INDEX review_events_item ON review_events (item_id);
    -- Mohamed's verdict per item: unverified (no row, or a tombstone),
    -- approved, edited (with his override) or wrong. Last writer wins.
    CREATE TABLE item_status (
        item_id     TEXT PRIMARY KEY,
        status      TEXT NOT NULL,
        value       TEXT NOT NULL,
        account_id  TEXT,
        device_id   TEXT NOT NULL,
        hlc_wall    INTEGER NOT NULL,
        hlc_counter INTEGER NOT NULL,
        deleted     INTEGER NOT NULL DEFAULT 0
    ) STRICT;
    CREATE TABLE settings (
        key         TEXT PRIMARY KEY,
        value       TEXT NOT NULL,
        account_id  TEXT,
        device_id   TEXT NOT NULL,
        hlc_wall    INTEGER NOT NULL,
        hlc_counter INTEGER NOT NULL,
        deleted     INTEGER NOT NULL DEFAULT 0
    ) STRICT;
",
    "
    -- Local writes waiting for Steps Cloud (store/sync.rs), oldest first.
    CREATE TABLE sync_outbox (
        seq   INTEGER PRIMARY KEY AUTOINCREMENT,
        op_id TEXT NOT NULL UNIQUE,
        op    TEXT NOT NULL
    ) STRICT;
",
    "
    -- Mohamed's own cards (steps_core::own::OwnCard as JSON). Last writer wins.
    CREATE TABLE own_cards (
        id          TEXT PRIMARY KEY,
        value       TEXT NOT NULL,
        account_id  TEXT,
        device_id   TEXT NOT NULL,
        hlc_wall    INTEGER NOT NULL,
        hlc_counter INTEGER NOT NULL,
        deleted     INTEGER NOT NULL DEFAULT 0
    ) STRICT;
",
];

const REVIEW_SETTINGS_KEY: &str = "review";

pub struct Store {
    conn: Mutex<Connection>,
    clock: Mutex<HlcClock>,
    /// Where the database lives (`None` in memory).
    pub path: Option<PathBuf>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_marked(path, None)
    }

    /// Opens the database and, with a `marker` in a folder that is never
    /// backed up, gives a database copied from another install its own
    /// device id (`install`).
    pub fn open_marked(path: &Path, marker: Option<&Path>) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn, Some(path.to_path_buf()), marker)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, None, None)
    }

    /// The web build's store: in memory, starting from the snapshot the
    /// browser kept (`platform::web`), and migrated like a file would be.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // web, and tests
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        if !bytes.is_empty() {
            conn.deserialize_read_exact(rusqlite::MAIN_DB, bytes, bytes.len(), false)?;
        }
        Self::init(conn, None, None)
    }

    /// The whole database as bytes, with SQLite's change counter, so the web
    /// build can keep it in the browser only when something changed.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // web, and tests
    pub fn snapshot(&self) -> Result<(u64, Vec<u8>)> {
        let mut conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let changes = conn.total_changes();
        let bytes = conn.serialize(rusqlite::MAIN_DB)?.to_vec();
        Ok((changes, bytes))
    }

    /// SQLite's count of rows changed since this connection opened.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn changes(&self) -> u64 {
        self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner).total_changes()
    }

    fn init(conn: Connection, path: Option<PathBuf>, marker: Option<&Path>) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version as usize > MIGRATIONS.len() {
            return Err(StoreError::TooNew(version));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as u32 + 1)?;
            tx.commit()?;
        }
        let device = match conn
            .query_row("SELECT value FROM meta WHERE key = 'device_id'", [], |r| r.get::<_, String>(0))
            .optional()?
        {
            Some(d) => d,
            None => {
                let d = new_device_id();
                conn.execute("INSERT INTO meta (key, value) VALUES ('device_id', ?1)", [&d])?;
                d
            }
        };
        let device = match marker {
            Some(m) => install::check(&conn, m, &device)?,
            None => device,
        };
        sync::seed(&conn, &device)?;
        sync::check_tables(&conn)?;
        let mut clock = HlcClock::new(device);
        // Start past every stamp already stored, whichever device wrote it.
        if let Some(last) = newest_stamp(&conn)? {
            clock.observe(&last);
        }
        Ok(Self { conn: Mutex::new(conn), clock: Mutex::new(clock), path })
    }

    /// Moves the clock past rows another connection wrote (the sync
    /// thread), so this device's next write sorts after them.
    pub fn observe_stored(&self) -> Result<()> {
        let last = newest_stamp(&self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner))?;
        if let Some(last) = last {
            self.clock.lock().unwrap_or_else(std::sync::PoisonError::into_inner).observe(&last);
        }
        Ok(())
    }

    pub fn device(&self) -> String {
        self.clock.lock().unwrap_or_else(std::sync::PoisonError::into_inner).device().to_string()
    }

    fn tick(&self, now_ms: i64) -> Hlc {
        self.clock.lock().unwrap_or_else(std::sync::PoisonError::into_inner).tick(now_ms)
    }

    /// Appends one answer and returns it as logged.
    pub fn append_review(&self, event: ReviewEvent) -> Result<LoggedReview> {
        let hlc = self.tick(now_ms().max(event.at_ms));
        let logged =
            LoggedReview { id: format!("{}-{}-{}", hlc.device, hlc.wall_ms, hlc.counter), hlc, event };
        // The answer and its outbox entry land together or not at all.
        let mut conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tx = conn.transaction()?;
        insert_review(&tx, &logged)?;
        sync::enqueue(&tx, "review_events", "id", &logged.id, &logged.hlc.device)?;
        tx.commit()?;
        Ok(logged)
    }

    /// Takes one answer back (Undo): its row gets the tombstone, queued for
    /// sync, so every device replays the schedule without it. False when it
    /// was already gone (or never existed): nothing is written twice.
    pub fn retract_review(&self, id: &str) -> Result<bool> {
        let mut conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tx = conn.transaction()?;
        let n = tx.execute("UPDATE review_events SET deleted = 1 WHERE id = ?1 AND deleted = 0", [id])?;
        if n > 0 {
            let device =
                self.clock.lock().unwrap_or_else(std::sync::PoisonError::into_inner).device().to_string();
            sync::enqueue(&tx, "review_events", "id", id, &device)?;
        }
        tx.commit()?;
        Ok(n > 0)
    }

    /// The whole review log, in HLC order. Rows this build can't read are skipped.
    pub fn reviews(&self) -> Result<Vec<LoggedReview>> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stmt = conn.prepare(
            "SELECT id, item_id, grade, origin, at_ms, took_ms, device_id, hlc_wall, hlc_counter
             FROM review_events WHERE deleted = 0
             ORDER BY hlc_wall, hlc_counter, device_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, u32>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, u32>(8)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, item_id, grade, origin, at_ms, took_ms, device, wall_ms, counter) = row?;
            let (Some(grade), Some(origin)) = (Grade::parse(&grade), Origin::parse(&origin)) else {
                continue;
            };
            out.push(LoggedReview {
                id,
                hlc: Hlc { wall_ms, counter, device },
                event: ReviewEvent { item_id, grade, origin, at_ms, took_ms },
            });
        }
        Ok(out)
    }

    /// Every current verdict, by item id.
    pub fn judgements(&self) -> Result<BTreeMap<String, Judgement>> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stmt = conn.prepare("SELECT item_id, value FROM item_status WHERE deleted = 0")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (id, value) = row?;
            if let Ok(j) = serde_json::from_str::<Judgement>(&value) {
                out.insert(id, j);
            }
        }
        Ok(out)
    }

    /// Records a verdict, or clears it (`None`, a tombstone: back to unverified).
    pub fn set_judgement(&self, item_id: &str, judgement: Option<&Judgement>) -> Result<()> {
        let hlc = self.tick(now_ms());
        let (status, value, deleted) = match judgement {
            Some(j) => (j.verdict.as_str(), serde_json::to_string(j)?, false),
            None => ("unverified", "{}".to_string(), true),
        };
        self.put_lww("item_status", "item_id", item_id, Some(status), &value, deleted, &hlc)
    }

    /// A synced setting (last writer wins across devices), by key.
    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.query_row("SELECT value FROM settings WHERE key = ?1 AND deleted = 0", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
    }

    /// Every synced setting whose key starts with `prefix`, in key order.
    /// For records that are one key each, so two devices never overwrite
    /// each other (the bank's finished papers).
    pub fn settings_with_prefix<T: DeserializeOwned>(&self, prefix: &str) -> Vec<(String, T)> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(mut stmt) = conn.prepare(
            "SELECT key, value FROM settings WHERE substr(key, 1, length(?1)) = ?1 AND deleted = 0 ORDER BY key",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([prefix], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
        rows.map(|rows| {
            rows.flatten().filter_map(|(k, v)| Some((k, serde_json::from_str(&v).ok()?))).collect()
        })
        .unwrap_or_default()
    }

    pub fn put_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let hlc = self.tick(now_ms());
        self.put_lww("settings", "key", key, None, &serde_json::to_string(value)?, false, &hlc)
    }

    /// A setting for this device only (never synced, never imported), kept
    /// in `meta` under `local.<key>`.
    pub fn local_setting<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [format!("local.{key}")], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
    }

    pub fn put_local_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner).execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![format!("local.{key}"), serde_json::to_string(value)?],
        )?;
        Ok(())
    }

    pub fn review_settings(&self) -> ReviewSettings {
        self.setting(REVIEW_SETTINGS_KEY).unwrap_or_default()
    }

    pub fn put_review_settings(&self, settings: &ReviewSettings) -> Result<()> {
        self.put_setting(REVIEW_SETTINGS_KEY, settings)
    }

    /// Last-writer-wins upsert: the row changes only when `hlc` is newer
    /// than what is stored.
    #[allow(clippy::too_many_arguments)]
    fn put_lww(
        &self,
        table: &str,
        key_col: &str,
        key: &str,
        status: Option<&str>,
        value: &str,
        deleted: bool,
        hlc: &Hlc,
    ) -> Result<()> {
        let (status_col, status_val, status_set) = match status {
            Some(_) => (", status", ", ?7", ", status = excluded.status"),
            None => ("", "", ""),
        };
        let sql = format!(
            "INSERT INTO {table} ({key_col}, value, device_id, hlc_wall, hlc_counter, deleted{status_col})
             VALUES (?1, ?2, ?3, ?4, ?5, ?6{status_val})
             ON CONFLICT ({key_col}) DO UPDATE SET
                value = excluded.value, device_id = excluded.device_id, hlc_wall = excluded.hlc_wall,
                hlc_counter = excluded.hlc_counter, deleted = excluded.deleted{status_set}
             WHERE (excluded.hlc_wall, excluded.hlc_counter, excluded.device_id)
                 > ({table}.hlc_wall, {table}.hlc_counter, {table}.device_id)"
        );
        let mine =
            hlc.device == self.clock.lock().unwrap_or_else(std::sync::PoisonError::into_inner).device();
        let mut conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        // The row and its outbox entry land together or not at all.
        let tx = conn.transaction()?;
        let base = params![key, value, hlc.device, hlc.wall_ms, hlc.counter, deleted];
        let changed = match status {
            Some(s) => {
                let mut all: Vec<&dyn rusqlite::ToSql> = base.to_vec();
                all.push(&s);
                tx.execute(&sql, all.as_slice())?
            }
            None => tx.execute(&sql, base)?,
        };
        // This device's writes go to Steps Cloud too.
        if changed > 0 && mine {
            sync::enqueue(&tx, table, key_col, key, &hlc.device)?;
        } else if mine {
            // Our clock should always be past every stored stamp (#257).
            crate::diag::report!("a local {table} write lost to a newer stored stamp: {key}");
        }
        tx.commit()?;
        Ok(())
    }
}

/// Inserts a logged event. Duplicates are ignored.
fn insert_review(conn: &rusqlite::Connection, l: &LoggedReview) -> Result<()> {
    let e = &l.event;
    conn.execute(
        "INSERT OR IGNORE INTO review_events
         (id, item_id, grade, origin, at_ms, took_ms, device_id, hlc_wall, hlc_counter)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            l.id,
            e.item_id,
            e.grade.as_str(),
            e.origin.as_str(),
            e.at_ms,
            e.took_ms,
            l.hlc.device,
            l.hlc.wall_ms,
            l.hlc.counter
        ],
    )?;
    Ok(())
}

/// The newest HLC in any synced table.
fn newest_stamp(conn: &Connection) -> Result<Option<Hlc>> {
    let mut newest: Option<Hlc> = None;
    // From the sync list, so a new synced table can't be missed (#257).
    for (table, _) in steps_protocol::sync::TABLES {
        let last: Option<(i64, u32, String)> = conn
            .query_row(
                &format!(
                    "SELECT hlc_wall, hlc_counter, device_id FROM {table}
                     ORDER BY hlc_wall DESC, hlc_counter DESC LIMIT 1"
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((wall_ms, counter, device)) = last {
            let h = Hlc { wall_ms, counter, device };
            if newest.as_ref().is_none_or(|n| h > *n) {
                newest = Some(h);
            }
        }
    }
    Ok(newest)
}

/// The app's store, opened once in the data directory. If that fails the
/// app still runs on an in-memory store and says so on Today.
pub fn shared() -> &'static Store {
    #[cfg(target_arch = "wasm32")]
    return STORE.get_or_init(|| match Store::from_snapshot(&crate::platform::web::take_snapshot()) {
        Ok(s) => s,
        Err(e) => {
            // A snapshot this build can't read stays in the browser untouched
            // (`web::saving_allowed`); this session runs in memory.
            crate::platform::web::stop_saving();
            let _ = OPEN_ERROR.set(format!("browser storage: {e}"));
            Store::open_in_memory().expect("an in-memory database always opens")
        }
    });
    // Unit tests: each test thread has its own store in memory.
    #[cfg(test)]
    return TEST_STORE.with(|s| *s);
    #[cfg(all(not(target_arch = "wasm32"), not(test)))]
    STORE.get_or_init(|| {
        let opened = paths::data_dir().and_then(|dir| {
            Store::open_marked(&dir.join("steps.db"), paths::install_marker().as_deref())
                .map_err(|e| format!("{}: {e}", dir.display()))
        });
        match opened {
            Ok(s) => s,
            Err(e) => {
                let _ = OPEN_ERROR.set(e);
                Store::open_in_memory().expect("an in-memory database always opens")
            }
        }
    })
}

#[cfg_attr(test, allow(dead_code))]
static STORE: OnceLock<Store> = OnceLock::new();
static OPEN_ERROR: OnceLock<String> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static TEST_STORE: &'static Store =
        Box::leak(Box::new(Store::open_in_memory().expect("an in-memory database always opens")));
}

/// Why the store fell back to memory, if it did.
pub fn open_error() -> Option<&'static str> {
    OPEN_ERROR.get().map(String::as_str)
}

static WRITE_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Saves a synced setting in the app's store. A failure is reported to the
/// log and shown on Today, so a choice never vanishes silently.
pub fn save_setting<T: Serialize>(key: &str, value: &T) -> bool {
    let key = key.to_owned();
    let value = serde_json::to_value(value).map_err(|e| e.to_string());
    crate::persistence::save(format!("synced.{key}"), move || {
        let operation = format!("setting {key}");
        let result = value
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|value| shared().put_setting(&key, value).map_err(|e| e.to_string()));
        match &result {
            Ok(()) => write_recovered(&operation),
            Err(e) => write_failed(&operation, e),
        }
        result
    })
}

/// Reports a write that didn't land, to the log and on Today.
pub fn write_failed(what: &str, e: &dyn std::fmt::Display) {
    crate::diag::report!("saving {what}: {e}");
    *WRITE_ERROR.lock().unwrap() = Some(format!("{what}: {e}"));
}

/// Clear only this operation's previous diagnostic after successful recovery.
pub fn write_recovered(what: &str) {
    clear_recovered(&mut WRITE_ERROR.lock().unwrap_or_else(std::sync::PoisonError::into_inner), what);
}

fn clear_recovered(error: &mut Option<String>, what: &str) {
    if error.as_deref().is_some_and(|message| message.starts_with(&format!("{what}: "))) {
        *error = None;
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::clear_recovered;

    #[test]
    fn unrelated_recovery_does_not_erase_another_write_error() {
        let mut error = Some("setting sounds: blocked".to_owned());
        clear_recovered(&mut error, "setting plan");
        assert!(error.is_some());
        clear_recovered(&mut error, "setting sound");
        assert!(error.is_some());
        clear_recovered(&mut error, "setting sounds");
        assert!(error.is_none());
    }
}

/// The last setting that couldn't be saved, if any.
pub fn write_error() -> Option<String> {
    WRITE_ERROR.lock().unwrap().clone()
}

pub use paths::export_dir;

pub fn now_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Minutes east of UTC right now (Cairo: 120, or 180 in summer time).
pub fn utc_offset_minutes() -> i32 {
    use chrono::Offset;
    chrono::Local::now().offset().fix().local_minus_utc() / 60
}

/// This device's time zone as a table of UTC offsets, 2020–2040, so every
/// moment uses the offset in force then (a change to or from summer time
/// never moves past answers to another day, #40). Built once: an hourly scan
/// of the system time zone, a few milliseconds.
pub fn zone() -> Zone {
    static ZONE: std::sync::OnceLock<Zone> = std::sync::OnceLock::new();
    *ZONE.get_or_init(|| {
        let offset = |ms: i64| {
            use chrono::{Offset, TimeZone};
            chrono::Local
                .timestamp_millis_opt(ms)
                .single()
                .map_or(0, |t| t.offset().fix().local_minus_utc() / 60)
        };
        const HOUR: i64 = 3_600_000;
        let (from, to) = (1_577_836_800_000_i64, 2_208_988_800_000_i64); // 2020-01-01 .. 2040-01-01 UTC
        let base = offset(from);
        let mut changes = Vec::new();
        let mut last = base;
        let mut t = from;
        while t < to {
            let o = offset(t + HOUR);
            if o != last {
                // Narrow the change to the minute.
                let (mut lo, mut hi) = (t, t + HOUR);
                while hi - lo > 60_000 {
                    let mid = lo + (hi - lo) / 2;
                    if offset(mid) == last { lo = mid } else { hi = mid }
                }
                changes.push((hi, o));
                last = o;
            }
            t += HOUR;
        }
        Zone { base, changes: Box::leak(changes.into_boxed_slice()) }
    })
}

/// A random-enough id for this installation, without a crypto dependency:
/// it only has to differ between Mohamed's three devices.
fn new_device_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_i64(now_ms());
    // The browser has no process ids (and std's hash keys are fixed there).
    #[cfg(not(target_arch = "wasm32"))]
    h.write_u32(std::process::id());
    h.write_u64(getrandom::u64().unwrap_or_default());
    format!("d{:012x}", h.finish() & 0xffff_ffff_ffff)
}

#[cfg(test)]
mod tests {
    use steps_core::review::verify::{Edit, FieldValue, Verdict};
    use steps_core::srs::Retention;

    use super::*;

    fn ev(item: &str, grade: Grade, at_ms: i64) -> ReviewEvent {
        ReviewEvent { item_id: item.into(), grade, origin: Origin::Study, at_ms, took_ms: 3000 }
    }

    fn judgement(verdict: Verdict) -> Judgement {
        Judgement { verdict, edit: Edit::new(), note: None, fingerprint: "f".into(), at_ms: 1 }
    }

    #[test]
    fn settings_by_prefix() {
        let s = Store::open_in_memory().unwrap();
        s.put_setting("bank.paper.a.1", &1).unwrap();
        s.put_setting("bank.paper.b.2", &2).unwrap();
        s.put_setting("bank.papers", &vec![9]).unwrap();
        s.put_setting("other", &3).unwrap();
        let got: Vec<(String, i32)> = s.settings_with_prefix("bank.paper.");
        assert_eq!(got, vec![("bank.paper.a.1".into(), 1), ("bank.paper.b.2".into(), 2)]);
    }

    #[test]
    fn a_write_and_its_outbox_entry_land_together() {
        let s = Store::open_in_memory().unwrap();
        s.conn.lock().unwrap().execute("DROP TABLE sync_outbox", []).unwrap();
        assert!(s.append_review(ev("cvs.L04.001", Grade::Good, 10)).is_err());
        assert!(s.reviews().unwrap().is_empty(), "the answer rolls back with its outbox entry");
        assert!(s.set_judgement("cvs.L04.001", Some(&judgement(Verdict::Approved))).is_err());
        assert!(s.judgements().unwrap().is_empty());
    }

    #[test]
    fn zone_matches_the_system_offset_now() {
        assert_eq!(zone().offset_at(now_ms()), utc_offset_minutes());
        assert!(zone().changes.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn an_own_card_from_a_clock_ahead_moves_this_clock_past_it() {
        let dir = std::env::temp_dir().join(format!("steps-store-{}", new_device_id()));
        let path = dir.join("steps.db");
        let card = |front: &str| steps_core::own::OwnCard {
            id: "own.t".into(),
            front: front.into(),
            back: "A".into(),
            lecture: None,
            page: None,
            source: None,
            created_ms: 1,
        };
        {
            // The tablet, two minutes ahead, wrote the card; it synced here.
            let s = Store::open(&path).unwrap();
            let ahead = Hlc { wall_ms: now_ms() + 120_000, counter: 0, device: "tablet".into() };
            let value = serde_json::to_string(&card("Q")).unwrap();
            s.put_lww("own_cards", "id", "own.t", None, &value, false, &ahead).unwrap();
        }
        // Edited here right after a restart, the edit still wins (#257).
        let s = Store::open(&path).unwrap();
        s.put_own_card(&card("Q edited")).unwrap();
        assert_eq!(s.own_cards()[0].front, "Q edited");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reviews_are_appended_in_order_and_survive_reopening() {
        let dir = std::env::temp_dir().join(format!("steps-store-{}", new_device_id()));
        let path = dir.join("steps.db");
        let device;
        {
            let s = Store::open(&path).unwrap();
            device = s.device();
            s.append_review(ev("cvs.L04.001", Grade::Good, 10)).unwrap();
            s.append_review(ev("cvs.L04.002", Grade::Again, 10)).unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.device(), device);
        let log = s.reviews().unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].hlc < log[1].hlc);
        assert_eq!(log[1].event.grade, Grade::Again);
        // A re-inserted event (sync echo) is not duplicated.
        insert_review(&s.conn.lock().unwrap(), &log[0]).unwrap();
        assert_eq!(s.reviews().unwrap().len(), 2);
        // New stamps sort after the stored ones.
        let next = s.append_review(ev("cvs.L04.003", Grade::Good, 0)).unwrap();
        assert!(next.hlc > log[1].hlc);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn verdicts_are_last_writer_wins_and_can_be_undone() {
        let s = Store::open_in_memory().unwrap();
        s.set_judgement("cvs.L04.001", Some(&judgement(Verdict::Wrong))).unwrap();
        let mut edited = judgement(Verdict::Edited);
        edited.edit.insert("back".into(), FieldValue::Text("mine".into()));
        s.set_judgement("cvs.L04.001", Some(&edited)).unwrap();
        assert_eq!(s.judgements().unwrap()["cvs.L04.001"], edited);
        // An older write (a late sync row) loses.
        let old = Hlc { wall_ms: 0, counter: 0, device: "tab".into() };
        s.put_lww("item_status", "item_id", "cvs.L04.001", Some("approved"), "{}", false, &old).unwrap();
        assert_eq!(s.judgements().unwrap()["cvs.L04.001"].verdict, Verdict::Edited);
        // Undo leaves a tombstone: unverified again.
        s.set_judgement("cvs.L04.001", None).unwrap();
        assert!(s.judgements().unwrap().is_empty());
    }

    #[test]
    fn generic_and_device_only_settings() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.setting::<u32>("sound"), None);
        s.put_setting("sound", &2u32).unwrap();
        s.put_setting("sound", &3u32).unwrap();
        assert_eq!(s.setting::<u32>("sound"), Some(3));
        s.put_local_setting("rings", &true).unwrap();
        assert_eq!(s.local_setting::<bool>("rings"), Some(true));
        assert_eq!(s.setting::<bool>("rings"), None);
    }

    #[test]
    fn settings_default_then_persist() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.review_settings(), ReviewSettings::default());
        let mine = ReviewSettings { retention: Retention::R95, new_per_day: 45, fsrs: None };
        s.put_review_settings(&mine).unwrap();
        assert_eq!(s.review_settings(), mine);
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::Store;

    /// The web build keeps the database as bytes: they come back whole,
    /// with the same device, and an empty snapshot is a fresh store.
    #[test]
    fn a_snapshot_round_trips() {
        let s = Store::open_in_memory().unwrap();
        s.put_setting("sounds", &serde_json::json!(true)).unwrap();
        let (changes, bytes) = s.snapshot().unwrap();
        assert!(changes > 0 && !bytes.is_empty());
        let back = Store::from_snapshot(&bytes).unwrap();
        assert_eq!(back.device(), s.device());
        assert_eq!(back.snapshot().unwrap().1.len(), bytes.len());
        let fresh = Store::from_snapshot(&[]).unwrap();
        assert_ne!(fresh.device(), s.device());
    }
}
