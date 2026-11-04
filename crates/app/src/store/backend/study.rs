//! StudyStore adapter for the review backend.

use super::*;

impl StudyStore for ReviewBackend {
    fn record(&mut self, event: ReviewEvent) {
        let _ = self.record_logged(event);
    }

    fn record_logged(&mut self, event: ReviewEvent) -> Result<Option<String>, String> {
        let id = event.item_id.clone();
        match self.store.append_review(event) {
            Ok(logged) => {
                // Rating again after a failed save clears Today's note.
                super::super::write_recovered(&format!("the answer to {id}"));
                let logged_id = logged.id.clone();
                self.log.push(logged);
                self.replay();
                self.changed();
                Ok(Some(logged_id))
            }
            // Today shows the note too, so a lost answer is never silent
            // (#244); the session keeps the item open.
            Err(e) => {
                super::super::write_failed(&format!("the answer to {id}"), &e);
                Err(e.to_string())
            }
        }
    }

    fn retract(&mut self, event_id: &str) -> bool {
        match self.store.retract_review(event_id) {
            Ok(_) => {
                super::super::write_recovered(&format!("taking back the answer {event_id}"));
                let before = self.log.len();
                self.log.retain(|l| l.id != event_id);
                if self.log.len() != before {
                    self.replay();
                    self.changed();
                }
                true
            }
            Err(e) => {
                super::super::write_failed(&format!("taking back the answer {event_id}"), &e);
                false
            }
        }
    }

    fn due(&self, now_ms: i64) -> Vec<String> {
        self.due_states(now_ms).into_iter().map(|s| s.item_id.clone()).collect()
    }

    fn is_new(&self, item_id: &str) -> bool {
        self.states.get(item_id).is_none_or(CardState::is_new)
    }

    fn is_hidden(&self, item_id: &str) -> bool {
        self.hidden(item_id)
    }

    fn effective(&self, item: &Item) -> Item {
        verify::effective(item, self.judgement(&item.id))
    }

    fn next_due_ms(&self, now_ms: i64) -> Option<i64> {
        self.states
            .values()
            .filter(|s| self.comes_back(&s.item_id))
            .filter_map(|s| s.due_ms)
            .filter(|d| *d > now_ms)
            .min()
    }

    fn new_left_today(&self, now_ms: i64) -> usize {
        srs::new_left_today(&self.settings, self.scheduler.started_today(self.states.values(), now_ms))
    }

    fn intervals(&self, item_id: &str, now_ms: i64) -> Option<[f32; 4]> {
        let state = self.states.get(item_id).cloned().unwrap_or_else(|| CardState::new(item_id));
        Some(self.scheduler.intervals(&state, now_ms))
    }

    fn log(&self) -> Vec<ReviewEvent> {
        self.log.iter().map(|l| l.event.clone()).collect()
    }

    fn marks(&self) -> BTreeMap<String, Status> {
        self.judgements
            .iter()
            .filter(|(id, j)| self.entry(id).is_some_and(|e| j.current(&e.item)))
            .map(|(id, j)| {
                let status = match j.verdict {
                    Verdict::Approved | Verdict::Edited => Status::Approved,
                    Verdict::Wrong => Status::Rejected,
                };
                (id.clone(), status)
            })
            .collect()
    }

    fn forecast(&self, days: usize, now_ms: i64) -> Vec<u32> {
        self.due_forecast(days, now_ms)
    }
}
