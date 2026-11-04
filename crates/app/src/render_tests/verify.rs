//! Verify-while-study: ✓ / ✎ / ✗ under an answer, saved as his verdict in
//! the store, undone, and the ✎ editor with its checks, for a card in a
//! study session and an MCQ in the bank.

use steps_core::content::Body;
use steps_core::review::verify::Verdict;

use super::App;
use super::kit::{first_of, ids_of, item};
use crate::Screen;
use crate::i18n::verify as tx;
use crate::study::{START, Start};

fn verdict(id: &str) -> Option<(Verdict, Option<String>)> {
    crate::store::shared().judgements().unwrap().get(id).map(|j| (j.verdict, j.note.clone()))
}

/// A study session on one card, its answer out.
fn card_revealed() -> (App, String, String, String) {
    let card = first_of("card");
    let Body::Card { front, back } = card.body.clone() else { unreachable!() };
    let mut app = App::open(Screen::Study);
    let id = card.id.clone();
    app.run(move || *START.write() = Some(Start { lectures: None, items: Some(vec![id]), minutes: 5 }));
    app.tap(crate::i18n::study::REVEAL);
    app.assert_has(tx::QUESTION);
    app.assert_has(tx::GROUP);
    (app, card.id, front, back)
}

#[test]
fn wrong_with_a_note_hides_the_item_and_undo_brings_the_question_back() {
    let (mut app, id, _, _) = card_revealed();
    app.tap(tx::WRONG);
    app.assert_has(tx::WRONG_NOTE);
    app.type_into(tx::WRONG_PLACEHOLDER, "First Aid p. 284 says 4-5 weeks");
    app.tap(tx::WRONG_CONFIRM);
    app.assert_has(tx::SAVED_WRONG);
    assert_eq!(verdict(&id), Some((Verdict::Wrong, Some("First Aid p. 284 says 4-5 weeks".into()))));
    app.tap(tx::UNDO);
    assert_eq!(verdict(&id), None);
    app.assert_has(tx::QUESTION);
}

#[test]
fn wrong_can_be_cancelled() {
    let (mut app, id, _, _) = card_revealed();
    app.tap(tx::WRONG);
    app.tap(tx::CANCEL);
    app.assert_lacks(tx::WRONG_NOTE);
    app.assert_has(tx::QUESTION);
    assert_eq!(verdict(&id), None);
}

#[test]
fn an_edit_is_saved_and_the_card_shows_his_words() {
    let (mut app, id, _, back) = card_revealed();
    app.tap(tx::EDIT);
    app.assert_has(tx::F_FRONT);
    app.assert_has(tx::F_BACK);
    app.assert_has(tx::EDIT_NOTE);
    let mine = "The heart; it beats spontaneously by week 4 to 5.";
    app.type_into(&back, mine);
    app.tap(tx::SAVE);
    app.assert_has(tx::SAVED_EDITED);
    assert_eq!(verdict(&id).map(|v| v.0), Some(Verdict::Edited));
    app.assert_has(mine);
}

#[test]
fn an_edit_with_no_change_or_an_empty_side_is_refused() {
    let (mut app, id, front, _) = card_revealed();
    app.tap(tx::EDIT);
    app.tap(tx::SAVE);
    app.assert_has(tx::NOTHING_CHANGED);
    // Typing clears the message; an empty front is a problem of its own.
    app.type_into(&front, "   ");
    app.assert_lacks(tx::NOTHING_CHANGED);
    app.tap(tx::SAVE);
    app.assert_has("الوش والضهر لازم يكون فيهم كلام.");
    assert_eq!(verdict(&id), None);
    app.tap(tx::CANCEL);
    app.assert_has(tx::QUESTION);
}

#[test]
fn a_note_alone_saves_an_edit() {
    let (mut app, id, _, _) = card_revealed();
    app.tap(tx::EDIT);
    // The note box comes after the front and the back.
    assert_eq!(app.count("input", ""), 3, "front, back and the note");
    app.form_nth("input", "", 2, "Add that it is the first functional organ.");
    app.tap(tx::SAVE);
    app.assert_has(tx::SAVED_EDITED);
    let (v, note) = verdict(&id).unwrap();
    assert_eq!(v, Verdict::Edited);
    assert_eq!(note.as_deref(), Some("Add that it is the first functional organ."));
}

#[test]
fn an_mcq_answer_is_edited_from_the_bank() {
    use crate::i18n::bank as bx;
    let mut app = App::open(Screen::Bank);
    app.tap(bx::START_TUTOR);
    let text = app.text();
    let q = ids_of("mcq")
        .into_iter()
        .map(|id| item(&id))
        .find(|i| matches!(&i.body, Body::Mcq { stem, .. } if text.contains(stem.as_str())))
        .unwrap();
    let Body::Mcq { options, answer, .. } = &q.body else { unreachable!() };
    app.tap(&options[*answer]).tap(bx::CHECK);
    app.tap(tx::EDIT);
    app.assert_has(tx::F_STEM);
    app.assert_has(tx::F_OPTIONS);
    app.assert_has(tx::F_ANSWER);
    app.assert_has(tx::F_WHY);
    // Another option as the answer.
    let other = (answer + 1) % options.len();
    app.form("change", &options[0], &other.to_string());
    app.tap(tx::SAVE);
    assert_eq!(verdict(&q.id).map(|v| v.0), Some(Verdict::Edited));
}

#[test]
fn his_checks_are_exported_and_a_failed_export_can_be_tried_again() {
    let (mut app, _, _, _) = card_revealed();
    app.tap(tx::RIGHT);
    app.tap(crate::study::grade::grade_label(steps_core::review::Grade::Good));
    // Settings › Data, from Today's row.
    app.tap(Screen::Today.label());
    app.tap(crate::i18n::today::CHECKED_ROW);
    app.assert_has(tx::EXPORT_TITLE);
    app.assert_has("✓ 1 · ✎ 0 · ✗ 0");
    app.tap(tx::EXPORT_BUTTON);
    app.assert_has(tx::EXPORT_SAVED);
    app.assert_has(tx::EXPORT_SHOW);
    app.assert_has("steps.verdicts/v1");
    // The file it wrote holds the verdict.
    let dir = crate::store::export_dir().unwrap();
    let date = crate::store::zone().calendar_day(crate::store::now_ms()).iso();
    let path = dir.join(format!("steps-verdicts-{date}.yaml"));
    let yaml = std::fs::read_to_string(&path).unwrap();
    assert!(yaml.contains("verdict: approved"), "{yaml}");
    // A folder where the file goes: the write fails and says so.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir_all(&path).unwrap();
    app.tap(tx::EXPORT_BUTTON);
    app.assert_has(tx::EXPORT_FAILED);
    std::fs::remove_dir_all(&path).unwrap();
    app.tap("save-retry");
    app.assert_has(tx::EXPORT_SAVED);
}
