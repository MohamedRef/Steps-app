use super::*;

#[test]
fn the_content_is_valid_and_points_at_real_lectures() {
    let lib = Library::get();
    let ids: BTreeSet<&str> = lib.index.lectures.iter().map(|l| l.id.as_str()).collect();
    let map: steps_core::qbank::Map =
        serde_yaml_ng::from_str(include_str!("../../../../content/qbank/bb-map.yaml")).unwrap();
    assert_eq!(steps_core::qbank::problems(&map, data(), &ids), Vec::<String>::new());
    assert!(data().lectures.len() > 60);
}

#[test]
fn a_qid_finds_its_lecture_and_lists_copy_as_uworld_takes_them() {
    let set = Set { video: "Cardiology › Atherosclerosis".into(), qids: vec![500, 501] };
    assert_eq!(qid_list([&set]), "500, 501");
    assert_eq!(video_name(&set), "Atherosclerosis");
    let (lecture, first) = data()
        .lectures
        .iter()
        .find_map(|l| l.sets.iter().find(|s| !s.qids.is_empty()).map(|s| (l.id.as_str(), s)))
        .unwrap();
    let qid = first.qids[0];
    assert_eq!(lookup(&format!(" #{qid} ")).map(|(l, _)| l), Some(lecture));
    assert!(lookup("abc").is_none());
    if let Some(r) = data().retired.first() {
        assert!(lookup(&r.to_string()).is_none(), "retired QIDs are filed nowhere");
    }
}

#[test]
fn hypertension_drugs_sit_with_cardiovascular_pharmacology() {
    let cvs5 = data().lecture("cvs.L05").expect("cvs.L05 has B&B videos");
    assert!(cvs5.sets.iter().any(|s| s.video == "Cardiology › Hypertension Drugs"));
    assert!(cvs5.sets.iter().any(|s| s.video == "Cardiology › Antiarrhythmic Drugs"));
}

#[derive(Props, Clone, PartialEq)]
struct With {
    show: &'static str,
    text: &'static str,
}

#[allow(non_snake_case)]
fn Host(p: With) -> Element {
    rsx! {
        crate::browser_fixture::Shell { screen: crate::Screen::Learn, focus: false,
            match p.show {
                "section" => rsx! { LectureSection { lecture: "cvs.L04" } },
                _ => rsx! { LookupWith { text: p.text } },
            }
        }
    }
}

#[component]
fn LookupWith(text: &'static str) -> Element {
    // The page's own field starts empty; a test types by rendering the
    // result block with the same lookup.
    let found = lookup(text);
    rsx! {
        LookupPage {}
        if let Some((l, _)) = found { span { class: "probe", "{l}" } }
    }
}

/// `text` as the HTML shows it ("B&B" → "B&#38;B").
fn esc(text: &str) -> String {
    text.replace('&', "&#38;")
}

fn render(show: &'static str, text: &'static str) -> String {
    let mut dom = VirtualDom::new_with_props(Host, With { show, text });
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[test]
fn a_lecture_lists_its_videos_with_copy_buttons() {
    let lec = data().lecture("cvs.L04").unwrap();
    let html = render("section", "");
    assert!(html.contains(&esc(tx::TITLE)), "{html}");
    assert_eq!(html.matches(r#"class="qb-set""#).count(), lec.sets.len());
    let with_qids = lec.sets.iter().filter(|s| !s.qids.is_empty()).count();
    assert_eq!(html.matches("tap-chip qb-copy").count(), with_qids);
    assert!(html.contains("Atherosclerosis"));
    assert!(html.contains(tx::FIND));
}

#[test]
fn the_lookup_page_asks_for_a_number() {
    let html = render("lookup", "");
    assert!(html.contains(tx::PAGE_TITLE) && html.contains(r#"inputmode="numeric""#), "{html}");
    assert!(html.contains(&esc(tx::SOURCE)));
}
