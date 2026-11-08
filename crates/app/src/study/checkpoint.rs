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
    let recovered = db.study_review_marker().ok().and_then(|marker| {
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
