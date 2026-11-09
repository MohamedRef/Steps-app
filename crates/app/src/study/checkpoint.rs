//! An unfinished Study session survives a restart: the queue is saved on this
//! device after every step, and comes back at the next start only when the
//! review log still matches it (`steps_core::session::SessionCheckpoint`).
//! Never synced, and restoring it never replays an answer.
//! Adapted from batna-app main: crates/app/src/study/checkpoint.rs.

use std::collections::HashMap;

use dioxus::prelude::*;
use steps_core::session::{Session, SessionCheckpoint};
use web_time::Instant;

use super::{PHASE, Phase, SESSION, STARTED, StudyStoreHandle};

pub(crate) const KEY: &str = "study.session.v1";

/// Called once by the app shell, after the review store is provided, so
/// Today can offer "Continue" on the very first screen after a restart.
pub(crate) fn use_restore() {
    let backend = super::use_store();
    use_hook(move || {
        if *PHASE.peek() != Phase::Setup || SESSION.peek().current().is_some() {
            return;
        }
        if let Some(saved) = load(&backend) {
            *SESSION.write() = saved;
            *PHASE.write() = Phase::Running;
            *STARTED.write() = Instant::now();
        }
    });
}

fn load(backend: &StudyStoreHandle) -> Option<Session> {
    let db = crate::store::shared();
    let checkpoint = db.local_setting::<SessionCheckpoint>(KEY)?;
    // A checkpoint from a build before the review revision is checked once
    // against the whole-log fingerprint it was saved with (never against a
    // weaker one); the next save writes the v2 marker.
    let marker = if checkpoint.uses_legacy_review_marker() {
        db.study_review_marker_legacy()
    } else {
        db.study_review_marker()
    };
    let recovered = marker.ok().and_then(|marker| {
        let live: HashMap<_, _> = db.reviews().ok()?.into_iter().map(|r| (r.id, r.event)).collect();
        checkpoint.recover(&marker, &live, |id| super::is_playable(id) && !backend.0.borrow().is_hidden(id))
    });
    if recovered.is_none() {
        clear();
    }
    recovered
}

/// Saves the queue *after* it has moved on. Never called between an answer
/// being saved and the session taking it: a crash in that gap leaves the old
/// marker, which no longer matches, so nothing is restored twice.
pub(super) fn save_running(session: &Session) {
    let db = crate::store::shared();
    let result = db
        .study_review_marker()
        .and_then(|marker| db.put_local_setting(KEY, &SessionCheckpoint::new(session, marker)));
    if let Err(error) = result {
        crate::diag::report!("saving the Study checkpoint: {error}");
    }
}

pub(super) fn clear() {
    if let Err(error) = crate::store::shared().put_local_setting(KEY, &Option::<SessionCheckpoint>::None) {
        crate::diag::report!("clearing the Study checkpoint: {error}");
    }
}

#[cfg(test)]
mod tests {
    //! Against the real SQLite store (each test thread has its own), not the
    //! global session (adapted from batna-app main: crates/app/src/study/checkpoint.rs).
    use std::cell::RefCell;
    use std::rc::Rc;

    use steps_core::review::{Grade, Origin, ReviewEvent};
    use steps_core::session::Planned;

    use super::*;

    fn memory_backend() -> StudyStoreHandle {
        StudyStoreHandle(Rc::new(RefCell::new(super::super::backend::MemoryStore::default())))
    }

    /// The first item the player shows of this kind.
    fn first(kind: &str) -> String {
        super::super::ready_lectures()
            .into_iter()
            .flat_map(|l| &l.items)
            .find(|i| i.body.kind() == kind)
            .map(|i| i.id.clone())
            .unwrap_or_else(|| panic!("a {kind} in the content"))
    }

    fn planned(id: &str) -> Planned {
        Planned {
            item_id: id.to_string(),
            lecture: steps_core::program::lecture_of(id).to_string(),
            topic: format!("{}/t", steps_core::program::lecture_of(id)),
            est_secs: 20,
            fresh: true,
        }
    }

    #[test]
    fn a_restart_reopens_the_exact_queue_and_keeps_undo() {
        let db = crate::store::shared();
        let (card, cloze) = (first("card"), first("cloze"));
        let mut session = Session::new(vec![planned(&card), planned(&cloze)]);
        save_running(&session);
        let backend = memory_backend();
        assert_eq!(load(&backend).unwrap().progress(), (0, 2));

        // Grading saves the answer first, then the checkpoint.
        let logged = db.append_review(ReviewEvent::new(&card, Grade::Again, Origin::Study, 100, 20)).unwrap();
        session.grade_logged(Grade::Again, Some(logged.id.clone()), |_| false);
        save_running(&session);

        let mut restored = load(&backend).expect("the saved answer and the checkpoint agree");
        assert_eq!(restored.progress(), (1, 3), "the Again retry comes back too");
        assert_eq!(restored.current().unwrap().item_id, cloze);
        assert_eq!(restored.undo().unwrap().event.as_deref(), Some(logged.id.as_str()));

        clear();
        assert!(load(&backend).is_none(), "an ended session does not reopen");
    }

    #[test]
    fn a_crash_between_the_answer_and_the_checkpoint_starts_fresh() {
        let db = crate::store::shared();
        let card = first("card");
        save_running(&Session::new(vec![planned(&card)]));
        db.append_review(ReviewEvent::new(&card, Grade::Good, Origin::Study, 100, 20)).unwrap();

        // The old checkpoint would ask for that item again and log a second
        // answer: it is dropped, and the saved answer stays.
        assert!(load(&memory_backend()).is_none());
        assert_eq!(db.reviews().unwrap().len(), 1);
        assert!(db.local_setting::<SessionCheckpoint>(KEY).is_none());
    }

    #[test]
    fn a_v1_checkpoint_reopens_once_and_the_next_save_is_v2() {
        let db = crate::store::shared();
        let session = Session::new(vec![planned(&first("card"))]);
        let legacy = db.study_review_marker_legacy().unwrap();
        db.put_local_setting(KEY, &SessionCheckpoint::new(&session, legacy)).unwrap();

        let backend = memory_backend();
        assert!(load(&backend).is_some(), "checked against the log it was saved with");
        save_running(&session);
        let saved = db.local_setting::<SessionCheckpoint>(KEY).unwrap();
        assert!(!saved.uses_legacy_review_marker(), "the next save writes the revision");
        assert!(load(&backend).is_some());
        clear();
    }

    #[test]
    fn a_v1_checkpoint_after_another_answer_starts_fresh() {
        let db = crate::store::shared();
        let card = first("card");
        let legacy = db.study_review_marker_legacy().unwrap();
        db.put_local_setting(KEY, &SessionCheckpoint::new(&Session::new(vec![planned(&card)]), legacy))
            .unwrap();
        db.append_review(ReviewEvent::new(&card, Grade::Good, Origin::Study, 100, 20)).unwrap();
        assert!(load(&memory_backend()).is_none());
    }
}
