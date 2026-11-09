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

use crate::content::{Index, Lecture};
use crate::lesson::LessonFile;
use crate::program::Program;

/// The first bytes of every pack. Bump the digit when the layout changes.
pub const MAGIC: &[u8; 8] = b"STEPSPK1";

/// The content types this build reads. Bump it whenever an item, lesson,
/// step or picture gains a field or a variant that an older app would drop
/// or misread: an older app then refuses the newer pack (and keeps its
/// built-in content) instead of showing it with pieces silently missing.
/// Packs from before the field read as 0. 2: courses can cite slides
/// (`program::Source`). 3: lesson steps can `remind` of a تحشيشة taught
/// elsewhere (`lesson::Reminder`).
pub const SCHEMA: u32 = 3;

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
        Pack { schema: SCHEMA, index, lectures, lessons: Vec::new(), look: BTreeMap::new(), program: None }
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
        if let Some(program) = &self.program {
            out.extend(program.problems(&self.index).into_iter().map(|p| format!("program: {p}")));
        }
        out
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
