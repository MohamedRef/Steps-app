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
