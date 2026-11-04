//! Mohamed's own cards in the `own_cards` table (synced, last writer wins).

use rusqlite::params;
use steps_core::own::{OwnCard, PREFIX};

use super::{Result, Store, now_ms};

impl Store {
    /// Every live card, newest first. Rows that don't parse are skipped.
    pub fn own_cards(&self) -> Vec<OwnCard> {
        let conn = self.conn.lock().unwrap();
        let Ok(mut stmt) = conn.prepare("SELECT value FROM own_cards WHERE deleted = 0") else {
            return Vec::new();
        };
        let mut cards: Vec<OwnCard> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map(|rows| rows.flatten().filter_map(|v| serde_json::from_str(&v).ok()).collect())
            .unwrap_or_default();
        cards.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then_with(|| a.id.cmp(&b.id)));
        cards
    }

    /// Adds or replaces a card (same id).
    pub fn put_own_card(&self, card: &OwnCard) -> Result<()> {
        let hlc = self.tick(now_ms());
        self.put_lww("own_cards", "id", &card.id, None, &serde_json::to_string(card)?, false, &hlc)
    }

    /// Deletes a card everywhere (a tombstone, so other devices drop it too).
    /// Its reviews stay in the log; they just no longer match a card.
    pub fn delete_own_card(&self, id: &str) -> Result<()> {
        let value: Option<String> = {
            let conn = self.conn.lock().unwrap();
            conn.query_row("SELECT value FROM own_cards WHERE id = ?1", params![id], |r| r.get(0)).ok()
        };
        let Some(value) = value else { return Ok(()) };
        let hlc = self.tick(now_ms());
        self.put_lww("own_cards", "id", id, None, &value, true, &hlc)
    }
}

/// `own.<16 hex digits>`.
pub fn new_id() -> String {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        bytes = now_ms().to_le_bytes();
    }
    format!("{PREFIX}{:016x}", u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, at: i64) -> OwnCard {
        OwnCard {
            id: id.into(),
            front: "Q".into(),
            back: "A".into(),
            lecture: None,
            page: None,
            source: None,
            created_ms: at,
        }
    }

    #[test]
    fn add_edit_delete_and_queue_for_sync() {
        let s = Store::open_in_memory().unwrap();
        let before = s.outbox_len();
        s.put_own_card(&card("own.a", 1)).unwrap();
        s.put_own_card(&card("own.b", 2)).unwrap();
        s.put_own_card(&OwnCard { front: "Q2".into(), ..card("own.a", 1) }).unwrap();
        let cards = s.own_cards();
        assert_eq!(cards.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["own.b", "own.a"]);
        assert_eq!(cards[1].front, "Q2");
        s.delete_own_card("own.b").unwrap();
        s.delete_own_card("own.missing").unwrap();
        assert_eq!(s.own_cards().len(), 1);
        assert_eq!(s.outbox_len(), before + 4);
    }

    #[test]
    fn ids_are_prefixed_and_differ() {
        let (a, b) = (new_id(), new_id());
        assert!(a.starts_with(PREFIX) && a.len() == PREFIX.len() + 16);
        assert_ne!(a, b);
    }
}
