use super::*;

#[test]
fn block_systems_follow_the_book_and_only_list_courses_with_questions() {
    let lib = Library::get();
    let pool = exam::mcqs(lib.lectures.values());
    let systems = block_courses(&pool);
    assert!(systems.iter().all(|(.., n)| *n > 0));
    assert_eq!(systems.iter().map(|s| s.3).sum::<usize>(), pool.len());
    let order: Vec<usize> = systems
        .iter()
        .map(|(_, id, ..)| lib.program.courses.iter().position(|c| &c.id == id).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]));
}

#[component]
fn BlockSetup(tutor: bool) -> Element {
    use_hook(|| {
        *RESTORED.write() = true;
        *MODE.write() = Mode::Mock;
        *BLOCK_TUTOR.write() = tutor;
        BLOCK_COURSES.write().clear();
    });
    rsx! {
        crate::browser_fixture::Shell { screen: crate::Screen::Bank, focus: false, Setup {} }
    }
}

fn render_block(tutor: bool) -> String {
    let mut dom = VirtualDom::new_with_props(BlockSetup, BlockSetupProps { tutor });
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[test]
fn the_block_card_offers_systems_sizes_and_both_modes() {
    let html = render_block(false);
    for needle in [
        tx::MODE_MOCK_NAME,
        tx::BLOCK_SYSTEMS,
        tx::BLOCK_ALL_SYSTEMS,
        tx::BLOCK_TIMED_HINT,
        tx::BLOCK_TUTOR,
        tx::START_MOCK,
    ] {
        assert!(html.contains(needle), "missing {needle}");
    }
    // The written lectures' courses are offered as chips, in Arabic.
    let lib = Library::get();
    let pool = exam::mcqs(lib.lectures.values());
    for (_, _, short, _) in block_courses(&pool) {
        assert!(html.contains(&short), "missing chip {short}");
    }
    for n in MOCK_COUNTS {
        assert!(html.contains(&format!(">{n}<")), "missing size {n}");
    }
    // Timed blocks show their clock; tutor blocks have none.
    let timed_minutes = minutes(exam::time_limit_ms(MOCK_SIZE.min(pool.len())));
    let clock = counted(timed_minutes, &MINUTE);
    assert!(html.contains(&clock), "missing {clock}");
    let tutor = render_block(true);
    assert!(!tutor.contains(&clock), "a tutor block shows no clock");
}

#[test]
fn a_setup_saved_before_blocks_still_loads() {
    let old = r#"{"mode":"mock","filter":{"lectures":[],"topics":[],"tags":[],"only":"all"},"count":20,"mock_count":100}"#;
    let saved: Saved = serde_json::from_str(old).unwrap();
    assert!(saved.block_courses.is_empty());
    assert!(!saved.block_tutor);
    // 100 is no longer a block size: it falls back to a full block.
    let saved = saved.pruned(&[]);
    assert_eq!(saved.mock_count, MOCK_SIZE);
    assert!(MOCK_COUNTS.iter().all(|&n| (10..=20).contains(&n)));
}
