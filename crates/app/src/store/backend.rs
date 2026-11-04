//! The review state the screens share: the log replayed through FSRS, the
//! verdicts, and the settings, kept in memory over the store. It implements
//! [`StudyStore`] for the study player and the other study screens.

mod study;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use dioxus::prelude::*;
use steps_core::content::{Body, Item, Status};
use steps_core::review::verify::{self, Check, Edit, Judgement, Verdict, Verdicts};
use steps_core::review::{LoggedReview, ReviewEvent, StudyStore, StudyStoreHandle};
use steps_core::srs::{self, CardState, ReviewSettings, Scheduler};

use super::pages::{self, Pages};
use super::{Store, now_ms};
use crate::content::Library;
use crate::study::PLAYABLE;

/// Kinds the Cases screen plays, so they come back there when due (#242).
pub const CASE_KINDS: &[&str] = &["case"];

/// One study item with the lecture it belongs to.
#[derive(Clone, Debug)]
pub struct Entry {
    pub lecture: String,
    pub item: Item,
}

/// Every item in the built-in content, by id, in book order of lectures.
pub type Catalog = BTreeMap<String, Entry>;

pub fn catalog(lib: &Library) -> Catalog {
    lib.lectures
        .values()
        .flat_map(|l| {
            l.items.iter().map(|i| (i.id.clone(), Entry { lecture: l.lecture.clone(), item: i.clone() }))
        })
        .collect()
}

pub struct ReviewBackend {
    store: &'static Store,
    catalog: Catalog,
    pub settings: ReviewSettings,
    scheduler: Scheduler,
    log: Vec<LoggedReview>,
    states: BTreeMap<String, CardState>,
    judgements: BTreeMap<String, Judgement>,
    /// First Aid pages read and the page gate (`store::pages`).
    pages: Pages,
    /// His notes (`steps_core::note`), re-read with everything else.
    notes: Vec<steps_core::note::Note>,
    /// Bumps the screens' revision signal after every write.
    on_change: Option<Box<dyn FnMut()>>,
}

/// Rough time per item, for "about N minutes" on Today.
pub fn seconds_for(body: &Body) -> u32 {
    match body {
        Body::Card { .. } | Body::Cloze { .. } => 15,
        Body::Mcq { .. } => 45,
        Body::Enumerate { .. } | Body::Order { .. } => 60,
        Body::Table { .. } => 90,
        Body::Essay { .. } => 120,
        Body::Case { .. } => 180,
    }
}

/// What Today shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overview {
    /// Due items a study session plays (`study::PLAYABLE`).
    pub due: usize,
    /// Due cases, which the Cases screen plays (#242).
    pub due_cases: Vec<String>,
    pub due_minutes: u32,
    /// New items he can still start today (the daily cap and what's written).
    pub new_today: usize,
    /// New items written and not started yet, ignoring the cap (and only
    /// from pages read, with the page gate on).
    pub unseen: usize,
    /// New items waiting for their First Aid page to be read.
    pub locked: usize,
    pub due_tomorrow: u32,
    /// Items he has checked (✓ or ✎), and items he has seen at all.
    pub checked: usize,
    pub seen: usize,
    pub lectures: Vec<LectureProgress>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LectureProgress {
    pub id: String,
    pub title: String,
    pub total: usize,
    pub seen: usize,
    pub due: usize,
}

impl ReviewBackend {
    pub fn new(store: &'static Store, catalog: Catalog) -> Self {
        let mut b = Self {
            store,
            catalog,
            settings: ReviewSettings::default(),
            scheduler: Scheduler::default(),
            log: Vec::new(),
            states: BTreeMap::new(),
            judgements: BTreeMap::new(),
            pages: Pages::default(),
            notes: Vec::new(),
            on_change: None,
        };
        b.reload();
        b
    }

    /// Re-reads everything from the store.
    pub fn reload(&mut self) {
        self.settings = self.store.review_settings();
        self.scheduler = Scheduler::from_settings(&self.settings, super::zone());
        self.log = self.store.reviews().unwrap_or_default();
        self.judgements = self.store.judgements().unwrap_or_default();
        self.pages = Pages::load(self.store);
        self.refresh_own();
        self.notes = self.store.notes();
        self.replay();
    }

    /// A content update switched the library while Steps is open: the
    /// catalog follows it, so a verdict is stored with the item as he sees
    /// it now (its fingerprint), and new items count on Today.
    pub fn library_replaced(&mut self) {
        self.catalog = catalog(Library::get());
        self.reload();
        self.changed();
    }

    /// Puts his own cards in the catalog (and publishes the deck for the
    /// study player), replacing the previous set.
    fn refresh_own(&mut self) {
        let lec = crate::own::publish(self.store.own_cards());
        self.catalog.retain(|id, _| !steps_core::own::is_own(id));
        for item in &lec.items {
            self.catalog.insert(item.id.clone(), Entry { lecture: lec.lecture.clone(), item: item.clone() });
        }
    }

    /// Adds or changes one of his cards.
    pub fn save_own(&mut self, card: &steps_core::own::OwnCard) -> super::Result<()> {
        self.store.put_own_card(card)?;
        self.refresh_own();
        self.changed();
        Ok(())
    }

    /// Deletes one of his cards on every device. Its reviews stay in the log.
    pub fn delete_own(&mut self, id: &str) -> super::Result<()> {
        self.store.delete_own_card(id)?;
        self.refresh_own();
        self.changed();
        Ok(())
    }

    /// Every note he has written.
    pub fn notes(&self) -> &[steps_core::note::Note] {
        &self.notes
    }

    /// The note on `anchor`, if any.
    pub fn note(&self, anchor: &steps_core::note::Anchor) -> Option<&steps_core::note::Note> {
        self.notes.iter().find(|n| &n.anchor == anchor)
    }

    /// Saves what he typed on `anchor`: empty text deletes the note.
    pub fn save_note(
        &mut self,
        anchor: steps_core::note::Anchor,
        text: &str,
        page: Option<u32>,
    ) -> std::result::Result<(), NoteError> {
        use steps_core::note::{Save, save};
        let existing = self.note(&anchor).cloned();
        match save(anchor.clone(), text, page, existing.as_ref(), now_ms()).map_err(NoteError::Check)? {
            Save::Put(n) => self.store.put_note(&n),
            Save::Delete => self.store.delete_note(&anchor),
        }
        .map_err(NoteError::Store)?;
        self.notes = self.store.notes();
        self.changed();
        Ok(())
    }

    /// Re-reads everything and tells the screens (after an import).
    /// After the store changed from outside this run (a sync, a restore, an
    /// import): reload the review state and every synced-setting global.
    pub fn reload_and_notify(&mut self) {
        self.reload();
        crate::synced::reload();
        self.changed();
    }

    fn replay(&mut self) {
        self.states = self.scheduler.replay_all(&self.log);
    }

    fn changed(&mut self) {
        if let Some(f) = &mut self.on_change {
            f();
        }
    }

    pub fn entry(&self, item_id: &str) -> Option<&Entry> {
        self.catalog.get(item_id)
    }

    pub fn judgement(&self, item_id: &str) -> Option<&Judgement> {
        self.judgements.get(item_id)
    }

    /// Whether to ask ✓ / ✎ / ✗ for an item. Unknown items never ask.
    pub fn check(&self, item_id: &str) -> Check {
        match self.entry(item_id) {
            Some(e) => verify::check(&e.item, self.judgement(item_id)),
            None => Check::Settled,
        }
    }

    /// Records ✓ / ✎ / ✗ on the item as it is now. False when it wasn't
    /// saved (reported to the log).
    pub fn judge(&mut self, item_id: &str, verdict: Verdict, edit: Edit, note: Option<String>) -> bool {
        let Some(entry) = self.entry(item_id) else {
            crate::diag::report!("no item {item_id} to judge");
            return false;
        };
        let j = Judgement {
            verdict,
            edit: if verdict == Verdict::Edited { edit } else { Edit::new() },
            note: note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
            fingerprint: verify::fingerprint(&entry.item),
            at_ms: now_ms(),
        };
        if let Err(e) = self.store.set_judgement(item_id, Some(&j)) {
            super::write_failed(&format!("the verdict on {item_id}"), &e);
            return false;
        }
        super::write_recovered(&format!("the verdict on {item_id}"));
        self.judgements.insert(item_id.to_string(), j);
        self.changed();
        true
    }

    /// Takes a verdict back: the item is unverified again. False when it
    /// wasn't saved (reported to the log).
    pub fn undo_judgement(&mut self, item_id: &str) -> bool {
        if let Err(e) = self.store.set_judgement(item_id, None) {
            super::write_failed(&format!("taking back the verdict on {item_id}"), &e);
            return false;
        }
        super::write_recovered(&format!("taking back the verdict on {item_id}"));
        self.judgements.remove(item_id);
        self.changed();
        true
    }

    pub fn pages(&self) -> &Pages {
        &self.pages
    }

    /// Marks First Aid's printed pages `first..=last` read or unread. False
    /// when a course's pages weren't saved (reported to the log).
    pub fn set_pages_read(&mut self, lib: &Library, first: u32, last: u32, read: bool) -> bool {
        let mut ok = true;
        for (course, read_pages) in self.pages.set(lib, first, last, read) {
            let what = format!("pages read in {course}");
            if let Err(e) = self.store.put_setting(&format!("{}{course}", pages::KEY), &read_pages) {
                super::write_failed(&what, &e);
                ok = false;
                continue;
            }
            super::write_recovered(&what);
            self.pages = self.pages.with_course(&course, read_pages);
        }
        self.changed();
        ok
    }

    /// The switch in Settings › Study: new items only from pages read.
    pub fn set_page_gate(&mut self, on: bool) -> bool {
        if let Err(e) = self.store.put_setting(pages::GATE_KEY, &on) {
            super::write_failed("the page gate", &e);
            return false;
        }
        super::write_recovered("the page gate");
        self.pages = self.pages.with_gate(on);
        self.changed();
        true
    }

    /// Whether a new item may start now: the page gate lets it through.
    fn startable(&self, lib: &Library, entry: &Entry) -> bool {
        let Some(read) = self.pages.gate() else { return true };
        steps_core::own::is_own(&entry.item.id)
            || lib
                .lectures
                .get(&entry.lecture)
                .is_none_or(|l| steps_core::pages::unlocks(&entry.item, &l.book, read))
    }

    /// False when the settings weren't saved (reported to the log).
    pub fn set_settings(&mut self, mut settings: ReviewSettings) -> bool {
        // The background refit writes `fsrs` straight to the store, so the
        // copy these settings were built from may be older (#248). No screen
        // edits the fit, so the stored one always wins.
        settings.fsrs = self.store.review_settings().fsrs;
        if let Err(e) = self.store.put_review_settings(&settings) {
            super::write_failed("review settings", &e);
            return false;
        }
        super::write_recovered("review settings");
        self.settings = settings;
        self.scheduler = Scheduler::from_settings(&self.settings, super::zone());
        self.replay();
        self.changed();
        true
    }

    /// Items falling due on each of the next `days` study days (index 0 is
    /// today, overdue included). Items marked ✗ are left out.
    pub fn due_forecast(&self, days: usize, now_ms: i64) -> Vec<u32> {
        self.scheduler.forecast(self.states.values().filter(|s| self.comes_back(&s.item_id)), now_ms, days)
    }

    /// Answers logged today (every screen, evidence included).
    pub fn answers_today(&self, now_ms: i64) -> usize {
        let today = self.scheduler.day.day(now_ms);
        self.log.iter().filter(|l| self.scheduler.day.day(l.event.at_ms) == today).count()
    }

    /// The study day `now_ms` falls in (the rollover hour applied).
    pub fn study_day(&self, now_ms: i64) -> i64 {
        self.scheduler.day.day(now_ms)
    }

    /// Answers logged on each of the last `days` study days, oldest first
    /// (the last entry is today).
    pub fn answers_per_day(&self, days: usize, now_ms: i64) -> Vec<u32> {
        let today = self.scheduler.day.day(now_ms);
        let mut out = vec![0; days];
        for l in &self.log {
            let ago = today - self.scheduler.day.day(l.event.at_ms);
            if (0..days as i64).contains(&ago) {
                out[days - 1 - ago as usize] += 1;
            }
        }
        out
    }

    /// New items started on each of the last `days` study days, oldest
    /// first (the last entry is today).
    pub fn learned_per_day(&self, days: usize, now_ms: i64) -> Vec<u32> {
        let today = self.scheduler.day.day(now_ms);
        let mut out = vec![0; days];
        for first in self.states.values().filter_map(|s| s.first_review_ms) {
            let ago = today - self.scheduler.day.day(first);
            if (0..days as i64).contains(&ago) {
                out[days - 1 - ago as usize] += 1;
            }
        }
        out
    }

    pub fn reviews_logged(&self) -> usize {
        self.log.len()
    }

    /// The verdicts export (`steps.verdicts/v1`), every verdict on every item.
    pub fn verdicts(&self, now_ms: i64) -> Verdicts {
        Verdicts::from_judgements(self.judgements.iter().map(|(id, j)| (id.as_str(), j)), now_ms)
    }

    pub fn verdict_counts(&self) -> HashMap<Verdict, usize> {
        let mut out = HashMap::new();
        for j in self.judgements.values() {
            *out.entry(j.verdict).or_default() += 1;
        }
        out
    }

    fn hidden(&self, item_id: &str) -> bool {
        self.check(item_id) == Check::Hidden
    }

    fn kind(&self, item_id: &str) -> Option<&'static str> {
        self.entry(item_id).map(|e| e.item.body.kind())
    }

    /// Items some screen brings back when due: a session's kinds and the
    /// Cases screen's. MCQs are scored in the bank and have no review round,
    /// so their schedule never shows as work (#242).
    fn comes_back(&self, item_id: &str) -> bool {
        self.kind(item_id).is_some_and(|k| PLAYABLE.contains(&k) || CASE_KINDS.contains(&k))
            && !self.hidden(item_id)
    }

    fn due_states(&self, now_ms: i64) -> Vec<&CardState> {
        self.scheduler.due(self.states.values().filter(|s| self.comes_back(&s.item_id)), now_ms)
    }

    /// Due items a study session can play, most urgent first.
    pub fn session_due(&self, now_ms: i64) -> Vec<String> {
        self.due_states(now_ms)
            .into_iter()
            .filter(|s| self.kind(&s.item_id).is_some_and(|k| PLAYABLE.contains(&k)))
            .map(|s| s.item_id.clone())
            .collect()
    }

    pub fn overview(&self, lib: &Library, now_ms: i64) -> Overview {
        let (due, cases): (Vec<_>, Vec<_>) = self
            .due_states(now_ms)
            .into_iter()
            .partition(|s| self.kind(&s.item_id).is_some_and(|k| PLAYABLE.contains(&k)));
        let due_minutes = due
            .iter()
            .filter_map(|s| self.entry(&s.item_id))
            .map(|e| seconds_for(&e.item.body))
            .sum::<u32>()
            .div_ceil(60);
        let (unseen, locked) =
            self.catalog.iter().filter(|(id, _)| !self.states.contains_key(*id) && !self.hidden(id)).fold(
                (0, 0),
                |(open, shut), (_, e)| {
                    if self.startable(lib, e) { (open + 1, shut) } else { (open, shut + 1) }
                },
            );
        let tomorrow = self.scheduler.day.day_start_ms(self.scheduler.day.day(now_ms) + 1);
        let due_tomorrow = self
            .states
            .values()
            .filter(|s| s.due_ms.is_some_and(|d| d > now_ms && d <= tomorrow) && self.comes_back(&s.item_id))
            .count() as u32;
        let checked = self
            .catalog
            .iter()
            .filter(|(id, e)| verify::check(&e.item, self.judgement(id)) == Check::Settled)
            .count();
        let mut lectures = Vec::new();
        for entry in &lib.index.lectures {
            let Some(lec) = lib.lectures.get(&entry.id) else { continue };
            let seen = lec.items.iter().filter(|i| self.states.contains_key(&i.id)).count();
            if seen == 0 {
                continue;
            }
            let lec_due =
                due.iter().filter(|s| self.entry(&s.item_id).is_some_and(|e| e.lecture == entry.id)).count();
            lectures.push(LectureProgress {
                id: entry.id.clone(),
                title: entry.title.clone(),
                total: lec.items.len(),
                seen,
                due: lec_due,
            });
        }
        Overview {
            due: due.len(),
            due_cases: cases.into_iter().map(|s| s.item_id.clone()).collect(),
            due_minutes,
            new_today: self.new_left_today(now_ms).min(unseen),
            unseen,
            locked,
            due_tomorrow,
            checked,
            seen: self.states.keys().filter(|id| self.catalog.contains_key(*id)).count(),
            lectures,
        }
    }
}

/// Why a note didn't save.
#[derive(Debug)]
pub enum NoteError {
    Check(steps_core::note::Problem),
    Store(super::StoreError),
}

/// The review state in context, with a revision the screens subscribe to.
#[derive(Clone)]
pub struct Review {
    pub backend: Rc<RefCell<ReviewBackend>>,
    pub revision: Signal<u64>,
}

/// Called once in `App`: opens the store, provides [`Review`] and the
/// [`StudyStoreHandle`] for the study screens, and starts the weekly FSRS
/// refit in the background.
pub fn use_provide_review() {
    let review = use_context_provider(|| {
        let backend = Rc::new(RefCell::new(ReviewBackend::new(super::shared(), catalog(Library::get()))));
        let mut revision = Signal::new(0);
        backend.borrow_mut().on_change = Some(Box::new(move || *revision.write() += 1));
        spawn_refit();
        Review { backend, revision }
    });
    use_context_provider(|| StudyStoreHandle(review.backend.clone() as Rc<RefCell<dyn StudyStore>>));
}

/// The review state, subscribed: the calling component re-renders after
/// every answer, verdict or settings change.
pub fn use_review() -> Review {
    let review = use_context::<Review>();
    let _ = (review.revision)();
    review
}

/// Fits personal FSRS parameters once a week when there are enough reviews
/// (`MIN_REVIEWS_TO_OPTIMIZE`). Off the UI thread; the new parameters apply
/// from the next launch.
fn spawn_refit() {
    let _ = std::thread::Builder::new().name("fsrs-refit".into()).spawn(|| {
        let store = super::shared();
        let now = now_ms();
        let settings = store.review_settings();
        if !settings.refit_due(now) {
            return;
        }
        let Ok(log) = store.reviews() else { return };
        let scheduler = Scheduler::from_settings(&settings, super::zone());
        if let Some(params) = scheduler.optimize(&log, now) {
            // Re-read so a settings change made meanwhile isn't lost.
            let mut latest = store.review_settings();
            latest.fsrs = Some(params);
            let _ = store.put_review_settings(&latest);
        }
    });
}

#[cfg(test)]
mod tests;
