//! The offline content pack: `content/index.yaml` plus every written lecture,
//! as one DEFLATE-compressed JSON blob. `crates/app/build.rs` builds it from
//! the YAML and embeds it with `include_bytes!`; `steps-tools packs build`
//! writes the same bytes to `dist/packs/` and `--check`s them in CI.
//!
//! JSON rather than YAML so the app parses it fast on the phone, and rather
//! than a binary format because items use `#[serde(flatten)]` and untagged
//! enums, which only self-describing formats support.

use std::collections::BTreeMap;

use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec_with_limit;
use serde::{Deserialize, Serialize};

use crate::content::{Index, Lecture, SourceBook};
use crate::lesson::LessonFile;
use crate::program::Program;
use crate::sealed::SealedLecture;

/// The first bytes of every pack. Bump the digit when the layout changes.
pub const MAGIC: &[u8; 8] = b"STEPSPK1";

/// The content types this build reads. Bump it whenever an item, lesson,
/// step or picture gains a field or a variant that an older app would drop
/// or misread: an older app then refuses the newer pack (and keeps its
/// built-in content) instead of showing it with pieces silently missing.
/// Packs from before the field read as 0. 2: courses can cite slides
/// (`program::Source`). 3: lesson steps can `remind` of a تحشيشة taught
/// elsewhere (`lesson::Reminder`). 4: a sealed VQ question can carry the
/// video that explains it (`sealed::Video`); apps from 2026.11.5 open
/// sealed questions with `deny_unknown_fields`, so they must keep their
/// pack rather than lose every question that has one.
///
/// Not bumped for [`Pack::sealed`] and [`Pack::sources`] (2026-11-18): they
/// are new fields of the pack itself, which older apps ignore, and an older
/// app could never open sealed questions anyway.
pub const SCHEMA: u32 = 4;

/// Highest miniz level: slower to pack (only when content changes), not to unpack.
const LEVEL: u8 = 10;

/// An unpacked pack can't be bigger than this (the whole chapter is a few MB).
const MAX_UNPACKED: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    /// [`SCHEMA`] of the build that wrote it.
    #[serde(default)]
    pub schema: u32,
    pub index: Index,
    /// Written lectures, in index order.
    pub lectures: Vec<Lecture>,
    /// Lessons (`steps.lesson/v1`), in lecture order. Older packs have none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lessons: Vec<LessonFile>,
    /// The app's stylesheets, by path under `crates/app` (`src/lecture/lecture.css`),
    /// so a change to the look reaches devices without a new app build. Only
    /// the build the feed names in `ui_build` uses them (`update::UpdateManifest`);
    /// the pack built into the app has none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub look: BTreeMap<String, String>,
    /// `content/program.yaml`: the program's courses. Older packs have none
    /// (`Program::fallback`); an older app ignores it and shows every lecture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<Program>,
    /// The sealed VQ files (`content/<course>/vq/LNN.yaml`, `crate::sealed`),
    /// copied as they are, in lecture order: a device opens them only with
    /// Mohamed's key. Older packs have none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sealed: Vec<SealedLecture>,
    /// `content/sources.yaml`'s sources, so a device can name an item's
    /// source on its page chip (`VQ-0001`). Older packs have none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceBook>,
}

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("not a Steps content pack (bad header)")]
    Magic,
    #[error("the pack is corrupt: {0}")]
    Inflate(String),
    #[error("the pack's JSON doesn't match the content types: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the pack needs content types {0}, this build reads up to {SCHEMA}")]
    TooNew(u32),
}

impl Pack {
    /// Orders the lectures as the index lists them; lectures it doesn't list go last.
    pub fn new(index: Index, mut lectures: Vec<Lecture>) -> Self {
        let pos = |id: &str| index.lectures.iter().position(|e| e.id == id).unwrap_or(usize::MAX);
        lectures.sort_by(|a, b| pos(&a.lecture).cmp(&pos(&b.lecture)).then(a.lecture.cmp(&b.lecture)));
        Pack {
            schema: SCHEMA,
            index,
            lectures,
            lessons: Vec::new(),
            look: BTreeMap::new(),
            program: None,
            sealed: Vec::new(),
            sources: Vec::new(),
        }
    }

    /// Adds the sealed VQ files, in the lectures' order.
    pub fn with_sealed(mut self, mut sealed: Vec<SealedLecture>) -> Self {
        let pos = |id: &str| self.index.lectures.iter().position(|e| e.id == id).unwrap_or(usize::MAX);
        sealed.sort_by(|a, b| pos(&a.lecture).cmp(&pos(&b.lecture)).then(a.lecture.cmp(&b.lecture)));
        self.sealed = sealed;
        self
    }

    /// Adds `content/sources.yaml`'s sources.
    pub fn with_sources(mut self, sources: Vec<SourceBook>) -> Self {
        self.sources = sources;
        self
    }

    /// Adds the lesson files, in the lectures' order.
    pub fn with_lessons(mut self, mut lessons: Vec<LessonFile>) -> Self {
        let pos = |id: &str| self.index.lectures.iter().position(|e| e.id == id).unwrap_or(usize::MAX);
        lessons.sort_by(|a, b| pos(&a.lecture).cmp(&pos(&b.lecture)).then(a.lecture.cmp(&b.lecture)));
        self.lessons = lessons;
        self
    }

    /// Adds the app's stylesheets (path under `crates/app` → CSS).
    pub fn with_program(mut self, program: Program) -> Self {
        self.program = Some(program);
        self
    }

    /// The pack's program, or one course per lecture prefix for an older pack.
    pub fn program_or_fallback(&self) -> Program {
        self.program.clone().unwrap_or_else(|| Program::fallback(&self.index))
    }

    pub fn with_look(mut self, look: BTreeMap<String, String>) -> Self {
        self.look = look;
        self
    }

    pub fn encode(&self) -> Vec<u8> {
        let json = serde_json::to_vec(self).expect("content types always serialize");
        let mut out = MAGIC.to_vec();
        out.extend(compress_to_vec(&json, LEVEL));
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PackError> {
        let body = bytes.strip_prefix(MAGIC.as_slice()).ok_or(PackError::Magic)?;
        let json = decompress_to_vec_with_limit(body, MAX_UNPACKED)
            .map_err(|e| PackError::Inflate(format!("{e:?}")))?;
        let pack: Pack = serde_json::from_slice(&json)?;
        if pack.schema > SCHEMA {
            return Err(PackError::TooNew(pack.schema));
        }
        Ok(pack)
    }

    /// Integrity problems a device must not activate: ids that repeat or
    /// sit under the wrong lecture, lectures and lessons the index doesn't
    /// list, schema tags this build doesn't read, references from lessons
    /// to items or topics that aren't there, and the program's own rules.
    /// Cheap (no I/O, no parsing): the app runs it on every pack it takes
    /// or opens, and `steps-tools packs build` before anything ships.
    /// Authoring rules that need the YAML (quote length, approvals) stay
    /// in `steps-tools validate`. Empty = sound.
    pub fn problems(&self) -> Vec<String> {
        use std::collections::{BTreeMap, BTreeSet};
        let mut out = Vec::new();
        if self.index.schema != crate::content::SCHEMA {
            out.push(format!(
                "index: schema is {:?}, expected {:?}",
                self.index.schema,
                crate::content::SCHEMA
            ));
        }
        let mut listed = BTreeSet::new();
        for e in &self.index.lectures {
            if !crate::validate::is_lecture_id(&e.id) {
                out.push(format!("index: lecture id {:?} must look like cvs.L04", e.id));
            }
            if !listed.insert(e.id.as_str()) {
                out.push(format!("index: lecture {} is listed twice", e.id));
            }
        }
        let under = |id: &str, lecture: &str| {
            id.strip_prefix(lecture).is_some_and(|r| r.starts_with('.') && r.len() > 1)
        };
        // Items and topics by lecture, for the lessons' references.
        let mut items: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        let mut topics: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        let mut all_items = BTreeSet::new();
        for lec in &self.lectures {
            let lid = lec.lecture.as_str();
            if lec.schema != crate::content::SCHEMA {
                out.push(format!("{lid}: schema is {:?}, expected {:?}", lec.schema, crate::content::SCHEMA));
            }
            if !listed.contains(lid) {
                out.push(format!("{lid}: not listed in the index"));
            }
            if items.contains_key(lid) {
                out.push(format!("{lid}: the lecture appears twice"));
            }
            let own = items.entry(lid).or_default();
            for item in &lec.items {
                if !under(&item.id, lid) {
                    out.push(format!("{}: item id isn't under its lecture {lid}", item.id));
                }
                if !all_items.insert(item.id.as_str()) {
                    out.push(format!("{}: item id is used twice", item.id));
                }
                own.insert(item.id.as_str());
            }
            topics.entry(lid).or_default().extend(lec.topics.iter().map(|t| t.id.as_str()));
        }
        let mut lesson_ids = BTreeSet::new();
        let mut lesson_files = BTreeSet::new();
        for file in &self.lessons {
            let lid = file.lecture.as_str();
            if file.schema != crate::lesson::SCHEMA {
                out.push(format!(
                    "{lid} lessons: schema is {:?}, expected {:?}",
                    file.schema,
                    crate::lesson::SCHEMA
                ));
            }
            if !listed.contains(lid) {
                out.push(format!("{lid} lessons: the lecture isn't listed in the index"));
            }
            if !lesson_files.insert(lid) {
                out.push(format!("{lid} lessons: the file appears twice"));
            }
            for lesson in &file.lessons {
                let at = lesson.id.as_str();
                if !under(at, lid) {
                    out.push(format!("{at}: lesson id isn't under its lecture {lid}"));
                }
                if !lesson_ids.insert(at) {
                    out.push(format!("{at}: lesson id is used twice"));
                }
                // References resolve only when the lecture's items ship too.
                if let Some(own) = items.get(lid) {
                    let tried = lesson.steps.iter().flat_map(|s| match &s.body {
                        crate::lesson::StepBody::Try { items } => items.as_slice(),
                        _ => &[],
                    });
                    for item in lesson.practice.iter().chain(tried) {
                        if !own.contains(item.as_str()) {
                            out.push(format!("{at}: item {item} isn't in lecture {lid}"));
                        }
                    }
                    let known = &topics[lid];
                    for topic in &lesson.topics {
                        if !known.contains(topic.as_str()) {
                            out.push(format!("{at}: topic {topic} isn't in lecture {lid}"));
                        }
                    }
                }
            }
        }
        // Sealed questions: listed lectures, `<lecture>.vqNNNN` ids that no
        // other item has, and one key for every file.
        let mut sealed_files = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for file in &self.sealed {
            let lid = file.lecture.as_str();
            if file.schema != crate::sealed::SCHEMA {
                out.push(format!(
                    "{lid} sealed: schema is {:?}, expected {:?}",
                    file.schema,
                    crate::sealed::SCHEMA
                ));
            }
            if !listed.contains(lid) {
                out.push(format!("{lid} sealed: the lecture isn't listed in the index"));
            }
            if !sealed_files.insert(lid) {
                out.push(format!("{lid} sealed: the file appears twice"));
            }
            keys.insert(file.key.as_str());
            for item in &file.items {
                if crate::sealed::under(&item.id, lid).is_none() {
                    out.push(format!("{}: sealed id isn't {lid}.vqNNNN", item.id));
                }
                if !all_items.insert(item.id.as_str()) {
                    out.push(format!("{}: item id is used twice", item.id));
                }
            }
        }
        if keys.len() > 1 {
            let keys: Vec<&str> = keys.into_iter().collect();
            out.push(format!("sealed: files were sealed with different keys ({})", keys.join(", ")));
        }
        if let Some(program) = &self.program {
            out.extend(program.problems(&self.index).into_iter().map(|p| format!("program: {p}")));
        }
        out
    }

    /// How many sealed questions the pack carries.
    pub fn sealed_count(&self) -> usize {
        self.sealed.iter().map(|f| f.items.len()).sum()
    }

    pub fn item_count(&self) -> usize {
        self.lectures.iter().map(|l| l.items.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Lecture {
        let text = include_str!("../../../fixtures/content/sample.yaml");
        serde_yaml_ng::from_str(text).unwrap()
    }

    fn index() -> Index {
        serde_yaml_ng::from_str(
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n  - { id: cvs.L05, title: Pharmacology, pages: [321, 328], chunk: 6 }\n",
        )
        .unwrap()
    }

    #[test]
    fn a_pack_for_newer_content_types_is_refused() {
        let mut pack = Pack::new(index(), vec![sample()]);
        assert_eq!(pack.schema, SCHEMA);
        pack.schema = SCHEMA + 1;
        assert!(matches!(Pack::decode(&pack.encode()), Err(PackError::TooNew(n)) if n == SCHEMA + 1));
        // A pack from before the field still opens.
        let json = serde_json::to_string(&Pack::new(index(), vec![sample()]))
            .unwrap()
            .replace(&format!("\"schema\":{SCHEMA},"), "");
        let mut old = MAGIC.to_vec();
        old.extend(compress_to_vec(json.as_bytes(), LEVEL));
        assert_eq!(Pack::decode(&old).unwrap().schema, 0);
    }

    #[test]
    fn a_pack_round_trips_and_keeps_index_order() {
        let mut second = sample();
        second.lecture = "cvs.L05".into();
        let pack = Pack::new(index(), vec![second, sample()]);
        assert_eq!(pack.lectures[0].lecture, "cvs.L04");
        let bytes = pack.encode();
        assert!(bytes.starts_with(MAGIC));
        let back = Pack::decode(&bytes).unwrap();
        assert_eq!(back, pack);
        assert_eq!(back.item_count(), 16);
    }

    #[test]
    fn the_program_travels_and_old_packs_fall_back() {
        let pack = Pack::new(index(), vec![sample()]);
        assert_eq!(pack.program_or_fallback().courses[0].id, "cvs");
        let program: Program =
            serde_yaml_ng::from_str(include_str!("../../../content/program.yaml")).unwrap();
        let pack = pack.with_program(program.clone());
        let back = Pack::decode(&pack.encode()).unwrap();
        assert_eq!(back.program.as_ref(), Some(&program));
        assert_eq!(back.program_or_fallback().courses.len(), 17);
    }

    #[test]
    fn a_sound_pack_has_no_problems_and_broken_ids_are_named() {
        let pack = Pack::new(index(), vec![sample()]);
        assert_eq!(pack.problems(), Vec::<String>::new());

        // The same lecture twice: every item id repeats.
        let twice = Pack::new(index(), vec![sample(), sample()]);
        let p = twice.problems();
        assert!(p.iter().any(|m| m == "cvs.L04: the lecture appears twice"), "{p:?}");
        assert!(p.iter().any(|m| m.ends_with("item id is used twice")));

        // A lecture the index doesn't list, and an item under the wrong lecture.
        let mut stray = sample();
        stray.lecture = "cvs.L09".into();
        let p = Pack::new(index(), vec![stray]).problems();
        assert!(p.iter().any(|m| m == "cvs.L09: not listed in the index"), "{p:?}");
        assert!(p.iter().any(|m| m.contains("isn't under its lecture cvs.L09")), "{p:?}");

        // A newer lecture schema this build doesn't read.
        let mut future = sample();
        future.schema = "steps.content/v9".into();
        assert!(Pack::new(index(), vec![future]).problems().iter().any(|m| m.contains("v9")));

        // A duplicate index entry.
        let mut doubled = index();
        doubled.lectures.push(doubled.lectures[0].clone());
        assert!(Pack::new(doubled, vec![sample()]).problems().iter().any(|m| m.contains("listed twice")));
    }

    #[test]
    fn lessons_must_point_at_items_and_topics_that_ship() {
        let text = include_str!("../../../fixtures/content/sample.yaml");
        let lecture: Lecture = serde_yaml_ng::from_str(text).unwrap();
        let first = lecture.items[0].id.clone();
        let lesson = |practice: &str, id: &str| -> LessonFile {
            serde_json::from_value(serde_json::json!({
                "schema": crate::lesson::SCHEMA,
                "lecture": "cvs.L04",
                "lessons": [{
                    "id": id, "title": "t", "hook": "h", "minutes": 5, "pages": [1, 2],
                    "steps": [], "recap": [], "practice": [practice], "status": "unverified"
                }]
            }))
            .unwrap()
        };
        let ok =
            Pack::new(index(), vec![lecture.clone()]).with_lessons(vec![lesson(&first, "cvs.L04.lesson1")]);
        assert_eq!(ok.problems(), Vec::<String>::new());
        let gone = Pack::new(index(), vec![lecture.clone()])
            .with_lessons(vec![lesson("cvs.L04.999", "cvs.L04.lesson1")]);
        assert!(gone.problems().iter().any(|m| m.contains("item cvs.L04.999 isn't in lecture cvs.L04")));
        let twice = Pack::new(index(), vec![lecture])
            .with_lessons(vec![lesson(&first, "cvs.L04.lesson1"), lesson(&first, "cvs.L04.lesson1")]);
        let p = twice.problems();
        assert!(p.iter().any(|m| m.contains("lesson id is used twice")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("the file appears twice")), "{p:?}");
    }

    /// A sealed file for `lecture` holding questions `ids`, sealed with a
    /// key made here.
    fn sealed(lecture: &str, fill: u8, ids: &[&str]) -> crate::sealed::SealedLecture {
        use crate::sealed::{Key, Opened, SealedLecture, seal_item};
        let key = Key::from_bytes([fill; 32]);
        let mcq = sample().items.into_iter().find(|i| i.body.kind() == "mcq").unwrap();
        let items = ids
            .iter()
            .map(|id| {
                seal_item(
                    &key,
                    &Opened::with_image(crate::content::Item { id: id.to_string(), ..mcq.clone() }, None),
                )
            })
            .collect();
        SealedLecture::new(lecture, &key, items)
    }

    #[test]
    fn sealed_files_and_sources_travel_as_they_are() {
        let sources: crate::content::Sources =
            serde_yaml_ng::from_str(include_str!("../../../content/sources.yaml")).unwrap();
        let pack = Pack::new(index(), vec![sample()])
            .with_sealed(vec![
                sealed("cvs.L05", 7, &["cvs.L05.vq0002"]),
                sealed("cvs.L04", 7, &["cvs.L04.vq0001"]),
            ])
            .with_sources(sources.sources.clone());
        // In the index's order.
        assert_eq!(
            pack.sealed.iter().map(|f| f.lecture.as_str()).collect::<Vec<_>>(),
            ["cvs.L04", "cvs.L05"]
        );
        assert_eq!(pack.sealed_count(), 2);
        assert_eq!(pack.problems(), Vec::<String>::new());
        let back = Pack::decode(&pack.encode()).unwrap();
        assert_eq!(back, pack);
        assert_eq!(back.sources, sources.sources);
        // A pack without them reads as before, and writes neither field.
        let plain = Pack::new(index(), vec![sample()]);
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("\"sealed\"") && !json.contains("\"sources\""), "{json}");
        assert!(Pack::decode(&plain.encode()).unwrap().sealed.is_empty());
    }

    #[test]
    fn sealed_ids_lectures_and_keys_are_checked() {
        let mut stray = sealed("cvs.L09", 7, &["cvs.L09.vq0001"]);
        stray.schema = "steps.sealed/v0".into();
        let pack = Pack::new(index(), vec![sample()]).with_sealed(vec![
            sealed("cvs.L04", 7, &["cvs.L04.vq0001", "cvs.L04.001", "cvs.L05.vq0002"]),
            sealed("cvs.L04", 7, &["cvs.L04.vq0003"]),
            sealed("cvs.L05", 8, &["cvs.L05.vq0001"]),
            stray,
        ]);
        let p = pack.problems();
        let key = |fill: u8| crate::sealed::Key::from_bytes([fill; 32]).id();
        let mut keys = [key(7), key(8)];
        keys.sort();
        for want in [
            "cvs.L04.001: sealed id isn't cvs.L04.vqNNNN".to_string(),
            "cvs.L04.001: item id is used twice".into(),
            "cvs.L05.vq0002: sealed id isn't cvs.L04.vqNNNN".into(),
            "cvs.L04 sealed: the file appears twice".into(),
            "cvs.L09 sealed: the lecture isn't listed in the index".into(),
            "cvs.L09 sealed: schema is \"steps.sealed/v0\", expected \"steps.sealed/v1\"".into(),
            format!("sealed: files were sealed with different keys ({}, {})", keys[0], keys[1]),
        ] {
            assert!(p.contains(&want), "{want}: {p:#?}");
        }
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(Pack::decode(b"PK\x03\x04"), Err(PackError::Magic)));
        let mut bad = MAGIC.to_vec();
        bad.extend([1, 2, 3]);
        assert!(matches!(Pack::decode(&bad), Err(PackError::Inflate(_))));
    }

    #[test]
    fn the_look_travels_with_the_pack() {
        let look: BTreeMap<String, String> =
            [("src/style.css".to_string(), ":root { --ink: #222; }".to_string())].into();
        let pack = Pack::new(index(), vec![sample()]).with_look(look.clone());
        assert_eq!(Pack::decode(&pack.encode()).unwrap().look, look);
    }

    #[test]
    fn index_lesson_and_topic_problems_are_named() {
        let mut idx = index();
        idx.schema = "steps.content/v0".into();
        idx.lectures[1].id = "CVS-5".into();
        let lesson = |lecture: &str, id: &str, topic: &str| -> LessonFile {
            serde_json::from_value(serde_json::json!({
                "schema": "steps.lesson/v0",
                "lecture": lecture,
                "lessons": [{
                    "id": id, "title": "الـ tamponade", "hook": "h", "minutes": 5, "pages": [317, 317],
                    "topics": [topic],
                    "steps": [{ "kind": "try", "items": ["cvs.L04.042"] }],
                    "recap": [], "status": "unverified"
                }]
            }))
            .unwrap()
        };
        let pack = Pack::new(idx, vec![sample()]).with_lessons(vec![
            lesson("cvs.L04", "cvs.L05.lesson1", "murmurs"),
            lesson("cvs.L07", "cvs.L07.lesson1", "shock"),
        ]);
        // Lessons follow the index order, then the lecture id.
        assert_eq!(
            pack.lessons.iter().map(|l| l.lecture.as_str()).collect::<Vec<_>>(),
            ["cvs.L04", "cvs.L07"]
        );
        let p = pack.problems();
        for want in [
            "index: schema is \"steps.content/v0\", expected \"steps.content/v1\"",
            "index: lecture id \"CVS-5\" must look like cvs.L04",
            "cvs.L04 lessons: schema is \"steps.lesson/v0\", expected \"steps.lesson/v1\"",
            "cvs.L07 lessons: the lecture isn't listed in the index",
            "cvs.L05.lesson1: lesson id isn't under its lecture cvs.L04",
            "cvs.L05.lesson1: item cvs.L04.042 isn't in lecture cvs.L04",
            "cvs.L05.lesson1: topic murmurs isn't in lecture cvs.L04",
        ] {
            assert!(p.iter().any(|m| m == want), "{want}: {p:#?}");
        }
        // A lecture with no items in the pack: its lessons' references aren't checked.
        assert!(!p.iter().any(|m| m.starts_with("cvs.L07.lesson1: item")), "{p:#?}");
    }

    #[test]
    fn program_problems_are_prefixed() {
        let program: Program =
            serde_yaml_ng::from_str(include_str!("../../../content/program.yaml")).unwrap();
        assert_eq!(
            Pack::new(index(), vec![sample()]).with_program(program.clone()).problems(),
            Vec::<String>::new()
        );
        // A lecture whose course the program doesn't have.
        let mut idx = index();
        idx.lectures[1].id = "zz.L01".into();
        let p = Pack::new(idx, vec![sample()]).with_program(program).problems();
        assert_eq!(p, ["program: lecture zz.L01 has no course zz in content/program.yaml"]);
    }
}
