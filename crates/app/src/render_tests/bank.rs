//! The question bank over the First Aid cardiovascular MCQs: tutor papers
//! checked one by one, timed papers with flags and the navigator, the mock,
//! leaving and resuming, the result screen and the scope filters.

use steps_core::content::{Body, Item};
use steps_core::review::{Grade, Origin};

use dioxus_html::{EventData, SerializedMouseData};

use super::App;
use super::kit::{ids_of, item, reviews};
use crate::Screen;
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

impl App {
    /// Taps the paper's option carrying `text`: only an option, never a
    /// glossary term in the stem whose title happens to hold the same words.
    #[track_caller]
    fn answer(&mut self, text: &str) -> &mut Self {
        let in_options = |mut n: usize| loop {
            if self.page.attr(n, "class").is_some_and(|c| c.split(' ').any(|c| c == "q-options")) {
                return true;
            }
            match self.page.nodes[n].parent {
                Some(p) => n = p,
                None => return false,
            }
        };
        let n = self
            .targets("click", text)
            .into_iter()
            .find(|&n| in_options(n) && self.page.text(n).contains(text))
            .unwrap_or_else(|| panic!("no option {text:?}:\n{}", self.listing("click")));
        self.fire("click", EventData::Mouse(SerializedMouseData::default()), n);
        self
    }
}

fn mcqs() -> usize {
    ids_of("mcq").len()
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

/// The bank set to 10 questions and `mode`, started.
fn paper(mode: &str, start: &str) -> App {
    let mut app = App::open(Screen::Bank);
    app.tap(mode);
    app.tap(tx::ALL);
    app.tap(start);
    app
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
    app.assert_has(&tx::questions(mcqs()));
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
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    app.assert_has(&format!("1 / {n}"));
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
        assert_eq!(reviews().len(), k, "nothing is saved before Check");
        app.tap(tx::CHECK);
        let saved = reviews();
        assert_eq!(saved.len(), k + 1);
        let last = &saved[k].event;
        assert_eq!((last.item_id.as_str(), last.origin), (q.id.as_str(), Origin::Bank));
        assert_eq!(last.grade, if k == 0 { Grade::Again } else { Grade::Good });
        if k == 0 {
            app.assert_has(tx::YOUR_ANSWER);
            app.assert_has(tx::RIGHT_ANSWER);
        } else {
            app.assert_has(tx::RIGHT_PICKED);
        }
        // The explanation: every option's why and the book page.
        let Body::Mcq { why, .. } = &q.body else { unreachable!() };
        app.assert_has(&why[0]);
        got.push(q.id.clone());
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
    }
    got.sort();
    got.dedup();
    assert_eq!(got.len(), n, "every question once");

    // The result: one wrong of n.
    app.assert_has(tx::RESULT);
    app.assert_has(tx::SENT_TO_REVIEW);
    let score = steps_core::exam::Score { right: n as u32 - 1, total: n as u32 };
    app.assert_has(&format!("{}%", score.percent()));
    app.assert_has(tx::BY_TOPIC);
    app.assert_has(tx::REVIEW);
    // Wrong ones listed first; all of them on request.
    app.tap(tx::SHOW_ALL);
    app.assert_has(&format!("{} {}", tx::QUESTION, n));
    app.tap(tx::SHOW_WRONG);

    // Retry the wrong one in tutor mode.
    app.tap(tx::RETRY_WRONG);
    app.assert_has("1 / 1");
}

#[test]
fn the_result_opens_each_question_and_goes_back_to_the_bank() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right).tap(tx::CHECK);
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
    }
    app.assert_has("100%");
    // The score takes the focus, to be read out.
    assert!(app.mount_events() > 0);
    // All right: no retry, a new paper instead; strong topics praised.
    app.assert_lacks(tx::RETRY_WRONG);
    app.assert_has(tx::NEW_PAPER);
    app.assert_has(tx::ALL_STRONG);
    // A question opens with its explanation, then closes.
    app.tap(&format!("{} 1", tx::QUESTION));
    app.assert_has(tx::CLOSE);
    app.assert_has(tx::RIGHT_PICKED);
    app.tap(tx::CLOSE);
    app.tap(tx::BACK);
    app.assert_has(tx::START_TUTOR);
}

#[test]
fn a_timed_paper_flags_moves_and_asks_before_it_goes_in() {
    let n = mcqs();
    assert!(n >= 2, "the timed paper needs two questions");
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    app.assert_has(tx::MODE_TIMED_NAME);
    app.assert_has(tx::LEFT_ABOUT);
    app.assert_has(&format!("{n} {}", tx::UNANSWERED));
    let first = shown(&app);
    let (right, _) = options(&first);
    app.answer(&right);
    // A timed pick is the answer, with no Check and no explanation yet.
    app.assert_lacks(tx::CHECK);
    app.assert_lacks(tx::RIGHT_PICKED);
    app.tap(tx::FLAG);
    app.assert_has(tx::FLAGGED);
    app.tap(tx::NEXT);
    app.assert_has(&format!("2 / {n}"));
    app.tap(tx::PREV);
    app.assert_has(&format!("1 / {n}"));
    // The navigator jumps to the last question.
    app.tap(&format!("{} {n}", tx::QUESTION));
    app.assert_has(&format!("{n} / {n}"));
    assert!(reviews().is_empty(), "a timed paper is saved when it goes in");

    // Blanks and a flag: he is asked first; going back lands on a blank.
    app.tap_nth(tx::SUBMIT, 0);
    app.assert_has(tx::CONFIRM_TITLE);
    app.assert_has(tx::CONFIRM_BLANK_B);
    app.assert_has(tx::CONFIRM_FLAGGED);
    app.tap(tx::CONFIRM_BACK_TO);
    app.assert_lacks(tx::CONFIRM_TITLE);
    app.assert_has(&format!("2 / {n}"));
    app.tap_nth(tx::SUBMIT, 0);
    app.tap(tx::CONFIRM_SUBMIT);

    // In: one answered (right), the rest blank.
    app.assert_has(tx::RESULT);
    app.assert_has(tx::TOOK);
    app.assert_has(tx::UNANSWERED);
    let saved = reviews();
    assert_eq!(saved.len(), 1, "blank answers are not reviews");
    assert_eq!((saved[0].event.item_id.as_str(), saved[0].event.grade), (first.id.as_str(), Grade::Good));
    assert_eq!(saved[0].event.origin, Origin::Bank);
    // Only flagged ones, on request.
    app.tap(tx::SHOW_FLAGGED);
    app.assert_has(&format!("{} 1", tx::QUESTION));

    // The paper is kept in the history on the setup.
    app.tap(tx::BACK);
    app.assert_has(tx::HISTORY);
    app.assert_has(tx::MODE_TIMED);
    app.assert_has(&format!("1/{n}"));
}

#[test]
fn a_whole_timed_paper_answered_goes_in_without_asking() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right);
        if k + 1 < n {
            app.tap(tx::NEXT);
        }
    }
    app.tap_nth(tx::SUBMIT, 0);
    app.assert_lacks(tx::CONFIRM_TITLE);
    app.assert_has("100%");
    assert_eq!(reviews().len(), n);
}

#[test]
fn a_timed_paper_left_waits_with_its_clock_and_can_be_resumed_or_submitted() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    let (right, _) = options(&shown(&app));
    app.answer(&right);
    app.tap(tx::LEAVE);
    app.assert_has(tx::PAUSED_TITLE);
    app.assert_has(&format!("1/{n}"));
    app.assert_has(tx::PAUSED_LEFT);
    // Back to it, on the same question with his answer.
    app.tap(tx::RESUME);
    app.assert_has(&format!("1 / {n}"));
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
    app.assert_has(tx::LEFT_ABOUT);
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
    app.assert_has(tx::MODE_MOCK_NAME);
    // A full block, or the whole pool when fewer are written.
    let n = app.run(|| {
        use dioxus::prelude::*;
        match &*crate::bank::PHASE.peek() {
            crate::bank::Phase::Running(p) => p.questions.len(),
            _ => panic!("the block runs"),
        }
    });
    assert_eq!(n, mcqs().min(steps_core::exam::MOCK_SIZE));
    let (_, wrong) = options(&shown(&app));
    app.answer(&wrong);
    app.tap_nth(tx::SUBMIT, 0);
    app.tap(tx::CONFIRM_SUBMIT);
    app.assert_has(tx::MOCK_TITLE);
    app.assert_has(tx::RETRY_WRONG);
    let saved = reviews();
    assert_eq!(saved.len(), 1);
    assert_eq!((saved[0].event.origin, saved[0].event.grade), (Origin::Mock, Grade::Again));
    app.tap(tx::BACK);
    app.assert_has(tx::MODE_MOCK);
    app.assert_has(&format!("0/{n}"));
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
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right).tap(tx::CHECK);
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
    }
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
fn a_finished_paper_row_opens_its_lecture() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right);
        if k + 1 < n {
            app.tap(tx::NEXT);
        }
    }
    app.tap_nth(tx::SUBMIT, 0);
    // A topic row practises the topic.
    app.tap(tx::PRACTISE_TOPIC);
    app.assert_has(tx::SCOPE_CHOSEN);
    // The history row opens the lecture page: one the paper drew from.
    app.tap(tx::OPEN_LECTURE);
    let lecture = open_lecture(&mut app).expect("a lecture page opened");
    assert!(ids_of("mcq").iter().any(|id| lecture_of(id) == lecture), "{lecture} has no MCQs");
    app.assert_has(crate::content::Library::get().title(&lecture));
}

/// A timed paper on every MCQ, started `ago_ms` before now, as the app keeps
/// one it was killed in.
/// Longer than any paper's time, whatever the bank holds (a paper's time
/// grows with its questions: a whole bank of 1000 is over a day).
const LONG_AGO_MS: i64 = 365 * 86_400_000;

fn open_paper(ago_ms: i64) -> crate::bank::Paper {
    use steps_core::exam::{Answer, Mode, Question};
    let questions: Vec<Question> = ids_of("mcq")
        .into_iter()
        .map(|id| {
            let i = item(&id);
            let Body::Mcq { options, answer, .. } = &i.body else { unreachable!() };
            Question {
                id: id.clone(),
                lecture: "cvs.L01".into(),
                topic: i.topic.clone(),
                answer: *answer,
                order: (0..options.len()).collect(),
            }
        })
        .collect();
    let now = crate::bank::now_ms();
    crate::bank::Paper {
        mode: Mode::Timed,
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
    let paper = open_paper(60_000);
    crate::store::shared().put_local_setting("bank.open_paper", &Some(paper)).unwrap();
    let mut app = App::open(Screen::Bank);
    app.assert_has(tx::PAUSED_TITLE);
    app.assert_has(tx::PAUSED_LEFT);
    app.tap(tx::RESUME);
    app.assert_has(tx::LEFT_ABOUT);
    app.assert_has(&format!("1 / {}", mcqs()));
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
        if let crate::bank::Phase::Running(p) = &mut *crate::bank::PHASE.write() {
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
        if let crate::bank::Phase::Running(p) = &mut *crate::bank::PHASE.write() {
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
fn a_full_paper_with_a_flag_asks_and_goes_back_to_it() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right);
        if k == 0 {
            app.tap(tx::FLAG);
            // Flag off and on again.
            app.tap(&format!("⚑ {}", tx::FLAGGED));
            app.tap(tx::FLAG);
        }
        if k + 1 < n {
            app.tap(tx::NEXT);
        }
    }
    app.tap_nth(tx::SUBMIT, 0);
    app.assert_has(tx::CONFIRM_FLAGGED);
    app.assert_lacks(tx::CONFIRM_BLANK_B);
    app.tap(tx::CONFIRM_BACK);
    app.assert_lacks(tx::CONFIRM_TITLE);
    // The backdrop closes it too.
    app.tap_nth(tx::SUBMIT, 0);
    app.tap("sheet-backdrop");
    app.assert_lacks(tx::CONFIRM_TITLE);
}

#[test]
fn answers_from_before_a_launch_fill_the_last_results() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right).tap(tx::CHECK);
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
    }
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
fn a_result_review_shows_flagged_marks() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TIMED_NAME, tx::START_TIMED);
    app.tap(tx::FLAG);
    app.tap(&format!("{} {n}", tx::QUESTION));
    app.tap_nth(tx::SUBMIT, 0);
    app.tap(tx::CONFIRM_SUBMIT);
    app.tap(tx::SHOW_ALL);
    app.assert_has("⚑");
    app.assert_has(tx::BY_TOPIC);
}

#[test]
fn a_new_paper_from_the_result_goes_back_to_the_setup() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    for k in 0..n {
        let (right, _) = options(&shown(&app));
        app.answer(&right).tap(tx::CHECK);
        // The next step takes the keyboard once Check is gone.
        app.mount_events();
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
    }
    app.tap(tx::NEW_PAPER);
    app.assert_has(tx::START_TUTOR);
}

#[test]
fn a_weak_topic_after_a_paper_opens_the_bank_on_it() {
    let n = mcqs();
    let mut app = paper(tx::MODE_TUTOR_NAME, tx::START_TUTOR);
    for k in 0..n {
        let (_, wrong) = options(&shown(&app));
        app.answer(&wrong).tap(tx::CHECK);
        app.tap(if k + 1 == n { tx::FINISH } else { tx::NEXT });
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
