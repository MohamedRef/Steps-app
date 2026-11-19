//! Checks for the sealed VQ files (`crate::sealed`, docs/VQ.md): what can be
//! checked without the key, and the rules every opened question meets on
//! top of an ordinary item's. The tools read the files and the key.

use std::collections::BTreeSet;

use super::{Context, Problem, item_rules};
use crate::content::{Body, Lecture};
use crate::sealed::{self, Opened, SealedLecture};

/// What one sealed file must be without opening it: the schema, a key id,
/// and `<lecture>.vqNNNN` ids of kind `mcq`, each once, sorted.
pub fn file(f: &SealedLecture) -> Vec<Problem> {
    let mut out = Vec::new();
    let lid = f.lecture.as_str();
    let mut bad = |at: &str, message: String| out.push(Problem { at: at.to_string(), message });
    if f.schema != sealed::SCHEMA {
        bad(lid, format!("schema is {:?}, expected {:?}", f.schema, sealed::SCHEMA));
    }
    if !super::is_lecture_id(lid) {
        bad(lid, "lecture id must look like cvs.L04".into());
    }
    if f.key.len() != 8 || !f.key.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        bad(lid, format!("key {:?} is not a key id (8 hex digits, `steps-tools vq keygen`)", f.key));
    }
    if f.items.is_empty() {
        bad(lid, "a sealed file with no questions; `steps-tools vq seal` removes it".into());
    }
    let mut seen = BTreeSet::new();
    for (i, item) in f.items.iter().enumerate() {
        let at = item.id.as_str();
        if sealed::under(at, lid).is_none() {
            bad(at, format!("id must be {lid}.vqNNNN (the VQ number, four digits)"));
        }
        if !seen.insert(at) {
            bad(at, "id is used twice".into());
        }
        if i > 0 && f.items[i - 1].id.as_str() > at {
            bad(at, "ids are out of order; `steps-tools vq seal` writes them sorted".into());
        }
        if item.kind != "mcq" {
            bad(at, format!("kind is {:?}; VQ questions are mcq", item.kind));
        }
        if item.sealed.trim().is_empty() {
            bad(at, "sealed is empty".into());
        }
    }
    out
}

/// An opened question, inside its lecture: an ordinary item's rules (topic,
/// MCQ shape, approvals: `validate::item_rules`) and the bank's own: `src`
/// is `vq` at the id's number, the `vq` tag, a figure, when it has one,
/// that is a WebP of at most [`sealed::MAX_IMAGE_BYTES`], an educational
/// objective, when it has one, that says something, and a video, when it
/// has one, with a YouTube id's shape and a title. Figures are optional: a
/// stem's "is shown" often means values shown as text.
pub fn opened(lec: &Lecture, o: &Opened, cx: &Context<'_>) -> Vec<Problem> {
    let item = &o.item;
    let mut out = item_rules(lec, item, cx);
    if item.src.book.as_deref() != Some(sealed::SOURCE) {
        out.push(format!("src.book must be {}", sealed::SOURCE));
    }
    match sealed::number(&item.id) {
        Some(n) if item.src.page != n => out.push(format!("src.page must be {n}, the id's VQ number")),
        _ => {}
    }
    if !item.tags.iter().any(|t| t == sealed::TAG) {
        out.push(format!("tags need `{}`", sealed::TAG));
    }
    if !matches!(item.body, Body::Mcq { .. }) {
        out.push("VQ questions are mcq".into());
    }
    match (&o.image, o.image_type.as_deref()) {
        (None, None) => {}
        (Some(_), Some(sealed::IMAGE_TYPE)) => match o.image_bytes() {
            None => out.push("image is not base64".into()),
            Some(b) if !sealed::is_webp(&b) => out.push("image is not a WebP file".into()),
            Some(b) if b.len() > sealed::MAX_IMAGE_BYTES => out.push(format!(
                "image is {} KB, over {} KB; save it smaller",
                b.len().div_ceil(1024),
                sealed::MAX_IMAGE_BYTES / 1024
            )),
            Some(_) => {}
        },
        (Some(_), t) => out.push(format!("image_type is {t:?}, expected {:?}", sealed::IMAGE_TYPE)),
        (None, Some(_)) => out.push("image_type without an image".into()),
    }
    if o.objective.as_deref().is_some_and(|t| t.trim().is_empty()) {
        out.push("objective is empty; write it or leave it out".into());
    }
    if let Some(v) = &o.video {
        if !sealed::Video::is_id(&v.youtube) {
            out.push(format!(
                "video.youtube {:?} is not a YouTube video id (11 of A-Z a-z 0-9 - _)",
                v.youtube
            ));
        }
        let title = v.title.trim();
        if title.is_empty() {
            out.push("video.title is empty".into());
        } else if title != v.title || title.contains('\n') {
            out.push("video.title has spaces at an end or a line break".into());
        } else if title.chars().count() > sealed::Video::MAX_TITLE {
            out.push(format!("video.title is over {} characters", sealed::Video::MAX_TITLE));
        }
    }
    out.into_iter().map(|message| Problem { at: item.id.clone(), message }).collect()
}

#[cfg(test)]
#[path = "sealed_tests.rs"]
mod tests;
