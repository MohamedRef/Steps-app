//! Lesson files: `content/lessons/<chapter>/LNN.yaml` (`steps.lesson/v1`).
//! `steps-tools validate` checks them against their lecture
//! ([`steps_core::lesson::validate`]); `packs` puts them in the content pack.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use steps_core::content::Lecture;
use steps_core::glossary::Glossary;
use steps_core::lesson::{self, LessonContext, LessonFile};

/// Every lesson file, sorted by path: `(path, parsed, raw text)`.
pub fn load(root: &Path) -> Result<Vec<(PathBuf, LessonFile, String)>, Vec<String>> {
    let dir = root.join("content/lessons");
    let mut paths = Vec::new();
    for chapter in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        for f in std::fs::read_dir(chapter.path()).into_iter().flatten().flatten() {
            if f.path().extension().is_some_and(|e| e == "yaml") {
                paths.push(f.path());
            }
        }
    }
    paths.sort();
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for p in paths {
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{}: {e}", p.display()));
                continue;
            }
        };
        match serde_yaml_ng::from_str::<LessonFile>(&text) {
            Ok(f) => out.push((p, f, text)),
            Err(e) => problems.push(format!("{}: {e}", crate::rel(root, &p))),
        }
    }
    if problems.is_empty() { Ok(out) } else { Err(problems) }
}

/// Every word a lesson step shows that looks like an abbreviation but has
/// no entry in `content/glossary.yaml`, so the step's "اختصارات" chip can
/// explain all of them. A capitalised word that isn't one goes in the
/// glossary's `plain:` list.
pub fn unknown_abbreviations(file: &LessonFile, g: &Glossary) -> Vec<String> {
    let g = g.finder();
    let mut out = Vec::new();
    for l in &file.lessons {
        // The recap shows too (#184).
        let mut unknown: Vec<String> = Vec::new();
        for t in &l.recap {
            for w in g.unknown_in(t) {
                if !unknown.contains(&w) {
                    unknown.push(w);
                }
            }
        }
        if !unknown.is_empty() {
            out.push(format!(
                "{} recap: {} not in content/glossary.yaml (add an entry, or list it under plain: if it isn't an abbreviation)",
                l.id,
                unknown.iter().map(|w| format!("`{w}`")).collect::<Vec<_>>().join(", ")
            ));
        }
        for (k, step) in l.steps.iter().enumerate() {
            let mut unknown: Vec<String> = Vec::new();
            for t in step.shown_text() {
                for w in g.unknown_in(&t) {
                    if !unknown.contains(&w) {
                        unknown.push(w);
                    }
                }
            }
            if !unknown.is_empty() {
                out.push(format!(
                    "{} step {}: {} not in content/glossary.yaml (add an entry, or list it under plain: if it isn't an abbreviation)",
                    l.id,
                    k + 1,
                    unknown.iter().map(|w| format!("`{w}`")).collect::<Vec<_>>().join(", ")
                ));
            }
        }
    }
    out
}

/// Checks every lesson file. Returns `(files, lessons)` and pushes problems.
pub fn validate_all(
    root: &Path,
    approved: &BTreeSet<String>,
    glossary: Option<&Glossary>,
    problems: &mut Vec<String>,
) -> (usize, usize) {
    let files = match load(root) {
        Ok(f) => f,
        Err(p) => {
            problems.extend(p);
            return (0, 0);
        }
    };
    let content = root.join("content");
    let mut lessons = 0;
    for (path, file, text) in &files {
        let at = crate::rel(root, path);
        // content/lessons/cvs/L01.yaml teaches cvs.L04.
        let chapter = path.parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string());
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
        let named = format!("{}.{}", chapter.unwrap_or_default(), stem.unwrap_or_default());
        if file.lecture != named {
            problems.push(format!("{at}: lecture is {}, the file name says {named}", file.lecture));
        }
        let lecture_path = crate::lecture_path(&content, &file.lecture);
        let lecture: Lecture = match crate::read_yaml(&lecture_path) {
            Ok(l) => l,
            Err(_) => {
                problems.push(format!("{at}: its lecture {} has no content file yet", file.lecture));
                continue;
            }
        };
        let cx =
            LessonContext { lecture: &lecture, approved: Some(approved), raw_step_keys: raw_step_keys(text) };
        for p in lesson::validate(file, &cx) {
            problems.push(format!("{at}: {p}"));
        }
        if let Some(g) = glossary {
            problems.extend(unknown_abbreviations(file, g).into_iter().map(|p| format!("{at}: {p}")));
        }
        lessons += file.lessons.len();
    }
    problems.extend(remind_targets(&content, &files));
    let mut all: Vec<LessonFile> = files.into_iter().map(|(_, f, _)| f).collect();
    problems.extend(
        lesson::embed_images(&mut all, read_image(root)).into_iter().map(|p| format!("lessons: {p}")),
    );
    (all.len(), lessons)
}

/// Every step's `remind` entry points at something that holds a hook: an
/// enumerate item (any course) with a `hook`, or a mnemonic step of a lesson
/// with that `word`. A step never reminds of itself.
// Adapted from batna-app main: crates/tools/src/lessons.rs (remind_targets).
fn remind_targets(content: &Path, files: &[(PathBuf, LessonFile, String)]) -> Vec<String> {
    use std::collections::BTreeMap;
    use steps_core::content::Body;
    use steps_core::lesson::{Reminder, StepBody};
    let lessons: BTreeMap<&str, &lesson::Lesson> =
        files.iter().flat_map(|(_, f, _)| f.lessons.iter().map(|l| (l.id.as_str(), l))).collect();
    let mut lectures: BTreeMap<String, Option<Lecture>> = BTreeMap::new();
    let mut out = Vec::new();
    for (_, f, _) in files {
        for l in &f.lessons {
            for (k, step) in l.steps.iter().enumerate() {
                let at = format!("{} step {}", l.id, k + 1);
                for r in &step.remind {
                    let problem = match Reminder::parse(r) {
                        // The step itself checks the shape (lesson::validate).
                        None => None,
                        Some(Reminder::Item(id)) => {
                            let lec = id.rsplit_once('.').map_or(id, |(l, _)| l).to_string();
                            let lecture = lectures.entry(lec.clone()).or_insert_with(|| {
                                crate::read_yaml(&crate::lecture_path(content, &lec)).ok()
                            });
                            match lecture.as_ref().and_then(|x| x.items.iter().find(|i| i.id == id)) {
                                None => Some(format!("remind `{r}`: no such item")),
                                Some(i) => match &i.body {
                                    Body::Enumerate { hook: Some(h), .. } if !h.trim().is_empty() => None,
                                    _ => Some(format!("remind `{r}`: not an enumerate item with a hook")),
                                },
                            }
                        }
                        Some(Reminder::Mnemonic { lesson, word }) => {
                            let found = lessons.get(lesson).and_then(|o| {
                                o.steps.iter().position(
                                    |s| matches!(&s.body, StepBody::Mnemonic { word: w, .. } if w == word),
                                )
                            });
                            match found {
                                None => Some(format!("remind `{r}`: no mnemonic `{word}` in {lesson}")),
                                Some(j) if lesson == l.id && j == k => {
                                    Some(format!("remind `{r}`: a step can't remind of itself"))
                                }
                                Some(_) => None,
                            }
                        }
                    };
                    out.extend(problem.map(|p| format!("lessons: {at}: {p}")));
                }
            }
        }
    }
    out
}

/// Reads `content/lessons/img/<path>`, for [`lesson::embed_images`].
pub fn read_image(root: &Path) -> impl FnMut(&str) -> Option<Vec<u8>> + '_ {
    move |path| std::fs::read(root.join("content/lessons/img").join(path)).ok()
}

/// The keys written on each step, by `(lesson, step)` index.
fn raw_step_keys(text: &str) -> Vec<((usize, usize), Vec<String>)> {
    let mut out = Vec::new();
    let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return out;
    };
    let seq = |v: &serde_yaml_ng::Value, k: &str| {
        v.get(k).and_then(|s| s.as_sequence()).cloned().unwrap_or_default()
    };
    for (li, lesson) in seq(&doc, "lessons").iter().enumerate() {
        for (si, step) in seq(lesson, "steps").iter().enumerate() {
            let keys = step
                .as_mapping()
                .map(|m| m.keys().filter_map(|k| k.as_str().map(String::from)).collect())
                .unwrap_or_default();
            out.push(((li, si), keys));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_must_point_at_a_real_hook() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = load(&root).expect("lessons load");
        let (_, f, _) = files.iter_mut().find(|(_, f, _)| f.lecture == "cvs.L05").expect("cvs.L05 lessons");
        let l = &mut f.lessons[0];
        let id = l.id.clone();
        let k =
            l.steps.iter().position(|s| s.remind.iter().any(|r| r == "cvs.L05.001")).expect("the DVT step");
        l.steps[k].remind = vec![
            "cvs.L05.001".into(),           // enumerate with a hook: fine
            "cvs.L04.lesson1:TTTTT".into(), // mnemonic in another lesson: fine
            "cvs.L05.999".into(),           // no such item
            "cvs.L05.002".into(),           // a card, not an enumerate
            "cvs.L04.lesson1:NOPE".into(),  // no such mnemonic
        ];
        let p = remind_targets(&root.join("content"), &files);
        let at = format!("lessons: {id} step {}", k + 1);
        assert_eq!(
            p,
            [
                format!("{at}: remind `cvs.L05.999`: no such item"),
                format!("{at}: remind `cvs.L05.002`: not an enumerate item with a hook"),
                format!("{at}: remind `cvs.L04.lesson1:NOPE`: no mnemonic `NOPE` in cvs.L04.lesson1"),
            ]
        );
    }

    #[test]
    fn repo_lessons_are_valid() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let approved = BTreeSet::new();
        let mut problems = Vec::new();
        validate_all(&root, &approved, None, &mut problems);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn typos_in_step_keys_are_caught() {
        let keys = raw_step_keys("lessons:\n  - steps:\n      - { kind: check, anwser: 1 }\n");
        assert_eq!(keys, vec![((0, 0), vec!["kind".to_string(), "anwser".to_string()])]);
    }

    use crate::testkit::{content_tree, write};

    const LESSON: &str = "schema: steps.lesson/v1
lecture: cvs.L04
lessons:
  - id: cvs.L04.lesson1
    title: الـ tamponade
    hook: ليه الضغط بيقع؟
    minutes: 5
    pages: [317, 317]
    steps:
      - { kind: explain, text: الـ JVP بيعلى في الـ tamponade, page: 317, quote: distended neck veins }
      - { kind: reveal, prompt: ليه؟, answer: الـ RA مش بيتملي }
      - kind: check
        question: Which sign?
        options: [Kussmaul sign, Pulsus paradoxus, Osler node]
        answer: 1
        anwser: 1
        why: [no, yes, no]
    recap: [Beck triad, ACS can look the same]
    status: unverified
";

    #[test]
    fn lesson_files_are_checked_against_their_lecture_and_the_glossary() {
        let dir = content_tree();
        let root = dir.path();
        write(root, "content/lessons/cvs/L04.yaml", LESSON);
        // Named after another lecture, and one whose lecture has no content yet.
        write(root, "content/lessons/cvs/L05.yaml", LESSON);
        write(root, "content/lessons/cvs/L09.yaml", &LESSON.replace("cvs.L04", "cvs.L09"));
        write(root, "content/lessons/cvs/notes.txt", "not a lesson");
        let g: Glossary = serde_yaml_ng::from_str(
            "schema: steps.glossary/v1\nentries:\n  - { term: JVP, kind: abbr, text: Jugular venous pressure, status: unverified }\n",
        )
        .unwrap();
        let mut problems = Vec::new();
        let (files, lessons) = validate_all(root, &BTreeSet::new(), Some(&g), &mut problems);
        assert_eq!((files, lessons), (3, 2), "{problems:#?}");
        for want in [
            "content/lessons/cvs/L05.yaml: lecture is cvs.L04, the file name says cvs.L05",
            "content/lessons/cvs/L09.yaml: its lecture cvs.L09 has no content file yet",
            "content/lessons/cvs/L04.yaml: cvs.L04.lesson1 step 3: unknown key `anwser` for a check step",
            "content/lessons/cvs/L04.yaml: cvs.L04.lesson1 step 2: `RA` not in content/glossary.yaml",
            "content/lessons/cvs/L04.yaml: cvs.L04.lesson1 recap: `ACS` not in content/glossary.yaml",
        ] {
            assert!(problems.iter().any(|p| p.starts_with(want)), "{want}: {problems:#?}");
        }
        assert!(!problems.iter().any(|p| p.contains("`JVP`")), "{problems:#?}");
    }

    #[test]
    fn a_lesson_file_that_does_not_parse_stops_the_check() {
        let dir = content_tree();
        let root = dir.path();
        write(root, "content/lessons/cvs/L04.yaml", "schema: [");
        let mut problems = Vec::new();
        assert_eq!(validate_all(root, &BTreeSet::new(), None, &mut problems), (0, 0));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("content/lessons/cvs/L04.yaml: "), "{problems:?}");
        // A directory with a .yaml name can't be read.
        std::fs::remove_file(root.join("content/lessons/cvs/L04.yaml")).unwrap();
        std::fs::create_dir_all(root.join("content/lessons/cvs/L04.yaml")).unwrap();
        assert!(load(root).is_err());
    }
}
