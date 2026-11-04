//! References (More › المراجع): every lecture with its book pages, lessons
//! and how much of it he has checked against the book, a search over titles
//! and topics, and his own cards. A lecture row opens the lecture's page;
//! Back from there returns here.
//!
//! "Checked" is his content check (✓ in the verify bar), never a lesson he
//! finished: the list says which is which.

use dioxus::prelude::*;
use steps_core::stats::Counts;

use crate::content::{Library, lecture_no};
use crate::i18n::refs as tx;
use crate::look::{Css, css};
use crate::parts::{Collection, Layout, Sort};
use crate::stats::StudyData;
use crate::store::use_review;
use crate::{Screen, Ui};

const CSS: Css = css!("src/lecture/refs.css");

/// One lecture row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub code: String,
    pub title: String,
    pub pages: [u32; 2],
    pub lessons: usize,
    /// `None` while the lecture is still being written.
    pub counts: Option<Counts>,
    /// Lowercased title, code and topic titles, for the search.
    haystack: String,
}

/// Every lecture in book order, with the numbers its row shows.
pub fn rows(lib: &Library, data: &StudyData) -> Vec<Row> {
    let stats = data.stats();
    lib.index
        .lectures
        .iter()
        .map(|e| {
            let topics = lib
                .lectures
                .get(&e.id)
                .map(|l| l.topics.iter().map(|t| t.title.as_str()).collect::<Vec<_>>());
            let code = lecture_no(&e.id);
            let course = lib.course_of(&e.id).map(|c| format!("{} {}", c.short, c.title)).unwrap_or_default();
            let haystack = format!(
                "{} {course} {} l{code:0>2} {}",
                e.title,
                lib.lecture_tag(&e.id),
                topics.unwrap_or_default().join(" ")
            )
            .to_lowercase();
            Row {
                id: e.id.clone(),
                code,
                title: e.title.clone(),
                pages: e.pages,
                lessons: lib.lessons.get(&e.id).map_or(0, |f| f.lessons.len()),
                counts: stats.iter().find(|s| s.id == e.id).map(|s| s.counts.clone()),
                haystack,
            }
        })
        .collect()
}

/// The lecture list's view choice: rows by default; "what's left first"
/// puts lectures with content he hasn't checked yet on top.
const VIEW: Collection = Collection {
    key: "refs",
    layout: Layout::DEFAULT,
    sorts: &[Sort::Book, Sort::Left, Sort::Name],
    groups: false,
};

impl Row {
    /// Written, with cards he hasn't checked against the book yet.
    fn unchecked(&self) -> bool {
        self.checked().is_some_and(|(a, t)| a < t)
    }

    /// Checked out of all, once the lecture has cards.
    fn checked(&self) -> Option<(u32, u32)> {
        self.counts.as_ref().filter(|c| c.total > 0).map(|c| (c.approved, c.total))
    }
}

/// The rows whose title, number or topics hold every word of `query`.
pub fn search<'a>(rows: &'a [Row], query: &str) -> Vec<&'a Row> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    rows.iter().filter(|r| words.iter().all(|w| r.haystack.contains(w.as_str()))).collect()
}

#[component]
pub fn References() -> Element {
    let ui = use_context::<Ui>();
    let review = use_review();
    let _ = review.revision.read();
    let mut query = use_signal(String::new);
    let lib = Library::get();
    let data = StudyData::load(&*review.backend.borrow());
    let all = rows(lib, &data);
    let mut shown = search(&all, &query.read());
    let pick = VIEW.pick();
    crate::parts::sort_by(&mut shown, pick.sort, |r| r.unchecked(), |_| false, |r| r.title.clone());
    let (approved, total) =
        all.iter().filter_map(|r| r.counts.as_ref()).fold((0, 0), |(a, t), c| (a + c.approved, t + c.total));

    rsx! {
        style { {CSS.text()} }
        style { {super::CSS.text()} }
        div { class: "lec-page refs",
            div { class: "refs-head",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: crate::i18n::lecture::BACK,
                    title: crate::i18n::lecture::BACK,
                    onclick: move |_| super::close(),
                    span { aria_hidden: "true", "→" }
                }
                header { class: "page-head refs-title",
                    h1 { {tx::TITLE} }
                }
                button {
                    class: "icon-button refs-add",
                    aria_label: tx::ADD_CARD,
                    title: tx::ADD_CARD,
                    onclick: move |_| crate::own::open_new(),
                    span { aria_hidden: "true", "＋" }
                }
            }
            label { class: "search",
                {crate::icons::search()}
                input {
                    r#type: "search",
                    aria_label: tx::SEARCH,
                    dir: "auto",
                    placeholder: tx::SEARCH_HINT,
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }

            section { class: "lec-section",
                h2 { {tx::LECTURES} }
                if shown.is_empty() {
                    p { class: "note", {tx::NO_MATCH} }
                } else {
                    crate::parts::ViewBar { of: VIEW }
                    for group in lib.by_course(shown, |r| r.id.as_str()) {
                        crate::parts::CourseHead { key: "{group.id}", short: group.short().to_string(), title: group.title().to_string() }
                        ul { class: "coll refs-list", "data-layout": pick.layout.as_str(),
                            for r in group.rows.iter() {
                                li { key: "{r.id}",
                                    button {
                                        class: "coll-btn refs-row",
                                        onclick: { let id = r.id.clone(); move |_| super::open(ui, &id) },
                                        span { class: "coll-lead", aria_hidden: "true",
                                            span { class: "coll-badge", "data-tone": if r.counts.is_none() { "quiet" } else { "" }, "{r.code}" }
                                        }
                                        span { class: "coll-main",
                                            bdi { dir: "ltr", lang: "en", class: "coll-title lecture-title", "{r.title}" }
                                            span { class: "coll-sub",
                                                if lib.program.slides(&r.id) { "{crate::i18n::lecture::SLIDES} " } else { "{crate::i18n::lecture::PAGES} " }
                                                span { class: "num-ltr", "{r.pages[0]}–{r.pages[1]}" }
                                                if r.lessons > 0 {
                                                    " · {crate::i18n::count::count(r.lessons, &crate::i18n::count::LESSON)}"
                                                }
                                            }
                                            if let Some((a, t)) = r.checked() {
                                                span { class: "coll-extra",
                                                    span {
                                                        class: "chip",
                                                        "data-tone": if a >= t { "good" } else { "" },
                                                        title: tx::CHECKED_HINT,
                                                        "{tx::CHECKED} " span { class: "num-ltr", "{a}/{t}" }
                                                    }
                                                }
                                            }
                                        }
                                        match r.checked() {
                                            Some((a, t)) => rsx! {
                                                span { class: "coll-end coll-when-compact", title: tx::CHECKED_HINT,
                                                    span { class: "num-ltr", "{a}/{t}" }
                                                }
                                            },
                                            None => rsx! { span { class: "coll-end", span { class: "chip", {tx::WRITING} } } },
                                        }
                                        span { class: "chev", aria_hidden: "true" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "lec-section",
                h2 { {tx::STATUS} }
                div { class: "note refs-status",
                    p { {tx::STATUS_BODY} }
                    p { class: "small",
                        "{tx::CHECKED_ALL} " span { class: "num-ltr", "{approved}/{total}" }
                    }
                }
            }

            crate::own::OwnCardsCard {
                on_study: move |_| {
                    let ids: Vec<String> = crate::own::cards().into_iter().map(|c| c.id).collect();
                    *crate::study::START.write() = Some(crate::study::Start {
                        lectures: Some(vec![steps_core::own::LECTURE.to_string()]),
                        items: Some(ids),
                        minutes: crate::onboarding::session_minutes(),
                    });
                    super::close_all();
                    let mut screen = ui.screen;
                    screen.set(Screen::Study);
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(marks: std::collections::BTreeMap<String, steps_core::content::Status>) -> StudyData {
        StudyData {
            lectures: Library::get()
                .index
                .lectures
                .iter()
                .filter_map(|e| Library::get().lectures.get(&e.id).cloned())
                .collect(),
            events: Vec::new(),
            marks,
            forecast: None,
            clock: steps_core::stats::Clock {
                now_ms: 0,
                zone: steps_core::zone::Zone::fixed(0),
                rollover_hour: 4,
            },
            demo: false,
        }
    }

    fn all() -> Vec<Row> {
        rows(Library::get(), &data(Default::default()))
    }

    #[test]
    fn every_lecture_has_a_row_in_book_order() {
        let rows = all();
        let ids: Vec<&str> = Library::get().index.lectures.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ids);
    }

    #[test]
    fn checked_counts_only_his_content_check() {
        // A ✓ in the verify bar counts; nothing else moves the number.
        let before = all();
        let row = before.iter().find(|r| r.counts.as_ref().is_some_and(|c| c.total > 0)).unwrap();
        let lecture = Library::get().lectures.get(&row.id).unwrap();
        let item = lecture.items.iter().find(|i| i.status != steps_core::content::Status::Approved).unwrap();
        let marks = [(item.id.clone(), steps_core::content::Status::Approved)].into_iter().collect();
        let after = rows(Library::get(), &data(marks));
        let was = row.counts.as_ref().unwrap().approved;
        let now = after.iter().find(|r| r.id == row.id).unwrap().counts.as_ref().unwrap().approved;
        assert_eq!(now, was + 1);
    }

    #[test]
    fn search_matches_titles_numbers_and_topics_in_any_case() {
        let rows = all();
        let first = &rows[0];
        let word = first.title.split_whitespace().next().unwrap().to_uppercase();
        assert!(search(&rows, &word).iter().any(|r| r.id == first.id));
        assert!(search(&rows, &first.code).iter().any(|r| r.id == first.id));
        assert_eq!(search(&rows, "").len(), rows.len());
        assert!(search(&rows, "zzzz-no-such-thing").is_empty());
    }
}
