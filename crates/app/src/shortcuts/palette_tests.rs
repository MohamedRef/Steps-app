use super::*;

#[test]
fn nothing_typed_lists_screens_tools_and_settings() {
    let hits = find("", Library::get());
    assert_eq!(hits.len(), Screen::NAV.len() + crate::more::TOOLS.len() + 1);
    assert!(hits.iter().any(|h| h.go == Go::Settings));
    assert!(hits.iter().all(|h| matches!(h.go, Go::Screen(_) | Go::Tool(_) | Go::Settings)));
}

#[test]
fn lectures_lessons_and_pages_are_found() {
    let lib = Library::get();
    let lecture = lib.index.lectures.iter().find(|e| lib.lectures.contains_key(&e.id)).expect("a lecture");
    assert!(find(&lecture.id, lib).iter().any(|h| h.go == Go::Lecture(lecture.id.clone())));
    let (_, lesson) = lib.all_lessons().next().expect("a lesson");
    let hits = find(&lesson.id, lib);
    assert!(hits.iter().any(|h| h.go == Go::Lesson(lesson.id.clone())));
    assert!(hits.len() <= MAX);
    let page = lecture.pages[0];
    let hits = find(&format!("ص {page}"), lib);
    assert!(matches!(&hits[0].go, Go::Page(p, l) if *p == page && *l == lecture.id), "{hits:?}");
}

#[test]
fn every_word_matches_in_any_case() {
    assert!(matches("Heart FAILURE", "heart failure and shock"));
    assert!(!matches("heart renal", "heart failure"));
}

#[test]
fn settings_sections_are_found_by_their_words() {
    let lib = Library::get();
    let hits = find("backup", lib);
    assert!(hits.iter().any(|h| h.go == Go::Section(Section::Data)), "{hits:?}");
    let hits = find("fsrs", lib);
    assert!(hits.iter().any(|h| h.go == Go::Section(Section::Study)), "{hits:?}");
    assert!(find("", lib).iter().all(|h| !matches!(h.go, Go::Section(_))));
}

#[test]
fn lecture_questions_only_when_asked_and_only_with_mcqs() {
    let lib = Library::get();
    let has_mcq = |id: &str| {
        lib.lectures
            .get(id)
            .is_some_and(|l| l.items.iter().any(|i| matches!(i.body, steps_core::content::Body::Mcq { .. })))
    };
    let lecture = lib.index.lectures.iter().find(|e| has_mcq(&e.id)).expect("a lecture with MCQs");
    for ask in ["questions", "أسئلة"] {
        let hits = find(&format!("{ask} {}", lecture.id), lib);
        assert!(hits.iter().any(|h| h.go == Go::Questions(lecture.id.clone())), "{hits:?}");
        assert!(hits.iter().all(|h| !matches!(&h.go, Go::Questions(id) if !has_mcq(id))));
    }
    assert!(find(&lecture.id, lib).iter().all(|h| !matches!(h.go, Go::Questions(_))));
    assert!(questions_query("questions ").is_none());
    assert!(questions_query("bank").is_none());
}

#[test]
fn topics_and_abbreviations_are_found_too() {
    // UX-15: a word of a topic's title finds the topic; an abbreviation
    // finds itself, with its meaning beside it.
    let lib = Library::get();
    let (lecture, topic) = lib
        .index
        .lectures
        .iter()
        .filter_map(|e| lib.lectures.get(&e.id))
        .find_map(|l| l.topics.first().map(|t| (l.lecture.clone(), t.clone())))
        .expect("a lecture with topics");
    let hits = find(&topic.title, lib);
    assert!(hits.iter().any(|h| h.go == Go::Topic(lecture.clone(), topic.id.clone())), "{hits:?}");
    let (course, terms) =
        crate::abbrdict::index().iter().find(|(_, t)| !t.is_empty()).expect("a course with abbreviations");
    let term = &terms[0].entry;
    let hits = find(&term.term, lib);
    let hit = hits.iter().find(|h| matches!(&h.go, Go::Term(_, t) if *t == term.term)).expect("the term");
    assert!(hit.detail.is_some(), "its meaning shows");
    let _ = course;
    assert!(hits.iter().filter(|h| matches!(h.go, Go::Term(..))).count() <= MAX_TERMS);
}

#[test]
fn nothing_found_offers_the_lessons_and_the_dictionary() {
    let hits = find("zzzz-nothing-here", Library::get());
    assert!(nothing_found(&hits));
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|h| h.go == Go::SearchLessons("zzzz-nothing-here".into())));
    assert!(hits.iter().any(|h| h.go == Go::SearchTerms("zzzz-nothing-here".into())));
    assert!(!nothing_found(&find("", Library::get())));
}
