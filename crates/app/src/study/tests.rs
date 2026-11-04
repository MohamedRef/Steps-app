//! Existing behavior regressions for the feature facade.

use super::*;

#[test]
fn explicit_items_keep_order_and_skip_unknown_and_duplicates() {
    let mut ids: Vec<String> = items()
        .values()
        .filter(|(_, i)| PLAYABLE.contains(&i.body.kind()))
        .map(|(_, i)| i.id.clone())
        .collect();
    ids.sort();
    let ids: Vec<String> = ids.into_iter().take(2).rev().collect();
    if ids.len() < 2 {
        return; // no content embedded
    }
    let store = StudyStoreHandle(std::rc::Rc::new(std::cell::RefCell::new(backend::MemoryStore::default())));
    let asked = [ids[0].clone(), "nope.L99.001".into(), ids[1].clone(), ids[0].clone()];
    let plan = plan_items(&store, &asked);
    let got: Vec<&str> = plan.iter().map(|p| p.item_id.as_str()).collect();
    assert_eq!(got, [ids[0].as_str(), ids[1].as_str()]);
    assert!(plan.iter().all(|p| p.fresh));
}

#[test]
fn when_labels() {
    // UTC, so study days start at 04:00 UTC. `now` is 05:00 on a day.
    let zone = steps_core::zone::Zone::fixed(0);
    let now = 20_000 * 86_400_000 + 5 * 3_600_000;
    let at = |h: i64| now + h * 3_600_000;
    let label = |due: i64| when_label(now, due, &zone);
    assert_eq!(label(now), tx::NOW);
    assert_eq!(label(now + 20 * 60_000), "بعد 20 دقيقة");
    assert_eq!(label(at(5)), "بعد 5 ساعات");
    assert_eq!(label(at(2)), "بعد ساعتين");
    assert_eq!(label(at(23)), tx::TOMORROW, "04:00 tomorrow");
    // A 2-day interval lands at 04:00 two study days on: 47 hours away.
    assert_eq!(label(at(47)), "بعد يومين");
    assert_eq!(label(at(71)), "بعد 3 أيام");
}

#[test]
fn study_controls_keep_48dp_targets() {
    let css = include_str!("../../assets/study.css");
    for rule in [
        ".st :is(.btn, .btn-link) { min-block-size: max(var(--target), var(--tap)); }",
        ".st .btn-lg { min-block-size: max(var(--target), var(--tap-lg)); }",
        ".st-setup .segmented > button { min-block-size: max(var(--target), var(--tap)); }",
        "min-block-size: max(var(--target), var(--tap)); padding-inline: var(--sp-3);",
        "font: inherit; font-size: var(--fs-md); min-block-size: var(--target);",
    ] {
        assert!(css.contains(rule), "missing {rule}");
    }
    assert!(css.contains(".st-point { display: flex; align-items: center; gap: var(--sp-3); min-block-size: max(var(--target), var(--tap));"));
    assert!(!css.contains("min-block-size: 40px"));
}

#[test]
fn summary_and_progress_speak_to_screen_readers() {
    fn app() -> Element {
        rsx! {
            outcome::SummaryView {
                items: 3,
                right: 2,
                fresh: 1,
                shaky: Vec::new(),
                next: None,
                can_undo: false,
                on_undo: |_| {},
                undo_failed: false,
                on_again: |_| {},
            }
        }
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("tabindex=\"-1\""), "{html}");
    let source = include_str!("mod.rs");
    assert!(source.contains("aria_valuetext: \"{pos + 1} {tx::OF} {len}\""));
}
