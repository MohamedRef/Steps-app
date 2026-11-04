use std::cell::RefCell;

use steps_core::review::Grade;
use steps_core::review::Grade::{Again, Good};

use super::*;

thread_local! {
    static SEEN: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

#[derive(Props, Clone, PartialEq)]
struct With {
    answers: Vec<(&'static str, Grade)>,
    /// What to draw: "probe", "step" or "sheet".
    show: &'static str,
}

#[allow(non_snake_case)]
fn Host(p: With) -> Element {
    crate::store::use_private_review(&p.answers);
    let review = crate::store::use_review();
    SEEN.set(review.backend.borrow().weak());
    use_hook(move || {
        if p.show == "sheet" {
            crate::pagesheet::open(284, "cvs.L01.001", None, false, None);
        }
    });
    rsx! {
        crate::browser_fixture::Shell { screen: crate::Screen::Learn, focus: false,
            match p.show {
                "step" => rsx! { StepMark { of: "cvs.L01.lesson1#0", page: 284 } },
                "sheet" => rsx! { crate::pagesheet::BookSheet {} },
                "settings" => rsx! { WeakSection {} },
                _ => rsx! {},
            }
        }
    }
}

fn render(show: &'static str, answers: &[(&'static str, Grade)]) -> String {
    let mut dom = VirtualDom::new_with_props(Host, With { answers: answers.to_vec(), show });
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

fn weak_after(answers: &[(&'static str, Grade)]) -> BTreeSet<String> {
    render("probe", answers);
    SEEN.take()
}

#[test]
fn a_miss_marks_and_the_next_right_answer_clears() {
    assert!(weak_after(&[]).is_empty());
    assert_eq!(weak_after(&[("cvs.L01.001", Again)]), BTreeSet::from(["cvs.L01.001".to_string()]));
    assert!(weak_after(&[("cvs.L01.001", Again), ("cvs.L01.001", Good)]).is_empty());
    assert_eq!(
        weak_after(&[("cvs.L01.001", Good), ("cvs.L01.002", Again), ("cvs.L01.001", Again)]),
        BTreeSet::from(["cvs.L01.001".to_string(), "cvs.L01.002".to_string()]),
        "each item keeps its own last answer"
    );
}

#[test]
fn only_the_lectures_own_page_counts() {
    let lib = Library::get();
    let weak = BTreeSet::from(["cvs.L01.001".to_string(), "cvs.L01.010".to_string()]);
    let ids =
        |lec: &str, page| on_page(lib, &weak, lec, page).iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids("cvs.L01", 284), ["cvs.L01.001"]);
    assert_eq!(ids("cvs.L01", 285), ["cvs.L01.010"]);
    assert!(ids("cvs.L01", 286).is_empty());
    assert!(ids("cvs.L02", 284).is_empty());
}

#[test]
fn a_lesson_step_is_marked_only_after_a_miss_on_its_page() {
    assert!(!render("step", &[]).contains("weak-mark"));
    assert!(!render("step", &[("cvs.L01.010", Again)]).contains("weak-mark"), "another page");
    let html = render("step", &[("cvs.L01.001", Again)]);
    assert!(html.contains("weak-mark"), "{html}");
    assert!(html.contains(tx::MARK) && html.contains(tx::WHY));
    assert!(!render("step", &[("cvs.L01.001", Again), ("cvs.L01.001", Good)]).contains("weak-mark"));
}

#[test]
fn the_page_sheet_lists_the_facts_to_go_over_again() {
    let html = render("sheet", &[("cvs.L01.001", Again), ("cvs.L01.009", Again)]);
    assert!(html.contains("bk-weak"), "{html}");
    assert_eq!(html.matches(r#"class="weak-fact""#).count(), 1, "the card, not the MCQ");
    assert!(html.contains(&tx::questions(1)));
    assert!(html.contains(&tx::study(1)));
    assert!(!render("sheet", &[]).contains("bk-weak"));
}

#[test]
fn the_setting_shows_its_toggle() {
    let html = render("settings", &[]);
    assert!(html.contains(tx::SETTING_TITLE) && html.contains(r#"type="checkbox""#), "{html}");
}
