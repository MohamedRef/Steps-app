//! "اللي ورا الغلطة": after a missed MCQ, the cards and clozes that hold the
//! facts it tested, and one tap to study them now. Its own topic on its own
//! First Aid page first, then the rest of its topic; another topic on the
//! same page only for a question filed under no topic (a page holds several
//! topics, and their facts are not what it tested); never another lecture.
//! Answers in that
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

/// The facts behind `mcq` in its lecture, in book order within each rank:
/// playable items of its topic on its printed page, then the rest of its
/// topic; items of the same page under another topic only when the MCQ has
/// no topic. Never the MCQ itself, never hidden ones. A question from a
/// question bank (VQ) cites no First Aid page, so its facts are its topic's.
pub fn facts<'a>(lecture: &'a Lecture, mcq: &Item, hidden: impl Fn(&str) -> bool) -> Vec<&'a Item> {
    let page = mcq.src.first_aid_page();
    let on_page = |i: &Item| page.is_some() && i.src.first_aid_page() == page;
    let in_topic = |i: &Item| mcq.topic.is_some() && i.topic == mcq.topic;
    // 0: same topic and page; 1: same topic; 2: same page, for an MCQ with no topic.
    let rank = |i: &Item| match (in_topic(i), on_page(i)) {
        (true, true) => Some(0),
        (true, false) => Some(1),
        (false, true) if mcq.topic.is_none() => Some(2),
        _ => None,
    };
    let mut found: Vec<(u8, &Item)> = lecture
        .items
        .iter()
        .filter(|i| i.id != mcq.id && crate::study::PLAYABLE.contains(&i.body.kind()) && !hidden(&i.id))
        .filter_map(|i| rank(i).map(|r| (r, i)))
        .collect();
    // Stable: book order stays within a rank.
    found.sort_by_key(|(r, _)| *r);
    found.into_iter().map(|(_, i)| i).take(MAX).collect()
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
    use steps_core::content::{Body, BookRef, Source, Status};

    /// A card under `topic` on printed page `page`. `facts` reads only the
    /// question's id, topic and page, so the question is one of these too.
    fn item(id: &str, topic: Option<&str>, page: u32) -> Item {
        Item {
            id: format!("bio.L01.{id}"),
            body: Body::Card { front: format!("front {id}"), back: "back".into() },
            topic: topic.map(str::to_string),
            src: Source { book: None, page, quote: None },
            status: Status::Unverified,
            update: None,
            tags: Vec::new(),
        }
    }

    /// One lecture, in book order: two topics share page 35, and the
    /// question's own topic also runs on to page 36.
    fn lecture(mcq_topic: Option<&str>) -> (Lecture, Item) {
        let mcq = item("900", mcq_topic, 35);
        let items = vec![
            item("001", Some("ada"), 35),
            item("002", Some("ada"), 35),
            item("003", Some("code"), 34),
            item("004", Some("code"), 35),
            item("005", Some("lesch"), 35),
            item("006", Some("code"), 36),
            item("007", Some("code"), 35),
            mcq.clone(),
            item("008", None, 35),
        ];
        let lecture = Lecture {
            schema: steps_core::content::SCHEMA.into(),
            lecture: "bio.L01".into(),
            title: "Molecular".into(),
            author: None,
            book: BookRef { chapter: 1, pages: [33, 40] },
            topics: Vec::new(),
            items,
        };
        (lecture, mcq)
    }

    fn ids(found: &[&Item]) -> Vec<String> {
        found.iter().map(|i| i.id.trim_start_matches("bio.L01.").to_string()).collect()
    }

    #[test]
    fn facts_come_from_the_question_s_topic_on_its_page_then_the_rest_of_the_topic() {
        // A genetic-code question on page 35: never ADA or Lesch-Nyhan,
        // though they are on the same page.
        let (lec, mcq) = lecture(Some("code"));
        assert_eq!(ids(&facts(&lec, &mcq, |_| false)), ["004", "007", "003", "006"]);
        // At most MAX; the same page outranks book order.
        let more = Lecture {
            items: [lec.items.clone(), vec![item("010", Some("code"), 35)]].concat(),
            ..lec.clone()
        };
        assert_eq!(ids(&facts(&more, &mcq, |_| false)), ["004", "007", "010", "003"]);
        // Hidden items and the question itself never show.
        assert_eq!(ids(&facts(&lec, &mcq, |id| id.ends_with("004"))), ["007", "003", "006"]);
        assert!(facts(&lec, &mcq, |_| true).is_empty());
    }

    #[test]
    fn a_question_with_no_topic_takes_its_page() {
        let (lec, mcq) = lecture(None);
        assert_eq!(ids(&facts(&lec, &mcq, |_| false)), ["001", "002", "004", "005"]);
        // No page and no topic (a question bank's item): nothing to show.
        let vq = Item { src: Source { book: Some("vq".into()), page: 12, quote: None }, ..mcq };
        assert!(facts(&lec, &vq, |_| false).is_empty());
    }

    #[test]
    fn a_question_bank_item_takes_its_topic_on_any_page() {
        let (lec, mcq) = lecture(Some("code"));
        let vq = Item { src: Source { book: Some("vq".into()), page: 12, quote: None }, ..mcq };
        assert_eq!(ids(&facts(&lec, &vq, |_| false)), ["003", "004", "006", "007"]);
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
        let lib = Library::get();
        let lec = &lib.lectures["cvs.L01"];
        let mcq = lec.items.iter().find(|i| i.id == "cvs.L01.009").unwrap();
        let found = facts(lec, mcq, |_| false);
        assert!(!found.is_empty(), "page {} has cards", mcq.src.page);
        assert!(found.iter().all(|f| f.topic == mcq.topic), "only its own topic's facts");
        let mut dom = VirtualDom::new_with_props(Panel, PanelProps { id: mcq.id.clone() });
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(tx::BEHIND), "{html}");
        assert!(html.contains(&tx::behind_study(found.len())));
        // Nothing studied yet in a fresh store: every fact is new.
        assert_eq!(html.matches(tx::FACT_NEW).count(), found.len());
    }
}
