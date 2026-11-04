//! The study content built into the app: build.rs packs it
//! (`steps_core::pack`) and this unpacks it on first use. A newer pack from
//! the update feed replaces it, at start or live once he is between
//! sessions ([`Library::reload`]), with no app install.

use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use steps_core::content::{Index, Lecture};
use steps_core::lesson::{Lesson, LessonFile};
use steps_core::pack::Pack;
use steps_core::program::{Course, Program};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// The per-course glossaries, `content/glossary/<course>.yaml` (build.rs).
pub const GLOSSARY_PARTS: &[&str] = embedded::GLOSSARY_PARTS;

pub struct Library {
    pub index: Index,
    /// Written lectures, by id (`cvs.L04`).
    pub lectures: BTreeMap<String, Lecture>,
    /// Lesson files, by lecture id (`cvs.L04`), in book order.
    pub lessons: BTreeMap<String, LessonFile>,
    /// First Aid's disciplines and systems as courses (`content/program.yaml`).
    pub program: Program,
}

/// The library in use. Each one is leaked: screens hold `&'static` items,
/// and it is replaced at most a few times a session.
static LIB: RwLock<Option<&'static Library>> = RwLock::new(None);
/// Bumped each time the library is replaced, so caches built from it know.
static GENERATION: AtomicU64 = AtomicU64::new(0);

impl Library {
    pub fn get() -> &'static Library {
        if let Some(lib) = *LIB.read().unwrap_or_else(|e| e.into_inner()) {
            return lib;
        }
        let mut lib = LIB.write().unwrap_or_else(|e| e.into_inner());
        match *lib {
            Some(l) => l,
            None => {
                let l: &'static Library = Box::leak(Box::new(Library::load()));
                *lib = Some(l);
                l
            }
        }
    }

    /// Opens the newest pack again (one the update feed just delivered).
    /// Screens built from the old one must be shown again afterwards.
    pub fn reload() {
        let fresh: &'static Library = Box::leak(Box::new(Library::load()));
        *LIB.write().unwrap_or_else(|e| e.into_inner()) = Some(fresh);
        GENERATION.fetch_add(1, Ordering::SeqCst);
    }

    /// Changes each time the library is replaced.
    pub fn generation() -> u64 {
        GENERATION.load(Ordering::SeqCst)
    }

    /// True once the running process has actually opened a library.
    /// A startup timer alone cannot certify a downloaded pack as working.
    pub(crate) fn is_initialized() -> bool {
        LIB.read().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    fn load() -> Library {
        // A newer pack from the update feed wins (checked and decoded
        // there); else the embedded one, which build.rs wrote from types
        // it just parsed, so it can't fail.
        // The web app's pack is a file of its own, fetched before it starts.
        #[cfg(target_arch = "wasm32")]
        let built_in = crate::platform::web::content_pack();
        #[cfg(not(target_arch = "wasm32"))]
        let built_in = embedded::PACK;
        let pack = crate::update::installed_pack()
            .unwrap_or_else(|| Pack::decode(built_in).expect("built-in content pack"));
        let program = pack.program_or_fallback();
        let Pack { index, lectures, lessons, look, .. } = pack;
        crate::look::set(look);
        let lectures = lectures.into_iter().map(|l| (l.lecture.clone(), l));
        let lessons = lessons.into_iter().map(|l| (l.lecture.clone(), l));
        // Screen tests need a lecture still being written ("بتتكتب"); with
        // every lecture written, the last one in the index plays that part.
        #[cfg(test)]
        let (lectures, lessons) = {
            let unwritten = index.lectures.last().map(|e| e.id.clone()).unwrap_or_default();
            let u = unwritten.clone();
            (lectures.filter(move |(id, _)| *id != unwritten), lessons.filter(move |(id, _)| *id != u))
        };
        let lectures = lectures.collect();
        let lessons = lessons.collect();
        Library { index, lectures, lessons, program }
    }

    /// Every lesson in book order, with its lecture id.
    pub fn all_lessons(&self) -> impl Iterator<Item = (&str, &Lesson)> {
        self.index
            .lectures
            .iter()
            .filter_map(|e| self.lessons.get(&e.id))
            .flat_map(|f| f.lessons.iter().map(move |l| (f.lecture.as_str(), l)))
    }

    pub fn lesson(&self, id: &str) -> Option<(&Lecture, &Lesson)> {
        let lecture = id.rsplit_once('.').map(|(l, _)| l)?;
        let lesson = self.lessons.get(lecture)?.lessons.iter().find(|l| l.id == id)?;
        Some((self.lectures.get(lecture)?, lesson))
    }
}

impl Library {
    /// A lecture's title as the index spells it, so every screen agrees.
    pub fn title<'a>(&'a self, id: &'a str) -> &'a str {
        let indexed = self.index.lectures.iter().find(|e| e.id == id).map(|e| e.title.as_str());
        indexed.or_else(|| self.lectures.get(id).map(|l| l.title.as_str())).unwrap_or(id)
    }
}

/// Bumped when the library is replaced while Steps is open; the shell
/// draws its screens again under this key (`update::view::use_content_switch`).
pub static REV: dioxus::prelude::GlobalSignal<u64> = dioxus::prelude::Signal::global(|| 0);

/// Something built from the library once, and again after it is replaced.
pub struct Derived<T: 'static> {
    cell: RwLock<Option<(u64, &'static T)>>,
}

impl<T: Send + Sync + 'static> Derived<T> {
    pub const fn new() -> Self {
        Self { cell: RwLock::new(None) }
    }

    pub fn get(&self, build: impl FnOnce(&'static Library) -> T) -> &'static T {
        let generation = Library::generation();
        if let Some((g, value)) = *self.cell.read().unwrap_or_else(|e| e.into_inner())
            && g == generation
        {
            return value;
        }
        let value: &'static T = Box::leak(Box::new(build(Library::get())));
        *self.cell.write().unwrap_or_else(|e| e.into_inner()) = Some((generation, value));
        value
    }
}

/// `cvs.L03` → `3`: a lecture's number inside its course. Screens show it
/// under the course's name, never as a bare code (Mohamed, 2026-10-29: the
/// course, then the lecture's real title, then its lessons).
pub fn lecture_no(id: &str) -> String {
    let n = steps_core::program::lecture_of(id).rsplit('.').next().unwrap_or(id);
    let n = n.trim_start_matches('L').trim_start_matches('0');
    if n.is_empty() { "0".into() } else { n.into() }
}

impl Library {
    /// The course a lecture, item or lesson id belongs to.
    pub fn course_of(&self, id: &str) -> Option<&Course> {
        self.program.course_for(id)
    }

    /// The course's short Arabic name (`قلب`), else its id.
    pub fn course_short<'a>(&'a self, id: &'a str) -> &'a str {
        self.course_of(id).map_or(steps_core::program::course_of(id), |c| c.short.as_str())
    }

    /// `قلب 3`: a lecture named where its course isn't on screen.
    pub fn lecture_tag(&self, id: &str) -> String {
        format!("{} {}", self.course_short(id), lecture_no(id))
    }

    /// `كبد › Portal hypertension`: a lecture by course and title, for chips
    /// and one-line labels.
    pub fn crumb(&self, id: &str) -> String {
        let lecture = steps_core::program::lecture_of(id);
        format!("{} › {}", self.course_short(lecture), self.title(lecture))
    }

    /// Splits rows into their courses, in the program's order (rows keep
    /// their order inside a course). Rows of a course the program doesn't
    /// list go last, under their prefix.
    pub fn by_course<'a, T>(&'a self, rows: Vec<T>, id: impl Fn(&T) -> &str) -> Vec<CourseRows<'a, T>> {
        let mut out: Vec<CourseRows<'a, T>> = Vec::new();
        for row in rows {
            let course = steps_core::program::course_of(id(&row)).to_string();
            match out.iter_mut().find(|g| g.id == course) {
                Some(g) => g.rows.push(row),
                None => {
                    out.push(CourseRows { course: self.program.course(&course), id: course, rows: vec![row] })
                }
            }
        }
        let pos = |g: &CourseRows<'a, T>| {
            self.program.courses.iter().position(|c| c.id == g.id).unwrap_or(usize::MAX)
        };
        out.sort_by_key(|g| pos(g));
        out
    }
}

/// One course's share of a list ([`Library::by_course`]).
pub struct CourseRows<'a, T> {
    pub id: String,
    pub course: Option<&'a Course>,
    pub rows: Vec<T>,
}

impl<T> CourseRows<'_, T> {
    pub fn short(&self) -> &str {
        self.course.map_or(self.id.as_str(), |c| c.short.as_str())
    }

    pub fn title(&self) -> &str {
        self.course.map_or(self.id.as_str(), |c| c.title.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lectures_are_named_by_course_number_and_title() {
        assert_eq!(lecture_no("cvs.L04"), "4");
        assert_eq!(lecture_no("cvs.L20"), "20");
        assert_eq!(lecture_no("phs.L03.012"), "3");
        assert_eq!(lecture_no("phs.L03.lesson2#4"), "3");
        let lib = Library::get();
        let first = &lib.index.lectures[0];
        assert_eq!(lib.title(&first.id), first.title);
        assert_eq!(lib.lecture_tag("cvs.L03"), "قلب 3");
        assert_eq!(lib.crumb("cvs.L03.004"), format!("قلب › {}", lib.title("cvs.L03")));
    }

    #[test]
    fn rows_group_by_course_in_program_order() {
        let lib = Library::get();
        let groups = lib.by_course(vec!["cvs.L02", "bio.L01", "cvs.L01", "zz.L01"], |s| s);
        let ids: Vec<_> = groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["bio", "cvs", "zz"]);
        assert_eq!(groups[1].rows, ["cvs.L02", "cvs.L01"]);
        assert_eq!(groups[1].short(), "قلب");
        assert_eq!(groups[2].short(), "zz");
    }

    #[test]
    fn every_embedded_file_parses() {
        let lib = Library::get();
        assert!(lib.index.lectures.iter().filter(|l| l.id.starts_with("cvs.")).count() == 5);
        let content = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");
        let written: usize = lib
            .program
            .courses
            .iter()
            .filter_map(|c| std::fs::read_dir(format!("{content}/{}", c.id)).ok())
            .flat_map(|dir| dir.flatten())
            .filter(|f| f.path().extension().is_some_and(|e| e == "yaml"))
            .count();
        assert_eq!(lib.lectures.len(), written, "every lecture file made it into the pack");
    }
}
