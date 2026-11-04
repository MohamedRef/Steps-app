//! The seam between the study player and the review store (chunk 3): the
//! store `App` provides as a `StudyStoreHandle`, or, until it does, answers
//! kept in memory for the run.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::review::{MemorySink, ReviewEvent, ReviewSink};
pub use steps_core::review::{StudyStore, StudyStoreHandle};

/// Answers kept in memory for this run, used until the real store exists.
#[derive(Default)]
pub(super) struct MemoryStore {
    sink: MemorySink,
    /// The id each event in `sink` was logged under.
    ids: Vec<String>,
    next: u64,
}

impl StudyStore for MemoryStore {
    fn record(&mut self, event: ReviewEvent) {
        let _ = self.record_logged(event);
    }
    fn record_logged(&mut self, event: ReviewEvent) -> Result<Option<String>, String> {
        self.next += 1;
        let id = format!("mem-{}", self.next);
        self.ids.push(id.clone());
        self.sink.record(event);
        Ok(Some(id))
    }
    fn retract(&mut self, event_id: &str) -> bool {
        let Some(i) = self.ids.iter().position(|x| x == event_id) else { return true };
        self.ids.remove(i);
        self.sink.events.remove(i);
        true
    }
    fn due(&self, _now_ms: i64) -> Vec<String> {
        Vec::new()
    }
    fn is_new(&self, item_id: &str) -> bool {
        !self.sink.events.iter().any(|e| e.item_id == item_id)
    }
    fn is_hidden(&self, _item_id: &str) -> bool {
        false
    }
    fn next_due_ms(&self, _now_ms: i64) -> Option<i64> {
        None
    }
}

/// The store from `App`'s context, or the in-memory one.
pub fn use_store() -> StudyStoreHandle {
    use_hook(|| {
        try_consume_context::<StudyStoreHandle>().unwrap_or_else(|| {
            thread_local! {
                static FALLBACK: StudyStoreHandle =
                    StudyStoreHandle(Rc::new(RefCell::new(MemoryStore::default())));
            }
            FALLBACK.with(Clone::clone)
        })
    })
}

pub fn now_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
