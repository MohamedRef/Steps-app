//! `try` steps: the lecture's own cards and questions, inside the lesson.
//! They use the study player's renderers and grade row, so an answer here is
//! a real review: it goes to the store and FSRS schedules the item.

use std::collections::HashMap;

use dioxus::prelude::*;
use steps_core::content::{Item, Lecture};
use steps_core::review::{Grade, Origin, ReviewEvent};

use crate::content::Derived;
use crate::i18n::learn as tx;
use crate::study::{ItemView, PLAYABLE, use_store};

/// Every item by id, with its lecture.
fn try_context() -> Option<Tries> {
    try_consume_context::<Tries>()
}

fn try_context_step() -> usize {
    try_consume_context::<super::steps::StepAt>().map_or(0, |s| s.0)
}

pub fn find(id: &str) -> Option<(&'static Lecture, &'static Item)> {
    static MAP: Derived<HashMap<&'static str, (&'static Lecture, &'static Item)>> = Derived::new();
    MAP.get(|lib| {
        lib.lectures.values().flat_map(|l| l.items.iter().map(move |i| (i.id.as_str(), (l, i)))).collect()
    })
    .get(id)
    .copied()
}

/// The open attempt (its Try logs) and a way to save it.
#[derive(Clone, Copy)]
pub struct Tries {
    pub attempt: Signal<steps_core::response::Attempt>,
    pub save: Callback<()>,
}

#[component]
pub fn TryItems(items: Vec<String>, on_answer: EventHandler<bool>) -> Element {
    let store = use_store();
    // Kinds the study player can't show yet are left out, and hidden (✗)
    // items. The list is taken once: a ✗ he marks here hides the card from
    // the next visit on, so the position always points at the same list.
    let playable: Vec<String> = use_hook(|| {
        let store = store.0.borrow();
        items
            .iter()
            .filter(|id| find(id).is_some_and(|(_, i)| PLAYABLE.contains(&i.body.kind())))
            .filter(|id| !store.is_hidden(id))
            .cloned()
            .collect()
    });
    let n = playable.len();
    // Cards graded before he left the lesson mid-step stay graded.
    let tries = try_context();
    let k = try_context_step();
    // Taken once, like the list: later grades go to the log, not to this.
    let before = use_hook(|| {
        tries.map(|t| t.attempt.peek().tries.get(k).cloned().unwrap_or_default()).unwrap_or_default()
    });
    let n_before = playable.iter().filter(|id| before.graded.contains(id)).count();
    let playable: Vec<String> = playable.into_iter().filter(|id| !before.graded.contains(id)).collect();
    let mut at = use_signal(|| 0usize);
    let mut again = use_signal(|| before.again as usize);
    let mut started = use_signal(web_time::Instant::now);
    let mut reported = use_signal(|| false);
    use_effect(move || {
        // Nothing playable, or every card graded before he left mid-step.
        if n == n_before && !reported() {
            reported.set(true);
            on_answer.call(again() == 0);
        }
    });

    let body = match playable.get(at()).and_then(|id| find(id)) {
        Some((lecture, item)) => {
            let item = store.0.borrow().effective(item);
            let fresh = store.0.borrow().is_new(&item.id);
            let topic_title = item
                .topic
                .as_ref()
                .and_then(|t| lecture.topics.iter().find(|x| &x.id == t))
                .map(|t| t.title.clone());
            let id = item.id.clone();
            let store = store.clone();
            let on_grade = move |grade: Grade| {
                store.0.borrow_mut().record(ReviewEvent {
                    item_id: id.clone(),
                    grade,
                    origin: Origin::Study,
                    at_ms: crate::store::now_ms(),
                    took_ms: started().elapsed().as_millis().min(u32::MAX as u128) as u32,
                });
                if grade == Grade::Again {
                    *again.write() += 1;
                }
                if let Some(mut t) = tries {
                    if let Some(log) = t.attempt.write().tries.get_mut(k) {
                        log.graded.push(id.clone());
                        log.again = again() as u32;
                    }
                    t.save.call(());
                }
                started.set(web_time::Instant::now());
                let next = at() + 1;
                at.set(next);
                if next + n_before == n && !reported() {
                    reported.set(true);
                    on_answer.call(again() == 0);
                }
            };
            rsx! {
                div { key: "{at()}-{item.id}", class: "ls-try-item",
                    p { class: "muted small", "{tx::TRY_OF} {at() + n_before + 1} {tx::OF} {n}" }
                    ItemView {
                        item,
                        lecture_title: lecture.title.clone(),
                        topic_title,
                        fresh,
                        retry: false,
                        on_grade,
                    }
                }
            }
        }
        None => rsx! {
            p { class: "ls-verdict good", {tx::TRY_DONE} }
        },
    };
    rsx! {
        div { class: "ls-try",
            p { class: "ls-kicker", {tx::TRY} }
            {body}
        }
    }
}
