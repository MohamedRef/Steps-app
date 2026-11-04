//! `content/program.yaml`: the whole MSc as courses (Mohamed's course sheet,
//! 2026-10-29). A course owns every lecture whose id starts with its id
//! (`hb.L01` belongs to `hb`), so lecture and item ids never change when a
//! course is added. Packs from before this file have no program; the app then
//! uses [`Program::fallback`], one course per lecture prefix.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::content::Index;

pub const SCHEMA: &str = "steps.program/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub schema: String,
    /// `MSc General Internal Medicine`.
    pub title: String,
    /// In the order of Mohamed's sheet.
    pub courses: Vec<Course>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Course {
    /// The lecture id prefix: `hb`, `hem`, `res`.
    pub id: String,
    /// As the sheet spells it: `Hematology`.
    pub title: String,
    /// A short Egyptian Arabic name for chips: `دم`.
    pub short: String,
    /// Credit hours (0.5 steps).
    pub hours: f32,
    pub group: Group,
    /// The semester he takes it in, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semester: Option<u32>,
    /// The default; Mohamed can change it in the app.
    pub status: CourseStatus,
    /// What `src.page` counts: book pages (one numbering for the course) or
    /// each lecture's slides (numbering starts again per lecture).
    #[serde(default, skip_serializing_if = "Source::is_book")]
    pub source: Source,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    #[default]
    Book,
    Slides,
}

impl Source {
    fn is_book(&self) -> bool {
        *self == Source::Book
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Group {
    /// First Aid Section II: High-Yield General Principles.
    General,
    /// First Aid Section III: High-Yield Organ Systems.
    Systems,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CourseStatus {
    /// Being studied now: on Today, in mixed review and in the planner.
    Active,
    /// After its exam: light reviews only, so it stays fresh for the final.
    Maintenance,
    /// Off Today; still in search, the map and the Anki file.
    Archived,
    /// Not started.
    Later,
}

impl CourseStatus {
    pub const ALL: [CourseStatus; 4] = [Self::Active, Self::Maintenance, Self::Archived, Self::Later];

    pub fn key(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Maintenance => "maintenance",
            Self::Archived => "archived",
            Self::Later => "later",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    /// Its cards come up in review.
    pub fn reviews(self) -> bool {
        matches!(self, Self::Active | Self::Maintenance)
    }
}

/// The course a lecture, item or lesson id belongs to: `hb.L01.003` → `hb`.
pub fn course_of(id: &str) -> &str {
    id.split_once('.').map_or(id, |(c, _)| c)
}

/// The lecture an item, lesson or step id belongs to: `res.L02.019` and
/// `res.L02.lesson1#3` → `res.L02`.
pub fn lecture_of(id: &str) -> &str {
    let id = id.split('#').next().unwrap_or(id);
    match id.match_indices('.').nth(1) {
        Some((i, _)) => &id[..i],
        None => id,
    }
}

impl Program {
    pub fn course(&self, id: &str) -> Option<&Course> {
        self.courses.iter().find(|c| c.id == id)
    }

    /// The course of a lecture, item or lesson id.
    pub fn course_for(&self, id: &str) -> Option<&Course> {
        self.course(course_of(id))
    }

    /// Whether an id's `src.page` is a slide of its own lecture.
    pub fn slides(&self, id: &str) -> bool {
        self.course_for(id).is_some_and(|c| c.source == Source::Slides)
    }

    pub fn total_hours(&self) -> f32 {
        self.courses.iter().map(|c| c.hours).sum()
    }

    /// For a pack without a program: one active course per lecture prefix,
    /// named by its prefix.
    pub fn fallback(index: &Index) -> Program {
        let mut seen = BTreeSet::new();
        let courses = index
            .lectures
            .iter()
            .map(|l| course_of(&l.id))
            .filter(|c| seen.insert(*c))
            .map(|c| Course {
                id: c.to_string(),
                title: c.to_string(),
                short: c.to_string(),
                hours: 0.0,
                group: Group::Systems,
                semester: None,
                status: CourseStatus::Active,
                source: Source::Book,
            })
            .collect();
        Program { schema: SCHEMA.into(), title: String::new(), courses }
    }

    /// Problems with the program itself and with the index's lectures.
    pub fn problems(&self, index: &Index) -> Vec<String> {
        let mut out = Vec::new();
        if self.schema != SCHEMA {
            out.push(format!("schema must be {SCHEMA}"));
        }
        let mut ids = BTreeSet::new();
        for c in &self.courses {
            if !ids.insert(c.id.as_str()) {
                out.push(format!("course {} is listed twice", c.id));
            }
            if c.id.is_empty() || !c.id.bytes().all(|b| b.is_ascii_lowercase()) {
                out.push(format!("course id {:?} must be lowercase letters", c.id));
            }
            if !(c.hours > 0.0 && (c.hours * 2.0).fract() == 0.0) {
                out.push(format!("course {}: hours must be a positive multiple of 0.5", c.id));
            }
            if c.short.trim().is_empty() {
                out.push(format!("course {}: short name is empty", c.id));
            }
        }
        for l in &index.lectures {
            if self.course_for(&l.id).is_none() {
                out.push(format!(
                    "lecture {} has no course {} in content/program.yaml",
                    l.id,
                    course_of(&l.id)
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::IndexEntry;

    fn index(ids: &[&str]) -> Index {
        Index {
            schema: crate::content::SCHEMA.into(),
            lectures: ids
                .iter()
                .map(|id| IndexEntry { id: (*id).into(), title: "t".into(), pages: [1, 2], chunk: 1 })
                .collect(),
        }
    }

    fn course(id: &str, hours: f32) -> Course {
        Course {
            id: id.into(),
            title: id.into(),
            short: id.into(),
            hours,
            group: Group::Systems,
            semester: Some(1),
            status: CourseStatus::Active,
            source: Source::Book,
        }
    }

    #[test]
    fn ids_belong_to_their_prefix() {
        assert_eq!(course_of("hb.L01.003"), "hb");
        assert_eq!(course_of("hem.L02"), "hem");
        assert_eq!(course_of("hb"), "hb");
        let p = Program { schema: SCHEMA.into(), title: "MSc".into(), courses: vec![course("hb", 2.0)] };
        assert_eq!(p.course_for("hb.L07.010").map(|c| c.id.as_str()), Some("hb"));
        assert!(p.course_for("hem.L01").is_none());
    }

    #[test]
    fn lectures_of_items_lessons_and_steps() {
        assert_eq!(lecture_of("res.L02.019"), "res.L02");
        assert_eq!(lecture_of("res.L02.lesson1#3"), "res.L02");
        assert_eq!(lecture_of("hb.L01"), "hb.L01");
        assert_eq!(lecture_of("hb"), "hb");
    }

    #[test]
    fn fallback_has_one_course_per_prefix() {
        let p = Program::fallback(&index(&["hb.L01", "hb.L02", "hem.L01"]));
        let ids: Vec<_> = p.courses.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["hb", "hem"]);
        assert!(p.courses.iter().all(|c| c.status == CourseStatus::Active));
    }

    #[test]
    fn problems_are_reported() {
        let mut p = Program {
            schema: SCHEMA.into(),
            title: "MSc".into(),
            courses: vec![course("hb", 2.0), course("hb", 0.3), course("Res", 3.0)],
        };
        p.courses[2].short = " ".into();
        let probs = p.problems(&index(&["hb.L01", "hem.L01"]));
        assert!(probs.iter().any(|s| s.contains("listed twice")), "{probs:?}");
        assert!(probs.iter().any(|s| s.contains("multiple of 0.5")), "{probs:?}");
        assert!(probs.iter().any(|s| s.contains("lowercase")), "{probs:?}");
        assert!(probs.iter().any(|s| s.contains("short name")), "{probs:?}");
        assert!(probs.iter().any(|s| s.contains("hem.L01 has no course hem")), "{probs:?}");
        assert_eq!(probs.len(), 5, "{probs:?}");
    }

    #[test]
    fn status_keys_round_trip() {
        for s in CourseStatus::ALL {
            assert_eq!(CourseStatus::from_key(s.key()), Some(s));
        }
        assert!(CourseStatus::Maintenance.reviews());
        assert!(!CourseStatus::Later.reviews());
    }

    #[test]
    fn the_repo_program_is_clean() {
        let p: Program = serde_yaml_ng::from_str(include_str!("../../../content/program.yaml")).unwrap();
        let i: Index = serde_yaml_ng::from_str(include_str!("../../../content/index.yaml")).unwrap();
        assert_eq!(p.problems(&i), Vec::<String>::new());
        assert_eq!(p.courses.len(), 16);
        assert!(p.courses.iter().all(|c| c.status == CourseStatus::Active));
        assert_eq!(p.courses.iter().filter(|c| c.group == Group::General).count(), 6);
        assert!(!p.slides("cvs.L04.019"));
    }
}
