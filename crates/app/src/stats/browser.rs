//! Browser fixtures for the self-assessment tab (tests/screens-browser.cjs):
//! no scores yet, a log with NBME and practice tests, and the add form open.

use super::*;
use crate::Screen;
use crate::browser_fixture::{Shell, out_dir, write_page};
use steps_core::plan::Day;
use steps_core::selfassess::{Kind, SelfAssessment};

fn root() -> Element {
    crate::store::use_provide_review();
    use_hook(|| *WANT.write() = Some(Tab::Scores));
    rsx! {
        Shell { screen: Screen::Stats, focus: false, Stats {} }
    }
}

fn seed() {
    let s = crate::store::shared();
    let rows = [
        (Kind::Nbme, Some("26"), "2026-09-01", 58, None, None),
        (Kind::Uwsa, Some("1"), "2026-09-20", 66, None, None),
        (Kind::Nbme, Some("28"), "2026-10-01", 66, Some(82), None),
        (Kind::Nbme, Some("30"), "2026-10-20", 71, Some(94), Some("Biostat and renal were weakest")),
        (Kind::Free120, None, "2026-11-01", 74, None, None),
    ];
    for (i, (kind, form, day, percent, prob, note)) in rows.into_iter().enumerate() {
        let e = SelfAssessment {
            id: format!("sa.fixture{i}"),
            kind,
            form: form.map(String::from),
            day: Day::parse(day).unwrap(),
            percent,
            pass_probability: prob,
            note: note.map(String::from),
            created_ms: i as i64,
        };
        s.put_self_assessment(&e).unwrap();
    }
}

#[test]
#[ignore = "exports production HTML for the screens browser CI job"]
fn export_screen_fixtures_scores() {
    crate::store::use_memory_store();
    let dir = out_dir("scores");
    let mut manifest = Vec::new();
    for state in ["empty", "logged", "form"] {
        if state == "logged" {
            seed();
        }
        scores::FORM_FOR_FIXTURE.store(state == "form", std::sync::atomic::Ordering::Relaxed);
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
        let file = format!("scores-{state}.html");
        write_page(&dir, &file, &dom);
        manifest.push(serde_json::json!({ "file": file, "screen": "scores", "state": state, "top": true }));
    }
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}
