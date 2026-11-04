//! The content ids Mohamed's records point at, and what changed about them
//! between two versions of the content (`docs/CONTENT-IDS.md`).
//!
//! Review events, item states and the planner are stored against item ids
//! (`cvs.L04.001`); lesson progress against lesson ids (`cvs.L04.lesson1`);
//! the planner and the course switcher against lecture, topic and course
//! ids. An id that disappears, or an item id that starts naming a
//! different kind of question, silently cuts his history loose. So every
//! content change is compared with the last version (`steps-tools ids
//! check`):
//!
//! - an id added is fine;
//! - an id removed is **breaking**, unless `content/ids-retired.txt` names it;
//! - an item whose `kind` changed is **breaking** (repurposed);
//! - an id that moved (a lesson to another lecture, a lecture to another
//!   course) is reported but allowed.
//!
//! Pure: callers build the two packs (from git, the working tree or a
//! published pack file).

use std::collections::{BTreeMap, BTreeSet};

use crate::pack::Pack;

/// Every persistent id → what it is attached to (its parent, or for an
/// item its kind). Keys read like `item cvs.L04.001`.
pub type Registry = BTreeMap<String, String>;

pub fn registry(pack: &Pack) -> Registry {
    let mut ids = Registry::new();
    let program = pack.program_or_fallback();
    for c in &program.courses {
        ids.insert(format!("course {}", c.id), "program".into());
    }
    for e in &pack.index.lectures {
        let course =
            program.course_for(&e.id).map_or_else(|| "no course".to_owned(), |c| format!("course {}", c.id));
        ids.insert(format!("lecture {}", e.id), course);
    }
    for lec in &pack.lectures {
        for t in &lec.topics {
            ids.insert(format!("topic {}#{}", lec.lecture, t.id), format!("lecture {}", lec.lecture));
        }
        for item in &lec.items {
            let kind = serde_json::to_value(&item.body)
                .ok()
                .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_owned))
                .unwrap_or_default();
            ids.insert(format!("item {}", item.id), format!("kind {kind}"));
        }
    }
    for file in &pack.lessons {
        for l in &file.lessons {
            ids.insert(format!("lesson {}", l.id), format!("lecture {}", file.lecture));
        }
    }
    ids
}

/// What changed between two registries.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub added: Vec<String>,
    /// Gone, and not retired: breaking.
    pub removed: Vec<(String, String)>,
    /// Gone, and listed in `content/ids-retired.txt`.
    pub retired: Vec<String>,
    /// Same item id, a different kind: breaking.
    pub repurposed: Vec<(String, String, String)>,
    /// Same id under another parent: allowed.
    pub moved: Vec<(String, String, String)>,
}

impl Report {
    pub fn breaking(&self) -> bool {
        !self.removed.is_empty() || !self.repurposed.is_empty()
    }

    /// One line per finding, breaking ones first.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (id, was) in &self.removed {
            out.push(format!("BREAKING removed: {id} ({was}); records that point at it lose it"));
        }
        for (id, old, new) in &self.repurposed {
            out.push(format!(
                "BREAKING repurposed: {id} was {old}, now {new}; its review history would mismatch"
            ));
        }
        for (id, old, new) in &self.moved {
            out.push(format!("moved: {id} from {old} to {new}"));
        }
        for id in &self.retired {
            out.push(format!("retired: {id} (content/ids-retired.txt)"));
        }
        if !self.added.is_empty() {
            out.push(format!("added: {} id(s)", self.added.len()));
        }
        out
    }
}

/// The ids in `content/ids-retired.txt`: one per line; a line starting
/// with `#` is a comment, and so is anything after ` #`.
pub fn parse_retired(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(|l| l.split(" #").next().unwrap_or_default().trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

pub fn compare(base: &Registry, head: &Registry, retired: &BTreeSet<String>) -> Report {
    let mut r = Report::default();
    for (id, was) in base {
        match head.get(id) {
            None if retired.contains(id) => r.retired.push(id.clone()),
            None => r.removed.push((id.clone(), was.clone())),
            Some(now) if now != was => {
                if id.starts_with("item ") {
                    r.repurposed.push((id.clone(), was.clone(), now.clone()));
                } else {
                    r.moved.push((id.clone(), was.clone(), now.clone()));
                }
            }
            Some(_) => {}
        }
    }
    r.added = head.keys().filter(|id| !base.contains_key(*id)).cloned().collect();
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Index, Lecture};

    fn lecture() -> Lecture {
        serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap()
    }

    fn index() -> Index {
        serde_yaml_ng::from_str(
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n",
        )
        .unwrap()
    }

    #[test]
    fn safe_changes_pass_and_breaking_ones_are_named() {
        let none = BTreeSet::new();
        let base_lec = lecture();
        let base = registry(&Pack::new(index(), vec![base_lec.clone()]));
        assert!(base.contains_key("lecture cvs.L04") && base.contains_key("course cvs"));
        let first = base_lec.items[0].id.clone();
        assert!(base.contains_key(&format!("item {first}")));

        // A text correction changes no id.
        let mut fixed = base_lec.clone();
        fixed.title.push_str(" (fixed)");
        assert_eq!(compare(&base, &registry(&Pack::new(index(), vec![fixed])), &none), Report::default());

        // A new item is an addition.
        let mut more = base_lec.clone();
        let mut extra = more.items[0].clone();
        extra.id = "cvs.L04.900".into();
        more.items.push(extra);
        let r = compare(&base, &registry(&Pack::new(index(), vec![more])), &none);
        assert!(!r.breaking() && r.added == ["item cvs.L04.900"]);

        // Renumbering an item removes the old id...
        let mut renumbered = base_lec.clone();
        renumbered.items[0].id = "cvs.L04.901".into();
        let head = registry(&Pack::new(index(), vec![renumbered]));
        let r = compare(&base, &head, &none);
        assert!(r.breaking() && r.removed[0].0 == format!("item {first}"), "{r:?}");
        // ...unless it was retired on purpose.
        let retired = parse_retired(&format!("# merged\nitem {first}  # into cvs.L04.901\n"));
        assert!(!compare(&base, &head, &retired).breaking());

        // The same id for another kind of question.
        let mut head = base.clone();
        head.insert(format!("item {first}"), "kind something_else".into());
        let r = compare(&base, &head, &none);
        assert!(r.breaking() && r.lines()[0].starts_with("BREAKING repurposed"));

        // A lecture under another course: reported, allowed.
        let mut head = base.clone();
        head.insert("lecture cvs.L04".into(), "course xx".into());
        let r = compare(&base, &head, &none);
        assert!(!r.breaking() && r.moved.len() == 1);
    }
}
