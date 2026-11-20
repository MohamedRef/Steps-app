//! The questions Mohamed adds from a file (`steps_core::ownq`,
//! docs/OWN-QUESTIONS.md): Settings › بياناتك › «ضيف أسئلة من ملف» reads
//! the file ([`file`]) and keeps them in the synced store; the library puts
//! each in the First Aid lecture it names ([`open_into`]), and the bank
//! shows each category as a bank of its own (`exam::Set::Own`).
//!
//! What a question carries beyond an MCQ (its figure, explanation,
//! objective and video) is kept here by item id, for the screens.

pub mod file;
pub(crate) mod import;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dioxus::prelude::*;
use steps_core::content::Lecture;
use steps_core::ownq::OwnQuestion;

pub use import::OwnQuestionsCard;

use crate::content::Library;
use crate::look::{Css, css};

pub(crate) const CSS: Css = css!("src/ownq/ownq.css");

/// The opened questions, by item id.
#[derive(Debug, Default)]
pub struct Mine {
    questions: BTreeMap<String, OwnQuestion>,
}

impl Mine {
    pub fn get(&self, item_id: &str) -> Option<&OwnQuestion> {
        self.questions.get(item_id)
    }

    /// Whether any of his questions is filed under MH (the MH bank's own).
    pub fn has_mh(&self) -> bool {
        self.questions.values().any(|q| steps_core::ownq::is_mh(&q.category))
    }

    /// The item id of a question asked for by its own id (`P-0001`), in
    /// any category.
    pub fn find(&self, id: &str) -> Vec<&OwnQuestion> {
        let id = id.trim();
        self.questions.values().filter(|q| q.id.eq_ignore_ascii_case(id)).collect()
    }
}

/// Appends each question to the lecture it names; one whose lecture this
/// library doesn't hold waits, kept in the store.
pub fn open_into(questions: Vec<OwnQuestion>, lectures: &mut BTreeMap<String, Lecture>) -> Mine {
    let mut mine = Mine::default();
    for q in questions {
        let Some(lecture) = lectures.get_mut(&q.lecture) else { continue };
        let item = q.item();
        if lecture.items.iter().any(|i| i.id == item.id) {
            continue;
        }
        mine.questions.insert(item.id.clone(), q);
        lecture.items.push(item);
    }
    mine
}

/// `Pathoma · P-0001`: how a question of his is named where a First Aid
/// item shows its page.
pub fn name(item_id: &str) -> Option<String> {
    Library::get().mine.get(item_id).map(|q| format!("{} · {}", q.category, q.id))
}

/// The question's figure as a `data:` URL, once all of it is on this device.
/// Kept after the first read, by its hash.
pub fn image_url(item_id: &str) -> Option<String> {
    static CACHE: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
    let image = Library::get().mine.get(item_id)?.image.clone()?;
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(url) = cache.get(&image.sha256) {
        return Some(url.clone());
    }
    let bytes = crate::store::shared().own_image(&image)?;
    let url = format!("data:{};base64,{}", image.kind, STANDARD.encode(bytes));
    if cache.len() > 64 {
        cache.clear();
    }
    cache.insert(image.sha256.clone(), url.clone());
    Some(url)
}

/// Whether the question has a figure still on its way from another device.
pub fn image_waiting(item_id: &str) -> bool {
    Library::get()
        .mine
        .get(item_id)
        .and_then(|q| q.image.as_ref())
        .is_some_and(|i| crate::store::shared().own_image(i).is_none())
}

/// Set when another device sent questions or figures: the library opens
/// again with them at the next calm moment (`update::view::use_content_switch`).
static CHANGED: AtomicBool = AtomicBool::new(false);

pub fn synced(tables: &std::collections::BTreeSet<String>) {
    if tables.iter().any(|t| t == "own_questions" || t == "own_images") {
        CHANGED.store(true, Ordering::SeqCst);
    }
}

/// Whether the library should open again for synced questions; asking clears it.
pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::SeqCst)
}

/// One of his questions' explanation, after the why lines; nothing for any
/// other question, or one without.
#[component]
pub fn Explanation(item_id: String) -> Element {
    let text = Library::get().mine.get(&item_id).and_then(|q| q.explanation.clone());
    let Some(text) = text else { return rsx! {} };
    rsx! {
        style { {CSS.text()} }
        aside { class: "note ownq-explain", lang: "en", dir: "ltr",
            h3 { lang: "ar", dir: "rtl", {crate::i18n::ownq::EXPLANATION} }
            p { bdi { "{text}" } }
        }
    }
}
