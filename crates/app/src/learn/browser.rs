//! Browser fixtures for the production lesson player: the first step of each
//! kind in book order (and every `try` and `table` step), in Steps and in Pages, opened at that step from a saved
//! attempt the way the app reopens a lesson (tests/screens-browser.cjs).

use super::*;
use crate::browser_fixture::{Shell, out_dir, write_page};
use steps_core::lesson::StepBody;
use steps_core::response::{Attempt, Draft, sig};

#[derive(Props, Clone, PartialEq)]
struct Case {
    lesson: String,
    at: usize,
    view: View,
    /// A response already on the step (sort: some cards placed, or checked).
    draft: Option<(Draft, Option<bool>)>,
    /// The step's abbreviations sheet is open.
    #[props(default)]
    abbr: bool,
}

fn root(case: Case) -> Element {
    use_hook(|| {
        let (_, lesson) = Library::get().lesson(&case.lesson).expect("lesson");
        let mut attempt = Attempt { at: case.at, ..Attempt::new(lesson.steps.len(), crate::store::now_ms()) };
        if let Some((draft, answer)) = case.draft.clone() {
            attempt.put(case.at, sig(&lesson.steps[case.at].body), draft);
            attempt.answers[case.at] = answer;
        }
        crate::store::shared().put_local_setting(&player::pos_key(&case.lesson), &attempt).unwrap();
        *OPEN.write() = Some(case.lesson.clone());
        *VIEW.write() = case.view;
        if case.abbr {
            let texts = lesson.steps[case.at].shown_text();
            let terms = crate::abbrsheet::entries(&texts).iter().map(|e| e.term.clone()).collect();
            *crate::abbrsheet::OPEN.write() = Some(terms);
        }
    });
    rsx! {
        Shell { screen: Screen::Learn, focus: true, Learn {} }
    }
}

#[test]
#[ignore = "exports production HTML for the screens browser CI job"]
fn export_screen_fixtures_learn() {
    crate::store::use_memory_store();
    let dir = out_dir("learn");
    let mut kinds = std::collections::BTreeSet::new();
    let mut manifest = Vec::new();
    for (_, lesson) in Library::get().all_lessons() {
        for (at, step) in lesson.steps.iter().enumerate() {
            let kind = step.body.kind();
            // The first step of each kind in both views; after that every
            // `try` and `table` step again in Steps, because the item or table
            // inside each one has its own width (a 3-column table at 320px).
            let first = kinds.insert(kind);
            let views: &[View] = match (first, kind) {
                (true, _) => &[View::Steps, View::Pages],
                (false, "try" | "table") => &[View::Steps],
                _ => continue,
            };
            for &view in views {
                let case = Case { lesson: lesson.id.clone(), at, view, draft: None, abbr: false };
                let mut dom = VirtualDom::new_with_props(root, case);
                dom.rebuild_in_place();
                let view = if view == View::Pages { "pages" } else { "steps" };
                let file = if first {
                    format!("lesson-{kind}-{view}.html")
                } else {
                    format!("lesson-{kind}-{}-{}-{view}.html", lesson.id, at + 1)
                };
                write_page(&dir, &file, &dom);
                manifest.push(serde_json::json!({
                    "file": file, "screen": "learn", "kind": kind, "view": view,
                    "lesson": lesson.id, "step": at + 1, "steps": lesson.steps.len(),
                    "question": step.body.is_question(),
                }));
            }
        }
    }
    sort_states(&dir, &mut manifest);
    case_states(&dir, &mut manifest);
    // One fixture per step kind the written lessons use; the set grows with
    // the content (Steps started with one lecture on 2026-11-04).
    assert!(kinds.len() >= 8, "the lessons use most step kinds: {kinds:?}");
    // Mixed Arabic and English (2026-11-01): "label: value" bullets, "الـ"
    // before English words, full stops after English, and arrows between
    // terms, plus a figure whose label once ran off its edge.
    // The last one has the step's abbreviations sheet open (2026-11-01).
    let mixed = [
        ("cvs.L01.lesson1", 2, false),
        ("cvs.L01.lesson1", 4, false),
        ("cvs.L01.lesson1", 6, false),
        ("cvs.L01.lesson1", 6, true),
        ("cvs.L01.lesson2", 3, false),
    ];
    for (lesson, at, abbr) in mixed {
        let (_, l) = Library::get().lesson(lesson).expect("mixed-text lesson");
        let step = &l.steps[at];
        let case = Case { lesson: lesson.into(), at, view: View::Steps, draft: None, abbr };
        let mut dom = VirtualDom::new_with_props(root, case);
        dom.rebuild_in_place();
        let open = if abbr { "-abbr" } else { "" };
        let file = format!("lesson-mixed-{lesson}-{}{open}-steps.html", at + 1);
        write_page(&dir, &file, &dom);
        manifest.push(serde_json::json!({
            "file": file, "screen": "learn", "kind": step.body.kind(), "view": "steps",
            "lesson": lesson, "step": at + 1, "steps": l.steps.len(),
            "question": step.body.is_question(),
        }));
    }
    dictionary(&dir, &mut manifest);
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}

/// The abbreviations dictionary (abbrdict/mod.rs) at the cardiovascular course:
/// a page, not a lesson, so the browser job checks only the shared layout
/// rules for it.
fn dictionary(dir: &std::path::Path, manifest: &mut Vec<serde_json::Value>) {
    fn page() -> Element {
        use_hook(|| *crate::abbrdict::COURSE.write() = Some("cvs".into()));
        rsx! {
            Shell { screen: Screen::Learn, focus: false, crate::abbrdict::Dictionary {} }
        }
    }
    let mut dom = VirtualDom::new(page);
    dom.rebuild_in_place();
    write_page(dir, "abbr-dictionary.html", &dom);
    manifest.push(serde_json::json!({ "file": "abbr-dictionary.html", "screen": "abbr-dictionary", "phase": "dictionary" }));
}

/// The sort step part-way through and after Check, on the step with the
/// longest bin and card names (portal hypertension, #-reported clipping).
fn sort_states(dir: &std::path::Path, manifest: &mut Vec<serde_json::Value>) {
    let longest = |b: &StepBody| match b {
        StepBody::Sort { bins, cards, .. } => {
            bins.iter().chain(cards.iter().map(|c| &c.text)).map(|t| t.len()).max().unwrap_or(0)
        }
        _ => 0,
    };
    // No sort step written yet: nothing to show.
    let Some((lesson, at, step)) = Library::get()
        .all_lessons()
        .flat_map(|(_, l)| l.steps.iter().enumerate().map(move |(at, s)| (l, at, s)))
        .filter(|(_, _, s)| s.body.kind() == "sort")
        .max_by_key(|(_, _, s)| longest(&s.body))
    else {
        return;
    };
    let StepBody::Sort { bins, cards, .. } = &step.body else { unreachable!() };
    // Half the cards placed; then all placed with the first one wrong.
    let half: Vec<_> = cards.iter().enumerate().map(|(i, c)| (i % 2 == 0).then_some(c.bin)).collect();
    let mut done: Vec<_> = cards.iter().map(|c| Some(c.bin)).collect();
    done[0] = Some((cards[0].bin + 1) % bins.len());
    let states = [
        ("placed", Draft::Sort { put: half, checked: false }, None),
        ("checked", Draft::Sort { put: done, checked: true }, Some(false)),
    ];
    for (name, draft, answer) in states {
        let case = Case {
            lesson: lesson.id.clone(),
            at,
            view: View::Steps,
            draft: Some((draft, answer)),
            abbr: false,
        };
        let mut dom = VirtualDom::new_with_props(root, case);
        dom.rebuild_in_place();
        let file = format!("lesson-sort-{name}-steps.html");
        write_page(dir, &file, &dom);
        manifest.push(serde_json::json!({
            "file": file, "screen": "learn", "kind": "sort", "view": "steps",
            "lesson": lesson.id, "step": at + 1, "steps": lesson.steps.len(),
            "question": answer.is_none(),
        }));
    }
}

/// A mini case after part 1 (checked wrong, part 2 open with its scenario)
/// and once every part is checked: each part's ↻, the scenario box and the
/// explanation card (Mohamed, 2026-11-01).
fn case_states(dir: &std::path::Path, manifest: &mut Vec<serde_json::Value>) {
    // No such case written yet: nothing to show.
    let Some((lesson, at, step)) = Library::get()
        .all_lessons()
        .flat_map(|(_, l)| l.steps.iter().enumerate().map(move |(at, s)| (l, at, s)))
        .find(|(_, _, s)| match &s.body {
            StepBody::Case { steps, .. } => {
                steps.len() > 1 && steps[1].reveal.is_some() && !steps[0].options.is_empty()
            }
            _ => false,
        })
    else {
        return;
    };
    let StepBody::Case { steps, .. } = &step.body else { unreachable!() };
    let n = steps.len();
    let wrong = |i: usize| steps[i].answer.map(|a| (a + 1) % steps[i].options.len().max(1));
    let mut part1 = vec![None; n];
    part1[0] = wrong(0);
    let all: Vec<_> =
        (0..n).map(|i| if steps[i].options.is_empty() { Some(0) } else { steps[i].answer }).collect();
    let mut first = all.clone();
    first[0] = wrong(0);
    let states = [
        ("part1", Draft::Case { picks: part1.clone(), at: 1, pick: None, first: part1 }, None),
        ("done", Draft::Case { picks: all, at: n - 1, pick: None, first }, Some(false)),
    ];
    for (name, draft, answer) in states {
        let case = Case {
            lesson: lesson.id.clone(),
            at,
            view: View::Steps,
            draft: Some((draft, answer)),
            abbr: false,
        };
        let mut dom = VirtualDom::new_with_props(root, case);
        dom.rebuild_in_place();
        let file = format!("lesson-case-{name}-steps.html");
        write_page(dir, &file, &dom);
        manifest.push(serde_json::json!({
            "file": file, "screen": "learn", "kind": "case", "view": "steps",
            "lesson": lesson.id, "step": at + 1, "steps": lesson.steps.len(),
            "question": answer.is_none(),
        }));
    }
}

/// Every drawn figure in every lesson, at the lesson's own figure styles, for
/// the browser check that no label runs past its picture's edge
/// (tests/screens-browser.cjs `figureChecks`).
#[test]
#[ignore = "exports production HTML for the screens browser CI job"]
fn export_screen_fixtures_figures() {
    let dir = out_dir("figures");
    let mut body = String::new();
    let mut count = 0;
    for (_, lesson) in Library::get().all_lessons() {
        for (at, step) in lesson.steps.iter().enumerate() {
            if let steps_core::lesson::StepBody::Figure { svg, image: None, .. } = &step.body {
                count += 1;
                body.push_str(&format!(
                    "<div class=\"ls-fig-svg\" dir=\"ltr\" data-at=\"{} step {}\">{svg}</div>\n",
                    lesson.id,
                    at + 1
                ));
            }
        }
    }
    assert!(count > 0, "the lessons have their figures: {count}");
    let page = format!(
        "<!doctype html><html lang=\"ar\" dir=\"rtl\"><head><meta charset=\"utf-8\">\
         <link rel=\"stylesheet\" href=\"app.css\">\
         <link id=\"prefs\" rel=\"stylesheet\" href=\"prefs-system-normal.css\">\
         <style>{}</style></head><body><div class=\"app\" lang=\"ar\" dir=\"rtl\">{body}</div></body></html>",
        include_str!("learn.css")
    );
    std::fs::write(dir.join("figures.html"), page).unwrap();
}
