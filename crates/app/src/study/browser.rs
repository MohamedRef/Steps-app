//! Browser fixtures for the production study screen: setup, a running session
//! on the first item of each kind the player shows (fresh, on an
//! empty review log), a card reopened by Undo, a cloze after «مش فاكر،
//! وريني», and the summary, also of a session ended part-way
//! (tests/screens-browser.cjs).

use super::*;
use crate::Screen;
use crate::browser_fixture::{Shell, out_dir, write_page};
use steps_core::content::Body;
use steps_core::review::StudyStoreHandle;

#[derive(Props, Clone, PartialEq)]
struct Case {
    phase: &'static str,
    item: Option<String>,
    reopened: bool,
    /// More items after `item`, and how many of the session to grade Good
    /// first (a summary of a session ended part-way).
    #[props(default)]
    more: Vec<String>,
    #[props(default)]
    graded: usize,
}

fn root(case: Case) -> Element {
    // The app's own review backend, over the in-memory shared store.
    crate::store::use_provide_review();
    let store = use_context::<StudyStoreHandle>();
    use_hook(|| {
        let ids: Vec<String> = case.item.iter().chain(&case.more).cloned().collect();
        let mut session = Session::new(plan_items(&store, &ids));
        for _ in 0..case.graded {
            let Some(at) = session.current().map(|p| p.item_id.clone()) else { break };
            let answer = ReviewEvent::new(at, Grade::Good, Origin::Study, now_ms(), 20_000);
            let event = store.0.borrow_mut().record_logged(answer).ok().flatten();
            session.grade_logged(Grade::Good, event, |_| false);
        }
        *SESSION.write() = session;
        *PHASE.write() = match case.phase {
            "running" => Phase::Running,
            "summary" => Phase::Summary,
            _ => Phase::Setup,
        };
        if case.reopened {
            *UNDO_NOTE.write() = Some(tx::UNDONE);
        }
    });
    rsx! {
        Shell { screen: Screen::Study, focus: case.phase == "running", Study {} }
    }
}

#[test]
#[ignore = "exports production HTML for the screens browser CI job"]
fn export_screen_fixtures_study() {
    let dir = out_dir("study");
    let case =
        |phase, item: Option<String>| Case { phase, item, reopened: false, more: Vec::new(), graded: 0 };
    let mut cases = vec![("setup".to_string(), case("setup", None))];
    for kind in item::PLAYABLE {
        let first = ready_lectures().into_iter().flat_map(|l| &l.items).find(|i| i.body.kind() == *kind);
        // A kind no written lecture uses yet has no fixture.
        let Some(first) = first else { continue };
        cases.push((format!("item-{kind}"), case("running", Some(first.id.clone()))));
        if *kind == "card" {
            let reopened = Case { reopened: true, ..case("running", Some(first.id.clone())) };
            cases.push(("item-card-reopened".to_string(), reopened));
        }
        if *kind == "cloze" {
            cases.push(("item-cloze-shown".to_string(), case("running", Some(first.id.clone()))));
        }
    }
    // An enumerate list with its points open and its تحشيشة under them.
    let hooked = ready_lectures()
        .into_iter()
        .flat_map(|l| &l.items)
        .find(|i| matches!(&i.body, Body::Enumerate { hook: Some(_), .. }))
        .expect("an enumerate item with a hook");
    cases.push(("item-enumerate-open".to_string(), case("running", Some(hooked.id.clone()))));
    let first = cases[1].1.item.clone();
    cases.push(("summary".to_string(), case("summary", first)));
    // Ended at the second of three cards, the first graded: where he
    // stopped, its counts and Undo as a text button (UX-36).
    let cards: Vec<String> = ready_lectures()
        .into_iter()
        .flat_map(|l| &l.items)
        .filter(|i| i.body.kind() == "card")
        .take(3)
        .map(|i| i.id.clone())
        .collect();
    let stopped = Case { more: cards[1..].to_vec(), graded: 1, ..case("summary", cards.first().cloned()) };
    cases.push(("summary-stopped".to_string(), stopped));
    let mut manifest = Vec::new();
    for (state, case) in cases {
        let (phase, item) = (case.phase, case.item.clone());
        let opened = state == "item-enumerate-open";
        enumerate::OPEN_FOR_FIXTURE.store(opened, std::sync::atomic::Ordering::Relaxed);
        cloze::SHOWN_FOR_FIXTURE.store(state == "item-cloze-shown", std::sync::atomic::Ordering::Relaxed);
        let mut dom = VirtualDom::new_with_props(root, case);
        dom.rebuild_in_place();
        let file = format!("study-{state}.html");
        write_page(&dir, &file, &dom);
        let kind = item.as_deref().and_then(find).map(|(_, i)| i.body.kind());
        manifest.push(serde_json::json!({
            "file": file, "screen": "study", "state": state, "phase": phase, "item": item, "kind": kind,
        }));
    }
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}
