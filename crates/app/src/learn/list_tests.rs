use super::*;

#[test]
fn search_finds_lessons_by_title_topic_and_lecture() {
    let lib = Library::get();
    let (lec, l) = lib.all_lessons().next().expect("a lesson is written");
    let title = lib.title(lec).to_string();
    let code = Library::get().lecture_tag(lec).to_lowercase();
    assert!(matches(&code, lec, &title, l));
    assert!(matches(&title.to_lowercase(), lec, &title, l));
    if let Some(t) = l.topics.first() {
        assert!(matches(&t.to_lowercase(), lec, &title, l));
    }
    assert!(!matches("zzz-no-such-word", lec, &title, l));
}

#[test]
fn selected_lecture_cta_is_directly_keyboard_operable() {
    let source = include_str!("mod.rs");
    let shown = source.split("fn Shown(").nth(1).unwrap().split("pub(crate) fn numbered").next().unwrap();
    let cta = shown.split("class: \"btn btn-primary ls-lec-go\"").nth(1).unwrap();
    assert!(cta.contains("r#type: \"button\""));
    assert!(cta.contains("e.stop_propagation();"));
    assert!(cta.contains("open(ui, id);"));
}

#[test]
fn following_next_lesson_keeps_starting_screen() {
    assert_eq!(lesson_origin(Screen::Learn, true, Screen::Today), Screen::Today);
    assert_eq!(lesson_origin(Screen::Learn, true, Screen::Study), Screen::Study);
    assert_eq!(lesson_origin(Screen::Learn, false, Screen::Today), Screen::Learn);
    assert_eq!(lesson_origin(Screen::Study, true, Screen::Today), Screen::Study);
}

#[test]
fn left_off_is_the_latest_start_and_book_order_breaks_ties() {
    assert_eq!(latest([("a", 5), ("b", 9), ("c", 7)]), Some("b"));
    assert_eq!(latest([("a", 0), ("b", 0)]), Some("a"));
    assert_eq!(latest::<&str>([]), None);
    assert_eq!(lecture_of("cvs.L04.lesson1"), "cvs.L04");
}

#[test]
fn resume_card_only_leads_back_from_another_lecture() {
    let source = include_str!("mod.rs");
    let list = source
        .split("fn LessonList(")
        .nth(1)
        .unwrap()
        .split("pub(crate) const LECTURES_VIEW")
        .next()
        .unwrap();
    assert!(list.contains("resume.filter(|l| lecture_of(&l.id) != shown)"));
    assert!(list.contains(".or(resume_lecture)"));
}

#[test]
fn an_item_no_lesson_practises_is_taught_by_the_lesson_on_its_topic() {
    let lib = Library::get();
    for (lec, file) in &lib.lessons {
        let Some(lecture) = lib.lectures.get(lec) else { continue };
        for item in &lecture.items {
            let Some(l) = lesson_for_item(&item.id) else { continue };
            let practised = l.practice.contains(&item.id)
                    || l.steps.iter().any(|s| matches!(&s.body, steps_core::lesson::StepBody::Try { items } if items.contains(&item.id)));
            if !practised {
                // Found by topic: the lesson covers the item's topic.
                let topic = item.topic.as_deref().expect("matched by its topic");
                assert!(l.topics.iter().any(|t| t == topic), "{} -> {}", item.id, l.id);
            }
            assert!(file.lessons.iter().any(|x| x.id == l.id));
        }
    }
    assert!(lesson_for_item("nolecture").is_none());
    // An item of a lecture with no lessons has none.
    if let Some(item) =
        lib.lectures.iter().find(|(id, _)| !lib.lessons.contains_key(*id)).and_then(|(_, l)| l.items.first())
    {
        assert!(lesson_for_item(&item.id).is_none(), "{}", item.id);
    }
    assert!(lesson_for_item("cvs.L01.zzz-no-such-item").is_none());
}

#[test]
fn lesson_counts_read_naturally() {
    assert_eq!(tx::lessons_n(1), "درس واحد");
    assert_eq!(tx::lessons_n(2), "درسين");
    assert_eq!(tx::lessons_n(7), "7 دروس");
    assert_eq!(tx::lessons_n(12), "12 درس");
}
