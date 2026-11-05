use super::*;

/// QIDs the mapping files: two from one lecture, one from another.
fn filed() -> (String, u32, u32, String, u32) {
    let mut lectures = data().lectures.iter().filter(|l| l.count() >= 2);
    let a = lectures.next().unwrap();
    let b = lectures.next().unwrap();
    let qa: Vec<u32> = a.sets.iter().flat_map(|s| s.qids.iter().copied()).take(2).collect();
    let qb = b.sets.iter().flat_map(|s| s.qids.iter().copied()).next().unwrap();
    (a.id.clone(), qa[0], qa[1], b.id.clone(), qb)
}

fn log(qid: u32, reason: ErrorReason, created_ms: i64, note: Option<&str>) {
    let m = Miss {
        qid,
        reason,
        note: note.map(str::to_string),
        day: steps_core::plan::Day::parse("2027-03-01").unwrap(),
        created_ms,
    };
    crate::store::shared().put_setting(&m.key(), &m).unwrap();
}

#[derive(Props, Clone, PartialEq)]
struct With {
    qid: Option<u32>,
    card: bool,
}

#[allow(non_snake_case)]
fn Host(p: With) -> Element {
    rsx! {
        crate::browser_fixture::Shell { screen: crate::Screen::Learn, focus: false,
            if p.card {
                UworldMistakes {}
            } else {
                if let Some(qid) = p.qid {
                    LogMiss { qid }
                }
                MissLog {}
            }
        }
    }
}

fn render(qid: Option<u32>) -> String {
    let mut dom = VirtualDom::new_with_props(Host, With { qid, card: false });
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[test]
fn the_card_asks_for_a_qid() {
    let mut dom = VirtualDom::new_with_props(Host, With { qid: None, card: true });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains(tx::CARD_TITLE) && html.contains(r#"inputmode="numeric""#), "{html}");
    assert!(html.contains(tx::EMPTY));
    assert!(!html.contains(tx::LOG_TITLE), "no form before a QID is typed");
}

#[test]
fn an_empty_log_says_how_to_start() {
    let html = render(None);
    assert!(html.contains(tx::TITLE) && html.contains(tx::EMPTY), "{html}");
    assert!(!html.contains("uwm-patterns"));
}

#[test]
fn a_typed_qid_offers_every_reason_and_a_save() {
    let html = render(Some(1707));
    assert!(html.contains(tx::LOG_TITLE) && html.contains(tx::SAVE), "{html}");
    assert_eq!(html.matches(r#"class="mc-chip""#).count(), ErrorReason::ALL.len());
    assert!(!html.contains(tx::LOGGED_AS));
}

#[test]
fn a_logged_qid_shows_its_reason_and_offers_an_update() {
    log(4242, ErrorReason::Misread, 1, Some("missed the word EXCEPT"));
    let html = render(Some(4242));
    assert!(html.contains(tx::LOGGED_AS) && html.contains(tx::UPDATE), "{html}");
    assert!(html.contains(r#"data-reason="misread" aria-pressed="true""#), "{html}");
    assert!(html.contains("missed the word EXCEPT"));
}

#[test]
fn the_log_shows_patterns_lectures_and_every_miss() {
    let (lec_a, a1, a2, lec_b, b1) = filed();
    log(a1, ErrorReason::Knowledge, 1, None);
    log(a2, ErrorReason::Knowledge, 2, None);
    log(b1, ErrorReason::Timing, 3, None);
    log(999_999, ErrorReason::Knowledge, 4, None);
    let html = render(None);
    let lib = Library::get();
    assert!(html.contains(tx::PATTERNS), "{html}");
    let top = tx::top_reason(reason_label(ErrorReason::Knowledge), 3, 4, 75);
    assert!(html.contains(&top), "{top} in {html}");
    assert!(html.contains(tx::advice(ErrorReason::Knowledge)));
    let crumb_a = lib.crumb(&lec_a).replace('&', "&#38;");
    let crumb_b = lib.crumb(&lec_b).replace('&', "&#38;");
    let (pa, pb) = (html.find(&crumb_a).unwrap(), html.find(&crumb_b).unwrap());
    assert!(pa < pb, "the lecture with more misses comes first");
    assert!(html.contains(&format!("{a1}, {a2}")) || html.contains(&format!("{a2}, {a1}")), "copyable QIDs");
    assert_eq!(html.matches(r#"class="uwm-row""#).count(), 4);
    assert!(html.contains(&format!("{} 1", tx::UNFILED)));
}

#[test]
fn too_few_misses_say_so_instead_of_a_pattern() {
    log(77, ErrorReason::Reasoning, 1, None);
    let html = render(None);
    assert!(html.contains(tx::TOO_FEW), "{html}");
    assert!(!html.contains("أكتر سبب"));
}

#[test]
fn copied_qids_are_unique_and_comma_separated() {
    assert_eq!(list(&[5, 3, 5]), "3, 5");
}

#[test]
fn reason_chips_are_buttons_with_counts() {
    log(11, ErrorReason::Timing, 1, None);
    log(12, ErrorReason::Timing, 2, None);
    log(13, ErrorReason::Misread, 3, None);
    let html = render(None);
    assert!(html.contains(r#"data-reason="timing" aria-pressed="false""#), "{html}");
    assert!(html.contains(tx::FILTER));
}
