//! Browser fixtures for the production study screen: setup, a running session
//! on the first item of each kind the player shows (fresh, on an
//! empty review log), a card reopened by Undo,
//! and the summary (tests/screens-browser.cjs).

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
}

fn root(case: Case) -> Element {
    // The app's own review backend, over the in-memory shared store.
    crate::store::use_provide_review();
    let store = use_context::<StudyStoreHandle>();
    use_hook(|| {
        let ids: Vec<String> = case.item.iter().cloned().collect();
        *SESSION.write() = Session::new(plan_items(&store, &ids));
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
    crate::store::use_memory_store();
    let dir = out_dir("study");
    let mut cases = vec![("setup".to_string(), Case { phase: "setup", item: None, reopened: false })];
    for kind in item::PLAYABLE {
        let first = ready_lectures().into_iter().flat_map(|l| &l.items).find(|i| i.body.kind() == *kind);
        let Some(first) = first else { panic!("no {kind} item to show") };
        cases.push((
            format!("item-{kind}"),
            Case { phase: "running", item: Some(first.id.clone()), reopened: false },
        ));
        if *kind == "card" {
            let case = Case { phase: "running", item: Some(first.id.clone()), reopened: true };
            cases.push(("item-card-reopened".to_string(), case));
        }
    }
    // An enumerate list with its points open and its تحشيشة under them.
    let hooked = ready_lectures()
        .into_iter()
        .flat_map(|l| &l.items)
        .find(|i| matches!(&i.body, Body::Enumerate { hook: Some(_), .. }))
        .expect("an enumerate item with a hook");
    let open = Case { phase: "running", item: Some(hooked.id.clone()), reopened: false };
    cases.push(("item-enumerate-open".to_string(), open));
    let first = cases[1].1.item.clone();
    cases.push(("summary".to_string(), Case { phase: "summary", item: first, reopened: false }));
    let mut manifest = Vec::new();
    for (state, case) in cases {
        let (phase, item) = (case.phase, case.item.clone());
        let opened = state == "item-enumerate-open";
        enumerate::OPEN_FOR_FIXTURE.store(opened, std::sync::atomic::Ordering::Relaxed);
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
