//! The MSc's courses (`content/program.yaml`, Mohamed's course sheet of
//! 2026-10-29): each course's status, and the MSc map (More › المقررات).
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
}

/// Every course of the program, with `status` (usually [`status`]).
pub fn summaries(lib: &Library, status: impl Fn(&Course) -> CourseStatus) -> Vec<Summary> {
    lib.program
        .courses
        .iter()
        .map(|c| {
            let ids: Vec<&str> = lib
                .index
                .lectures
                .iter()
                .map(|e| e.id.as_str())
                .filter(|id| steps_core::program::course_of(id) == c.id)
                .collect();
            Summary {
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

/// Credit hours (done, now, all): done = after its exam (maintenance or
/// archived), now = active.
pub fn hours(all: &[Summary]) -> (f32, f32, f32) {
    let sum = |f: &dyn Fn(CourseStatus) -> bool| {
        all.iter().filter(|s| f(s.status)).map(|s| s.course.hours).sum::<f32>()
    };
    (
        sum(&|s| matches!(s, CourseStatus::Maintenance | CourseStatus::Archived)),
        sum(&|s| s == CourseStatus::Active),
        sum(&|_| true),
    )
}

/// `2.5` → `2.5`, `2.0` → `2`.
pub fn fmt_hours(h: f32) -> String {
    // An empty sum is -0.0.
    let h = h + 0.0;
    if h.fract() == 0.0 { format!("{h:.0}") } else { format!("{h:.1}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steps_core::program::Group;

    fn summary(id: &str, hours: f32, status: CourseStatus) -> Summary {
        Summary {
            course: Course {
                id: id.into(),
                title: id.into(),
                short: id.into(),
                hours,
                group: Group::Systems,
                semester: None,
                status,
                source: Default::default(),
            },
            status,
            lectures: 0,
            written: 0,
            items: 0,
            lessons: 0,
        }
    }

    #[test]
    fn hours_split_by_status() {
        let all = [
            summary("a", 2.0, CourseStatus::Maintenance),
            summary("b", 3.0, CourseStatus::Active),
            summary("c", 0.5, CourseStatus::Later),
            summary("d", 1.0, CourseStatus::Archived),
        ];
        assert_eq!(hours(&all), (3.0, 3.0, 6.5));
        assert_eq!(fmt_hours(2.0), "2");
        assert_eq!(fmt_hours(0.5), "0.5");
        assert_eq!(fmt_hours(std::iter::empty::<f32>().sum()), "0");
    }

    #[test]
    fn the_built_in_library_has_the_whole_program() {
        let lib = Library::get();
        let all = summaries(lib, |c| c.status);
        assert_eq!(all.len(), 16);
        let cvs = all.iter().find(|s| s.course.id == "cvs").unwrap();
        assert_eq!(cvs.lectures, 5);
        assert!(cvs.items > 0 && cvs.lessons > 0);
        // Every lecture belongs to a course on the map.
        assert_eq!(all.iter().map(|s| s.lectures).sum::<usize>(), lib.index.lectures.len());
    }
}
