//! The sealed VQ bank on screen (docs/VQ.md), over a one-question fixture
//! sealed with a test key (`vq::testing`): locked, nothing of it shows;
//! unlocked in Settings › بياناتك, a bank of its own («أسئلة MH»), the
//! question, its figure, its video and its VQ-0001 chip; another key, typed
//! or kept, leaves it locked.

use super::App;
use crate::Screen;
use crate::content::Library;
use crate::i18n::vq as tx;
use crate::vq::{testing, vault};

fn open_data(app: &mut App) {
    app.tap(Screen::Settings.label());
    app.tap(crate::i18n::settings::DATA);
}

fn type_key(app: &mut App, text: &str) {
    app.type_into(tx::PLACEHOLDER, text);
    app.form("submit", tx::OPEN, "");
}

/// Quick search for `query`.
fn search(app: &mut App, query: &str) {
    app.run(|| crate::shortcuts::open_palette(true));
    app.type_into(crate::i18n::keys::QS_TITLE, query);
}

fn in_library() -> bool {
    Library::get().lectures[testing::LECTURE].items.iter().any(|i| i.id == testing::ID)
}

#[test]
fn locked_nothing_of_the_bank_shows() {
    testing::install();
    let mut app = App::open(Screen::Bank);
    assert_eq!((Library::get().vq.sealed, Library::get().vq.opened()), (1, 0));
    assert!(!in_library());
    // No set switch and no MH entry in the side menu.
    app.assert_lacks(tx::SET);
    app.assert_lacks(crate::i18n::bank::SET_STEPS);
    search(&mut app, "VQ-0001");
    app.assert_lacks(tx::KIND);
    app.run(|| crate::shortcuts::open_palette(false));
    open_data(&mut app);
    app.assert_has(tx::TITLE);
    app.assert_has(tx::LOCKED);
    app.assert_lacks(tx::FORGET);
}

#[test]
fn unlocked_the_question_its_figure_and_its_chip_show() {
    testing::install();
    let mut app = App::open(Screen::Today);
    open_data(&mut app);
    // Spaces, small letters and no dashes: the key still reads.
    type_key(&mut app, &testing::key().text().to_lowercase().replace('-', " "));
    app.assert_has(tx::SAID_OPENED);
    app.assert_has(&tx::opened(1));
    app.assert_has(tx::FORGET);
    assert!(in_library());
    // The key is kept on the device, never in his data.
    assert_eq!(vault::load(), Some(testing::key()));
    let export = crate::store::shared().export("test", 1).unwrap();
    assert!(!serde_json::to_string(&export.document).unwrap().contains(&testing::key().text()[..9]));
    app.tap(crate::i18n::settings::CLOSE);

    // A bank of its own: Steps' questions don't count it, and its set (from
    // the switch or the side menu) holds it alone.
    app.tap(Screen::Bank.label());
    app.assert_has(crate::i18n::bank::SET_STEPS);
    app.assert_has(crate::i18n::bank::TITLE);
    let steps = Library::get()
        .lectures
        .values()
        .flat_map(|l| &l.items)
        .filter(|i| {
            matches!(i.body, steps_core::content::Body::Mcq { .. }) && !i.tags.iter().any(|t| t == "vq")
        })
        .count();
    assert!(steps > 1);
    app.tap(tx::SET);
    app.assert_has(tx::BANK_TITLE);
    app.assert_has(&crate::i18n::bank::questions(1));
    app.assert_has(&crate::i18n::bank::questions(steps));
    // Practice from it is the question alone.
    app.tap(crate::i18n::bank::START_TUTOR);
    app.assert_has(testing::STEM);
    app.assert_has("1 / 1");
    app.tap(crate::i18n::bank::LEAVE);
    app.tap(crate::i18n::bank::END);
    app.tap_tab(Screen::Bank);
    app.tap(crate::i18n::bank::SET_STEPS);
    app.assert_lacks(tx::BANK_TITLE);

    // Quick search: the question, and its video by name and link.
    search(&mut app, "vq 1");
    app.assert_has(tx::KIND);
    app.assert_has(tx::KIND_VIDEO);
    app.assert_has(testing::VIDEO_TITLE);
    app.assert_has("https://www.youtube.com/watch?v=Fixture_0-1");
    app.tap("VQ-0001");
    app.assert_has(testing::STEM);
    let alt = tx::figure_alt("VQ-0001");
    assert!(app.html().contains("data:image/webp;base64,UklGR"), "the figure, from the sealed file");
    app.assert_has(&alt);
    // A tap opens it full size; its close button closes it.
    app.tap(tx::FIGURE_ZOOM);
    assert!(app.html().contains(r#"class="vq-full""#));
    app.tap(tx::FIGURE_CLOSE);
    assert!(!app.html().contains(r#"class="vq-full""#));
    // Its educational objective waits for the answer.
    app.assert_lacks(testing::OBJECTIVE);
    // Answered: the objective after the why lines, and its number where a
    // First Aid question shows its page.
    app.answer(testing::RIGHT);
    app.tap(crate::i18n::bank::CHECK);
    app.assert_has("Why Mitral regurgitation from the fixture.");
    app.assert_has(tx::OBJECTIVE);
    app.assert_has(testing::OBJECTIVE);
    let html = app.html();
    let whys = html.find(r#"class="q-why-list""#).expect("the why lines");
    let objective = html.find(r#"class="note vq-objective""#).expect("the objective");
    assert!(whys < objective, "the objective comes after the why lines");
    assert!(html.contains(r#"class="note vq-objective" lang="en" dir="ltr""#), "{html}");
    assert!(app.html().contains(r#"class="chip vq-chip""#), "the VQ-0001 chip");
    app.assert_has("VQ-0001");
    // The video after the objective: its title opens it, its link is there
    // to read and copy.
    let html = app.html();
    let objective = html.find(r#"class="note vq-objective""#).unwrap();
    let video = html.find(r#"class="note vq-video""#).expect("the video");
    assert!(objective < video, "the video comes after the objective");
    app.assert_has(tx::VIDEO);
    app.assert_has(testing::VIDEO_TITLE);
    app.assert_has("https://www.youtube.com/watch?v=Fixture_0-1");
    crate::cloud::OPENED.lock().unwrap().clear();
    app.tap(tx::VIDEO_OPEN);
    assert_eq!(*crate::cloud::OPENED.lock().unwrap(), ["https://www.youtube.com/watch?v=Fixture_0-1"]);
    app.tap(tx::VIDEO_COPY);
    app.assert_has(tx::VIDEO_COPIED);

    // At the next start the key is still here.
    Library::reload();
    assert!(in_library());
    // Locked again on this device.
    app.tap(Screen::Settings.label());
    app.tap(crate::i18n::settings::DATA);
    app.tap(tx::FORGET);
    app.assert_has(tx::SAID_LOCKED);
    app.assert_has(tx::LOCKED);
    assert!(!in_library());
    assert_eq!(vault::load(), None);
}

#[test]
fn another_key_stays_locked_with_no_error() {
    testing::install();
    let mut app = App::open(Screen::Today);
    open_data(&mut app);
    type_key(&mut app, "");
    app.assert_has(tx::SAID_EMPTY);
    type_key(&mut app, "not a key at all");
    app.assert_has(tx::SAID_NOT_A_KEY);
    type_key(&mut app, &testing::other_key().text());
    app.assert_has(tx::SAID_OTHER_KEY);
    app.assert_has(tx::LOCKED);
    assert!(!in_library());
    assert_eq!(vault::load(), None, "a key that doesn't fit is never kept");

    // A key kept here that isn't the questions' (another bank's, an old
    // one): the questions stay locked, every screen works, and the card
    // says so kindly.
    assert!(vault::save(&testing::other_key()));
    Library::reload();
    assert!(!in_library());
    app.tap(crate::i18n::settings::CLOSE);
    app.tap(Screen::Bank.label());
    app.assert_has(crate::i18n::bank::TITLE);
    open_data(&mut app);
    app.assert_has(tx::STORED_OTHER);
    app.assert_has(tx::LOCKED);
    app.tap(tx::FORGET);
    assert_eq!(vault::load(), None);
}
