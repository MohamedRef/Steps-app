//! Export everything: every table of the local store as one JSON document
//! (`steps-export`). Tables are found through `sqlite_master`, so tables a
//! later chunk adds are included without touching this file. The document
//! is what [`super::import`] reads back.
//!
//! Secrets never leave: Steps keeps none in SQLite, and as a second line of
//! defence any table, column or row key that names a secret is skipped.
// adapted from kabar-app main: crates/app/src/store/export.rs

use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};

use super::{Result, Store};

pub const EXPORT_FORMAT: &str = "steps-export";
/// Bumped when the export layout changes; import refuses newer ones.
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// Words that mark a name as secret (matched case-insensitively, as a substring).
const SECRET_WORDS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "credential",
    "private_key",
    "cookie",
    "bearer",
];

/// Columns whose value names the row (a settings or meta key, an id).
const KEY_COLUMNS: &[&str] = &["key", "id", "row_id"];

#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub document: Value,
    /// Rows written, over all tables.
    pub rows: u64,
    pub tables: usize,
}

pub fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|w| lower.contains(w))
}

pub(super) fn cell(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
        ValueRef::Blob(b) => json!({ "hex": b.iter().map(|x| format!("{x:02x}")).collect::<String>() }),
    }
}

impl Store {
    /// The whole local store as one JSON document.
    pub fn export(&self, app_version: &str, now_ms: i64) -> Result<Export> {
        let device = self.device();
        let conn = self.conn.lock().unwrap();
        let schema_version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let names: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )?;
            stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };

        let mut tables = Map::new();
        let mut total = 0u64;
        // The sync outbox is this device's queue, not data: restore re-reads rows.
        for name in names.into_iter().filter(|n| !is_secret_name(n) && n != "sync_outbox") {
            let mut stmt = conn.prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))?;
            let columns: Vec<String> = stmt.column_names().map(String::from).collect();
            let keep: Vec<bool> = columns.iter().map(|c| !is_secret_name(c)).collect();
            let key_cols: Vec<usize> = columns
                .iter()
                .enumerate()
                .filter(|(_, c)| KEY_COLUMNS.contains(&c.as_str()))
                .map(|(i, _)| i)
                .collect();
            let mut rows = Vec::new();
            let mut cursor = stmt.query([])?;
            while let Some(row) = cursor.next()? {
                let secret_row = key_cols.iter().any(|&i| {
                    matches!(row.get_ref(i), Ok(ValueRef::Text(t)) if is_secret_name(&String::from_utf8_lossy(t)))
                });
                if secret_row {
                    continue;
                }
                let mut obj = Map::new();
                for (i, col) in columns.iter().enumerate() {
                    if keep[i] {
                        obj.insert(col.clone(), cell(row.get_ref(i)?));
                    }
                }
                rows.push(Value::Object(obj));
            }
            total += rows.len() as u64;
            let kept: Vec<&String> = columns.iter().zip(&keep).filter(|(_, k)| **k).map(|(c, _)| c).collect();
            tables.insert(name, json!({ "columns": kept, "rows": rows }));
        }

        let count = tables.len();
        let document = json!({
            "format": EXPORT_FORMAT,
            "format_version": EXPORT_FORMAT_VERSION,
            "app_version": app_version,
            "schema_version": schema_version,
            "device_id": device,
            "exported_at_ms": now_ms,
            "row_count": total,
            "note": "Everything Steps keeps on this device: reviews, content checks and settings. Import it in Settings › Your data on any device.",
            "tables": tables,
        });
        Ok(Export { document, rows: total, tables: count })
    }
}

#[cfg(test)]
mod tests {
    use steps_core::review::{Grade, Origin, ReviewEvent};

    use super::*;

    #[test]
    fn exports_every_table_and_no_secrets() {
        let store = Store::open_in_memory().unwrap();
        store
            .append_review(ReviewEvent {
                item_id: "cvs.L04.003".into(),
                grade: Grade::Good,
                origin: Origin::Review,
                at_ms: 1,
                took_ms: 4000,
                confidence: None,
                reason: None,
            })
            .unwrap();
        store
            .conn
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TABLE future_feature (id TEXT PRIMARY KEY, body TEXT, api_key TEXT);
                 INSERT INTO future_feature VALUES ('a', 'SAAG ≥ 1.1 g/dL', 'sk-xyz');
                 INSERT INTO meta VALUES ('refresh_token', 'kr1.a.b.c');
                 CREATE TABLE oauth_tokens (id TEXT, value TEXT);
                 INSERT INTO oauth_tokens VALUES ('x', 'ya29.abc');",
            )
            .unwrap();

        let export = store.export("0.1.0", 1_793_610_000_000).unwrap();
        let doc = &export.document;
        let text = serde_json::to_string(doc).unwrap();
        assert_eq!(doc["format"], EXPORT_FORMAT);
        assert_eq!(doc["device_id"], store.device());
        let tables = doc["tables"].as_object().unwrap();
        for t in ["meta", "settings", "review_events", "item_status", "future_feature"] {
            assert!(tables.contains_key(t), "{t} missing");
        }
        assert!(!tables.contains_key("oauth_tokens"));
        assert!(text.contains("SAAG ≥ 1.1 g/dL"));
        for secret in ["sk-xyz", "ya29.abc", "kr1.a.b.c", "api_key"] {
            assert!(!text.contains(secret), "{secret} leaked");
        }
        let counted: u64 = tables.values().map(|t| t["rows"].as_array().unwrap().len() as u64).sum();
        assert_eq!(export.rows, counted);
        assert_eq!(doc["row_count"], counted);
    }

    #[test]
    fn secret_names() {
        assert!(is_secret_name("access_token") && is_secret_name("GEMINI_API_KEY"));
        assert!(!is_secret_name("item_status") && !is_secret_name("review_events") && !is_secret_name("key"));
    }
}
