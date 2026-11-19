//! Mohamed's own question bank, VQ (docs/VQ.md). Its questions come sealed
//! in every content pack (`steps_core::sealed`) and open on this device once
//! he enters his key in Settings › بياناتك. They then join their First Aid
//! lectures as ordinary MCQs, so the bank, the blocks, the mistakes log and
//! his ✓ / ✎ / ✗ checks take them like any other question. Without the key,
//! or with another one, Steps is exactly as it was.
//!
//! - [`open_into`] opens them while the library loads (`content::Library`).
//! - `vault` keeps the key on the device, never in the database.
//! - [`KeyCard`] is the Settings card; [`Figure`] shows a question's figure,
//!   [`Objective`] its educational objective with the explanation,
//!   [`VideoLink`] the video that explains it, and [`SourceChip`] its
//!   `VQ-0001` in place of a page.
//!
//! The bank shows them as their own set, «أسئلة MH» (`exam::Set::Vq`), apart
//! from Steps' own questions.

mod figure;
mod key;
#[cfg(test)]
pub(crate) mod testing;
pub mod vault;

use std::collections::BTreeMap;

use steps_core::content::{Item, Lecture};
use steps_core::sealed::{self, Key, SealedLecture, Video};

pub use figure::{Figure, Objective, SourceChip, VideoLink};
pub use key::KeyCard;

use crate::content::Library;
use crate::look::{Css, css};

pub(crate) const CSS: Css = css!("src/vq/vq.css");

/// The kind of source whose items show their number, not a page.
const QBANK: &str = "qbank";

/// The pack's VQ questions as this device sees them: how many it carries
/// sealed, under which key, and those this device's key opened.
#[derive(Debug, Default)]
pub struct Bank {
    /// The key id the sealed files name; none when the pack has no questions.
    pub key_id: Option<String>,
    /// Questions the pack carries sealed.
    pub sealed: usize,
    /// The opened questions by VQ number: their item ids.
    numbers: BTreeMap<u32, String>,
    /// The opened questions' figures, by item id, as `data:` URLs.
    images: BTreeMap<String, String>,
    /// The opened questions' educational objectives, by item id.
    objectives: BTreeMap<String, String>,
    /// The videos that explain the opened questions, by item id.
    videos: BTreeMap<String, Video>,
}

impl Bank {
    /// How many questions this device opened.
    pub fn opened(&self) -> usize {
        self.numbers.len()
    }

    /// Whether `key` is the one the questions were sealed with (any key fits
    /// a pack that has none yet).
    pub fn fits(&self, key: &Key) -> bool {
        self.key_id.as_ref().is_none_or(|k| *k == key.id())
    }

    /// The opened question VQ-`n`, by item id.
    pub fn question(&self, n: u32) -> Option<&str> {
        self.numbers.get(&n).map(String::as_str)
    }

    /// Whether `id` is an opened VQ question.
    pub fn has(&self, id: &str) -> bool {
        sealed::number(id).and_then(|n| self.question(n)) == Some(id)
    }

    /// The question's figure, as a `data:` URL.
    pub fn image(&self, id: &str) -> Option<&str> {
        self.images.get(id).map(String::as_str)
    }

    /// The question's educational objective.
    pub fn objective(&self, id: &str) -> Option<&str> {
        self.objectives.get(id).map(String::as_str)
    }

    /// The video that explains the question.
    pub fn video(&self, id: &str) -> Option<&Video> {
        self.videos.get(id)
    }
}

/// Opens the sealed questions with `key` when it is the key they were
/// sealed with, and appends each to its lecture. Another key, or none,
/// opens nothing; a question that doesn't open is left out and noted in the
/// diagnostics, never shown as an error.
pub fn open_into(
    files: &[SealedLecture],
    key: Option<&Key>,
    lectures: &mut BTreeMap<String, Lecture>,
) -> Bank {
    let mut bank = Bank {
        key_id: files.first().map(|f| f.key.clone()),
        sealed: files.iter().map(|f| f.items.len()).sum(),
        ..Bank::default()
    };
    let Some(key) = key.filter(|k| bank.key_id.as_deref() == Some(k.id().as_str())) else { return bank };
    for file in files {
        let Some(lecture) = lectures.get_mut(&file.lecture) else { continue };
        for item in &file.items {
            let opened = match sealed::open_item(key, item) {
                Ok(o) => o,
                Err(e) => {
                    crate::diag::report!("VQ question {} didn't open: {e}", item.id);
                    continue;
                }
            };
            let Some(n) = sealed::number(&opened.item.id) else { continue };
            if let (Some(image), Some(sealed::IMAGE_TYPE)) = (&opened.image, opened.image_type.as_deref()) {
                bank.images
                    .insert(opened.item.id.clone(), format!("data:{};base64,{image}", sealed::IMAGE_TYPE));
            }
            if let Some(objective) = opened.objective.filter(|o| !o.trim().is_empty()) {
                bank.objectives.insert(opened.item.id.clone(), objective);
            }
            if let Some(video) = opened.video.filter(|v| Video::is_id(&v.youtube)) {
                bank.videos.insert(opened.item.id.clone(), video);
            }
            bank.numbers.insert(n, opened.item.id.clone());
            lecture.items.push(opened.item);
        }
    }
    bank
}

/// `VQ-0001`: how an item from a question bank (a source of kind `qbank`)
/// is named where a First Aid item shows its page.
pub fn source_label(item: &Item) -> Option<String> {
    let book = item.src.book.as_deref()?;
    let source = Library::get().sources.iter().find(|s| s.id == book && s.kind == QBANK)?;
    Some(sealed::label(&source.short, item.src.page))
}

/// `VQ-0001` for a VQ question's id.
pub fn name(id: &str) -> Option<String> {
    let short =
        Library::get().sources.iter().find(|s| s.id == sealed::SOURCE).map_or("VQ", |s| s.short.as_str());
    sealed::number(id).map(|n| sealed::label(short, n))
}

/// The VQ number a search asks for: `VQ-0001`, `vq0001`, `vq 1`.
pub fn search_number(query: &str) -> Option<u32> {
    let q = query.trim().to_lowercase();
    let digits = q.strip_prefix("vq")?.trim_start_matches(['-', ' ', '#']);
    let n: u32 =
        digits.parse().ok().filter(|_| digits.len() <= 4 && digits.bytes().all(|b| b.is_ascii_digit()))?;
    (n > 0).then_some(n)
}

#[cfg(test)]
mod tests;
