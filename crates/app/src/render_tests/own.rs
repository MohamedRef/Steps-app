//! His own cards: the add-a-card sheet (checks, preview, save, cancel),
//! the list on the Study screen (edit, delete, study just these) and a
//! session on them with the grade saved.

use steps_core::review::{Grade, Origin};

use super::App;
use super::kit::reviews;
use crate::Screen;
use crate::i18n::own as tx;

const FRONT: &str = "Which embryonic structure gives rise to the ascending aorta and pulmonary trunk?";
const BACK: &str = "The truncus arteriosus (neural crest cells form the aorticopulmonary septum).";

fn stored() -> Vec<steps_core::own::OwnCard> {
    crate::store::shared().own_cards()
}

/// The Study screen with the add-a-card sheet open.
fn sheet() -> App {
    let mut app = App::open(Screen::Study);
    app.tap(tx::NEW);
    app.assert_has(tx::FRONT);
    app
}

/// A card saved through the sheet.
fn saved() -> App {
    let mut app = sheet();
    app.type_into(tx::FRONT_HINT, FRONT);
    app.type_into(tx::BACK_HINT, BACK);
    app.tap(tx::SAVE);
    app
}

#[test]
fn the_study_screen_offers_his_cards() {
    let app = App::open(Screen::Study);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::NONE_YET);
    assert!(!app.can_tap(tx::STUDY_THEM));
}

#[test]
fn a_new_card_is_checked_before_it_saves() {
    let mut app = sheet();
    app.assert_has(tx::BACK);
    app.assert_has(tx::NO_LECTURE);
    // Nothing typed: the front is asked for first.
    app.tap(tx::SAVE);
    app.assert_has(tx::E_FRONT);
    app.type_into(tx::FRONT_HINT, FRONT);
    app.tap(tx::SAVE);
    app.assert_lacks(tx::E_FRONT);
    app.assert_has(tx::E_BACK);
    assert!(stored().is_empty(), "nothing saved while a field is missing");
    // A source past its limit.
    app.type_into(tx::BACK_HINT, BACK);
    app.type_into(tx::SOURCE_HINT, &"P".repeat(steps_core::own::MAX_SOURCE + 1));
    app.tap(tx::SAVE);
    app.assert_has(tx::E_SOURCE_LONG);
    // Everything he typed is still there.
    app.assert_has(FRONT);
    app.assert_has(&format!("{}/{}", FRONT.chars().count(), steps_core::own::MAX_FRONT));
}

#[test]
fn a_card_with_its_lecture_page_and_source_previews_and_saves() {
    let mut app = sheet();
    app.type_into(tx::FRONT_HINT, FRONT);
    app.type_into(tx::BACK_HINT, BACK);
    app.form("change", tx::NO_LECTURE, "cvs.L01");
    // The page box: the third field without a hint.
    app.form_nth("input", "", 2, "284");
    app.type_into(tx::SOURCE_HINT, "Pathoma");
    app.assert_has(tx::PREVIEW);
    app.assert_has(&crate::i18n::cite::page(284));
    app.assert_has(&format!("{} Pathoma", tx::FROM));
    app.tap(tx::SAVE);
    // The sheet closed and the card is in the store.
    app.assert_lacks(tx::FRONT_HINT);
    let cards = stored();
    assert_eq!(cards.len(), 1);
    let c = &cards[0];
    assert_eq!((c.front.as_str(), c.back.as_str()), (FRONT, BACK));
    assert_eq!(
        (c.lecture.as_deref(), c.page, c.source.as_deref()),
        (Some("cvs.L01"), Some(284), Some("Pathoma"))
    );
    // Listed on the Study screen.
    app.assert_has(&tx::cards(1));
    app.assert_has(FRONT);
}

#[test]
fn a_card_is_studied_on_its_own_and_its_grade_saved() {
    let mut app = saved();
    app.tap(tx::STUDY_THEM);
    app.assert_has(FRONT);
    app.tap(crate::i18n::study::REVEAL);
    app.assert_has(BACK);
    app.tap(crate::i18n::study::GOOD);
    let r = reviews();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].event.item_id, stored()[0].id);
    assert_eq!(r[0].event.origin, Origin::Study);
    assert_eq!(r[0].event.grade, Grade::Good);
}

#[test]
fn a_card_is_edited_and_deleted() {
    let mut app = saved();
    app.tap(tx::EDIT);
    app.assert_has(tx::FORM_EDIT);
    app.type_into(tx::FRONT_HINT, "What does the truncus arteriosus become?");
    app.tap(tx::SAVE);
    assert_eq!(stored()[0].front, "What does the truncus arteriosus become?");
    assert_eq!(stored().len(), 1, "an edit keeps one card");

    // Delete asks first; cancel keeps it.
    app.tap(tx::DELETE);
    app.assert_has(tx::DELETE_SURE);
    app.tap_nth(tx::CANCEL, 0);
    app.assert_lacks(tx::DELETE_SURE);
    assert_eq!(stored().len(), 1);
    app.tap(tx::DELETE).tap(tx::DELETE_YES);
    assert!(stored().is_empty());
    app.assert_has(tx::NONE_YET);
}

#[test]
fn the_sheet_closes_without_saving() {
    let mut app = sheet();
    app.type_into(tx::FRONT_HINT, FRONT);
    app.tap(tx::CANCEL);
    app.assert_lacks(tx::FRONT_HINT);
    assert!(stored().is_empty());
    // Escape and the backdrop close it too.
    app.tap(tx::NEW);
    app.press(tx::FORM_NEW, "Escape");
    app.assert_lacks(tx::FRONT_HINT);
    app.tap(tx::NEW);
    app.tap("own-backdrop");
    app.assert_lacks(tx::FRONT_HINT);
}

#[test]
fn a_shared_draft_opens_the_sheet_and_one_shared_meanwhile_waits() {
    use crate::own::form::{DRAFT, Draft, DraftKind};
    let mut app = App::open(Screen::Study);
    let draft = |id: &str, trimmed| {
        let (card, _) = steps_core::own::draft_from_shared(
            "Patent foramen ovale: failure of septum primum and secundum to fuse.",
            Some("UWorld"),
            id.into(),
            1,
        );
        Draft { card, kind: DraftKind::Shared, trimmed }
    };
    let first = draft("own.a", true);
    app.run(move || *DRAFT.write() = Some(first));
    app.assert_has(tx::FORM_SHARED);
    app.assert_has(tx::SHARED_HINT);
    app.assert_has(tx::SHARED_TRIMMED);
    app.assert_has("Patent foramen ovale");
    // A second share while he writes waits, and opens once this sheet closes.
    let second = draft("own.b", false);
    app.run(move || *crate::own::form::QUEUED.write() = Some(second));
    app.assert_has(tx::SHARE_WAITING);
    app.tap(tx::CANCEL);
    app.assert_has(tx::FORM_SHARED);
    app.assert_lacks(tx::SHARE_WAITING);
    app.assert_lacks(tx::SHARED_TRIMMED);
    assert!(app.run(crate::own::drafting));
}
