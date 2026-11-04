//! Mohamed's notes in the `notes` table (synced, last writer wins), one row
//! per anchor (`steps_core::note::Anchor::row_id`). The table has no tie to
//! content: a content-pack update never reads or writes it.

use rusqlite::params;
use steps_core::note::{Anchor, Note};

use super::{Result, Store, now_ms};

impl Store {
    /// Every live note. Rows that don't parse are skipped.
    pub fn notes(&self) -> Vec<Note> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(mut stmt) = conn.prepare("SELECT value FROM notes WHERE deleted = 0 ORDER BY id") else {
            return Vec::new();
        };
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map(|rows| rows.flatten().filter_map(|v| serde_json::from_str(&v).ok()).collect())
            .unwrap_or_default()
    }

    /// Adds or replaces the note on its anchor.
    pub fn put_note(&self, note: &Note) -> Result<()> {
        let hlc = self.tick(now_ms());
        self.put_lww("notes", "id", &note.anchor.row_id(), None, &serde_json::to_string(note)?, false, &hlc)
    }

    /// Deletes the note on `anchor` everywhere (a tombstone). Nothing is
    /// written when there is no note.
    pub fn delete_note(&self, anchor: &Anchor) -> Result<()> {
        let id = anchor.row_id();
        let value: Option<String> = {
            let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            conn.query_row("SELECT value FROM notes WHERE id = ?1 AND deleted = 0", params![id], |r| r.get(0))
                .ok()
        };
        let Some(value) = value else { return Ok(()) };
        let hlc = self.tick(now_ms());
        self.put_lww("notes", "id", &id, None, &value, true, &hlc)
    }
}

#[cfg(test)]
mod tests {
    use steps_protocol::sync::{Op, OpKind, SeqOp};

    use super::*;

    fn note(anchor: Anchor, text: &str) -> Note {
        Note { anchor, text: text.into(), page: Some(284), created_ms: 1, edited_ms: 1 }
    }

    #[test]
    fn add_edit_delete_and_queue_for_sync() {
        let s = Store::open_in_memory().unwrap();
        let before = s.outbox_len();
        let item = Anchor::Item("cvs.L01.003".into());
        s.put_note(&note(item.clone(), "first")).unwrap();
        s.put_note(&note(Anchor::Page(284), "page")).unwrap();
        s.put_note(&note(item.clone(), "edited")).unwrap();
        let notes = s.notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes.iter().find(|n| n.anchor == item).unwrap().text, "edited");
        s.delete_note(&item).unwrap();
        // Nothing to delete: nothing queued.
        s.delete_note(&item).unwrap();
        s.delete_note(&Anchor::Page(1)).unwrap();
        assert_eq!(s.notes().len(), 1);
        assert_eq!(s.outbox_len(), before + 4);
        // A deleted note can be written again.
        s.put_note(&note(item.clone(), "back")).unwrap();
        assert_eq!(s.notes().len(), 2);
    }

    /// A note written on one device reaches another through the ops the
    /// outbox sends, edits and deletes included, in any arrival order.
    #[test]
    fn notes_round_trip_between_devices() {
        let phone = Store::open_in_memory().unwrap();
        let tablet = Store::open_in_memory().unwrap();
        let start = phone.outbox_len();
        let anchor = Anchor::Item("renal.L03.010".into());
        phone.put_note(&note(anchor.clone(), "RTA 1 = distal")).unwrap();
        phone.put_note(&note(anchor.clone(), "RTA 1 = distal, stones")).unwrap();
        let ops: Vec<SeqOp> = phone
            .outbox(100)
            .unwrap()
            .into_iter()
            .skip(start as usize)
            .enumerate()
            .map(|(i, op)| SeqOp { seq: i as u64 + 1, op })
            .collect();
        assert!(ops.iter().all(|s| s.op.table == "notes" && s.op.kind == OpKind::Upsert));
        // The newer edit wins even when it arrives first.
        let mut reversed = ops.clone();
        reversed.reverse();
        let changed = tablet.apply_remote(&reversed, 2).unwrap();
        assert!(changed.contains("notes"));
        assert_eq!(tablet.notes()[0].text, "RTA 1 = distal, stones");

        // A delete on the tablet tombstones the phone's copy.
        tablet.delete_note(&anchor).unwrap();
        let del: Vec<Op> = tablet.outbox(100).unwrap().into_iter().filter(|o| o.table == "notes").collect();
        let last = del.last().unwrap().clone();
        assert_eq!(last.payload.as_ref().unwrap()["deleted"], 1);
        phone.apply_remote(&[SeqOp { seq: 3, op: last }], 3).unwrap();
        assert!(phone.notes().is_empty());
    }
}
