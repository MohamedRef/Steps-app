//! "اللي ورا الغلطة": after a missed MCQ, the cards and clozes that hold the
//! facts it tested, and one tap to study them now. The same First Aid page
//! first, then the same topic, never another lecture. Answers in that
//! session are ordinary reviews, so FSRS sees them like any other. The
//! paper stays where it was: the bank keeps it until he comes back.
//! Inspired by AMBOSS's "Get Anki cards" on a Qbank question and the AnKing
//! "unsuspend the cards for missed questions" loop.

use dioxus::prelude::*;
use steps_core::content::{Item, Lecture};
use steps_core::review::StudyStore;

use crate::i18n::bank as tx;
use crate::store::use_review;

/// At most this many facts under one question.
pub const MAX: usize = 4;

/// The facts behind `mcq` in its lecture: playable items on the same printed
/// page first, then the same topic, in book order; never the MCQ itself,
/// never hidden ones.
pub fn facts<'a>(lecture: &'a Lecture, mcq: &Item, hidden: impl Fn(&str) -> bool) -> Vec<&'a Item> {
    let usable =
        |i: &&Item| i.id != mcq.id && crate::study::PLAYABLE.contains(&i.body.kind()) && !hidden(&i.id);
    let same_page = lecture.items.iter().filter(usable).filter(|i| i.src.page == mcq.src.page);
    let same_topic = lecture
        .items
        .iter()
        .filter(usable)
        .filter(|i| i.src.page != mcq.src.page && mcq.topic.is_some() && i.topic == mcq.topic);
    same_page.chain(same_topic).take(MAX).collect()
}

/// Where a fact stands in review, in words.
fn state(store: &dyn StudyStore, id: &str, due: &[String]) -> &'static str {
    if store.is_new(id) {
        tx::FACT_NEW
    } else if due.iter().any(|d| d == id) {
        tx::FACT_DUE
    } else {
        tx::FACT_LEARNED
    }
}

/// The panel under a missed question's explanation; nothing when no card
/// holds its facts yet.
#[component]
pub fn Behind(lecture: String, item: Item) -> Element {
    let ui = use_context::<crate::Ui>();
    let review = use_review();
    let Some(lec) = crate::content::Library::get().lectures.get(&lecture) else {
        return rsx! {};
    };
    let rows: Vec<(String, String, &'static str)> = {
        let b = review.backend.borrow();
        let due = b.due(crate::store::now_ms());
        facts(lec, &item, |id| b.is_hidden(id))
            .into_iter()
            .map(|i| (i.id.clone(), crate::study::short_text(&b.effective(i)), state(&*b, &i.id, &due)))
            .collect()
    };
    if rows.is_empty() {
        return rsx! {};
    }
    let ids: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    rsx! {
        section { class: "q-behind", aria_labelledby: "q-behind-{item.id}",
            h3 { id: "q-behind-{item.id}", class: "label", {tx::BEHIND} }
            ul { class: "q-behind-list",
                for (id, text, st) in rows {
                    li { key: "{id}",
                        bdi { class: "q-behind-text", lang: "en", dir: "ltr", "{text}" }
                        span { class: "chip", {st} }
                    }
                }
            }
            button {
                class: "btn btn-tonal",
                onclick: move |_| {
                    *crate::study::START.write() = Some(crate::study::Start {
                        lectures: Some(vec![lecture.clone()]),
                        items: Some(ids.clone()),
                        minutes: crate::onboarding::session_minutes(),
                    });
                    crate::lecture::close_all();
                    crate::learn::forget();
                    let mut screen = ui.screen;
                    screen.set(crate::Screen::Study);
                },
                {tx::behind_study(ids.len())}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Library;
    use steps_core::content::Body;

    fn first_mcq(lec: &Lecture) -> &Item {
        lec.items.iter().find(|i| matches!(i.body, Body::Mcq { .. })).expect("an MCQ")
    }

    #[test]
    fn facts_come_from_the_same_page_then_topic_and_never_another_lecture() {
        let lib = Library::get();
        let lec = lib
            .lectures
            .values()
            .find(|l| l.items.iter().any(|i| matches!(i.body, Body::Mcq { .. })))
            .unwrap();
        for mcq in lec.items.iter().filter(|i| matches!(i.body, Body::Mcq { .. })) {
            let found = facts(lec, mcq, |_| false);
            assert!(found.len() <= MAX);
            let mut left_page = false;
            for f in &found {
                assert!(f.id.starts_with(&lec.lecture));
                assert_ne!(f.id, mcq.id);
                assert!(crate::study::PLAYABLE.contains(&f.body.kind()));
                if f.src.page == mcq.src.page {
                    assert!(!left_page, "same-page facts come first");
                } else {
                    left_page = true;
                    assert_eq!(f.topic, mcq.topic);
                }
            }
        }
        // Hidden items never show.
        let mcq = first_mcq(lec);
        assert!(facts(lec, mcq, |_| true).is_empty());
    }

    #[component]
    fn Panel(id: String) -> Element {
        crate::store::use_provide_review();
        let lecture = steps_core::program::lecture_of(&id).to_string();
        let item = Library::get().lectures[&lecture].items.iter().find(|i| i.id == id).unwrap().clone();
        rsx! {
            crate::browser_fixture::Shell { screen: crate::Screen::Bank, focus: false,
                Behind { lecture, item }
            }
        }
    }

    #[test]
    fn a_missed_question_lists_its_cards_and_offers_a_session() {
        crate::store::use_memory_store();
        let lib = Library::get();
        let lec = &lib.lectures["cvs.L01"];
        let mcq = lec.items.iter().find(|i| i.id == "cvs.L01.009").unwrap();
        let found = facts(lec, mcq, |_| false);
        assert!(!found.is_empty(), "page {} has cards", mcq.src.page);
        assert!(found.iter().all(|f| f.src.page == mcq.src.page));
        let mut dom = VirtualDom::new_with_props(Panel, PanelProps { id: mcq.id.clone() });
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(tx::BEHIND), "{html}");
        assert!(html.contains(&tx::behind_study(found.len())));
        // Nothing studied yet in a fresh store: every fact is new.
        assert_eq!(html.matches(tx::FACT_NEW).count(), found.len());
    }
}
