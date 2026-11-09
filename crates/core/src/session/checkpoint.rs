//! Versioned, device-local Study checkpoints, so an unfinished session comes
//! back after the app is closed or Android ends it in the background. A
//! checkpoint is only reopened when the review log still matches the state
//! that wrote it (adapted from batna-app main: crates/core/src/session/checkpoint.rs).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::review::ReviewEvent;

use super::Session;

/// Kept on this device only: never a synced setting, never a review event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCheckpoint {
    version: u32,
    review_marker: String,
    session: Session,
}

impl SessionCheckpoint {
    pub const VERSION: u32 = 1;

    pub fn new(session: &Session, review_marker: String) -> Self {
        Self { version: Self::VERSION, review_marker, session: session.clone() }
    }

    /// Saved by a build before the review revision: its marker is the
    /// whole-log fingerprint, so the app checks it against that once, and
    /// the next save writes the v2 revision.
    pub fn uses_legacy_review_marker(&self) -> bool {
        self.review_marker.starts_with("v1:")
    }

    /// The saved session, or `None` for an old or corrupt snapshot, a grade
    /// missing from (or changed in) the log, or an item that can't be shown:
    /// better to start fresh than to risk a second FSRS answer.
    ///
    /// The marker is taken *after* the newest saved answer. If an answer was
    /// saved but its checkpoint wasn't, the markers differ at the next start.
    pub fn recover(
        self,
        current_marker: &str,
        live_reviews: &HashMap<String, ReviewEvent>,
        playable: impl Fn(&str) -> bool,
    ) -> Option<Session> {
        (self.version == Self::VERSION
            && self.review_marker == current_marker
            && self.session.valid_for_restore(live_reviews, playable))
        .then_some(self.session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Grade, Origin};
    use crate::session::Planned;

    fn plan(id: &str) -> Planned {
        Planned {
            item_id: id.into(),
            lecture: "cvs.L04".into(),
            topic: "cvs.L04/hypertension".into(),
            est_secs: 20,
            fresh: true,
        }
    }

    fn event(id: &str, grade: Grade) -> ReviewEvent {
        ReviewEvent::new(id, grade, Origin::Study, 42, 20)
    }

    #[test]
    fn checkpoint_round_trips_retry_order_summary_and_real_undo_event() {
        let mut session = Session::new(vec![plan("a"), plan("b")]);
        session.grade_logged(Grade::Again, Some("review-a".into()), |_| false);
        let logged = HashMap::from([("review-a".into(), event("a", Grade::Again))]);
        let raw = serde_json::to_string(&SessionCheckpoint::new(&session, "1:0".into())).unwrap();
        let copy: SessionCheckpoint = serde_json::from_str(&raw).unwrap();
        let mut restored = copy.recover("1:0", &logged, |_| true).unwrap();
        assert_eq!(restored.progress(), (1, 3));
        assert_eq!(restored.current().unwrap().item_id, "b");
        assert_eq!(restored.summary(), session.summary());
        assert_eq!(restored.undo().unwrap().event.as_deref(), Some("review-a"));
        assert_eq!(restored.progress(), (0, 2));
    }

    #[test]
    fn failed_or_stale_checkpoints_never_advance_or_duplicate_reviews() {
        let mut session = Session::new(vec![plan("a"), plan("b")]);
        session.grade_logged(Grade::Good, Some("review-a".into()), |_| false);
        let live = HashMap::from([("review-a".into(), event("a", Grade::Good))]);
        let checkpoint = SessionCheckpoint::new(&session, "1:0".into());
        assert!(checkpoint.clone().recover("2:0", &live, |_| true).is_none(), "stale marker");
        assert!(checkpoint.clone().recover("1:0", &HashMap::new(), |_| true).is_none(), "grade gone");
        assert!(checkpoint.clone().recover("1:0", &live, |id| id != "b").is_none(), "item gone");
        let mut wrong = checkpoint.clone();
        wrong.version += 1;
        assert!(wrong.recover("1:0", &live, |_| true).is_none(), "other version");
        let mut altered = live.clone();
        altered.insert("review-a".into(), event("a", Grade::Easy));
        assert!(checkpoint.clone().recover("1:0", &altered, |_| true).is_none(), "grade changed");
        let mut bank = live.clone();
        bank.insert("review-a".into(), ReviewEvent::new("a", Grade::Good, Origin::Bank, 42, 20));
        assert!(checkpoint.recover("1:0", &bank, |_| true).is_none(), "not a Study answer");
    }

    #[test]
    fn a_finished_or_unlogged_session_is_not_restored() {
        let mut done = Session::new(vec![plan("a")]);
        done.grade_logged(Grade::Good, Some("review-a".into()), |_| false);
        let live = HashMap::from([("review-a".into(), event("a", Grade::Good))]);
        assert!(SessionCheckpoint::new(&done, "m".into()).recover("m", &live, |_| true).is_none());
        let mut unlogged = Session::new(vec![plan("a"), plan("b")]);
        unlogged.grade(Grade::Good);
        assert!(SessionCheckpoint::new(&unlogged, "m".into()).recover("m", &live, |_| true).is_none());
        assert!(
            SessionCheckpoint::new(&Session::default(), "m".into()).recover("m", &live, |_| true).is_none()
        );
    }

    #[test]
    fn only_a_v1_marker_reads_as_legacy() {
        let session = Session::new(vec![plan("a")]);
        assert!(SessionCheckpoint::new(&session, "v1:0:cbf29ce484222325".into()).uses_legacy_review_marker());
        assert!(!SessionCheckpoint::new(&session, "v2:00ff".into()).uses_legacy_review_marker());
    }

    #[test]
    fn unanswered_session_needs_no_replayed_grade() {
        let session = Session::new(vec![plan("a")]);
        let back =
            SessionCheckpoint::new(&session, "empty".into()).recover("empty", &HashMap::new(), |_| true);
        assert_eq!(back.unwrap().current().unwrap().item_id, "a");
    }
}
