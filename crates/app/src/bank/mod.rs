//! The question bank (docs/WORKSTREAMS.md): practise MCQs by lecture, topic or
//! tag, in tutor mode (the explanation after every question) or timed mode
//! (the review at the end), and sit timed mock exams scored per lecture and
//! topic. Picking, timing and scoring live in `steps_core::exam`.
//!
//! A paper on screen works like a Step 1 question bank's test interface, in
//! Steps' own look: a top bar (`bar.rs`) with the mark, the moves, the
//! tools and the clock (`clock.rs`), the navigator (`navigator.rs`), choices
//! he can strike out (`question.rs`) and a stem he can highlight
//! (`stem.rs`), the verdict over an answered question, and at the end the
//! block's summary (`result.rs`) and its review (`review.rs`). A mark
//! outlives its paper: marked questions are kept in the synced setting
//! `bank.marked`, and the setup can draw a paper from them.

mod bar;
mod behind;
mod clock;
mod navigator;
mod player;
mod question;
mod result;
mod review;
mod setup;
mod stem;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use web_time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use steps_core::content::Body;
use steps_core::content::{Item, Lecture};
use steps_core::exam::{self, Answer, History, Mode, Outcome, Question};
use steps_core::review::{Confidence, ErrorReason, Grade, Origin, ReviewEvent, StudyStoreHandle};

use crate::content::Library;
use crate::look::{Css, css};

const BANK_CSS: Css = css!("src/bank/bank.css");

/// A paper in progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paper {
    pub mode: Mode,
    /// Started as a block (the setup's «بلوك زي الامتحان», or a new block
    /// from a finished one) rather than as practice: its top bar says so.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub block: bool,
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>,
    pub at: usize,
    pub started_ms: i64,
    /// When the current question came on screen, for `Answer::took_ms`.
    pub shown_ms: i64,
    /// Tutor mode: the option picked on the current question but not yet
    /// checked. Checking sends it to review; moving on clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<usize>,
    /// Tutor mode: the review event the last checked question was logged
    /// under, so a reason picked after it can replace it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logged: Option<Logged>,
}

/// A tutor answer already in the review log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logged {
    /// The question's index in the paper.
    pub at: usize,
    pub event_id: String,
    pub at_ms: i64,
}

impl Paper {
    fn new(mode: Mode, questions: Vec<Question>) -> Self {
        let now = now_ms();
        Self {
            mode,
            block: false,
            answers: vec![Answer::default(); questions.len()],
            questions,
            at: 0,
            started_ms: now,
            shown_ms: now,
            draft: None,
            logged: None,
        }
    }

    /// Select an option. Tutor mode holds it as a draft until it is checked
    /// (a checked question is locked); timed papers take it as the answer,
    /// changeable until the paper goes in.
    fn pick(&mut self, option: usize) {
        let at = self.at;
        match self.mode {
            Mode::Tutor if self.answers[at].chosen.is_some() => {}
            Mode::Tutor => self.draft = Some(option),
            _ => self.answers[at].chosen = Some(option),
        }
    }

    /// Tutor mode: check the draft once. Returns the question and answer to
    /// send to review, or `None` when there is no draft or it was already
    /// checked (a second tap or Enter). Its time is every moment it was on
    /// screen until now; the explanation read after it doesn't count.
    fn check(&mut self, now: i64) -> Option<(Question, Answer)> {
        let at = self.at;
        if self.mode != Mode::Tutor || self.answers[at].chosen.is_some() {
            return None;
        }
        let option = self.draft.take()?;
        let spent = u32::try_from(now.saturating_sub(self.shown_ms).max(0)).unwrap_or(u32::MAX);
        let a = &mut self.answers[at];
        a.chosen = Some(option);
        a.took_ms = a.took_ms.saturating_add(spent);
        Some((self.questions[at].clone(), a.clone()))
    }

    /// Whether the question at `at` shows its answer: a tutor question once
    /// checked. A timed paper shows nothing until it goes in.
    pub fn revealed(&self, at: usize) -> bool {
        self.mode == Mode::Tutor && self.answers.get(at).is_some_and(|a| a.chosen.is_some())
    }

    /// Marks the current question, or unmarks it: its id and whether it is
    /// marked now.
    fn toggle_mark(&mut self) -> (String, bool) {
        let at = self.at;
        let a = &mut self.answers[at];
        a.flagged ^= true;
        (self.questions[at].id.clone(), a.flagged)
    }

    /// Strikes an option of the current question out, or brings it back.
    /// A checked tutor question is done with.
    fn strike(&mut self, option: usize) {
        if !self.revealed(self.at) {
            self.answers[self.at].toggle_strike(option);
        }
    }

    /// How sure he is of the current question, before it is checked (tutor)
    /// or while the paper runs (timed). Never once the answer is out.
    fn set_confidence(&mut self, confidence: Option<Confidence>) {
        let at = self.at;
        if self.mode == Mode::Tutor && self.answers[at].chosen.is_some() {
            return;
        }
        self.answers[at].confidence = confidence;
    }

    /// Tutor mode: why he missed the current, checked question (a second tap
    /// on the same reason takes it back). Returns the answer to log in place
    /// of the one logged at check, with the event it replaces; `None` when
    /// there is nothing to change.
    fn set_reason(&mut self, reason: ErrorReason) -> Option<(ReviewEvent, Option<Logged>)> {
        let at = self.at;
        let q = &self.questions[at];
        let a = &mut self.answers[at];
        if self.mode != Mode::Tutor || a.chosen.is_none() || a.correct(q) {
            return None;
        }
        a.reason = crate::metacog::toggled(a.reason, reason);
        let old = self.logged.take().filter(|l| l.at == at);
        let at_ms = old.as_ref().map_or(self.shown_ms, |l| l.at_ms);
        let event = exam::review_event(q, a, Mode::Tutor, at_ms)?;
        Some((event, old))
    }

    /// Add the time on the current question before moving off it (a
    /// checked tutor question's time stopped at its check).
    fn leave(&mut self, now: i64) {
        let spent = u32::try_from(now.saturating_sub(self.shown_ms).max(0)).unwrap_or(u32::MAX);
        if !self.revealed(self.at)
            && let Some(a) = self.answers.get_mut(self.at)
        {
            a.took_ms = a.took_ms.saturating_add(spent);
        }
        self.shown_ms = now;
    }

    pub fn overdue(&self, now: i64) -> bool {
        self.mode.timed() && exam::remaining_ms(self.questions.len(), self.started_ms, now) == 0
    }
}

#[derive(Clone, PartialEq)]
pub enum Phase {
    Setup,
    Running(Paper),
    Done { paper: Paper, outcome: Outcome },
}

// Globals, so a paper survives switching tabs mid-exam.
pub static PHASE: GlobalSignal<Phase> = Signal::global(|| Phase::Setup);
pub static HISTORY: GlobalSignal<History> = Signal::global(History::default);
static HISTORY_LOADED: GlobalSignal<bool> = Signal::global(|| false);

/// Finished timed papers and mocks: one synced setting per paper,
/// `bank.paper.<device>.<at_ms>`, so papers from two devices never
/// overwrite each other (#33).
const PAPER_PREFIX: &str = "bank.paper.";
/// The single list the first version kept; still read, never written.
const LEGACY_PAPERS_KEY: &str = "bank.papers";

/// A paper Mohamed left part-way (✕), timed or tutor: kept until he goes
/// back to it or ends it (#35, UX-03). A timed paper's clock keeps running.
pub static PAUSED: GlobalSignal<Option<Paper>> = Signal::global(|| None);

/// The questions he marked, on every device (`exam::History::marked`): one
/// synced setting holding the ids.
const MARKED_KEY: &str = "bank.marked";

fn stored_marks() -> BTreeSet<String> {
    crate::store::shared().setting(MARKED_KEY).unwrap_or_default()
}

/// Keeps a mark, or its removal, in the synced list. It starts from the
/// stored list, so a mark another device synced meanwhile stays.
fn keep_mark(id: &str, on: bool) {
    let mut history = History { marked: stored_marks(), ..History::default() };
    if history.mark(id, on) {
        crate::store::save_setting(MARKED_KEY, &history.marked);
    }
    if HISTORY.peek().marked != history.marked {
        HISTORY.write().marked = history.marked;
    }
}

#[component]
pub fn Bank() -> Element {
    use_hook(load_history);
    use_hook(restore_open);
    let phase = match &*PHASE.read() {
        Phase::Setup => 0,
        Phase::Running(_) => 1,
        Phase::Done { .. } => 2,
    };
    rsx! {
        style { {BANK_CSS.text()} }
        style { {crate::verify::CSS.text()} }
        div { class: "bank",
            match phase {
                0 => rsx! { setup::Setup {} },
                1 => rsx! { player::Player {} },
                _ => rsx! { result::Result {} },
            }
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// The lecture and item behind a question id.
/// With Mohamed's ✎ edit applied.
pub fn lookup(id: &str) -> Option<(&'static Lecture, Item)> {
    let lecture_id = id.rsplit_once('.').map(|(l, _)| l)?;
    let lecture = Library::get().lectures.get(lecture_id)?;
    let item = lecture.items.iter().find(|i| i.id == id)?;
    let item = match try_consume_context::<StudyStoreHandle>() {
        Some(store) => store.0.borrow().effective(item),
        None => item.clone(),
    };
    Some((lecture, item))
}

/// The MCQs of every written lecture of the course in view
/// (`courses::focus`) as Mohamed should see them: his ✎ edits applied, his
/// ✗ items left out.
pub fn study_lectures() -> Vec<Lecture> {
    study_lectures_where(crate::courses::focus::shows)
}

/// The MCQs of every written lecture that `keep` accepts (by lecture id),
/// as [`study_lectures`] gives them. A block picks its own systems, so it
/// looks past the course in view.
pub fn study_lectures_where(keep: impl Fn(&str) -> bool) -> Vec<Lecture> {
    let store = try_consume_context::<StudyStoreHandle>();
    let store = store.as_ref().map(|s| s.0.borrow());
    Library::get()
        .lectures
        .values()
        .filter(|l| keep(&l.lecture))
        .map(|l| Lecture {
            items: l
                .items
                .iter()
                .filter(|i| matches!(i.body, Body::Mcq { .. }))
                .filter(|i| !store.as_ref().is_some_and(|s| s.is_hidden(&i.id)))
                .map(|i| store.as_ref().map_or_else(|| i.clone(), |s| s.effective(i)))
                .collect(),
            ..l.clone()
        })
        .collect()
}

/// Send an answer to the review store, where FSRS counts it as evidence.
fn record(event: ReviewEvent) {
    if let Some(store) = try_consume_context::<StudyStoreHandle>() {
        store.0.borrow_mut().record(event);
    }
}

/// [`record`], keeping the id the answer was logged under.
fn record_logged(event: ReviewEvent) -> Option<String> {
    let store = try_consume_context::<StudyStoreHandle>()?;
    store.0.borrow_mut().record_logged(event).ok().flatten()
}

/// After a sync or restore (see `crate::synced`): take the finished papers
/// from the store now, and rebuild the last results from the log the next
/// time the bank opens.
pub fn reload_history() {
    let papers = stored_papers();
    if HISTORY.peek().papers != papers {
        HISTORY.write().papers = papers;
    }
    let marked = stored_marks();
    if HISTORY.peek().marked != marked {
        HISTORY.write().marked = marked;
    }
    if *HISTORY_LOADED.peek() {
        *HISTORY_LOADED.write() = false;
    }
}

/// Fill the history once per launch (and again after a sync): last results
/// from the logged bank and mock answers, finished papers from the store's
/// settings.
fn load_history() {
    if *HISTORY_LOADED.peek() {
        return;
    }
    *HISTORY_LOADED.write() = true;
    let mut history = History::default();
    if let Some(store) = try_consume_context::<StudyStoreHandle>() {
        for e in store.0.borrow().log() {
            if matches!(e.origin, Origin::Bank | Origin::Mock) {
                history.last.insert(e.item_id, e.grade != Grade::Again);
            }
        }
    }
    history.papers = stored_papers();
    history.marked = stored_marks();
    *HISTORY.write() = history;
}

/// A lecture's topic title, falling back to the slug.
pub fn topic_title(lecture: &str, topic: &str) -> String {
    Library::get()
        .lectures
        .get(lecture)
        .and_then(|l| l.topics.iter().find(|t| t.id == topic))
        .map_or_else(|| topic.to_string(), |t| t.title.clone())
}

pub fn lecture_title(lecture: &str) -> String {
    Library::get().lectures.get(lecture).map_or_else(|| lecture.to_string(), |l| l.title.clone())
}

/// The index's shorter title (`Pathology`, not the chapter's full
/// heading), for chips on a phone; falls back to the lecture's own title.
pub fn short_lecture_title(lecture: &str) -> String {
    Library::get().title(lecture).to_string()
}

/// Every finished paper on any device, oldest first.
fn stored_papers() -> Vec<Outcome> {
    let store = crate::store::shared();
    let mut papers: Vec<Outcome> = store.setting(LEGACY_PAPERS_KEY).unwrap_or_default();
    papers.extend(store.settings_with_prefix::<Outcome>(PAPER_PREFIX).into_iter().map(|(_, o)| o));
    merge_papers(papers)
}

/// Oldest first, each paper once (the legacy list and the per-paper keys
/// can both hold one).
fn merge_papers(mut papers: Vec<Outcome>) -> Vec<Outcome> {
    papers.sort_by_key(|o| o.at_ms);
    papers.dedup();
    papers
}

/// Opens the bank's setup narrowed to a lecture (or one of its topics), ready
/// to start. A paper in progress stays on screen; the filters wait for the next.
pub fn practise(mut ui: crate::Ui, lecture: &str, topic: Option<String>) {
    // Another course's lecture brings its course into view, or its
    // questions wouldn't be in the pool.
    crate::courses::focus::follow(lecture);
    setup::focus(lecture, topic);
    if matches!(*PHASE.peek(), Phase::Done { .. }) {
        *PHASE.write() = Phase::Setup;
    }
    ui.screen.set(crate::Screen::Bank);
    to_top();
}

/// The course in view changed: lecture and topic picks of other courses go.
pub fn course_changed() {
    setup::course_changed();
}

/// One question as a tutor set (quick search: `VQ-0001`). A paper in
/// progress stays: the bank shows it, and the question waits.
pub fn open_question(mut ui: crate::Ui, id: &str) {
    ui.screen.set(crate::Screen::Bank);
    if matches!(*PHASE.peek(), Phase::Running(_)) {
        return;
    }
    let lecture = steps_core::program::lecture_of(id);
    let lectures = study_lectures_where(|l| l == lecture);
    let pool: Vec<exam::Mcq> = exam::mcqs(&lectures).into_iter().filter(|q| q.id == id).collect();
    start(Mode::Tutor, exam::practice(&pool, 1, now_ms() as u64));
}

pub fn start(mode: Mode, questions: Vec<Question>) {
    start_paper(mode, false, questions);
}

/// A new paper, a block or practice. The questions he marked before come
/// marked.
pub fn start_paper(mode: Mode, block: bool, questions: Vec<Question>) {
    // Quick search can start one before the bank ever drew: its memory (the
    // marks) and a paper the app was killed in are read first, or the new
    // paper would come unmarked and its save would clear the killed one.
    load_history();
    restore_open();
    // A new paper never throws away one he left: that one is scored first.
    if let Some(paused) = PAUSED.write().take() {
        close(paused, now_ms());
    }
    if !questions.is_empty() {
        let mut paper = Paper { block, ..Paper::new(mode, questions) };
        HISTORY.peek().marks_on(&paper.questions, &mut paper.answers);
        *PHASE.write() = Phase::Running(paper);
        to_top();
    }
    keep_open();
}

/// The open paper (running or left), kept on this device so it survives
/// Android killing the app (#255). Device-only: a paper half done on the
/// phone is not one to continue on the tablet.
const OPEN_KEY: &str = "bank.open_paper";
/// Whether `OPEN_KEY` may hold a paper, so clicks with no paper open don't
/// rewrite an empty key every time.
static OPEN_STORED: AtomicBool = AtomicBool::new(true);

/// Write the open paper, or clear it once none is open. Called after every
/// change to a paper: answers, flags, moves, leave, submit.
fn keep_open() {
    let open = match (&*PHASE.peek(), &*PAUSED.peek()) {
        (_, Some(p)) => Some(p.clone()),
        (Phase::Running(p), None) => Some(p.clone()),
        _ => None,
    };
    if open.is_none() && !OPEN_STORED.load(Ordering::Relaxed) {
        return;
    }
    OPEN_STORED.store(open.is_some(), Ordering::Relaxed);
    let _ = crate::store::shared().put_local_setting(OPEN_KEY, &open);
}

/// Once per launch: a paper the app was killed in comes back as the left
/// paper, with "continue" and "end it now". A timed one's clock kept
/// running, as it does after Leave.
fn restore_open() {
    if PAUSED.peek().is_some() || !matches!(*PHASE.peek(), Phase::Setup) {
        return;
    }
    match crate::store::shared().local_setting::<Option<Paper>>(OPEN_KEY).flatten() {
        Some(p) if !p.questions.is_empty() => *PAUSED.write() = Some(p),
        _ => OPEN_STORED.store(false, Ordering::Relaxed),
    }
}

/// A new bank view (paper, result, setup) opens at its top, not at the
/// scroll offset of the long setup or result page before it.
fn to_top() {
    document::eval(r#"document.querySelector("main.content")?.scrollTo(0, 0);"#);
}

/// Select an option: a draft in tutor mode, the answer on a timed paper.
pub fn choose(option: usize) {
    let now = now_ms();
    {
        let mut phase = PHASE.write();
        let Phase::Running(p) = &mut *phase else { return };
        if p.overdue(now) {
            drop(phase);
            finish();
            return;
        }
        p.pick(option);
    }
    keep_open();
}

/// «تأكيد الإجابة»: tutor mode checks the selected option; a timed paper
/// already holds the pick as its answer, so it moves on to the next
/// question.
pub fn confirm() {
    enum Then {
        Check,
        Go(usize),
        Nothing,
    }
    let then = match &*PHASE.peek() {
        Phase::Running(p) if !p.mode.timed() => Then::Check,
        Phase::Running(p) if p.answers[p.at].chosen.is_some() && p.at + 1 < p.questions.len() => {
            Then::Go(p.at + 1)
        }
        _ => Then::Nothing,
    };
    match then {
        Then::Check => check(),
        Then::Go(next) => go(next),
        Then::Nothing => {}
    }
}

/// Tutor mode: check the selected option, reveal the explanation and send
/// the answer to review, exactly once per question.
pub fn check() {
    let now = now_ms();
    let sent = match &mut *PHASE.write() {
        Phase::Running(p) => p.check(now).map(|qa| (p.at, qa)),
        _ => None,
    };
    if let Some((at, (q, a))) = sent {
        if let Some(e) = exam::review_event(&q, &a, Mode::Tutor, now) {
            let id = record_logged(e);
            if let (Some(event_id), Phase::Running(p)) = (id, &mut *PHASE.write()) {
                p.logged = Some(Logged { at, event_id, at_ms: now });
            }
        }
        HISTORY.write().record(std::slice::from_ref(&q), std::slice::from_ref(&a));
    }
}

/// Confidence on the current question (see [`Paper::set_confidence`]).
pub fn set_confidence(confidence: Option<Confidence>) {
    if let Phase::Running(p) = &mut *PHASE.write() {
        p.set_confidence(confidence);
    }
    keep_open();
}

/// Tutor mode: why he missed the checked question. The answer was logged
/// at check; the log is append-only, so the logged event is taken back (as
/// Undo does) and the same answer is logged again with its reason, at the
/// time it was first given.
pub fn set_reason(reason: ErrorReason) {
    let change = match &mut *PHASE.write() {
        Phase::Running(p) => p.set_reason(reason).map(|c| (p.at, c)),
        _ => None,
    };
    let Some((at, (event, old))) = change else { return };
    // Only an answer that can be taken back is replaced, so it is never logged twice.
    let Some(old) = old else { return };
    let Some(store) = try_consume_context::<StudyStoreHandle>() else { return };
    let gone = store.0.borrow_mut().retract(&old.event_id);
    let logged = if gone {
        store.0.borrow_mut().record_logged(event).ok().flatten().map(|event_id| Logged {
            at,
            event_id,
            at_ms: old.at_ms,
        })
    } else {
        Some(old)
    };
    if let Phase::Running(p) = &mut *PHASE.write() {
        p.logged = logged;
    }
}

/// Move to question `to` (clamped).
pub fn go(to: usize) {
    let now = now_ms();
    let overdue = {
        let mut phase = PHASE.write();
        let Phase::Running(p) = &mut *phase else { return };
        p.leave(now);
        p.at = to.min(p.questions.len().saturating_sub(1));
        p.draft = None;
        p.overdue(now)
    };
    if overdue {
        finish();
    } else {
        keep_open();
    }
}

/// «علّم» on the question on screen: marked on the paper and in the synced
/// list of marked questions, or taken off both.
pub fn toggle_mark() {
    let marked = match &mut *PHASE.write() {
        Phase::Running(p) => Some(p.toggle_mark()),
        _ => None,
    };
    if let Some((id, on)) = marked {
        keep_mark(&id, on);
    }
    keep_open();
}

/// «علّم» while reviewing a finished paper: the paper's question `at`.
pub fn toggle_review_mark(at: usize) {
    let marked = match &mut *PHASE.write() {
        Phase::Done { paper, .. } if at < paper.questions.len() => {
            let a = &mut paper.answers[at];
            a.flagged ^= true;
            Some((paper.questions[at].id.clone(), a.flagged))
        }
        _ => None,
    };
    if let Some((id, on)) = marked {
        keep_mark(&id, on);
    }
}

/// A long press or a secondary click on an option: struck out, or back.
pub fn strike(option: usize) {
    if let Phase::Running(p) = &mut *PHASE.write() {
        p.strike(option);
    }
    keep_open();
}

/// Words selected in question `at`'s stem: highlighted, or no longer
/// (`exam::stem::toggle`), on the running paper or a finished one.
pub fn highlight(at: usize, start: usize, end: usize) {
    match &mut *PHASE.write() {
        Phase::Running(p) => {
            if let Some(a) = p.answers.get_mut(at) {
                a.toggle_highlight(start, end);
            }
        }
        Phase::Done { paper, .. } => {
            if let Some(a) = paper.answers.get_mut(at) {
                a.toggle_highlight(start, end);
            }
        }
        Phase::Setup => {}
    }
    keep_open();
}

/// «بلوك جديد من الغلط والمتعلّم عليه»: a new paper of the finished one's
/// wrong, blank and marked questions (`exam::retake`), in its mode, shuffled
/// again and as Mohamed sees them now (his edits in, his ✗ items out).
pub fn retake() {
    let (mode, block, ids) = match &*PHASE.peek() {
        Phase::Done { paper, .. } => {
            (paper.mode, paper.block, exam::retake(&paper.questions, &paper.answers))
        }
        _ => return,
    };
    let lectures: BTreeSet<&str> = ids.iter().map(|id| steps_core::program::lecture_of(id)).collect();
    let owned = study_lectures_where(|l| lectures.contains(l));
    let pool: Vec<exam::Mcq> =
        exam::mcqs(&owned).into_iter().filter(|q| ids.iter().any(|id| id == q.id)).collect();
    start_paper(mode, block, exam::practice(&pool, pool.len(), now_ms() as u64));
}

/// End the paper: score it, and for timed papers send every answer to
/// review and keep the result in the history.
pub fn finish() {
    let now = now_ms();
    let paper = {
        let mut phase = PHASE.write();
        let Phase::Running(p) = &mut *phase else { return };
        p.leave(now);
        p.clone()
    };
    let outcome = close(paper.clone(), now);
    *PHASE.write() = Phase::Done { paper, outcome };
    to_top();
    keep_open();
}

/// Score a paper; for timed ones, record the answers and save the result.
fn close(paper: Paper, now: i64) -> Outcome {
    let took = u64::try_from(now.saturating_sub(paper.started_ms)).unwrap_or(0);
    let outcome = exam::score(&paper.questions, &paper.answers, paper.mode, now, took);
    if paper.mode.timed() {
        for (q, a) in paper.questions.iter().zip(&paper.answers) {
            if let Some(e) = exam::review_event(q, a, paper.mode, now) {
                record(e);
            }
        }
        let mut history = HISTORY.write();
        history.record(&paper.questions, &paper.answers);
        // Start from the stored papers, not this run's copy, so one another
        // device finished meanwhile shows too (#32).
        history.papers = stored_papers();
        history.papers.push(outcome.clone());
        let store = crate::store::shared();
        crate::store::save_setting(&format!("{PAPER_PREFIX}{}.{}", store.device(), outcome.at_ms), &outcome);
    }
    outcome
}

/// "Leave" (✕) on the player: the paper is kept for later, tutor or timed,
/// never thrown away; the setup offers it back (UX-03). Its checked tutor
/// answers are already in review.
pub fn leave() {
    let now = now_ms();
    let mut phase = PHASE.write();
    if let Phase::Running(p) = &mut *phase {
        p.leave(now);
        *PAUSED.write() = Some(p.clone());
    }
    *phase = Phase::Setup;
    drop(phase);
    to_top();
    keep_open();
}

/// Back to the paper he left, on the question he left it at. A timed
/// paper's clock kept running; past the limit it is submitted as it stands.
pub fn resume() {
    let Some(mut p) = PAUSED.write().take() else { return };
    let now = now_ms();
    p.shown_ms = now;
    let overdue = p.overdue(now);
    *PHASE.write() = Phase::Running(p);
    to_top();
    if overdue {
        finish();
    }
}

/// Submit the paper he left without going back to it.
pub fn submit_paused() {
    let Some(mut p) = PAUSED.write().take() else { return };
    p.shown_ms = now_ms();
    *PHASE.write() = Phase::Running(p);
    finish();
}

pub fn back_to_setup() {
    *PHASE.write() = Phase::Setup;
    to_top();
}

pub fn minutes(ms: u64) -> u64 {
    ms.div_ceil(60_000)
}

/// Unix ms → `2026-10-27`. Pass local wall-clock ms (`store::zone().local_ms`).
pub fn date(ms: i64) -> String {
    // Howard Hinnant's civil_from_days.
    let z = ms.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests;
