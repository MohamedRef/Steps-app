//! The question bank over the First Aid MCQs, as a Step 1 question bank's
//! test screen: the setup and its scope filters, tutor papers checked one by
//! one, timed papers and blocks, the top bar and the navigator, marks, struck
//! options and highlights, leaving and resuming, and the end of the block
//! with its review. Papers are 10 questions at most, so no test grows with
//! the bank.

use std::collections::BTreeSet;

use steps_core::content::{Body, Item};
use steps_core::exam::{Mode, Question};
use steps_core::review::{Grade, Origin};

use dioxus_html::{EventData, SerializedMouseData};

use super::App;
use super::kit::{ids_of, item, reviews};
use crate::Screen;
use crate::bank::{PHASE, Paper, Phase};
use crate::i18n::bank as tx;

/// The MCQ on screen, from its stem.
#[track_caller]
fn shown(app: &App) -> Item {
    let text = app.text();
    ids_of("mcq")
        .into_iter()
        .map(|id| item(&id))
        .find(|i| matches!(&i.body, Body::Mcq { stem, .. } if text.contains(stem.as_str())))
        .unwrap_or_else(|| panic!("no MCQ stem on screen:\n{text}"))
}

/// The right option's text and a wrong one's.
fn options(i: &Item) -> (String, String) {
    let Body::Mcq { options, answer, .. } = &i.body else { panic!("{} is not an MCQ", i.id) };
    let wrong = (0..options.len()).find(|&o| o != *answer).unwrap();
    (options[*answer].clone(), options[wrong].clone())
}

/// The option text at original index `at`.
fn option_text(i: &Item, at: usize) -> String {
    let Body::Mcq { options, .. } = &i.body else { panic!("{} is not an MCQ", i.id) };
    options[at].clone()
}

impl App {
    /// The paper's option carrying `text` that listens for `event`: only an
    /// option, never a glossary term in the stem whose title happens to hold
    /// the same words.
    #[track_caller]
    fn option(&self, event: &str, text: &str) -> usize {
        let in_options = |mut n: usize| loop {
            if self.page.attr(n, "class").is_some_and(|c| c.split(' ').any(|c| c == "q-options")) {
                return true;
            }
            match self.page.nodes[n].parent {
                Some(p) => n = p,
                None => return false,
            }
        };
        self.targets(event, text)
            .into_iter()
            .find(|&n| in_options(n) && self.page.text(n).contains(text))
            .unwrap_or_else(|| panic!("no option {text:?}:\n{}", self.listing(event)))
    }

    /// Taps the paper's option carrying `text`.
    #[track_caller]
    fn answer(&mut self, text: &str) -> &mut Self {
        let n = self.option("click", text);
        self.fire("click", EventData::Mouse(SerializedMouseData::default()), n);
        self
    }

    /// A long press or a secondary click on the option carrying `text`.
    #[track_caller]
    fn strike(&mut self, text: &str) -> &mut Self {
        let n = self.option("contextmenu", text);
        self.fire("contextmenu", EventData::Mouse(SerializedMouseData::default()), n);
        self
    }

    /// An attribute of the option button carrying `text`, answered (and so
    /// disabled) or not.
    #[track_caller]
    fn option_attr(&self, text: &str, name: &str) -> String {
        let n = with_class(self, "q-option")
            .into_iter()
            .find(|&n| self.page.text(n).contains(text))
            .unwrap_or_else(|| panic!("no option {text:?}"));
        self.page.attr(n, name).unwrap_or_default().to_string()
    }
}

/// The nodes whose class list holds `class`, in page order.
fn with_class(app: &App, class: &str) -> Vec<usize> {
    app.page
        .walk()
        .into_iter()
        .filter(|&n| app.page.attr(n, "class").is_some_and(|c| c.split(' ').any(|c| c == class)))
        .collect()
}

/// An attribute of the first node with `class`.
#[track_caller]
fn attr_of(app: &App, class: &str, name: &str) -> String {
    let n = *with_class(app, class).first().unwrap_or_else(|| panic!("no .{class}"));
    app.page.attr(n, name).unwrap_or_default().to_string()
}

/// The words of every node with `class`, in page order.
fn texts_of(app: &App, class: &str) -> Vec<String> {
    with_class(app, class).into_iter().map(|n| app.page.text(n)).collect()
}

/// The `name` attribute of every node with `class`, in page order.
fn attrs_of(app: &App, class: &str, name: &str) -> Vec<String> {
    with_class(app, class)
        .into_iter()
        .map(|n| app.page.attr(n, name).unwrap_or_default().to_string())
        .collect()
}

/// The paper on screen.
#[track_caller]
fn running(app: &mut App) -> Paper {
    use dioxus::prelude::*;
    app.run(|| match &*PHASE.peek() {
        Phase::Running(p) => p.clone(),
        _ => panic!("a paper runs"),
    })
}

/// The finished paper on the result screen.
#[track_caller]
fn finished(app: &mut App) -> Paper {
    use dioxus::prelude::*;
    app.run(|| match &*PHASE.peek() {
        Phase::Done { paper, .. } => paper.clone(),
        _ => panic!("a paper is done"),
    })
}

/// How many questions the paper on screen has.
fn paper_n(app: &mut App) -> usize {
    running(app).questions.len()
}

/// The letter question `q`'s original option `orig` is shown under.
fn letter(q: &Question, orig: usize) -> char {
    char::from(b'A' + q.order.iter().position(|&o| o == orig).expect("the option is shown") as u8)
}

/// The kept list of marked questions (the synced setting `bank.marked`).
fn kept_marks() -> BTreeSet<String> {
    crate::store::shared().setting("bank.marked").unwrap_or_default()
}

/// The timed paper kept on this device while it is open.
fn kept_paper() -> Option<Paper> {
    crate::store::shared().local_setting::<Option<Paper>>("bank.open_paper").flatten()
}

/// The lecture an item belongs to.
fn lecture_of(item_id: &str) -> String {
    crate::content::Library::get()
        .lectures
        .iter()
        .find(|(_, l)| l.items.iter().any(|i| i.id == item_id))
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| panic!("no lecture holds {item_id}"))
}

/// A lecture's topics that have MCQs, in the lecture's order.
fn mcq_topics(lecture: &str) -> Vec<steps_core::content::Topic> {
    let l = &crate::content::Library::get().lectures[lecture];
    l.topics
        .iter()
        .filter(|t| l.items.iter().any(|i| i.topic.as_deref() == Some(&t.id) && i.body.kind() == "mcq"))
        .cloned()
        .collect()
}

/// The first written lecture with at least `n` topics that have MCQs.
#[track_caller]
fn lecture_with_mcq_topics(n: usize) -> Option<String> {
    crate::content::Library::get().lectures.keys().find(|id| mcq_topics(id).len() >= n).cloned()
}

/// The lecture opened over the bank, if one is.
fn open_lecture(app: &mut App) -> Option<String> {
    app.run(|| crate::lecture::shown(Screen::Bank))
}

/// Questions in a test paper. A paper of the whole bank (every MCQ, over
/// 2000 once the book was written) made each of these tests take minutes:
/// every answer redraws a navigator with a button per question.
const PAPER: usize = 10;

/// The bank set to 10 questions and `mode`, started.
fn paper(mode: &str, start: &str) -> App {
    let mut app = App::open(Screen::Bank);
    app.tap(mode);
    app.tap(&PAPER.to_string());
    app.tap(start);
    app
}

/// The bank narrowed to the first topic with MCQs, every question of it in
/// one paper in `mode`, started, and how many that is: for a test that
/// needs a whole scope answered.
fn topic_paper(mode: &str, start: &str) -> (App, usize) {
    let id = lecture_with_mcq_topics(1).expect("a lecture has MCQs filed under a topic");
    let topic = mcq_topics(&id).remove(0);
    let lecture = &crate::content::Library::get().lectures[&id];
    let n = lecture
        .items
        .iter()
        .filter(|i| i.topic.as_deref() == Some(&topic.id) && i.body.kind() == "mcq")
        .count();
    PRACTISE.with(|p| *p.borrow_mut() = id.clone());
    let mut app = App::with_part(Screen::Bank, practise_lecture_button);
    app.tap("practise the lecture");
    app.tap(tx::CUSTOMISE);
    app.tap(&topic.title);
    app.assert_has(&tx::questions(n));
    app.tap(tx::CUSTOMISE_CLOSE);
    app.tap(mode);
    app.tap(tx::ALL);
    app.tap(start);
    (app, n)
}

/// Every question of a tutor paper answered right and checked, then the
/// paper ended from its last question.
fn all_right_tutor(app: &mut App) {
    let n = paper_n(app);
    for k in 0..n {
        let (right, _) = options(&shown(app));
        app.answer(&right).tap(tx::CHECK);
        next_or_end(app, k, n);
    }
}

/// The step after a question: the next one, or on the last the foot's
/// «إنهاء البلوك».
fn next_or_end(app: &mut App, k: usize, n: usize) {
    if k + 1 == n {
        app.tap("run-main-action");
    } else {
        app.tap(tx::NEXT);
    }
}

#[test]
fn the_setup_offers_the_three_modes_and_counts_the_pool() {
    let mut app = App::open(Screen::Bank);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::MODE_TUTOR_NAME);
    app.assert_has(tx::MODE_TIMED_NAME);
    app.assert_has(tx::MODE_MOCK_NAME);
    app.assert_has(tx::PRESET_TUTOR);
    app.assert_has(tx::SCOPE_ALL);
    app.assert_has(tx::POOL_OF);
    app.tap(tx::MODE_TIMED_NAME);
    app.assert_has(tx::PRESET_TIMED);
    app.assert_has(tx::START_TIMED);
    app.tap(tx::MODE_MOCK_NAME);
    app.assert_has(tx::MOCK_SCOPE_CHIP);
    app.assert_has(tx::MOCK_SCOPE_NOTE);
    app.assert_has(tx::START_MOCK);
    // A block is 10, 15 or 20 questions, as on the 2026 exam.
    assert!(!app.tappables().iter().any(|t| t.ends_with("|40")), "{:?}", app.tappables());
    app.tap("15");
    app.assert_has(tx::POOL_OF);
}

#[test]
fn a_tutor_paper_checks_each_answer_and_sends_it_to_review() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let n = paper_n(&mut app);
    app.assert_has(&format!("1 / {n}"));
    app.assert_has(&tx::question_of(1, n));
    app.assert_has(tx::PICK_FIRST);
    let mut got = Vec::new();
    for k in 0..n {
        let q = shown(&app);
        let (right, wrong) = options(&q);
        // First question wrong (after changing his mind), the rest right.
        if k == 0 {
            app.answer(&right);
            app.answer(&wrong);
        } else {
            app.answer(&right);
        }
        app.assert_lacks(tx::PICK_FIRST);
        assert_eq!(reviews().len(), k, "nothing is saved before «تأكيد الإجابة»");
        app.tap(tx::CHECK);
        let saved = reviews();
        assert_eq!(saved.len(), k + 1);
        let last = &saved[k].event;
        assert_eq!((last.item_id.as_str(), last.origin), (q.id.as_str(), Origin::Bank));
        assert_eq!(last.grade, if k == 0 { Grade::Again } else { Grade::Good });
        if k == 0 {
            app.assert_has(tx::VERDICT_WRONG);
            app.assert_has(tx::YOUR_ANSWER);
            app.assert_has(tx::RIGHT_ANSWER);
        } else {
            app.assert_has(tx::VERDICT_RIGHT);
            app.assert_has(tx::RIGHT_PICKED);
        }
        // The explanation: every option's why.
        let Body::Mcq { why, .. } = &q.body else { unreachable!() };
        app.assert_has(tx::EXPLANATION);
        app.assert_has(&why[0]);
        got.push(q.id.clone());
        next_or_end(&mut app, k, n);
    }
    got.sort();
    got.dedup();
    assert_eq!(got.len(), n, "every question once");

    // The end of the paper: one wrong of n.
    app.assert_has(tx::RESULT);
    app.assert_has(tx::SENT_TO_REVIEW);
    let score = steps_core::exam::Score { right: n as u32 - 1, total: n as u32 };
    app.assert_has(&format!("{}%", score.percent()));
    app.assert_has(&format!("{} / {n}", n - 1));
    app.assert_has(tx::BY_COURSE);
    app.assert_has(tx::TABLE);
    app.assert_has(tx::REVIEW_WRONG);

    // A new paper of the wrong one.
    app.tap(tx::RETAKE);
    app.assert_has("1 / 1");
}

#[test]
fn the_result_opens_each_question_and_goes_back_to_the_bank() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    all_right_tutor(&mut app);
    app.assert_has("100%");
    // The score takes the focus, to be read out.
    assert!(app.mount_events() > 0);
    // All right and nothing marked: nothing to review or retake, a new
    // paper instead; strong topics praised.
    app.assert_lacks(tx::REVIEW_WRONG);
    app.assert_lacks(tx::RETAKE);
    app.assert_has(tx::NEW_PAPER);
    app.assert_has(tx::ALL_STRONG);
    // A row opens its question with the explanation, then closes.
    let n = finished(&mut app).questions.len();
    app.tap(&tx::open_question(1));
    app.assert_has(tx::BACK_TO_RESULT);
    app.assert_has(&tx::question_of(1, n));
    app.assert_has(tx::RIGHT_PICKED);
    app.assert_has(tx::EXPLANATION);
    app.tap("exam-end");
    app.assert_lacks(tx::BACK_TO_RESULT);
    app.tap(tx::BACK);
    app.assert_has(tx::START_TUTOR);
}

#[test]
fn a_timed_paper_marks_moves_and_asks_before_it_goes_in() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    assert!(n >= 2, "the timed paper needs two questions");
    app.assert_has(&tx::kind(false, true));
    app.assert_has(tx::TIME_LEFT);
    let first = shown(&app);
    let (right, _) = options(&first);
    app.answer(&right);
    // A timed pick is the answer, with no check and no explanation yet.
    app.assert_lacks(tx::RIGHT_PICKED);
    app.assert_lacks(tx::EXPLANATION);
    app.tap("exam-mark");
    app.assert_has(tx::MARKED);
    app.tap(tx::NEXT);
    app.assert_has(&format!("2 / {n}"));
    app.tap(tx::PREV);
    app.assert_has(&format!("1 / {n}"));
    // The navigator jumps to the last question.
    app.tap(&format!("{} {n}،", tx::QUESTION));
    app.assert_has(&format!("{n} / {n}"));
    assert!(reviews().is_empty(), "a timed paper is saved when it goes in");

    // Blanks: he is asked first, with how many; going back lands on a blank.
    app.tap("exam-end");
    app.assert_has(tx::CONFIRM_TITLE);
    app.assert_has(&tx::confirm_blank(n - 1));
    app.assert_has(tx::CONFIRM_MARKED);
    app.tap(tx::CONFIRM_BACK_TO);
    app.assert_lacks(tx::CONFIRM_TITLE);
    app.assert_has(&format!("2 / {n}"));
    app.tap("exam-end");
    app.tap(tx::CONFIRM_END);

    // In: one answered (right), the rest blank.
    app.assert_has(tx::RESULT);
    app.assert_has(tx::TOOK);
    app.assert_has(tx::TILE_BLANK);
    let saved = reviews();
    assert_eq!(saved.len(), 1, "blank answers are not reviews");
    assert_eq!((saved[0].event.item_id.as_str(), saved[0].event.grade), (first.id.as_str(), Grade::Good));
    assert_eq!(saved[0].event.origin, Origin::Bank);
    // The marked tile reviews the marked one.
    app.tap(tx::MARKED);
    app.assert_has(&format!("{} · {}", tx::REVIEWING, tx::SHOW_MARKED));
    app.assert_has(&tx::question_of(1, n));
    app.tap("exam-end");

    // The paper is kept in the history on the setup.
    app.tap(tx::BACK);
    app.assert_has(tx::HISTORY);
    app.assert_has(tx::MODE_TIMED);
    app.assert_has(&format!("1/{n}"));
}

#[test]
fn a_whole_timed_paper_answered_goes_in_without_asking() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right);
        // A mark alone never stops the paper going in.
        if k == 0 {
            app.tap("exam-mark");
        }
        if k + 1 < n {
            // «تأكيد الإجابة» keeps the pick and goes on.
            app.tap(tx::CHECK);
            app.assert_has(&format!("{} / {n}", k + 2));
        }
    }
    // The last question's main step ends the paper.
    app.assert_lacks(tx::CHECK);
    app.tap("run-main-action");
    app.assert_lacks(tx::CONFIRM_TITLE);
    app.assert_has("100%");
    assert_eq!(reviews().len(), n);
}

#[test]
fn a_timed_paper_left_waits_with_its_clock_and_can_be_resumed_or_submitted() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.tap(tx::LEAVE);
    app.assert_has(tx::PAUSED_TITLE);
    app.assert_has(&format!("1/{n}"));
    app.assert_has(tx::PAUSED_LEFT);
    // Back to it, on the same question with his answer.
    app.tap(tx::RESUME);
    app.assert_has(&format!("1 / {n}"));
    assert_eq!(app.option_attr(&right, "aria-pressed"), "true");
    app.tap(tx::LEAVE);
    // Submitted from the setup, as it stands.
    app.tap(tx::SUBMIT_NOW);
    app.assert_has(tx::RESULT);
    assert_eq!(reviews().len(), 1);
}

#[test]
fn a_new_paper_scores_the_one_left_first() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.tap(tx::LEAVE);
    app.tap(tx::START_TIMED);
    // The left paper went in on its own; this one runs.
    assert_eq!(reviews().len(), 1);
    app.assert_has(tx::TIME_LEFT);
    app.assert_lacks(tx::PAUSED_TITLE);
}

#[test]
fn a_tutor_paper_left_just_ends() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    app.tap(tx::LEAVE);
    app.assert_has(tx::START_TUTOR);
    app.assert_lacks(tx::PAUSED_TITLE);
}

#[test]
fn a_mock_is_scored_and_kept_as_a_mock() {
    let mut app = App::open(Screen::Bank);
    app.tap(tx::MODE_MOCK_NAME);
    app.tap("20");
    app.tap(tx::START_MOCK);
    // A block says so in its bar.
    app.assert_has(&tx::kind(true, true));
    let n = paper_n(&mut app);
    assert!((1..=steps_core::exam::MOCK_SIZE).contains(&n), "a block of at most 20: {n}");
    let (_, wrong) = options(&shown(&app));
    app.answer(&wrong);
    app.tap("exam-end");
    app.tap(tx::CONFIRM_END);
    app.assert_has(tx::MOCK_TITLE);
    app.assert_has(&tx::kind(true, true));
    app.assert_has(tx::RETAKE);
    let saved = reviews();
    assert_eq!(saved.len(), 1);
    assert_eq!((saved[0].event.origin, saved[0].event.grade), (Origin::Mock, Grade::Again));
    app.tap(tx::BACK);
    app.assert_has(tx::MODE_MOCK);
    app.assert_has(&format!("0/{n}"));
}

#[test]
fn a_block_with_the_explanation_says_so_and_checks_each_answer() {
    let mut app = App::open(Screen::Bank);
    app.tap(tx::MODE_MOCK_NAME);
    app.tap("10");
    // The block's own «بشرح», by its hint: the mode row has a «بشرح» too.
    app.tap(tx::BLOCK_TUTOR_HINT);
    app.tap(tx::START_MOCK);
    app.assert_has(&tx::kind(true, false));
    // No clock against him: the time on the question instead.
    app.assert_has(tx::ON_QUESTION);
    app.assert_lacks(tx::TIME_LEFT);
    let (right, _) = options(&shown(&app));
    app.answer(&right).tap(tx::CHECK);
    app.assert_has(tx::VERDICT_RIGHT);
    assert_eq!(reviews().len(), 1);
}

#[test]
fn the_scope_narrows_to_a_topic_and_widens_back() {
    let id = lecture_with_mcq_topics(1).expect("a lecture has MCQs filed under a topic");
    let lecture = &crate::content::Library::get().lectures[&id];
    // A lecture page's practise button narrows the bank to the lecture,
    // whose topics are then offered.
    PRACTISE.with(|p| *p.borrow_mut() = id.clone());
    let mut app = App::with_part(Screen::Bank, practise_lecture_button);
    app.tap("practise the lecture");
    app.tap(tx::CUSTOMISE);
    app.assert_has(tx::CUSTOMISE_CLOSE);
    app.assert_has(tx::TOPICS);
    app.assert_has(tx::TAGS);
    // The first topic that has MCQs.
    let topic = &mcq_topics(&id)[0];
    let in_topic = lecture
        .items
        .iter()
        .filter(|i| i.topic.as_deref() == Some(&topic.id) && i.body.kind() == "mcq")
        .count();
    app.tap(&topic.title);
    app.assert_has(tx::SCOPE_CHOSEN);
    app.assert_has(&topic.title);
    app.assert_has(&tx::questions(in_topic));
    // Only wrong or unseen: all unseen, the same count.
    app.tap(tx::ONLY_WRONG);
    app.assert_has(&tx::questions(in_topic));
    app.tap(tx::ONLY_WRONG);
    app.tap(tx::RESET_SCOPE);
    app.assert_has(tx::SCOPE_ALL);
    app.tap(tx::CUSTOMISE_CLOSE);
    app.assert_lacks(tx::TAGS);
    // The scope card opens the choices too.
    app.tap(tx::SCOPE_ALL);
    app.assert_has(tx::TAGS);
}

#[test]
fn a_scope_answered_all_right_says_so_and_widens() {
    let (mut app, _) = topic_paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    all_right_tutor(&mut app);
    app.tap(tx::BACK);
    app.tap(tx::CUSTOMISE);
    app.tap(tx::ONLY_WRONG);
    app.assert_has(tx::NONE_MATCH);
    app.assert_has(tx::EMPTY_ALL_RIGHT);
    assert!(!app.can_tap(tx::START_TUTOR));
    app.tap(tx::WIDEN_ALL_RIGHT);
    app.assert_lacks(tx::NONE_MATCH);
    app.assert_has(tx::START_TUTOR);
}

#[test]
fn a_tag_with_no_question_in_the_topic_offers_to_drop_the_tag() {
    let lib = crate::content::Library::get();
    let lecture = &lib.lectures["cvs.L01"];
    let tags =
        [("high-yield", tx::TAG_HIGH_YIELD), ("book-question", tx::TAG_BOOK), ("numbers", tx::TAG_NUMBERS)];
    let mcqs: Vec<&Item> = lecture.items.iter().filter(|i| i.body.kind() == "mcq").collect();
    // A topic and a tag the bank offers that no question has together.
    let pair = tags.iter().filter(|(t, _)| mcqs.iter().any(|i| i.tags.iter().any(|x| x == t))).find_map(
        |(t, label)| {
            let topic = lecture.topics.iter().find(|tp| {
                let here: Vec<_> = mcqs.iter().filter(|i| i.topic.as_deref() == Some(&tp.id)).collect();
                !here.is_empty() && here.iter().all(|i| !i.tags.iter().any(|x| x == t))
            })?;
            Some((topic.title.clone(), *label))
        },
    );
    let mut app = App::open(Screen::Bank);
    app.tap(tx::CUSTOMISE);
    let Some((topic, label)) = pair else {
        // Every tagged topic has the tag: the empty state comes from a topic.
        return;
    };
    app.tap(&topic);
    app.tap(label);
    app.assert_has(tx::NONE_MATCH);
    app.assert_has(tx::EMPTY_TAGS);
    app.tap(tx::WIDEN_TAGS);
    app.assert_lacks(tx::NONE_MATCH);
}

#[test]
fn the_setup_is_kept_for_the_next_launch() {
    let mut app = App::open(Screen::Bank);
    app.tap(tx::MODE_TIMED_NAME);
    app.tap("40");
    drop(app);
    // A new app on the same device data: the bank opens timed.
    let app = App::open(Screen::Bank);
    app.assert_has(tx::START_TIMED);
}

/// A button that opens the bank on cvs.L01, as a lecture page's does.
fn practise_button() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let ui = use_context::<crate::Ui>();
    rsx! {
        button { class: "test-practise", onclick: move |_| crate::bank::practise(ui, "cvs.L01", None), "practise cvs.L01" }
    }
}

thread_local! {
    /// The lecture `practise_lecture_button` opens the bank on.
    static PRACTISE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// A button that opens the bank on the lecture in `PRACTISE`.
fn practise_lecture_button() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let ui = use_context::<crate::Ui>();
    let id = PRACTISE.with(|p| p.borrow().clone());
    rsx! {
        button { class: "test-practise", onclick: move |_| crate::bank::practise(ui, &id, None), "practise the lecture" }
    }
}

#[test]
fn practise_from_a_lecture_narrows_the_bank_and_leaves_a_mock() {
    let mut app = App::with_part(Screen::Bank, practise_button);
    app.tap(tx::MODE_MOCK_NAME);
    app.tap("practise cvs.L01");
    app.assert_has(tx::START_TUTOR);
    app.assert_has(tx::SCOPE_CHOSEN);
    let title = crate::content::Library::get().title("cvs.L01").to_string();
    app.assert_has(&title);
    // From another screen it switches to the bank.
    app.tap(Screen::Today.label());
    app.tap("practise cvs.L01");
    assert!(app.html().contains("data-screen=\"Bank\""));
}

#[test]
fn the_report_link_reopens_the_check_after_a_verdict() {
    use crate::i18n::verify as vx;
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let q = shown(&app);
    let (right, _) = options(&q);
    app.answer(&right).tap(tx::CHECK);
    // First time: the ✓ / ✎ / ✗ check; once answered, a quiet report link.
    app.assert_has(vx::QUESTION);
    app.tap(vx::RIGHT);
    let verdicts = crate::store::shared().judgements().unwrap();
    assert_eq!(verdicts[&q.id].verdict, steps_core::review::verify::Verdict::Approved);
    app.assert_lacks(vx::QUESTION);
    app.tap(tx::REPORT);
    app.assert_has(vx::QUESTION);
    // The verdict was taken back: the store asks again.
    let check = crate::store::shared().judgements().unwrap();
    assert!(!check.contains_key(&q.id));
}

/// The text of the chips over an answered question, if it shows them.
fn meta_row(app: &App) -> Option<String> {
    app.page
        .walk()
        .into_iter()
        .find(|&n| app.page.attr(n, "class").is_some_and(|c| c.split(' ').any(|c| c == "q-meta")))
        .map(|n| app.page.text(n))
}

#[test]
fn the_lecture_and_topic_chips_wait_for_the_answer() {
    // The topic is a First Aid heading, often the diagnosis itself (#743),
    // and the lecture a cue in a mixed block.
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let q = shown(&app);
    let lecture = lecture_of(&q.id);
    let topic =
        crate::bank::topic_title(&lecture, q.topic.as_deref().expect("an MCQ is filed under a topic"));
    let crumb = crate::content::Library::get().crumb(&lecture);
    assert_eq!(meta_row(&app), None, "no lecture or topic before the answer");
    let (right, _) = options(&q);
    app.answer(&right);
    assert_eq!(meta_row(&app), None, "choosing is not answering");
    app.tap(tx::CHECK);
    let row = meta_row(&app).expect("the chips with the answer");
    assert!(row.contains(&crumb) && row.contains(&topic), "the lecture and topic show with the answer: {row}");
}

#[test]
fn a_timed_paper_never_names_the_lecture_or_topic_while_it_runs() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    assert_eq!(meta_row(&app), None, "no chips on a running paper");
}

#[test]
fn a_lecture_chip_on_an_answered_question_opens_the_lecture_page() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let q = shown(&app);
    let (right, _) = options(&q);
    app.answer(&right).tap(tx::CHECK);
    app.tap(tx::OPEN_LECTURE);
    let lecture = lecture_of(&q.id);
    assert_eq!(open_lecture(&mut app), Some(lecture.clone()));
    app.assert_has(crate::content::Library::get().title(&lecture));
}

#[test]
fn a_finished_paper_in_the_history_opens_its_lecture() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right);
        if k + 1 < n {
            app.tap(tx::NEXT);
        }
    }
    app.tap("exam-end");
    app.tap(tx::BACK);
    // The history row opens the lecture page: one the paper drew from.
    app.tap(tx::OPEN_LECTURE);
    let lecture = open_lecture(&mut app).expect("a lecture page opened");
    assert!(ids_of("mcq").iter().any(|id| lecture_of(id) == lecture), "{lecture} has no MCQs");
    app.assert_has(crate::content::Library::get().title(&lecture));
}

/// Longer than any test paper's time (a paper's time grows with its
/// questions: 90 seconds each).
const LONG_AGO_MS: i64 = 365 * 86_400_000;

/// Questions in a paper the app keeps open.
const KEPT: usize = 3;

/// A timed paper of the first three MCQs, started `ago_ms` before now, as
/// the app keeps one it was killed in.
fn open_paper(ago_ms: i64) -> Paper {
    use steps_core::exam::Answer;
    let questions: Vec<Question> = ids_of("mcq")
        .into_iter()
        .take(KEPT)
        .map(|id| {
            let i = item(&id);
            let Body::Mcq { options, answer, .. } = &i.body else { unreachable!() };
            Question {
                id: id.clone(),
                lecture: lecture_of(&id),
                topic: i.topic.clone(),
                answer: *answer,
                order: (0..options.len()).collect(),
            }
        })
        .collect();
    let now = crate::bank::now_ms();
    Paper {
        mode: Mode::Timed,
        block: false,
        answers: vec![Answer::default(); questions.len()],
        questions,
        at: 0,
        started_ms: now - ago_ms,
        shown_ms: now - ago_ms,
        draft: None,
        logged: None,
    }
}

#[test]
fn a_timed_paper_the_app_was_killed_in_comes_back_as_the_left_one() {
    let mut paper = open_paper(60_000);
    // Its struck option and highlight come back with it.
    paper.answers[0].toggle_strike(1);
    paper.answers[0].toggle_highlight(0, 5);
    crate::store::shared().put_local_setting("bank.open_paper", &Some(paper.clone())).unwrap();
    let mut app = App::open(Screen::Bank);
    app.assert_has(tx::PAUSED_TITLE);
    app.assert_has(tx::PAUSED_LEFT);
    app.tap(tx::RESUME);
    app.assert_has(tx::TIME_LEFT);
    app.assert_has(&format!("1 / {KEPT}"));
    let first = item(&paper.questions[0].id);
    assert_eq!(app.option_attr(&option_text(&first, 1), "data-struck"), "true");
    assert_eq!(with_class(&app, "q-hl").len(), 1, "the highlight is drawn");
}

#[test]
fn a_left_paper_past_its_time_goes_in_as_it_stands() {
    let paper = open_paper(LONG_AGO_MS);
    crate::store::shared().put_local_setting("bank.open_paper", &Some(paper)).unwrap();
    let mut app = App::open(Screen::Bank);
    app.assert_has(tx::PAUSED_OVER);
    // Going back to it hands it in: its time is up.
    app.tap(tx::RESUME);
    app.assert_has(tx::RESULT);
    app.assert_has("0%");
}

#[test]
fn a_running_paper_past_its_time_goes_in_on_the_next_tap() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    // The clock ran out while he looked away.
    app.run(|| {
        if let Phase::Running(p) = &mut *PHASE.write() {
            p.started_ms -= LONG_AGO_MS;
        }
    });
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.assert_has(tx::RESULT);
    assert!(reviews().is_empty(), "the late pick is not taken");
}

#[test]
fn a_move_past_the_time_hands_the_paper_in() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    app.run(|| {
        if let Phase::Running(p) = &mut *PHASE.write() {
            p.started_ms -= LONG_AGO_MS;
        }
    });
    app.tap(tx::NEXT);
    app.assert_has(tx::RESULT);
}

#[test]
fn the_clock_running_out_hands_the_paper_in() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    app.animation_end("timer-fill");
    app.assert_has(tx::RESULT);
}

#[test]
fn ending_with_blanks_asks_once_and_the_backdrop_closes_it() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.tap("exam-end");
    app.assert_has(&tx::confirm_blank(n - 1));
    // Nothing marked: the sheet says nothing of marks.
    app.assert_lacks(tx::CONFIRM_MARKED);
    // The way back is to the first blank, the second question.
    app.assert_has(tx::CONFIRM_BACK_TO);
    app.tap("sheet-backdrop");
    app.assert_lacks(tx::CONFIRM_TITLE);
    app.assert_has(&format!("1 / {n}"));
    app.tap("exam-end");
    app.tap(tx::CONFIRM_BACK_TO);
    app.assert_has(&format!("2 / {n}"));
}

#[test]
fn answers_from_before_a_launch_fill_the_last_results() {
    let (mut app, _) = topic_paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    all_right_tutor(&mut app);
    drop(app);
    // A new launch reads the log: every question was right last time.
    let mut app = App::open(Screen::Bank);
    app.tap(tx::CUSTOMISE);
    app.tap(tx::ONLY_WRONG);
    app.assert_has(tx::EMPTY_ALL_RIGHT);
    // A sync reloads the history from the store.
    app.run(crate::bank::reload_history);
    app.tap(Screen::Today.label());
    app.tap(Screen::Bank.label());
    app.assert_has(tx::EMPTY_ALL_RIGHT);
}

#[test]
fn a_kept_setup_with_odd_counts_falls_back_to_the_offered_ones() {
    let filter = steps_core::exam::Filter {
        lectures: ["zzz.L09".to_string()].into(),
        tags: ["no-such-tag".to_string()].into(),
        ..Default::default()
    };
    let saved = serde_json::json!({ "count": 7, "mock_count": 3, "advanced": true, "filter": filter });
    crate::store::shared().put_local_setting("bank.setup", &saved).unwrap();
    let app = App::open(Screen::Bank);
    // The lost lecture is dropped, so every question is in scope; the
    // advanced choices stay open.
    app.assert_has(tx::SCOPE_ALL);
    app.assert_has(tx::CUSTOMISE_CLOSE);
    let kept: serde_json::Value = crate::store::shared().local_setting("bank.setup").unwrap();
    assert_eq!(
        (kept["count"].as_u64(), kept["mock_count"].as_u64()),
        (Some(20), Some(steps_core::exam::MOCK_SIZE as u64))
    );
}

#[test]
fn a_course_with_no_questions_says_so() {
    // A course none of whose lectures has an MCQ written yet.
    let with_mcqs: Vec<String> =
        ids_of("mcq").iter().map(|id| steps_core::program::course_of(&lecture_of(id)).to_string()).collect();
    let Some(course) =
        crate::content::Library::get().program.courses.iter().find(|c| !with_mcqs.contains(&c.id))
    else {
        return; // every course has questions
    };
    crate::store::shared().put_setting("course_focus", &course.id).unwrap();
    let app = App::open(Screen::Bank);
    app.assert_has(tx::EMPTY_TITLE);
    app.assert_has(tx::EMPTY_BODY);
}

#[test]
fn another_course_in_view_drops_the_lecture_picks() {
    let mut app = App::with_part(Screen::Bank, practise_button);
    app.tap("practise cvs.L01");
    app.assert_has(tx::SCOPE_CHOSEN);
    app.run(|| crate::courses::focus::set(Some("cvs".into())));
    app.run(|| crate::courses::focus::set(Some("bio".into())));
    app.run(|| crate::courses::focus::set(None));
    app.assert_has(tx::SCOPE_ALL);
}

#[test]
fn two_topics_are_named_by_count_and_a_topic_with_a_tag_empties_to_topics() {
    let id = lecture_with_mcq_topics(2)
        .or_else(|| lecture_with_mcq_topics(1))
        .expect("a lecture has MCQs filed under a topic");
    let with_mcqs = mcq_topics(&id);
    PRACTISE.with(|p| *p.borrow_mut() = id.clone());
    let mut app = App::with_part(Screen::Bank, practise_lecture_button);
    app.tap("practise the lecture");
    app.tap(tx::CUSTOMISE);
    if with_mcqs.len() >= 2 {
        app.tap(&with_mcqs[0].title).tap(&with_mcqs[1].title);
        app.assert_has(&crate::i18n::count::count(2, &crate::i18n::count::TOPIC));
        app.tap(&with_mcqs[1].title);
    }
    // A tag chip toggles on and off.
    for label in [tx::TAG_HIGH_YIELD, tx::TAG_BOOK, tx::TAG_NUMBERS] {
        if app.tap_if(label) {
            app.tap(label);
        }
    }
    app.tap(tx::RESET_SCOPE);
    app.assert_has(tx::SCOPE_ALL);
}

#[test]
fn a_new_paper_from_the_result_goes_back_to_the_setup() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let n = paper_n(&mut app);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right).tap(tx::CHECK);
        // The next step takes the keyboard once «تأكيد الإجابة» is gone.
        app.mount_events();
        next_or_end(&mut app, k, n);
    }
    app.tap(tx::NEW_PAPER);
    app.assert_has(tx::START_TUTOR);
}

#[test]
fn a_weak_topic_after_a_paper_opens_the_bank_on_it() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let n = paper_n(&mut app);
    for k in 0..n {
        let (_, wrong) = options(&shown(&app));
        app.answer(&wrong).tap(tx::CHECK);
        next_or_end(&mut app, k, n);
    }
    app.assert_has(tx::WEAK);
    app.tap("chip weak tap");
    app.assert_has(tx::SCOPE_CHOSEN);
    app.assert_has(tx::START_TUTOR);
}

thread_local! {
    static EMPTY_TOPIC: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn empty_topic_button() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let ui = use_context::<crate::Ui>();
    let topic = EMPTY_TOPIC.with(|t| t.borrow().clone());
    rsx! {
        button { class: "test-topic", onclick: move |_| crate::bank::practise(ui, "cvs.L01", Some(topic.clone())), "practise a topic" }
    }
}

#[test]
fn a_topic_with_no_questions_offers_every_question_instead() {
    let lecture = &crate::content::Library::get().lectures["cvs.L01"];
    let Some(topic) = lecture
        .topics
        .iter()
        .find(|t| !lecture.items.iter().any(|i| i.topic.as_deref() == Some(&t.id) && i.body.kind() == "mcq"))
    else {
        return; // every topic has an MCQ
    };
    EMPTY_TOPIC.with(|t| *t.borrow_mut() = topic.id.clone());
    let mut app = App::with_part(Screen::Bank, empty_topic_button);
    app.tap("practise a topic");
    app.assert_has(tx::NONE_MATCH);
    app.assert_has(tx::EMPTY_TOPICS);
    app.tap(tx::WIDEN_ALL);
    app.assert_has(tx::SCOPE_ALL);
}

// ---------- the paper's screen, as a Step 1 question bank draws it ----------

#[test]
fn the_top_bar_names_the_paper_and_holds_its_tools() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    // The kind, «سؤال 1 من 10», and «1 / 10», which opens the navigator.
    assert_eq!(texts_of(&app, "exam-kind"), [tx::kind(false, true)]);
    assert_eq!(texts_of(&app, "exam-of"), [tx::question_of(1, n)]);
    assert_eq!(attr_of(&app, "exam-count", "aria-label"), format!("{}: {}", tx::NAV_OPEN, tx::question_of(1, n)));
    // A First Aid question has no number of its own to show.
    assert!(with_class(&app, "exam-qid").is_empty(), "no id label for a First Aid question");
    // The mark, with its key and its state in words.
    assert_eq!(attr_of(&app, "exam-mark", "aria-pressed"), "false");
    assert_eq!(attr_of(&app, "exam-mark", "aria-label"), tx::MARK);
    assert_eq!(attr_of(&app, "exam-mark", "aria-keyshortcuts"), "F");
    // The tools: lab values and a note.
    app.assert_has(crate::i18n::labs::OPEN);
    app.assert_has(crate::i18n::notes::CHIP_ADD);
    // The clock: the time left, said in words.
    assert_eq!(attr_of(&app, "exam-clock", "data-kind"), "left");
    assert!(attr_of(&app, "exam-clock", "aria-label").starts_with(tx::TIME_LEFT));
    assert_eq!(texts_of(&app, "exam-end"), [tx::END]);
    // The first question has nowhere to go back to.
    assert!(app.can_tap(tx::NEXT));
    assert!(!app.can_tap(tx::PREV));
    // The tools open what they name (⋮'s label names them too).
    app.tap("exam-tool");
    assert!(app.html().contains("lab-title"), "the lab values sheet opens");
}

#[test]
fn a_tutor_clock_counts_the_time_on_the_question() {
    let app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    assert_eq!(texts_of(&app, "exam-kind"), [tx::kind(false, false)]);
    assert_eq!(attr_of(&app, "exam-clock", "data-kind"), "on");
    assert!(attr_of(&app, "exam-clock", "aria-label").starts_with("الوقت على السؤال ده"));
    app.assert_has(tx::ON_QUESTION);
    // A tutor paper has no drain bar: nothing runs out.
    assert!(with_class(&app, "timer-fill").is_empty());
}

#[test]
fn a_vq_question_shows_its_number_in_the_bar() {
    use crate::vq::{testing, vault};
    let mut app = App::open(Screen::Bank);
    testing::install();
    assert!(vault::save(&testing::key()));
    crate::content::Library::reload();
    app.run(|| crate::shortcuts::open_palette(true));
    app.type_into(crate::i18n::keys::QS_TITLE, "VQ-0001");
    app.tap("VQ-0001");
    app.assert_has(testing::STEM);
    assert_eq!(texts_of(&app, "exam-qid"), ["VQ-0001"]);
    assert_eq!(attr_of(&app, "exam-qid", "dir"), "ltr");
    // Six options, A–F: F stays the key that marks.
    let keys = attrs_of(&app, "q-option", "aria-keyshortcuts");
    assert_eq!(keys, ["A", "B", "C", "D", "E", ""]);
    app.answer(testing::RIGHT).tap(tx::CHECK);
    app.assert_has(tx::VERDICT_RIGHT);
}

#[test]
fn the_navigator_says_each_question_s_state() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.tap("exam-mark");
    // One cell per question in the column, each with its state.
    assert_eq!(with_class(&app, "nav-cell").len(), n);
    let states = attrs_of(&app, "nav-cell", "data-state");
    assert_eq!(states[0], "answered");
    assert!(states[1..].iter().all(|s| s == "blank"), "{states:?}");
    assert_eq!(attrs_of(&app, "nav-cell", "data-flag")[..2], ["yes", "no"]);
    assert_eq!(attrs_of(&app, "nav-cell", "aria-current")[..2], ["true", "false"]);
    // Said in words, not only drawn.
    let labels = attrs_of(&app, "nav-cell", "aria-label");
    assert_eq!(labels[0], format!("{} 1، {}، {}، {}", tx::QUESTION, tx::ANSWERED, tx::MARKED, tx::NOW_HERE));
    assert_eq!(labels[n - 1], format!("{} {n}، {}", tx::QUESTION, tx::UNANSWERED));
    // The mark is a flag as well as a colour.
    assert_eq!(with_class(&app, "nav-flag").len(), 2, "the marked cell's flag and the legend's");
    // A timed paper's legend has no right or wrong yet.
    app.assert_has(tx::LEGEND_ANSWERED);
    app.assert_has(tx::LEGEND_BLANK);
    app.assert_lacks(tx::LEGEND_WRONG);
    // A tap on a number goes there.
    app.tap(&format!("{} 3،", tx::QUESTION));
    app.assert_has(&tx::question_of(3, n));
    assert_eq!(attrs_of(&app, "nav-cell", "aria-current")[2], "true");
}

#[test]
fn a_tutor_navigator_shows_right_and_wrong_with_a_symbol() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let (right, _) = options(&shown(&app));
    app.answer(&right).tap(tx::CHECK);
    app.tap(tx::NEXT);
    let (_, wrong) = options(&shown(&app));
    app.answer(&wrong).tap(tx::CHECK);
    assert_eq!(attrs_of(&app, "nav-cell", "data-state")[..3], ["right", "wrong", "blank"]);
    let labels = attrs_of(&app, "nav-cell", "aria-label");
    assert_eq!(labels[0], format!("{} 1، {}", tx::QUESTION, tx::RIGHT));
    assert_eq!(labels[1], format!("{} 2، {}، {}", tx::QUESTION, tx::WRONG, tx::NOW_HERE));
    // A symbol too, so it never rests on colour alone.
    assert_eq!(texts_of(&app, "nav-mark"), ["✓", "✗"]);
    app.assert_has(tx::LEGEND_RIGHT);
    app.assert_has(tx::LEGEND_WRONG);
}

#[test]
fn a_struck_option_is_crossed_out_still_selectable_and_kept() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let q = shown(&app);
    let (right, wrong) = options(&q);
    app.strike(&wrong);
    assert_eq!(app.option_attr(&wrong, "data-struck"), "true");
    assert_eq!(app.option_attr(&right, "data-struck"), "false");
    app.assert_has(tx::STRUCK);
    // Still his to pick.
    app.answer(&wrong);
    assert_eq!(app.option_attr(&wrong, "aria-pressed"), "true");
    // Kept with the open paper, so a resumed one keeps it.
    let kept = kept_paper().expect("the open paper is kept");
    assert_eq!(kept.answers[0].struck.len(), 1);
    app.tap(tx::LEAVE);
    app.tap(tx::RESUME);
    assert_eq!(app.option_attr(&wrong, "data-struck"), "true");
    // A second long press brings it back.
    app.strike(&wrong);
    assert_eq!(app.option_attr(&wrong, "data-struck"), "false");
    app.assert_lacks(tx::STRUCK);
    // Another question has its own strikes.
    app.strike(&right);
    app.tap(tx::NEXT);
    app.assert_lacks(tx::STRUCK);
}

#[test]
fn a_checked_tutor_question_keeps_its_strike_and_takes_no_more() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let (right, wrong) = options(&shown(&app));
    app.strike(&wrong);
    app.answer(&right).tap(tx::CHECK);
    assert_eq!(app.option_attr(&wrong, "data-struck"), "true");
    app.assert_has(tx::STRUCK);
    // Answered: the options are done with.
    assert_eq!(app.count("contextmenu", &right), 0);
}

#[test]
fn highlighted_words_are_marked_in_the_stem_and_kept() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let q = shown(&app);
    let Body::Mcq { stem, .. } = &q.body else { unreachable!() };
    let words: String = stem.chars().take(9).collect();
    app.run(|| crate::bank::highlight(0, 0, 9));
    assert_eq!(texts_of(&app, "q-hl"), [words]);
    // The stem still reads whole.
    assert!(app.text().contains(stem.as_str()));
    assert_eq!(kept_paper().expect("the open paper is kept").answers[0].highlights.len(), 1);
    // The same words again take it off.
    app.run(|| crate::bank::highlight(0, 0, 9));
    assert!(with_class(&app, "q-hl").is_empty());
    // Each question has its own.
    app.run(|| crate::bank::highlight(0, 2, 6));
    app.tap(tx::NEXT);
    assert!(with_class(&app, "q-hl").is_empty());
    app.tap(tx::PREV);
    assert_eq!(with_class(&app, "q-hl").len(), 1);
}

#[test]
fn a_mark_is_kept_beyond_the_paper_and_brings_the_question_back() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let q = shown(&app);
    app.tap("exam-mark");
    assert_eq!(attr_of(&app, "exam-mark", "aria-pressed"), "true");
    assert_eq!(attr_of(&app, "exam-mark", "aria-label"), tx::MARKED);
    assert_eq!(kept_marks(), BTreeSet::from([q.id.clone()]), "the synced list holds it");
    // Unmarked: off the list; marked again: back on.
    app.tap("exam-mark");
    assert!(kept_marks().is_empty());
    app.tap("exam-mark");
    app.tap(tx::LEAVE);

    // The setup draws a paper from the marked ones.
    app.tap(tx::CUSTOMISE);
    app.tap(tx::ONLY_MARKED);
    app.assert_has(&tx::questions(1));
    app.tap(tx::START_TUTOR);
    app.assert_has("1 / 1");
    assert_eq!(shown(&app).id, q.id);
    // It comes marked; unmarking it here takes it off the list.
    assert_eq!(attr_of(&app, "exam-mark", "aria-pressed"), "true");
    app.tap("exam-mark");
    assert!(kept_marks().is_empty());
    app.tap(tx::LEAVE);
    app.assert_has(tx::EMPTY_NONE_MARKED);
    app.tap(tx::WIDEN_ALL_RIGHT);
    app.assert_lacks(tx::NONE_MATCH);
}

#[test]
fn marks_from_another_device_come_with_a_sync() {
    let mut app = App::open(Screen::Bank);
    let id = ids_of("mcq").remove(0);
    crate::store::shared().put_setting("bank.marked", &BTreeSet::from([id])).unwrap();
    app.run(crate::bank::reload_history);
    app.tap(tx::CUSTOMISE);
    app.tap(tx::ONLY_MARKED);
    app.assert_has(&tx::questions(1));
}

#[test]
fn an_answered_question_says_right_or_wrong_what_he_picked_and_its_time() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    let (_, wrong) = options(&shown(&app));
    app.answer(&wrong).tap(tx::CHECK);
    let p = running(&mut app);
    let (q, a) = (&p.questions[0], &p.answers[0]);
    let picked = letter(q, a.chosen.expect("answered"));
    let key = letter(q, q.answer);
    assert_eq!(attr_of(&app, "q-verdict", "data-tone"), "bad");
    app.assert_has(tx::VERDICT_WRONG);
    app.assert_has(&tx::picked(Some(picked), key));
    // Read out once too: the button that was pressed is gone.
    app.assert_has(&tx::checked(false, key));
    // The why lines, his pick and the right one named in words.
    app.assert_has(tx::EXPLANATION);
    app.assert_has(tx::YOUR_ANSWER);
    app.assert_has(tx::RIGHT_ANSWER);
    // The next one, answered right.
    app.tap(tx::NEXT);
    let (right, _) = options(&shown(&app));
    app.answer(&right).tap(tx::CHECK);
    assert_eq!(attr_of(&app, "q-verdict", "data-tone"), "good");
    app.assert_has(tx::VERDICT_RIGHT);
}

#[test]
fn the_end_of_a_block_sums_it_up() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    // First right, second wrong and marked, fourth left blank and marked.
    let (right, _) = options(&shown(&app));
    app.answer(&right).tap(tx::NEXT);
    let (_, wrong) = options(&shown(&app));
    app.answer(&wrong);
    app.tap("exam-mark");
    app.tap(&format!("{} 4،", tx::QUESTION));
    app.tap("exam-mark");
    app.tap("exam-end");
    app.tap(tx::CONFIRM_END);
    let paper = finished(&mut app);

    // The score, n / m and %, the time used of the time allowed, per question.
    app.assert_has(&format!("1 / {n}"));
    app.assert_has(&format!("{}%", steps_core::exam::Score { right: 1, total: n as u32 }.percent()));
    let allowed = crate::bank::minutes(steps_core::exam::time_limit_ms(n));
    app.assert_has(&format!("{allowed}:00"));
    app.assert_has(tx::PER_QUESTION);
    // Tiles: right, wrong, blank and marked.
    let tiles = texts_of(&app, "stat-tile");
    let want: Vec<String> = [(1, tx::RIGHT), (1, tx::WRONG), (n - 2, tx::TILE_BLANK), (2, tx::MARKED)]
        .iter()
        .map(|(count, label)| format!("{count}{label}"))
        .collect();
    assert_eq!(tiles, want);
    // The table: a row per question, his marks flagged, each opening it.
    for i in 1..=n {
        assert!(app.can_tap(&tx::open_question(i)), "row {i}");
    }
    assert_eq!(with_class(&app, "sum-flag").len(), 2);
    assert_eq!(attrs_of(&app, "sum-row", "data-graded")[..4], ["right", "wrong", "blank", "blank"]);
    // By course, each with n / m.
    app.assert_has(tx::BY_COURSE);
    let lib = crate::content::Library::get();
    for q in &paper.questions {
        let course = steps_core::program::course_of(&q.lecture);
        let title = lib.program.course(course).map_or(course.to_string(), |c| c.title.clone());
        app.assert_has(&title);
    }

    // «راجع الأسئلة الغلط» opens the review on the missed ones.
    app.tap(tx::REVIEW_WRONG);
    assert_eq!(texts_of(&app, "exam-kind"), [format!("{} · {}", tx::REVIEWING, tx::SHOW_WRONG)]);
    app.assert_has(&tx::question_of(2, n));
    app.assert_has(tx::VERDICT_WRONG);
    app.assert_has(tx::EXPLANATION);
    app.tap(tx::NEXT);
    app.assert_has(&tx::question_of(3, n));
    app.assert_has(tx::VERDICT_BLANK);
    app.tap("exam-end");

    // A new paper of the wrong, blank and marked ones, marks kept.
    app.tap(tx::RETAKE);
    let retake = running(&mut app);
    assert_eq!(retake.questions.len(), n - 1);
    assert_eq!(retake.answers.iter().filter(|a| a.flagged).count(), 2);
    assert!(!retake.questions.iter().any(|q| q.id == paper.questions[0].id), "the right one stays out");
}

#[test]
fn the_compact_bar_opens_the_navigator_and_the_tools_as_sheets() {
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let n = paper_n(&mut app);
    // «N / M» opens the navigator over the paper.
    app.tap(tx::NAV_OPEN);
    assert_eq!(with_class(&app, "nav-sheet").len(), 1);
    // A number in it goes there and closes it. The column's cells come first.
    app.tap_nth(&format!("{} 5،", tx::QUESTION), 1);
    assert!(with_class(&app, "nav-sheet").is_empty());
    app.assert_has(&tx::question_of(5, n));
    // The moves sit at the foot as well, beside the main step.
    assert_eq!(with_class(&app, "exam-move").len(), 4, "two in the bar, two at the foot");
    // ⋮ holds the lab values, a note and the end of the block.
    app.tap("exam-more");
    assert_eq!(with_class(&app, "exam-tools-sheet").len(), 1);
    app.tap("exam-tools-end");
    assert!(with_class(&app, "exam-tools-sheet").is_empty());
    app.assert_has(tx::CONFIRM_TITLE);
    app.tap(tx::CONFIRM_BACK_TO);
    app.tap("exam-more");
    // The bar's own note chip comes first.
    app.tap_nth(crate::i18n::notes::CHIP_ADD, 1);
    assert!(with_class(&app, "exam-tools-sheet").is_empty(), "the menu closes as the note opens");
}

#[test]
fn keys_select_confirm_and_move_the_right_way() {
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    // A–E pick; F marks; Escape leaves.
    let keys = attrs_of(&app, "q-option", "aria-keyshortcuts");
    assert!(keys.iter().zip(["A", "B", "C", "D", "E"]).all(|(k, want)| k == want), "{keys:?}");
    assert_eq!(attr_of(&app, "exam-mark", "aria-keyshortcuts"), "F");
    assert_eq!(attr_of(&app, "run-leave", "aria-keyshortcuts"), "Escape");
    // ← goes on and → back: the app reads right to left.
    for (label, key) in [(tx::NEXT, "ArrowLeft"), (tx::PREV, "ArrowRight")] {
        let moves: Vec<String> = with_class(&app, "exam-move")
            .into_iter()
            .filter(|&n| app.page.attr(n, "aria-label") == Some(label))
            .map(|n| app.page.attr(n, "aria-keyshortcuts").unwrap_or_default().to_string())
            .collect();
        assert_eq!(moves, [key, key], "{label}");
    }
    // Enter confirms, then goes on.
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    assert_eq!(attr_of(&app, "run-main-action", "aria-keyshortcuts"), "Enter");
    app.tap(tx::CHECK);
    assert_eq!(attr_of(&app, "run-main-action", "aria-keyshortcuts"), "Enter");
}
