//! Session selection decisions shared by the study setup and start requests.
//! Keep composition and item filtering separate from Dioxus rendering.

use std::collections::HashSet;

use steps_core::content::Lecture;
use steps_core::session::{Inputs, Planned, compose};

use super::{PLAYABLE, StudyStoreHandle, find, now_ms};

pub(super) fn plan(
    store: &StudyStoreHandle,
    lectures: &[&Lecture],
    topics: &HashSet<String>,
    minutes: u32,
) -> Vec<Planned> {
    let now = now_ms();
    let s = store.0.borrow();
    let all = lectures.iter().flat_map(|l| l.items.iter());
    let seen: HashSet<String> = all.clone().filter(|i| !s.is_new(&i.id)).map(|i| i.id.clone()).collect();
    let hidden: HashSet<String> = all.filter(|i| s.is_hidden(&i.id)).map(|i| i.id.clone()).collect();
    compose(&Inputs {
        lectures,
        topics,
        kinds: PLAYABLE,
        due: &s.due(now),
        seen: &seen,
        hidden: &hidden,
        max_new: crate::plan::new_left_today(&*s, now),
        budget_secs: minutes * 60,
    })
}

/// A session on exactly the given items, in order, skipping duplicates and
/// anything this player can't show or the learner hid.
pub(super) fn plan_items(store: &StudyStoreHandle, ids: &[String]) -> Vec<Planned> {
    let s = store.0.borrow();
    let mut seen = HashSet::new();
    ids.iter()
        .filter(|id| seen.insert(id.as_str()))
        .filter_map(|id| find(id))
        .filter(|(_, i)| PLAYABLE.contains(&i.body.kind()) && !s.is_hidden(&i.id))
        .map(|(l, i)| Planned {
            item_id: i.id.clone(),
            lecture: l.lecture.clone(),
            topic: i.topic.as_ref().map_or_else(|| l.lecture.clone(), |t| format!("{}/{t}", l.lecture)),
            est_secs: steps_core::session::estimate_secs(&i.body),
            fresh: s.is_new(&i.id),
        })
        .collect()
}
