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

#[cfg(test)]
mod tests {
    use super::*;
    use steps_core::review::{Grade, Origin};

    fn event(id: &str) -> ReviewEvent {
        ReviewEvent { item_id: id.into(), grade: Grade::Good, origin: Origin::Study, at_ms: 1, took_ms: 2 }
    }

    #[test]
    fn answers_in_memory_are_logged_retracted_and_never_due() {
        let mut s = MemoryStore::default();
        assert!(s.is_new("cvs.L01.001"));
        s.record(event("cvs.L01.001"));
        let second = s.record_logged(event("cvs.L01.003")).unwrap().unwrap();
        assert_eq!(second, "mem-2");
        assert!(!s.is_new("cvs.L01.001") && !s.is_new("cvs.L01.003"));
        // Undo takes exactly that answer back; an unknown id is already gone.
        assert!(s.retract(&second));
        assert!(s.is_new("cvs.L01.003"));
        assert!(s.retract("mem-99"));
        assert!(s.due(0).is_empty());
        assert!(!s.is_hidden("cvs.L01.001"));
        assert_eq!(s.next_due_ms(0), None);
    }

    #[test]
    fn without_the_app_store_the_player_keeps_answers_in_memory() {
        fn app() -> Element {
            let store = use_store();
            let id = store.0.borrow_mut().record_logged(event("cvs.L01.001")).unwrap().unwrap_or_default();
            rsx! { p { "{id}" } }
        }
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        assert!(dioxus::ssr::render(&dom).contains("mem-"));
        assert!(now_ms() > 1_700_000_000_000);
    }
}
