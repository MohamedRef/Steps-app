//! The questions he adds from a file (`steps_core::ownq`): one JSON row per
//! question in `own_questions`, and each figure in `own_images` as chunks
//! that each fit one sync op. Both synced, last writer wins.

use rusqlite::params;
use steps_core::ownq::{self, Image, OwnQuestion};

use super::{Result, Store, now_ms};

/// A stored figure chunk.
#[derive(serde::Serialize, serde::Deserialize)]
struct Chunk {
    kind: String,
    data: String,
}

impl Store {
    /// Every live question, by category and id. Rows that don't parse are skipped.
    pub fn own_questions(&self) -> Vec<OwnQuestion> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(mut stmt) = conn.prepare("SELECT value FROM own_questions WHERE deleted = 0 ORDER BY id")
        else {
            return Vec::new();
        };
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map(|rows| rows.flatten().filter_map(|v| serde_json::from_str(&v).ok()).collect())
            .unwrap_or_default()
    }

    /// Adds or replaces questions (same category and id), with the figures
    /// they bring; a figure already stored is not written again.
    pub fn put_own_questions(&self, questions: &[OwnQuestion], figures: &[(Image, Vec<u8>)]) -> Result<()> {
        for (image, bytes) in figures {
            if self.own_image_stored(image) {
                continue;
            }
            for (id, data) in ownq::chunks(&image.sha256, bytes) {
                let value = serde_json::to_string(&Chunk { kind: image.kind.clone(), data })?;
                let hlc = self.tick(now_ms());
                self.put_lww("own_images", "id", &id, None, &value, false, &hlc)?;
            }
        }
        for q in questions {
            let hlc = self.tick(now_ms());
            self.put_lww("own_questions", "id", &q.key(), None, &serde_json::to_string(q)?, false, &hlc)?;
        }
        Ok(())
    }

    /// Deletes a category's questions everywhere (tombstones). Their answers
    /// stay in the log; figures stay, since another question may use them.
    pub fn delete_own_category(&self, category: &str) -> Result<usize> {
        let gone: Vec<OwnQuestion> =
            self.own_questions().into_iter().filter(|q| q.category == category).collect();
        for q in &gone {
            let hlc = self.tick(now_ms());
            self.put_lww("own_questions", "id", &q.key(), None, &serde_json::to_string(q)?, true, &hlc)?;
        }
        Ok(gone.len())
    }

    /// A figure's bytes, once every chunk is here.
    pub fn own_image(&self, image: &Image) -> Option<Vec<u8>> {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        ownq::join(image, |id| {
            let value: String = conn
                .query_row("SELECT value FROM own_images WHERE id = ?1 AND deleted = 0", params![id], |r| {
                    r.get(0)
                })
                .ok()?;
            serde_json::from_str::<Chunk>(&value).ok().map(|c| c.data)
        })
    }

    fn own_image_stored(&self, image: &Image) -> bool {
        let conn = self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let ids: Vec<String> = (0..image.chunks).map(|i| format!("{}.{i}", image.sha256)).collect();
        ids.iter().all(|id| {
            conn.query_row("SELECT 1 FROM own_images WHERE id = ?1 AND deleted = 0", params![id], |_| Ok(()))
                .is_ok()
        })
    }
}

#[cfg(test)]
#[path = "ownq_tests.rs"]
mod tests;
