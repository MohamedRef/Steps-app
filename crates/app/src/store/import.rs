//! Import / restore: merges a `steps-export` file into this device's store,
//! so a backup or a move to a new device needs no cloud.
//!
//! The merge follows the sync rules: a row is added when it's new, and
//! replaces a local row only when its HLC is newer (last writer wins).
//! Append-only review events never change, so they simply dedupe by id.
//! Local rows are never deleted; a tombstone in the file only wins over an
//! older local row, as it would in sync. Tables without sync columns
//! (`meta`, which holds this device's id) stay local.

use std::collections::BTreeMap;

use rusqlite::types::Value as Sql;
use serde_json::Value;
use steps_core::review::Hlc;

use super::export::{EXPORT_FORMAT, EXPORT_FORMAT_VERSION};
use super::{Result as StoreResult, Store, StoreError, sync};

const SYNC_COLUMNS: [&str; 3] = ["device_id", "hlc_wall", "hlc_counter"];

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    NotJson(String),
    NotSteps,
    TooNew(u64),
    Store(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson(e) => write!(f, "not JSON: {e}"),
            Self::NotSteps => write!(f, "not a Steps export"),
            Self::TooNew(v) => write!(f, "export format {v} is newer than this Steps"),
            Self::Store(e) => write!(f, "{e}"),
        }
    }
}

impl From<StoreError> for ImportError {
    fn from(e: StoreError) -> Self {
        Self::Store(e.to_string())
    }
}

impl From<rusqlite::Error> for ImportError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Store(e.to_string())
    }
}

/// What a file holds and what importing it would change, per table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportPlan {
    pub device_id: String,
    pub exported_at_ms: i64,
    pub app_version: String,
    pub tables: Vec<TablePlan>,
    /// Tables in the file that stay local or that this build doesn't know.
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TablePlan {
    pub name: String,
    pub rows: usize,
    /// Rows this device doesn't have.
    pub new: usize,
    /// Rows newer than this device's copy (they replace it).
    pub newer: usize,
    /// After an import: rows this build couldn't take (a missing key, HLC or
    /// NOT NULL column). Always 0 in a plan.
    pub failed: usize,
}

impl ImportPlan {
    pub fn changes(&self) -> usize {
        self.tables.iter().map(|t| t.new + t.newer).sum()
    }

    /// Rows an import couldn't take.
    pub fn failed(&self) -> usize {
        self.tables.iter().map(|t| t.failed).sum()
    }

    #[cfg(test)]
    pub fn table(&self, name: &str) -> Option<&TablePlan> {
        self.tables.iter().find(|t| t.name == name)
    }
}

/// Parses an export file and checks it is one Steps can read.
pub fn parse(text: &str) -> Result<Value, ImportError> {
    let doc: Value = serde_json::from_str(text).map_err(|e| ImportError::NotJson(e.to_string()))?;
    if doc["format"] != EXPORT_FORMAT || !doc["tables"].is_object() {
        return Err(ImportError::NotSteps);
    }
    let version = doc["format_version"].as_u64().unwrap_or(0);
    if version > u64::from(EXPORT_FORMAT_VERSION) {
        return Err(ImportError::TooNew(version));
    }
    Ok(doc)
}

/// A local table the import (and sync) may merge into: its columns and its key.
pub(super) struct Target {
    pub columns: Vec<String>,
    pub key: String,
}

/// Inserts one row, or replaces the local copy when the row's HLC is newer
/// (last writer wins). Columns this build doesn't have are ignored. Returns
/// whether anything changed.
pub(super) fn merge_row(
    conn: &rusqlite::Connection,
    name: &str,
    target: &Target,
    obj: &serde_json::Map<String, Value>,
) -> rusqlite::Result<bool> {
    let cols: Vec<&String> = target.columns.iter().filter(|c| obj.contains_key(c.as_str())).collect();
    let values: Vec<Sql> = cols.iter().map(|c| to_sql(&obj[c.as_str()])).collect();
    let quoted: Vec<String> = cols.iter().map(|c| format!("\"{c}\"")).collect();
    let marks: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let set: Vec<String> =
        cols.iter().filter(|c| **c != &target.key).map(|c| format!("\"{c}\" = excluded.\"{c}\"")).collect();
    let sql = format!(
        "INSERT INTO \"{name}\" ({}) VALUES ({}) ON CONFLICT (\"{key}\") DO UPDATE SET {}
         WHERE (excluded.hlc_wall, excluded.hlc_counter, excluded.device_id)
             > (\"{name}\".hlc_wall, \"{name}\".hlc_counter, \"{name}\".device_id)",
        quoted.join(", "),
        marks.join(", "),
        set.join(", "),
        key = target.key,
    );
    Ok(conn.execute(&sql, rusqlite::params_from_iter(values))? > 0)
}

impl Store {
    pub(super) fn target(&self, table: &str) -> StoreResult<Option<Target>> {
        if table == "meta" {
            return Ok(None);
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, pk FROM pragma_table_info(?1)")?;
        let cols: Vec<(String, i64)> =
            stmt.query_map([table], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        let keys: Vec<&String> = cols.iter().filter(|(_, pk)| *pk > 0).map(|(c, _)| c).collect();
        let names: Vec<String> = cols.iter().map(|(c, _)| c.clone()).collect();
        if keys.len() != 1 || !SYNC_COLUMNS.iter().all(|s| names.iter().any(|c| c == s)) {
            return Ok(None);
        }
        Ok(Some(Target { key: keys[0].clone(), columns: names }))
    }

    /// The HLC of this device's copy of a row, if it has one.
    fn local_hlc(&self, table: &str, key: &str, id: &Sql) -> StoreResult<Option<Hlc>> {
        use rusqlite::OptionalExtension;
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                &format!("SELECT hlc_wall, hlc_counter, device_id FROM \"{table}\" WHERE \"{key}\" = ?1"),
                [id],
                |r| Ok(Hlc { wall_ms: r.get(0)?, counter: r.get(1)?, device: r.get(2)? }),
            )
            .optional()?)
    }

    /// What importing `doc` would add and replace. Changes nothing.
    pub fn plan_import(&self, doc: &Value) -> Result<ImportPlan, ImportError> {
        let mut plan = ImportPlan {
            device_id: doc["device_id"].as_str().unwrap_or_default().to_string(),
            exported_at_ms: doc["exported_at_ms"].as_i64().unwrap_or(0),
            app_version: doc["app_version"].as_str().unwrap_or_default().to_string(),
            ..ImportPlan::default()
        };
        let tables = doc["tables"].as_object().ok_or(ImportError::NotSteps)?;
        for (name, table) in tables {
            let Some(target) = self.target(name)? else {
                plan.skipped.push(name.clone());
                continue;
            };
            let rows = rows_of(table);
            let mut t = TablePlan { name: name.clone(), rows: rows.len(), ..TablePlan::default() };
            for row in rows {
                let Some((id, hlc)) = key_and_hlc(row, &target.key) else { continue };
                match self.local_hlc(name, &target.key, &id)? {
                    None => t.new += 1,
                    Some(local) if hlc > local => t.newer += 1,
                    Some(_) => {}
                }
            }
            plan.tables.push(t);
        }
        Ok(plan)
    }

    /// Merges `doc` into the store in one transaction. Returns what actually
    /// changed, counted row by row as it is written, and the rows that
    /// couldn't be taken, so the screen never reports the plan as done.
    pub fn apply_import(&self, doc: &Value) -> Result<ImportPlan, ImportError> {
        let planned = self.plan_import(doc)?;
        let tables = doc["tables"].as_object().ok_or(ImportError::NotSteps)?;
        let mut targets = BTreeMap::new();
        for t in &planned.tables {
            if let Some(target) = self.target(&t.name)? {
                targets.insert(t.name.clone(), target);
            }
        }
        let mut done = ImportPlan { tables: Vec::new(), ..planned.clone() };
        let mut newest: Option<Hlc> = None;
        let device = self.device();
        {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn.transaction()?;
            for t in &planned.tables {
                let Some(target) = targets.get(&t.name) else { continue };
                let name = &t.name;
                let mut out = TablePlan { name: name.clone(), rows: t.rows, ..TablePlan::default() };
                for row in rows_of(&tables[name]) {
                    let (Some(obj), Some((id, hlc))) = (row.as_object(), key_and_hlc(row, &target.key))
                    else {
                        out.failed += 1;
                        continue;
                    };
                    let had: bool = tx.query_row(
                        &format!("SELECT EXISTS (SELECT 1 FROM \"{name}\" WHERE \"{}\" = ?1)", target.key),
                        [&id],
                        |r| r.get(0),
                    )?;
                    // A row this build can't take (a missing NOT NULL column) is
                    // counted, not hidden.
                    let changed = match merge_row(&tx, name, target, obj) {
                        Ok(changed) => changed,
                        Err(e) => {
                            crate::diag::report!("import: a row of {name} was not merged: {e}");
                            out.failed += 1;
                            continue;
                        }
                    };
                    if changed {
                        if had {
                            out.newer += 1
                        } else {
                            out.new += 1
                        }
                        // What the backup brought goes on to Steps Cloud too, as a
                        // relay when another device wrote it.
                        if let Some(key) = row[target.key.as_str()].as_str() {
                            sync::enqueue(&tx, name, &target.key, key, &device)?;
                        }
                    }
                    if newest.as_ref().is_none_or(|n| hlc > *n) {
                        newest = Some(hlc);
                    }
                }
                done.tables.push(out);
            }
            tx.commit()?;
        }
        // New local stamps sort after everything imported.
        if let Some(h) = newest {
            self.clock.lock().unwrap().observe(&h);
        }
        Ok(done)
    }
}

fn rows_of(table: &Value) -> &[Value] {
    table["rows"].as_array().map(Vec::as_slice).unwrap_or_default()
}

pub(super) fn key_and_hlc(row: &Value, key: &str) -> Option<(Sql, Hlc)> {
    let id = to_sql(row.get(key)?);
    let hlc = Hlc {
        wall_ms: row["hlc_wall"].as_i64()?,
        counter: u32::try_from(row["hlc_counter"].as_i64()?).ok()?,
        device: row["device_id"].as_str()?.to_string(),
    };
    Some((id, hlc))
}

fn to_sql(v: &Value) -> Sql {
    match v {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(i64::from(*b)),
        Value::Number(n) => {
            n.as_i64().map(Sql::Integer).unwrap_or_else(|| Sql::Real(n.as_f64().unwrap_or(0.0)))
        }
        Value::String(s) => Sql::Text(s.clone()),
        Value::Object(o) => match o.get("hex").and_then(Value::as_str) {
            Some(hex) => Sql::Blob(
                (0..hex.len() / 2)
                    .filter_map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok())
                    .collect(),
            ),
            None => Sql::Text(v.to_string()),
        },
        Value::Array(_) => Sql::Text(v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use steps_core::review::verify::{Edit, Judgement, Verdict};
    use steps_core::review::{Grade, Origin, ReviewEvent};
    use steps_core::srs::{Retention, ReviewSettings};

    use super::*;

    fn ev(item: &str, at_ms: i64) -> ReviewEvent {
        ReviewEvent { item_id: item.into(), grade: Grade::Good, origin: Origin::Review, at_ms, took_ms: 3000 }
    }

    fn judged(verdict: Verdict) -> Judgement {
        Judgement { verdict, edit: Edit::new(), note: None, fingerprint: "f".into(), at_ms: 1 }
    }

    fn round_trip(from: &Store) -> Value {
        let doc = from.export("0.1.0", 2).unwrap().document;
        parse(&serde_json::to_string(&doc).unwrap()).unwrap()
    }

    #[test]
    fn restores_onto_a_fresh_device() {
        let phone = Store::open_in_memory().unwrap();
        phone.append_review(ev("hb.L01.001", 10)).unwrap();
        phone.append_review(ev("hb.L01.002", 20)).unwrap();
        phone.set_judgement("hb.L01.001", Some(&judged(Verdict::Approved))).unwrap();
        phone
            .put_review_settings(&ReviewSettings { retention: Retention::R95, ..Default::default() })
            .unwrap();
        let doc = round_trip(&phone);

        let tablet = Store::open_in_memory().unwrap();
        let plan = tablet.plan_import(&doc).unwrap();
        assert_eq!(plan.device_id, phone.device());
        assert_eq!(plan.table("review_events").unwrap().new, 2);
        assert_eq!(plan.table("item_status").unwrap().new, 1);
        assert!(plan.skipped.contains(&"meta".to_string()));
        tablet.apply_import(&doc).unwrap();

        assert_eq!(tablet.reviews().unwrap(), phone.reviews().unwrap());
        assert_eq!(tablet.judgements().unwrap(), phone.judgements().unwrap());
        assert_eq!(tablet.review_settings().retention, Retention::R95);
        // The tablet keeps its own identity, and its next stamp sorts last.
        assert_ne!(tablet.device(), phone.device());
        let next = tablet.append_review(ev("hb.L01.003", 0)).unwrap();
        assert!(phone.reviews().unwrap().iter().all(|l| l.hlc < next.hlc));
        // Importing again changes nothing.
        assert_eq!(tablet.plan_import(&doc).unwrap().changes(), 0);
        tablet.apply_import(&doc).unwrap();
        assert_eq!(tablet.reviews().unwrap().len(), 3);
    }

    #[test]
    fn rows_that_fail_are_counted_not_reported_as_merged() {
        let phone = Store::open_in_memory().unwrap();
        phone.append_review(ev("hb.L01.001", 10)).unwrap();
        phone.append_review(ev("hb.L01.002", 20)).unwrap();
        phone.append_review(ev("hb.L01.003", 30)).unwrap();
        let mut doc = round_trip(&phone);
        let rows = doc["tables"]["review_events"]["rows"].as_array_mut().unwrap();
        // One row this build can't take (a NOT NULL column missing), one
        // with no HLC at all.
        rows[0].as_object_mut().unwrap().remove("item_id");
        rows[1].as_object_mut().unwrap().remove("hlc_wall");

        let tablet = Store::open_in_memory().unwrap();
        assert_eq!(tablet.plan_import(&doc).unwrap().table("review_events").unwrap().new, 2);
        let done = tablet.apply_import(&doc).unwrap();
        let t = done.table("review_events").unwrap();
        assert_eq!((t.rows, t.new, t.newer, t.failed), (3, 1, 0, 2));
        assert_eq!(done.changes(), 1);
        assert_eq!(done.failed(), 2);
        assert_eq!(tablet.reviews().unwrap().len(), 1);
    }

    #[test]
    fn a_restore_goes_on_to_steps_cloud() {
        let phone = Store::open_in_memory().unwrap();
        let answer = phone.append_review(ev("hb.L01.001", 10)).unwrap();
        phone.set_judgement("hb.L01.001", Some(&judged(Verdict::Approved))).unwrap();
        let doc = round_trip(&phone);

        let tablet = Store::open_in_memory().unwrap();
        let before = tablet.outbox(100).unwrap().len();
        tablet.apply_import(&doc).unwrap();
        let outbox = tablet.outbox(100).unwrap();
        let relayed: Vec<_> = outbox.iter().filter(|o| o.relay).collect();
        assert!(relayed.iter().any(|o| o.table == "review_events" && o.row_id == answer.id));
        assert!(relayed.iter().any(|o| o.table == "item_status" && o.row_id == "hb.L01.001"));
        assert!(relayed.iter().all(|o| o.hlc.device == phone.device()));
        // The ids match the phone's own ops, so the hub takes each only once.
        let phone_ids: Vec<_> = phone.outbox(100).unwrap().into_iter().map(|o| o.id).collect();
        assert!(relayed.iter().all(|o| phone_ids.contains(&o.id)), "{relayed:?} vs {phone_ids:?}");
        // A second restore queues nothing more.
        let after = outbox.len();
        assert!(after > before);
        tablet.apply_import(&doc).unwrap();
        assert_eq!(tablet.outbox(100).unwrap().len(), after);
    }

    #[test]
    fn merges_by_hlc_and_never_deletes_local_rows() {
        let old = Store::open_in_memory().unwrap();
        old.set_judgement("hb.L01.001", Some(&judged(Verdict::Wrong))).unwrap();
        old.set_judgement("hb.L01.002", Some(&judged(Verdict::Wrong))).unwrap();
        old.append_review(ev("hb.L01.009", 5)).unwrap();
        let backup = round_trip(&old);

        // Later, on another device: a newer verdict for 001 and its own review.
        let now = Store::open_in_memory().unwrap();
        now.apply_import(&backup).unwrap();
        now.set_judgement("hb.L01.001", Some(&judged(Verdict::Approved))).unwrap();
        now.append_review(ev("hb.L01.004", 50)).unwrap();

        // Restoring the older backup keeps the newer verdict and the new review.
        let plan = now.plan_import(&backup).unwrap();
        assert_eq!(plan.changes(), 0);
        now.apply_import(&backup).unwrap();
        let j = now.judgements().unwrap();
        assert_eq!(j["hb.L01.001"].verdict, Verdict::Approved);
        assert_eq!(j["hb.L01.002"].verdict, Verdict::Wrong);
        assert_eq!(now.reviews().unwrap().len(), 2);

        // And the newer device's export wins over the old one's rows.
        let newer = round_trip(&now);
        assert_eq!(old.plan_import(&newer).unwrap().table("item_status").unwrap().newer, 1);
        old.apply_import(&newer).unwrap();
        assert_eq!(old.judgements().unwrap()["hb.L01.001"].verdict, Verdict::Approved);
        assert_eq!(old.reviews().unwrap().len(), 2);
    }

    #[test]
    fn rejects_files_that_are_not_steps_exports() {
        assert!(matches!(parse("not json"), Err(ImportError::NotJson(_))));
        assert_eq!(parse(r#"{"format":"kabar-export","tables":{}}"#), Err(ImportError::NotSteps));
        assert_eq!(
            parse(r#"{"format":"steps-export","format_version":9,"tables":{}}"#),
            Err(ImportError::TooNew(9))
        );
    }

    #[test]
    fn reads_the_recorded_fixture() {
        let doc = parse(include_str!("../../../../fixtures/export/steps-export-v1.json")).unwrap();
        let s = Store::open_in_memory().unwrap();
        let plan = s.apply_import(&doc).unwrap();
        assert_eq!(plan.table("review_events").unwrap().new, 2);
        let j = s.judgements().unwrap();
        assert_eq!(j["hb.L01.003"].verdict, Verdict::Edited);
        assert_eq!(s.reviews().unwrap()[0].event.item_id, "hb.L01.003");
        assert_eq!(s.review_settings().new_per_day, 45);
    }
}
