//! The question bank (docs/WORKSTREAMS.md): practise MCQs by lecture, topic or
//! tag, in tutor mode (the explanation after every question) or timed mode
//! (the review at the end), and sit timed mock exams scored per lecture and
//! topic. Picking, timing and scoring live in `steps_core::exam`.

mod behind;
mod player;
mod question;
mod result;
mod setup;

use std::sync::atomic::{AtomicBool, Ordering};
use web_time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use steps_core::content::Body;
use steps_core::content::{Item, Lecture};
use steps_core::exam::{self, Answer, History, Mode, Outcome, Question};
use steps_core::review::{Grade, Origin, ReviewEvent, StudyStoreHandle};

use crate::content::Library;
use crate::look::{Css, css};

const BANK_CSS: Css = css!("src/bank/bank.css");

/// A paper in progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paper {
    pub mode: Mode,
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
}

impl Paper {
    fn new(mode: Mode, questions: Vec<Question>) -> Self {
        let now = now_ms();
        Self {
            mode,
            answers: vec![Answer::default(); questions.len()],
            questions,
            at: 0,
            started_ms: now,
            shown_ms: now,
            draft: None,
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
    /// checked (a second tap or Enter).
    fn check(&mut self, now: i64) -> Option<(Question, Answer)> {
        let at = self.at;
        if self.mode != Mode::Tutor || self.answers[at].chosen.is_some() {
            return None;
        }
        let option = self.draft.take()?;
        let a = &mut self.answers[at];
        a.chosen = Some(option);
        a.took_ms = u32::try_from(now.saturating_sub(self.shown_ms).max(0)).unwrap_or(u32::MAX);
        Some((self.questions[at].clone(), a.clone()))
    }

    /// Add the time on the current question before moving off it.
    fn leave(&mut self, now: i64) {
        let spent = u32::try_from(now.saturating_sub(self.shown_ms).max(0)).unwrap_or(u32::MAX);
        if let Some(a) = self.answers.get_mut(self.at) {
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

/// A timed paper Mohamed left part-way: kept, with its clock running, until
/// he goes back to it or submits it (#35).
pub static PAUSED: GlobalSignal<Option<Paper>> = Signal::global(|| None);

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

/// After a sync or restore (see `crate::synced`): take the finished papers
/// from the store now, and rebuild the last results from the log the next
/// time the bank opens.
pub fn reload_history() {
    let papers = stored_papers();
    if HISTORY.peek().papers != papers {
        HISTORY.write().papers = papers;
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

/// The index's shorter title (`Hepatitis C virus`, not the chapter's full
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

pub fn start(mode: Mode, questions: Vec<Question>) {
    // A new paper never throws away one he left: that one is scored first.
    if let Some(paused) = PAUSED.write().take() {
        close(paused, now_ms());
    }
    if !questions.is_empty() {
        *PHASE.write() = Phase::Running(Paper::new(mode, questions));
        to_top();
    }
    keep_open();
}

/// The open timed paper (running or left), kept on this device so it
/// survives Android killing the app (#255). Device-only: a paper half done
/// on the phone is not one to continue on the tablet.
const OPEN_KEY: &str = "bank.open_paper";
/// Whether `OPEN_KEY` may hold a paper, so tutor clicks don't rewrite an
/// empty key every time.
static OPEN_STORED: AtomicBool = AtomicBool::new(true);

/// Write the open timed paper, or clear it once none is open. Called after
/// every change to a paper: answers, flags, moves, leave, submit.
fn keep_open() {
    let open = match (&*PHASE.peek(), &*PAUSED.peek()) {
        (_, Some(p)) => Some(p.clone()),
        (Phase::Running(p), None) if p.mode.timed() => Some(p.clone()),
        _ => None,
    };
    if open.is_none() && !OPEN_STORED.load(Ordering::Relaxed) {
        return;
    }
    OPEN_STORED.store(open.is_some(), Ordering::Relaxed);
    let _ = crate::store::shared().put_local_setting(OPEN_KEY, &open);
}

/// Once per launch: a timed paper the app was killed in comes back as the
/// left paper, with "continue" and "submit now". Its clock kept running,
/// as it does after Leave.
fn restore_open() {
    if PAUSED.peek().is_some() || !matches!(*PHASE.peek(), Phase::Setup) {
        return;
    }
    match crate::store::shared().local_setting::<Option<Paper>>(OPEN_KEY).flatten() {
        Some(p) if p.mode.timed() && !p.questions.is_empty() => *PAUSED.write() = Some(p),
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

/// Tutor mode: check the selected option, reveal the explanation and send
/// the answer to review, exactly once per question.
pub fn check() {
    let now = now_ms();
    let sent = match &mut *PHASE.write() {
        Phase::Running(p) => p.check(now),
        _ => None,
    };
    if let Some((q, a)) = sent {
        if let Some(e) = exam::review_event(&q, &a, Mode::Tutor, now) {
            record(e);
        }
        HISTORY.write().record(std::slice::from_ref(&q), std::slice::from_ref(&a));
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

pub fn toggle_flag() {
    if let Phase::Running(p) = &mut *PHASE.write() {
        let at = p.at;
        p.answers[at].flagged ^= true;
    }
    keep_open();
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

/// "Leave" on the player. Tutor answers are already recorded, so a tutor set
/// just ends; a timed paper is kept for later instead of thrown away.
pub fn leave() {
    let now = now_ms();
    let mut phase = PHASE.write();
    if let Phase::Running(p) = &mut *phase
        && p.mode.timed()
    {
        p.leave(now);
        *PAUSED.write() = Some(p.clone());
    }
    *phase = Phase::Setup;
    drop(phase);
    to_top();
    keep_open();
}

/// Back to the paper he left. Its clock kept running; past the limit it is
/// submitted as it stands.
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
mod tests {
    use super::*;

    #[test]
    fn dates_and_minutes() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_793_059_200_000), "2026-10-27");
        assert_eq!(minutes(48 * 60_000), 48);
        assert_eq!(minutes(1), 1);
    }

    fn outcome(at_ms: i64, right: u32) -> Outcome {
        Outcome { total: exam::Score { right, total: 4 }, ..exam::score(&[], &[], Mode::Mock, at_ms, 0) }
    }

    #[test]
    fn papers_from_the_legacy_list_and_per_paper_keys_merge_once() {
        let merged = merge_papers(vec![outcome(30, 1), outcome(10, 2), outcome(30, 1), outcome(20, 3)]);
        assert_eq!(merged.iter().map(|o| o.at_ms).collect::<Vec<_>>(), vec![10, 20, 30]);
    }

    #[test]
    fn an_open_paper_round_trips_through_json() {
        let q =
            Question { id: "x".into(), lecture: "hb.L01".into(), topic: None, answer: 0, order: vec![1, 0] };
        let mut p = Paper::new(Mode::Mock, vec![q.clone(), q]);
        p.answers[1].chosen = Some(0);
        p.answers[0].flagged = true;
        p.at = 1;
        let json = serde_json::to_string(&Some(p.clone())).unwrap();
        assert_eq!(serde_json::from_str::<Option<Paper>>(&json).unwrap(), Some(p));
        assert_eq!(serde_json::from_str::<Option<Paper>>("null").unwrap(), None);
    }

    #[test]
    fn question_counts_read_naturally() {
        use crate::i18n::bank::questions;
        assert_eq!(questions(1), "سؤال واحد");
        assert_eq!(questions(2), "سؤالين");
        assert_eq!(questions(7), "7 أسئلة");
        assert_eq!(questions(34), "34 سؤال");
    }

    #[test]
    fn paper_time_adds_up_per_question() {
        let q =
            Question { id: "x".into(), lecture: "hb.L01".into(), topic: None, answer: 0, order: vec![0, 1] };
        let mut p = Paper::new(Mode::Timed, vec![q.clone(), q]);
        p.shown_ms = 1_000;
        p.leave(4_000);
        p.at = 1;
        p.leave(5_000);
        p.at = 0;
        p.leave(6_000);
        assert_eq!(p.answers[0].took_ms, 4_000);
        assert_eq!(p.answers[1].took_ms, 1_000);
    }

    fn question(id: &str) -> Question {
        Question { id: id.into(), lecture: "hb.L01".into(), topic: None, answer: 1, order: vec![0, 1, 2] }
    }

    #[test]
    fn tutor_selects_then_checks_once() {
        let mut p = Paper::new(Mode::Tutor, vec![question("a"), question("b")]);
        p.shown_ms = 0;
        // Nothing selected: nothing to check.
        assert_eq!(p.check(10), None);
        // A selection is a draft he can change; nothing is graded yet.
        p.pick(0);
        p.pick(1);
        assert_eq!((p.draft, p.answers[0].chosen), (Some(1), None));
        let (q, a) = p.check(2_000).expect("the draft is checked");
        assert_eq!((q.id.as_str(), a.chosen, a.took_ms), ("a", Some(1), 2_000));
        // A second Check, and a later pick, change nothing.
        assert_eq!(p.check(3_000), None);
        p.pick(2);
        assert_eq!((p.draft, p.answers[0].chosen), (None, Some(1)));
    }

    #[test]
    fn moving_on_drops_an_unchecked_draft() {
        let mut p = Paper::new(Mode::Tutor, vec![question("a"), question("b")]);
        p.pick(2);
        p.leave(5);
        p.at = 1;
        p.draft = None;
        assert_eq!(p.check(6), None);
        assert!(p.answers.iter().all(|a| a.chosen.is_none()));
    }

    #[test]
    fn timed_papers_take_the_pick_as_the_answer() {
        let mut p = Paper::new(Mode::Mock, vec![question("a")]);
        p.pick(0);
        p.pick(2);
        assert_eq!((p.draft, p.answers[0].chosen), (None, Some(2)));
        assert_eq!(p.check(1), None, "timed papers are never checked one by one");
    }

    #[test]
    fn papers_saved_before_drafts_still_load() {
        let p = Paper::new(Mode::Timed, vec![question("a")]);
        let mut json: serde_json::Value = serde_json::to_value(&p).unwrap();
        json.as_object_mut().unwrap().remove("draft");
        assert_eq!(serde_json::from_value::<Paper>(json).unwrap(), p);
    }
}
