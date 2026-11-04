//! First Aid pages Mohamed marked read, and whether new items wait for
//! them (feature list A2). Synced settings, last-writer-wins:
//!
//! - `pages.<course>`: that course's read pages as ranges (`[[290, 294]]`),
//!   one key per course so two devices reading two courses never
//!   overwrite each other;
//! - `study.page_gate`: `true`/`false` once he sets the switch. Until he
//!   does, the gate is on as soon as any page is marked read, so a fresh
//!   install still starts new cards from page one.

use std::collections::BTreeMap;

use steps_core::pages::PagesRead;

use super::Store;
use crate::content::Library;

pub const KEY: &str = "pages.";
pub const GATE_KEY: &str = "study.page_gate";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pages {
    by_course: BTreeMap<String, PagesRead>,
    all: PagesRead,
    gate: Option<bool>,
}

impl Pages {
    pub fn load(store: &Store) -> Self {
        let by_course: BTreeMap<String, PagesRead> = store
            .settings_with_prefix::<PagesRead>(KEY)
            .into_iter()
            .filter_map(|(k, v)| Some((k.strip_prefix(KEY)?.to_string(), v)))
            .collect();
        Self::new(by_course, store.setting(GATE_KEY))
    }

    fn new(by_course: BTreeMap<String, PagesRead>, gate: Option<bool>) -> Self {
        let mut all = PagesRead::default();
        for p in by_course.values() {
            all.extend(p);
        }
        Self { by_course, all, gate }
    }

    /// Every page read, all courses.
    pub fn all(&self) -> &PagesRead {
        &self.all
    }

    /// The switch in Settings: his choice, else on once a page is read.
    pub fn gate_on(&self) -> bool {
        self.gate.unwrap_or(!self.all.is_empty())
    }

    /// What the composer gets: the pages when the gate is on.
    pub fn gate(&self) -> Option<&PagesRead> {
        self.gate_on().then_some(&self.all)
    }

    pub fn with_gate(&self, on: bool) -> Self {
        Self { gate: Some(on), ..self.clone() }
    }

    /// Marks `first..=last` read or unread. Each lecture the range touches
    /// takes its own pages into its course; pages outside every lecture are
    /// ignored. Returns the courses that changed, with their new pages.
    pub fn set(&self, lib: &Library, first: u32, last: u32, read: bool) -> Vec<(String, PagesRead)> {
        let (first, last) = (first.min(last), first.max(last));
        let mut changed: BTreeMap<String, PagesRead> = BTreeMap::new();
        for e in &lib.index.lectures {
            let (lo, hi) = (first.max(e.pages[0]), last.min(e.pages[1]));
            if lo > hi {
                continue;
            }
            let course = steps_core::program::course_of(&e.id).to_string();
            let pages = changed
                .entry(course.clone())
                .or_insert_with(|| self.by_course.get(&course).cloned().unwrap_or_default());
            if read {
                pages.mark(lo, hi);
            } else {
                pages.unmark(lo, hi);
            }
        }
        changed.retain(|c, p| self.by_course.get(c).map_or(!p.is_empty(), |old| old != p));
        changed.into_iter().collect()
    }

    pub fn with_course(&self, course: &str, pages: PagesRead) -> Self {
        let mut by_course = self.by_course.clone();
        by_course.insert(course.to_string(), pages);
        Self::new(by_course, self.gate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_follows_the_switch_else_any_page_read() {
        let none = Pages::default();
        assert!(!none.gate_on() && none.gate().is_none());
        let read = none.with_course("cvs", PagesRead::from(vec![[290, 291]]));
        assert!(read.gate_on());
        assert_eq!(read.gate().unwrap().ranges(), &[[290, 291]]);
        assert!(read.with_gate(false).gate().is_none());
        assert!(none.with_gate(true).gate().is_some_and(PagesRead::is_empty));
    }

    #[test]
    fn courses_union_into_all() {
        let p = Pages::default()
            .with_course("cvs", PagesRead::from(vec![[290, 291]]))
            .with_course("bio", PagesRead::from(vec![[33, 34]]));
        assert_eq!(p.all().ranges(), &[[33, 34], [290, 291]]);
    }

    #[test]
    fn a_range_splits_by_lecture_into_courses() {
        let lib = Library::get();
        let mut lectures = lib.index.lectures.iter();
        let a = lectures.next().unwrap();
        let next_course = lectures
            .find(|e| steps_core::program::course_of(&e.id) != steps_core::program::course_of(&a.id))
            .unwrap();
        let p = Pages::default();
        let changed = p.set(lib, a.pages[0], a.pages[0], true);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, steps_core::program::course_of(&a.id));
        assert!(changed[0].1.contains(a.pages[0]));
        // Unmarking a page never read changes nothing.
        assert!(p.set(lib, a.pages[0], a.pages[0], false).is_empty());
        // A range across two courses writes both.
        let both = p.set(lib, a.pages[1], next_course.pages[0], true);
        assert!(both.len() >= 2, "{both:?}");
        // Unmarking takes the page back out.
        let p = p.with_course(&changed[0].0, changed[0].1.clone());
        let back = p.set(lib, a.pages[0], a.pages[0], false);
        assert_eq!(back.len(), 1);
        assert!(back[0].1.is_empty());
    }
}
