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
                "strip" => rsx! { LessonStrip { lecture: "cvs.L04" } },
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
        IndexPage {}
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
    assert_eq!(html.matches("tap-chip qb-copy").count(), with_qids + 1, "each video and the whole lecture");
    assert!(html.contains("Atherosclerosis"));
    assert!(html.contains(tx::FIND));
}

#[test]
fn the_index_page_searches_by_name_or_number() {
    let html = render("lookup", "");
    assert!(html.contains(tx::PAGE_TITLE) && html.contains(tx::PLACEHOLDER), "{html}");
    assert!(html.contains(&esc(tx::SOURCE)));
}

#[test]
fn the_index_lists_every_course_with_a_tick_and_a_copy_button() {
    let html = render("lookup", "");
    let courses = Library::get()
        .program
        .courses
        .iter()
        .filter(|c| course_lectures(&c.id).any(|l| l.count() > 0))
        .count();
    assert!(courses >= 15, "{courses}");
    assert_eq!(html.matches("qb-node qb-course").count(), courses);
    assert_eq!(html.matches(r#"class="qb-pick""#).count(), courses, "closed courses show no lectures");
    assert!(html.matches("tap-chip qb-copy").count() >= courses);
    assert!(html.contains("qb-basket") && html.contains(&esc(tx::BASKET_EMPTY)), "{html}");
    assert!(
        html.contains(tx::SHOW_BASKET) && !html.contains(r#"class="search qb-paste""#),
        "the basket opens on a tap"
    );
}

#[test]
fn a_lesson_opens_with_its_lectures_videos_and_qids() {
    let lec = data().lecture("cvs.L04").unwrap();
    let html = render("strip", "");
    assert!(
        html.contains(&tx::videos(lec.sets.len())) && html.contains(&tx::questions(lec.count())),
        "{html}"
    );
    assert!(html.contains("qb-copy") && html.contains(tx::VIDEOS));
    assert_eq!(html.matches(r#"class="qb-set""#).count(), 0, "the list opens on a tap");
}

#[test]
fn the_basket_takes_ticked_nodes_in_book_order_and_pasted_qids() {
    let lec = data().lecture("cvs.L04").unwrap();
    let first = lec.sets.iter().find(|s| !s.qids.is_empty()).unwrap();
    let none = BTreeSet::new();
    let picked = BTreeSet::from([format!("v:{}", first.video)]);
    assert_eq!(basket(&picked, "", &none, true), first.qids);
    let whole = BTreeSet::from(["l:cvs.L04".to_string()]);
    assert_eq!(basket(&whole, "7 8", &none, true).len(), lec.count() + 2);
    let done = BTreeSet::from([first.video.clone()]);
    assert_eq!(basket(&whole, "", &done, true).len(), lec.count() - first.qids.len(), "done videos left out");
    assert_eq!(basket(&whole, "", &done, false).len(), lec.count(), "unless he keeps them");
    let course = BTreeSet::from(["c:cvs".to_string()]);
    let all_cvs: usize = course_lectures("cvs").map(LectureSets::count).sum();
    assert_eq!(basket(&course, "", &none, true).len(), all_cvs);
    let tests = cut(&basket(&course, "", &none, true), TestPrefs::default(), 1);
    assert!(tests.iter().all(|t| t.len() <= MAX_TEST) && tests.concat().len() == all_cvs);
}

#[test]
fn only_a_whole_number_looks_a_qid_up() {
    assert!(lookup("Atherosclerosis").is_none());
    assert!(lookup("12 34").is_none(), "two numbers are a paste, not a lookup");
}
