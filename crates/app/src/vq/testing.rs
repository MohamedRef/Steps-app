//! A one-question sealed fixture for the screen tests: VQ-0001 under
//! Cardiovascular › its first lecture, with a figure, sealed with a key made
//! here (never a real one). Each test thread has its own, and its own
//! library (`content::Library::reload`), so tests side by side never see
//! each other's questions, and a test with the fixture never sees the real
//! bank's questions either.

use std::cell::RefCell;

use steps_core::content::{Body, Item, Source, Status};
use steps_core::sealed::{Key, Opened, SealedLecture, seal_item};

pub const LECTURE: &str = "cvs.L01";
pub const ID: &str = "cvs.L01.vq0001";
/// What the fixture's stem says, to look for on screen.
pub const STEM: &str = "A fixture question: a 30-year-old man has a murmur heard best at the apex.";
pub const RIGHT: &str = "Mitral regurgitation from the fixture";
/// A figure: a WebP header is all the screens need.
pub const FIGURE: &[u8] = b"RIFF\x0c\x00\x00\x00WEBPVP8L\x00\x00";
/// What the fixture teaches, shown with its explanation.
pub const OBJECTIVE: &str = "The fixture's objective: an apical holosystolic murmur is mitral regurgitation.";

thread_local! {
    static EXTRA: RefCell<Vec<SealedLecture>> = const { RefCell::new(Vec::new()) };
}

/// The key the fixture is sealed with.
pub fn key() -> Key {
    Key::from_bytes([42; 32])
}

/// A sound key, but not the fixture's.
pub fn other_key() -> Key {
    Key::from_bytes([43; 32])
}

pub fn question() -> Item {
    let lecture = &crate::content::Library::get().lectures[LECTURE];
    let options = [
        RIGHT,
        "Aortic stenosis",
        "Mitral stenosis",
        "Tricuspid regurgitation",
        "Pulmonic stenosis",
        "Ventricular septal defect",
    ];
    Item {
        id: ID.into(),
        body: Body::Mcq {
            stem: format!(
                "{STEM} His echocardiogram is shown. Which of the following is the most likely diagnosis?"
            ),
            options: options.iter().map(|o| o.to_string()).collect(),
            answer: 0,
            why: options.iter().map(|o| format!("Why {o}.")).collect(),
        },
        topic: lecture.topics.first().map(|t| t.id.clone()),
        src: Source { book: Some("vq".into()), page: 1, quote: None },
        status: Status::Unverified,
        update: None,
        tags: vec!["vq".into()],
    }
}

/// This thread's packs carry the sealed fixture from now on, and its
/// library opens again with it (locked until a key is kept).
pub fn install() {
    let opened = Opened::with_image(question(), Some(FIGURE)).with_objective(Some(OBJECTIVE.into()));
    let sealed = seal_item(&key(), &opened);
    EXTRA.with(|e| *e.borrow_mut() = vec![SealedLecture::new(LECTURE, &key(), vec![sealed])]);
    crate::content::Library::reload();
}

/// The sealed questions this thread's library opens (`content::Library::load`):
/// the fixture alone once a test installed it, in place of the real bank, so
/// what the screen tests count and unlock never depends on the questions
/// sealed in the content; otherwise the pack's own.
pub fn with_fixture(sealed: Vec<SealedLecture>) -> Vec<SealedLecture> {
    let fixture = EXTRA.with(|e| e.borrow().clone());
    if fixture.is_empty() { sealed } else { fixture }
}
