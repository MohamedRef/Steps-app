//! Study: a short session of mixed items from the lectures Mohamed picks,
//! with feedback right after each answer (docs/WORKSTREAMS.md).
//!
//! Setup → items one by one → summary. The order comes from
//! `steps_core::session::compose`; answers go to the review store.

mod outcome;
use outcome::*;

mod setup;
use setup::*;

mod backend;
#[cfg(test)]
mod browser;
mod card;
mod cloze;
mod enumerate;
mod essay;
pub mod grade;
mod item;
mod planning;

use std::collections::{BTreeSet, HashMap, HashSet};
use web_time::Instant;

use dioxus::prelude::*;
use steps_core::content::{Item, Lecture};
use steps_core::review::{Confidence, ErrorReason, Grade, Origin, ReviewEvent};
use steps_core::session::Session;

pub(crate) use backend::use_store;
pub use backend::{StudyStore, StudyStoreHandle};
pub use item::{ItemView, PLAYABLE};

use crate::content::{Derived, Library};
use crate::i18n::count::{DAY, HOUR, ITEM, MINUTE, REVIEW, count};
use crate::i18n::study as tx;
use crate::look::{Css, css};
use backend::now_ms;
use item::short_text;
use planning::{plan, plan_items};

pub(crate) const STUDY_CSS: Css = css!("assets/study.css");
const MINUTES: [u32; 4] = [5, 10, 15, 20];

/// A request to start a session straight away (Today's "start" button sets
/// it, then switches to the Study screen).
#[derive(Debug, Clone, PartialEq)]
pub struct Start {
    /// Lecture ids (`hb.L01`); `None` means every ready lecture.
    pub lectures: Option<Vec<String>>,
    /// Exactly these item ids, in this order ("study these", a session from
    /// mistakes). Overrides `lectures` and the time budget; ids the player
    /// can't show or that are hidden are skipped.
    pub items: Option<Vec<String>>,
    pub minutes: u32,
}

pub static START: GlobalSignal<Option<Start>> = Signal::global(|| None);

/// A START that came while a session was running, waiting for him to choose
/// between adding it to the session, starting it instead, or dropping it.
static PENDING: GlobalSignal<Option<Start>> = Signal::global(|| None);

/// Every item by id, with its lecture.
fn items() -> &'static HashMap<String, (&'static Lecture, &'static Item)> {
    static MAP: Derived<HashMap<String, (&'static Lecture, &'static Item)>> = Derived::new();
    MAP.get(|lib| {
        lib.lectures.values().flat_map(|l| l.items.iter().map(move |i| (i.id.clone(), (l, i)))).collect()
    })
}

/// The name a lecture shows under: the book's title, or "your cards".
fn lecture_title(l: &Lecture) -> String {
    if l.lecture == steps_core::own::LECTURE {
        crate::i18n::own::TITLE.to_string()
    } else {
        Library::get().title(&l.lecture).to_string()
    }
}

/// Whether the study player can show this item (MCQs go to the bank, cases
/// to their own screen).
pub fn is_playable(id: &str) -> bool {
    find(id).is_some_and(|(_, i)| PLAYABLE.contains(&i.body.kind()))
}

/// A book item or one of his own cards, by id.
fn find(id: &str) -> Option<(&'static Lecture, &'static Item)> {
    items().get(id).copied().or_else(|| crate::own::find(id))
}

/// Lectures that have at least one item this player can show, then his own
/// cards' deck when he has any.
fn ready_lectures() -> Vec<&'static Lecture> {
    Library::get()
        .lectures
        .values()
        .filter(|l| l.items.iter().any(|i| PLAYABLE.contains(&i.body.kind())))
        .chain(crate::own::lecture())
        .collect()
}

/// A lecture the setup offers: one of the course in view, or his own deck.
fn in_view(lecture: &str) -> bool {
    lecture == steps_core::own::LECTURE || crate::courses::focus::shows(lecture)
}

/// Cards due now in the courses out of view (none with every course in view).
fn due_elsewhere(store: &StudyStoreHandle) -> usize {
    if crate::courses::focus::current().is_none() {
        return 0;
    }
    let s = store.0.borrow();
    s.due(now_ms())
        .iter()
        .filter(|id| !in_view(steps_core::program::lecture_of(id)) && !s.is_hidden(id))
        .count()
}

static SESSION: GlobalSignal<Session> = Signal::global(Session::default);
static PHASE: GlobalSignal<Phase> = Signal::global(|| Phase::Setup);
static STARTED: GlobalSignal<Instant> = Signal::global(Instant::now);
/// What the last Undo said: taken back (the item is reopened) or not saved,
/// or that the last rating wasn't saved.
static UNDO_NOTE: GlobalSignal<Option<&'static str>> = Signal::global(|| None);
/// Ratings that couldn't be saved so far. The rating row lets him rate again
/// once this moves past the count it was pressed at.
pub(crate) static SAVE_FAILS: GlobalSignal<u32> = Signal::global(|| 0);

/// The optional taps on the item on screen (`crate::metacog`): his
/// confidence before the answer came out, and the reason a reason chip
/// rated it "تاني" with. Cleared when an item opens and once its rating is saved.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Tags {
    pub confidence: Option<Confidence>,
    pub reason: Option<ErrorReason>,
}

pub(crate) static TAGS: GlobalSignal<Tags> = Signal::global(Tags::default);

/// The answer to log: the grade plus the taps, a reason only on "تاني".
fn tagged_event(item_id: String, grade: Grade, at_ms: i64, took_ms: u32, tags: Tags) -> ReviewEvent {
    ReviewEvent {
        confidence: tags.confidence,
        reason: tags.reason.filter(|_| grade == Grade::Again),
        ..ReviewEvent::new(item_id, grade, Origin::Study, at_ms, took_ms)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Setup,
    Running,
    Summary,
}

#[component]
pub fn Study() -> Element {
    let store = use_store();
    // His own cards' deck changes with the review store (a save, an import, a sync).
    let _ = crate::store::use_review().revision.read();
    // The course in view (`courses::focus`), and his own cards' deck.
    let focus = crate::courses::focus::current();
    let ready: Vec<&Lecture> = ready_lectures().into_iter().filter(|l| in_view(&l.lecture)).collect();
    let mut chosen = use_signal(|| ready.iter().map(|l| l.lecture.clone()).collect::<BTreeSet<String>>());
    // A new course in view starts with all its lectures picked.
    let mut chosen_for = use_signal(|| focus.clone());
    use_effect(move || {
        let now = crate::courses::focus::current();
        if *chosen_for.peek() != now {
            chosen_for.set(now);
            chosen.set(
                ready_lectures()
                    .into_iter()
                    .filter(|l| in_view(&l.lecture))
                    .map(|l| l.lecture.clone())
                    .collect(),
            );
        }
    });
    let topics = use_signal(HashSet::<String>::new);
    let mut minutes = use_signal(crate::onboarding::session_minutes);
    // Global, so a session survives switching tabs (like Bank and Oral).
    let mut session = SESSION.signal();
    let mut phase = PHASE.signal();
    let mut started = STARTED.signal();
    let mut undo_note = UNDO_NOTE.signal();
    // A running session gets the whole phone screen; "end" is the way out.
    crate::parts::use_focus_mode(phase() == Phase::Running);
    // Coming back to a running session: don't count the time away.
    let mut quiet = use_hook(|| {
        if *phase.peek() == Phase::Running {
            started.set(Instant::now());
        }
        Signal::new(*phase.peek() == Phase::Summary)
    });
    // The calm end-of-session tone, however the session ended (not again when
    // coming back to a summary already shown).
    use_effect(move || {
        let ended = phase() == Phase::Summary;
        if *quiet.peek() {
            quiet.set(false);
            return;
        }
        if ended && session.peek().summary().items > 0 {
            crate::sounds::play(crate::sounds::Cue::SessionEnd);
        }
    });

    let mut begin = {
        let store = store.clone();
        move |lectures: Vec<&'static Lecture>, topics: HashSet<String>, mins: u32| {
            let planned = plan(&store, &lectures, &topics, mins);
            let empty = planned.is_empty();
            undo_note.set(None);
            session.set(Session::new(planned));
            started.set(Instant::now());
            phase.set(if empty { Phase::Summary } else { Phase::Running });
        }
    };

    // Starts what a START request asks for, replacing whatever is shown.
    let apply = {
        let store = store.clone();
        let begin = begin.clone();
        move |req: Start| {
            if let Some(ids) = &req.items {
                let planned = plan_items(&store, ids);
                let empty = planned.is_empty();
                undo_note.set(None);
                session.set(Session::new(planned));
                started.set(Instant::now());
                phase.set(if empty { Phase::Summary } else { Phase::Running });
                return;
            }
            let picked: Vec<&Lecture> = ready_lectures()
                .into_iter()
                .filter(|l| req.lectures.as_ref().is_none_or(|ids| ids.contains(&l.lecture)))
                .collect();
            minutes.set(req.minutes);
            let mut begin = begin.clone();
            begin(picked, HashSet::new(), req.minutes);
        }
    };
    let mut pending = PENDING.signal();
    // The question only makes sense while that session runs.
    use_effect(move || {
        if phase() != Phase::Running && pending.peek().is_some() {
            pending.set(None);
        }
    });

    // Today's "start a session" button, and any "study these" asked for
    // while Study is already open (a book page's sheet, a lecture page over
    // it): an effect, so a request made here is not left waiting. A session
    // in progress is never dropped without asking (#319): the request waits
    // in PENDING for his answer.
    use_effect({
        let mut apply = apply.clone();
        move || {
            if START.read().is_none() {
                return;
            }
            if let Some(req) = START.write().take() {
                let live = *phase.peek() == Phase::Running && !session.peek().is_done();
                if live {
                    pending.set(Some(req));
                } else {
                    apply(req);
                }
            }
        }
    });

    // Takes the newest grade back: its review event gets a tombstone (the
    // schedule replays without it) and the session steps back to that item.
    // The session only moves once the store has let the answer go, so the
    // two never disagree.
    let undo = {
        let store = store.clone();
        move || {
            let mut back = session.peek().clone();
            let Some(undone) = back.undo() else { return };
            let gone = undone.event.as_deref().is_none_or(|id| store.0.borrow_mut().retract(id));
            if !gone {
                undo_note.set(Some(tx::UNDO_FAILED));
                return;
            }
            session.set(back);
            undo_note.set(Some(tx::UNDONE));
            started.set(Instant::now());
            phase.set(Phase::Running);
        }
    };

    let body = match phase() {
        Phase::Setup if ready.is_empty() => rsx! {
            crate::ui::Card {
                h1 { {tx::TITLE} }
                p { {tx::EMPTY_LIBRARY} }
            }
            crate::own::OwnCardsCard { on_study: move |_| {} }
        },
        Phase::Setup => {
            let picked: Vec<&Lecture> =
                ready.iter().copied().filter(|l| chosen.read().contains(&l.lecture)).collect();
            let preview = plan(&store, &picked, &topics.read(), minutes());
            let mut begin_own = begin.clone();
            rsx! {
                Setup {
                    ready: ready.iter().map(|l| (l.lecture.clone(), lecture_title(l))).collect::<Vec<_>>(),
                    chosen,
                    topics,
                    minutes,
                    preview_count: preview.len(),
                    preview_new: preview.iter().filter(|p| p.fresh).count(),
                    preview_mins: preview.iter().map(|p| p.est_secs).sum::<u32>().div_ceil(60),
                    elsewhere: due_elsewhere(&store),
                    on_start: move |_| begin(picked.clone(), topics.read().clone(), minutes()),
                }
                crate::own::OwnCardsCard {
                    on_study: move |_| begin_own(crate::own::lecture().into_iter().collect(), HashSet::new(), minutes()),
                }
            }
        }
        Phase::Running => {
            let s = session.read();
            let (pos, len) = s.progress();
            let current = s.current().cloned();
            let retry = s.is_retry();
            let can_undo = s.can_undo();
            drop(s);
            let reopened = undo_note() == Some(tx::UNDONE);
            match current.and_then(|p| find(&p.item_id).map(|(l, i)| (p, l, i))) {
                // Only when content changed under a running session; "end" still works.
                None => rsx! {
                    button { class: "btn btn-primary btn-lg", onclick: move |_| phase.set(Phase::Summary), {tx::END} }
                },
                Some((p, lecture, item)) => {
                    let item = store.0.borrow().effective(item);
                    let topic_title = item
                        .topic
                        .as_ref()
                        .and_then(|t| lecture.topics.iter().find(|x| &x.id == t))
                        .map(|t| t.title.clone());
                    let store_for_add = store.clone();
                    let store = store.clone();
                    let item_id = p.item_id.clone();
                    let on_grade = move |grade: Grade| {
                        let took = started().elapsed().as_millis().min(u32::MAX as u128) as u32;
                        let tags = *TAGS.peek();
                        let answer = tagged_event(item_id.clone(), grade, now_ms(), took, tags);
                        let event = match store.0.borrow_mut().record_logged(answer) {
                            Ok(event) => event,
                            // Not saved: stay on this item with his answer on
                            // screen, say so, and let him rate again.
                            Err(_) => {
                                undo_note.set(Some(tx::SAVE_FAILED));
                                *SAVE_FAILS.write() += 1;
                                return;
                            }
                        };
                        *TAGS.write() = Tags::default();
                        let hidden = |id: &str| store.0.borrow().is_hidden(id);
                        session.write().grade_logged(grade, event, hidden);
                        undo_note.set(None);
                        started.set(Instant::now());
                        if session.read().is_done() {
                            phase.set(Phase::Summary);
                        } else {
                            crate::sounds::play(if grade == Grade::Again {
                                crate::sounds::Cue::Wrong
                            } else {
                                crate::sounds::Cue::Right
                            });
                        }
                    };
                    rsx! {
                        div { class: "st-progress",
                            div { class: "progress st-bar", "data-tone": "good", role: "progressbar",
                                aria_valuemin: "0", aria_valuemax: "{len}", aria_valuenow: "{pos}",
                                aria_label: "{pos + 1} {tx::OF} {len}",
                                aria_valuetext: "{pos + 1} {tx::OF} {len}",
                                div { class: "progress-fill", style: "inline-size: {pos * 100 / len.max(1)}%" }
                            }
                            span { class: "muted small num-ltr st-count", "{pos + 1} / {len}" }
                            if can_undo {
                                button {
                                    class: "btn-link st-undo",
                                    title: tx::UNDO_HINT,
                                    aria_label: tx::UNDO_HINT,
                                    onclick: {
                                        let mut undo = undo.clone();
                                        move |_| undo()
                                    },
                                    {tx::UNDO}
                                }
                            }
                            button {
                                class: "btn-link",
                                "aria-keyshortcuts": "Escape",
                                "data-back": "true",
                                onclick: move |_| phase.set(Phase::Summary),
                                {tx::END}
                            }
                        }
                        if let Some(note) = undo_note() {
                            p {
                                class: "note st-undo-note",
                                role: "status",
                                "data-failed": if note == tx::UNDO_FAILED || note == tx::SAVE_FAILED { "yes" } else { "no" },
                                {note}
                            }
                        }
                        if let Some(req) = pending() {
                            PendingCard {
                                req,
                                on_add: {
                                    move |ids: Vec<String>| {
                                        let more = plan_items(&store_for_add, &ids);
                                        session.write().append(more);
                                        pending.set(None);
                                    }
                                },
                                on_replace: {
                                    let mut apply = apply.clone();
                                    move |req: Start| {
                                        pending.set(None);
                                        apply(req);
                                    }
                                },
                                on_keep: move |_| pending.set(None),
                            }
                        }
                        // Keyed so every item (and a retry of the same one) starts fresh.
                        for (item , topic_title , on_grade) in std::iter::once((item, topic_title, on_grade)) {
                            ItemView {
                                key: "{pos}-{p.item_id}",
                                item,
                                lecture_title: lecture_title(lecture),
                                topic_title,
                                fresh: p.fresh,
                                retry,
                                reopened,
                                tagging: true,
                                on_grade,
                            }
                        }
                    }
                }
            }
        }
        Phase::Summary => {
            let summary = session.read().summary();
            let next = store.0.borrow().next_due_ms(now_ms());
            let shaky: Vec<(String, String)> = summary
                .shaky
                .iter()
                .filter_map(|id| find(id).map(|(_, i)| (id.clone(), short_text(i))))
                .collect();
            rsx! {
                SummaryView {
                    items: summary.items,
                    right: summary.right,
                    fresh: summary.fresh,
                    shaky,
                    next: next.map(|t| when_label(now_ms(), t, &crate::store::zone())),
                    can_undo: session.read().can_undo(),
                    on_undo: {
                        let mut undo = undo.clone();
                        move |_| undo()
                    },
                    undo_failed: undo_note() == Some(tx::UNDO_FAILED),
                    on_again: move |_| phase.set(Phase::Setup),
                }
            }
        }
    };

    rsx! {
        style { {STUDY_CSS.text()} }
        div { class: "st", {body} }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tag_tests {
    use super::*;

    #[test]
    fn a_reason_goes_only_on_again() {
        let tags = Tags { confidence: Some(Confidence::Sure), reason: Some(ErrorReason::Knowledge) };
        let again = tagged_event("x.1".into(), Grade::Again, 1, 2, tags);
        assert_eq!((again.confidence, again.reason), (Some(Confidence::Sure), Some(ErrorReason::Knowledge)));
        let good = tagged_event("x.1".into(), Grade::Good, 1, 2, tags);
        assert_eq!((good.confidence, good.reason), (Some(Confidence::Sure), None));
        assert_eq!(
            tagged_event("x.1".into(), Grade::Hard, 1, 2, Tags::default()),
            ReviewEvent::new("x.1", Grade::Hard, Origin::Study, 1, 2)
        );
    }
}

/// Asked when "study this" arrives during a session: his session, its
/// retries and its summary stay unless he says otherwise.
#[component]
fn PendingCard(
    req: Start,
    on_add: EventHandler<Vec<String>>,
    on_replace: EventHandler<Start>,
    on_keep: EventHandler<()>,
) -> Element {
    let n = req.items.as_ref().map(Vec::len);
    rsx! {
        crate::ui::Card { class: "st-pending", role: "alertdialog", aria_label: tx::PENDING_TITLE,
            p { strong { {tx::PENDING_TITLE} } }
            p { class: "muted small", {tx::PENDING_BODY} }
            div { class: "st-actions",
                if let (Some(ids), Some(n)) = (req.items.clone(), n) {
                    button { class: "btn btn-primary", onclick: move |_| on_add.call(ids.clone()), "{tx::PENDING_ADD} ({n})" }
                }
                button {
                    class: "btn btn-secondary",
                    onclick: {
                        let req = req.clone();
                        move |_| on_replace.call(req.clone())
                    },
                    {tx::PENDING_REPLACE}
                }
                button { class: "btn-link", "data-back": "true", onclick: move |_| on_keep.call(()), {tx::PENDING_KEEP} }
            }
        }
    }
}
