//! First Aid's disciplines and organ systems as courses
//! (`content/program.yaml`): each course's status, and the map
//! (More › المواد).
//!
//! A course's status starts as the program says and Mohamed changes it on
//! the map. Each course is its own synced setting, `course.<id>`, so two
//! devices changing two courses never overwrite each other.

pub mod focus;
mod map;

use std::collections::BTreeMap;

use dioxus::prelude::*;
use steps_core::program::{Course, CourseStatus};

pub use focus::{CourseChip, Picker};
pub use map::Map;

use crate::content::Library;
use crate::look::{Css, css};

pub(crate) const CSS: Css = css!("src/courses/courses.css");

const KEY: &str = "course.";

/// The page id of the map on the lecture page stack (`lecture::open`).
pub const MAP: &str = "courses";

/// `STEPS_OPEN_LECTURE=courses` opens the map at launch (previews).
pub fn open_map(ui: crate::Ui) {
    crate::lecture::open(ui, MAP);
}

/// Statuses Mohamed set, by course id.
static STATUSES: GlobalSignal<BTreeMap<String, CourseStatus>> = Signal::global(load);

fn load() -> BTreeMap<String, CourseStatus> {
    crate::store::shared()
        .settings_with_prefix::<String>(KEY)
        .into_iter()
        .filter_map(|(k, v)| Some((k.strip_prefix(KEY)?.to_string(), CourseStatus::from_key(&v)?)))
        .collect()
}

/// Re-reads the statuses (a sync or restore changed the store).
pub fn reload() {
    focus::reload();
    let fresh = load();
    if *STATUSES.peek() != fresh {
        *STATUSES.write() = fresh;
    }
}

/// A course's status: Mohamed's choice, else the program's.
pub fn status(course: &Course) -> CourseStatus {
    STATUSES.read().get(&course.id).copied().unwrap_or(course.status)
}

pub fn set_status(id: &str, status: CourseStatus) {
    if crate::store::save_setting(&format!("{KEY}{id}"), &status.key()) {
        STATUSES.write().insert(id.to_string(), status);
    }
}

/// What the map shows for one course.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub course: Course,
    pub status: CourseStatus,
    /// Lectures in the index, and how many are written.
    pub lectures: usize,
    pub written: usize,
    pub items: usize,
    pub lessons: usize,
    /// First Aid's printed pages: the first and last of its lectures, and
    /// how many pages they cover.
    pub pages: Option<[u32; 2]>,
    pub page_count: u32,
}

/// Every course of the program, with `status` (usually [`status`]).
pub fn summaries(lib: &Library, status: impl Fn(&Course) -> CourseStatus) -> Vec<Summary> {
    lib.program
        .courses
        .iter()
        .map(|c| {
            let entries: Vec<_> =
                lib.index.lectures.iter().filter(|e| steps_core::program::course_of(&e.id) == c.id).collect();
            let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
            let pages = entries.iter().map(|e| e.pages).reduce(|a, b| [a[0].min(b[0]), a[1].max(b[1])]);
            let page_count = entries.iter().map(|e| e.pages[1].saturating_sub(e.pages[0]) + 1).sum();
            Summary {
                pages,
                page_count,
                status: status(c),
                lectures: ids.len(),
                written: ids.iter().filter(|id| lib.lectures.contains_key(**id)).count(),
                items: ids.iter().filter_map(|id| lib.lectures.get(*id)).map(|l| l.items.len()).sum(),
                lessons: ids.iter().filter_map(|id| lib.lessons.get(*id)).map(|f| f.lessons.len()).sum(),
                course: c.clone(),
            }
        })
        .collect()
}

/// Cards started per course id, from the review overview's lectures.
pub fn started(lectures: &[crate::store::LectureProgress]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for l in lectures {
        *out.entry(steps_core::program::course_of(&l.id).to_string()).or_default() += l.seen;
    }
    out
}

/// A whole percent, 0 when there is nothing to count.
pub fn percent(part: usize, all: usize) -> u32 {
    (part * 100).checked_div(all).unwrap_or(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percent_rounds_down_and_survives_empty_courses() {
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(5, 5), 100);
    }

    #[test]
    fn the_built_in_library_has_the_whole_program() {
        let lib = Library::get();
        let all = summaries(lib, |c| c.status);
        assert_eq!(all.len(), 16);
        let cvs = all.iter().find(|s| s.course.id == "cvs").unwrap();
        assert_eq!(cvs.lectures, 5);
        assert!(cvs.items > 0 && cvs.lessons > 0);
        // Cardiovascular is First Aid's printed pages 284–328.
        assert_eq!(cvs.pages, Some([284, 328]));
        assert_eq!(cvs.page_count, 45);
        // Every lecture belongs to a course on the map.
        assert_eq!(all.iter().map(|s| s.lectures).sum::<usize>(), lib.index.lectures.len());
    }
}
