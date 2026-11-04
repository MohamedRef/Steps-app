//! Mohamed's self-assessment scores in the `self_assessments` table (synced,
//! last writer wins, a tombstone on delete), like his own cards (`own.rs`).

use rusqlite::params;
use steps_core::selfassess::{PREFIX, SelfAssessment};

use super::{Result, Store, now_ms};

impl Store {
    /// Every live entry, oldest first (`selfassess::sorted`). Rows that
    /// don't parse are skipped.
    pub fn self_assessments(&self) -> Vec<SelfAssessment> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(mut stmt) = conn.prepare("SELECT value FROM self_assessments WHERE deleted = 0") else {
            return Vec::new();
        };
        let rows: Vec<SelfAssessment> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map(|rows| rows.flatten().filter_map(|v| serde_json::from_str(&v).ok()).collect())
            .unwrap_or_default();
        steps_core::selfassess::sorted(&rows)
    }

    /// Adds or replaces an entry (same id).
    pub fn put_self_assessment(&self, entry: &SelfAssessment) -> Result<()> {
        let hlc = self.tick(now_ms());
        self.put_lww("self_assessments", "id", &entry.id, None, &serde_json::to_string(entry)?, false, &hlc)
    }

    /// Deletes an entry on every device (a tombstone).
    pub fn delete_self_assessment(&self, id: &str) -> Result<()> {
        let value: Option<String> = {
            let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            conn.query_row("SELECT value FROM self_assessments WHERE id = ?1", params![id], |r| r.get(0)).ok()
        };
        let Some(value) = value else { return Ok(()) };
        let hlc = self.tick(now_ms());
        self.put_lww("self_assessments", "id", id, None, &value, true, &hlc)
    }
}

/// `sa.<16 hex digits>`.
pub fn new_id() -> String {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        bytes = now_ms().to_le_bytes();
    }
    format!("{PREFIX}{:016x}", u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use steps_core::plan::Day;
    use steps_core::selfassess::Kind;
    use steps_protocol::sync::SeqOp;

    use super::*;

    fn entry(id: &str, day: &str, percent: u8) -> SelfAssessment {
        SelfAssessment {
            id: id.into(),
            kind: Kind::Nbme,
            form: Some("30".into()),
            day: Day::parse(day).unwrap(),
            percent,
            pass_probability: None,
            note: None,
            created_ms: 1,
        }
    }

    #[test]
    fn add_edit_delete_and_queue_for_sync() {
        let s = Store::open_in_memory().unwrap();
        let before = s.outbox_len();
        s.put_self_assessment(&entry("sa.b", "2027-06-08", 66)).unwrap();
        s.put_self_assessment(&entry("sa.a", "2027-06-01", 60)).unwrap();
        s.put_self_assessment(&SelfAssessment { percent: 64, ..entry("sa.b", "2027-06-08", 66) }).unwrap();
        let all = s.self_assessments();
        assert_eq!(all.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["sa.a", "sa.b"]);
        assert_eq!(all[1].percent, 64);
        s.delete_self_assessment("sa.a").unwrap();
        s.delete_self_assessment("sa.missing").unwrap();
        assert_eq!(s.self_assessments().len(), 1);
        assert_eq!(s.outbox_len(), before + 4);
    }

    #[test]
    fn an_entry_and_its_delete_reach_the_other_device() {
        let phone = Store::open_in_memory().unwrap();
        let tablet = Store::open_in_memory().unwrap();
        let relay = |from: &Store, seq: u64| -> Vec<SeqOp> {
            let ops = from.outbox(500).unwrap();
            from.remove_from_outbox(ops.iter().map(|o| o.id.as_str())).unwrap();
            ops.into_iter().enumerate().map(|(i, op)| SeqOp { seq: seq + i as u64, op }).collect()
        };
        phone.put_self_assessment(&entry("sa.x", "2027-06-01", 70)).unwrap();
        let ops = relay(&phone, 1);
        assert!(ops.iter().all(|o| o.op.table == "self_assessments"));
        tablet.apply_remote(&ops, 1).unwrap();
        assert_eq!(tablet.self_assessments(), phone.self_assessments());
        phone.delete_self_assessment("sa.x").unwrap();
        tablet.apply_remote(&relay(&phone, 2), 2).unwrap();
        assert!(tablet.self_assessments().is_empty());
    }

    #[test]
    fn ids_are_prefixed_and_differ() {
        let (a, b) = (new_id(), new_id());
        assert!(a.starts_with(PREFIX) && a.len() == PREFIX.len() + 16);
        assert_ne!(a, b);
    }
}
