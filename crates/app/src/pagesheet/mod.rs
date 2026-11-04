//! A book page's sheet. Every "ص N" chip in the app opens it (Mohamed,
//! 2026-10-28: no dead chips or links): the book quote the chip itself
//! carries, when it has one, the lecture the page is in, what Steps has made from that page,
//! and two ways on: a session on just that page, or the lecture's page.
//!
//! It opens over whatever is on screen (a lesson, a running session, a
//! paper) and changes nothing until he picks one of its buttons.

use dioxus::prelude::*;
use steps_core::review::StudyStore;

use crate::content::Library;
use crate::i18n::book as tx;
use crate::i18n::count::{CASE, ITEM, QUESTION, count};
use crate::look::{Css, css};
use crate::store::use_review;
use crate::{Screen, Ui};

const CSS: Css = css!("src/pagesheet/pagesheet.css");

/// The open page, where its number counts ([`Scope`]), the quote its chip
/// carried, whether a quote is held back until he answers (it would give
/// the answer away), and the تحشيشة of the list it cites.
type Open = (u32, Scope, Option<String>, bool, Option<Hook>);

/// The تحشيشة of the list or table a quote holds, shown under the quote
/// (Mohamed, 2026-11-02), with its letters as a small table when it has some.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hook {
    pub text: String,
    /// A mnemonic's letters and what each stands for.
    pub parts: Vec<(String, String)>,
}

/// Where a cited page number counts. Each course's book numbers its own
/// pages (Gastroenterology starts again at 1), and a course that cites
/// slides numbers them per lecture.
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    /// A page of this course's book (`hb`).
    Book(String),
    /// A slide of this lecture (`res.L02`).
    Slides(String),
}
static OPEN: GlobalSignal<Option<Open>> = Signal::global(|| None);

/// Opens the sheet for page `page` cited by `of` (an item, lesson step or
/// lecture id; empty for the primary book). A course that cites slides
/// numbers them per lecture, so the sheet stays in `of`'s lecture.
pub fn open(page: u32, of: &str, quote: Option<String>, held: bool, hook: Option<Hook>) {
    document::eval("window.__stepsHover && window.__stepsHover.hide();");
    crate::abbrsheet::close();
    *OPEN.write() =
        Some((page, scope(Library::get(), of), quote.filter(|q| !q.trim().is_empty()), held, hook));
}

/// Where `of`'s page numbers count; empty `of` is the primary book.
pub fn scope(lib: &Library, of: &str) -> Scope {
    use steps_core::program::{course_of, lecture_of};
    if of.is_empty() {
        Scope::Book(lib.program.courses.first().map_or("cvs", |c| c.id.as_str()).to_string())
    } else if lib.program.slides(of) {
        Scope::Slides(lecture_of(of).to_string())
    } else {
        Scope::Book(course_of(of).to_string())
    }
}

pub fn close() {
    *OPEN.write() = None;
}

/// What Steps has made from one book page.
#[derive(Debug, Default, PartialEq)]
pub struct PageContent {
    /// The lecture whose page range holds the page.
    pub lecture: Option<String>,
    /// Items a session can play, in book order.
    pub playable: Vec<String>,
    pub mcqs: usize,
    pub cases: usize,
}

/// Collects the page's items from the library, leaving out hidden ones.
/// `scope` keeps it to one course's book or one lecture's slides.
pub fn page_content(lib: &Library, page: u32, scope: &Scope, hidden: impl Fn(&str) -> bool) -> PageContent {
    let in_scope = |id: &str| match scope {
        Scope::Slides(l) => id == l,
        Scope::Book(c) => steps_core::program::course_of(id) == c && !lib.program.slides(id),
    };
    let lecture = lib
        .index
        .lectures
        .iter()
        .find(|e| in_scope(&e.id) && (e.pages[0]..=e.pages[1]).contains(&page))
        .map(|e| e.id.clone());
    let mut out = PageContent { lecture, ..Default::default() };
    for item in lib.lectures.values().filter(|l| in_scope(&l.lecture)).flat_map(|l| l.items.iter()) {
        if item.src.page != page || hidden(&item.id) {
            continue;
        }
        let kind = item.body.kind();
        if crate::study::PLAYABLE.contains(&kind) {
            out.playable.push(item.id.clone());
        }
        match kind {
            "mcq" => out.mcqs += 1,
            "case" => out.cases += 1,
            _ => {}
        }
    }
    out
}

/// The sheet, drawn once by the shell; empty while no page is open.
#[component]
pub fn BookSheet() -> Element {
    let ui = use_context::<Ui>();
    let review = use_review();
    let Some((page, scope, chip_quote, held, hook)) = OPEN.read().clone() else {
        return rsx! {};
    };
    let lib = Library::get();
    let content = {
        let b = review.backend.borrow();
        page_content(lib, page, &scope, |id| b.is_hidden(id))
    };
    let title = if matches!(scope, Scope::Slides(_)) { tx::SLIDE_TITLE } else { tx::TITLE };
    // Only the chip's own quote: another item's line from the same page
    // reads as the source of this one and points at the wrong fact
    // (Mohamed, 2026-10-29).
    let quote = chip_quote;
    let entry = content.lecture.as_ref().and_then(|id| lib.index.lectures.iter().find(|e| &e.id == id));
    let playable = content.playable.clone();
    let n = playable.len();
    let lecture_id = content.lecture.clone();

    rsx! {
        style { {CSS.text()} }
        crate::sheet::Sheet {
            label_id: "bk-title",
            on_close: move |_| close(),
            head: rsx! {
                div { class: "bk-head",
                    span { class: "bk-page", aria_hidden: "true", bdi { "{page}" } }
                    div {
                        h2 { id: "bk-title", "{title} " bdi { "{page}" } }
                        if let Some(e) = entry {
                            p { class: "muted small",
                                bdi { {lib.crumb(&e.id)} }
                            }
                        } else {
                            p { class: "muted small", {tx::NO_LECTURE} }
                        }
                    }
                }
            },
            div { class: "bk-body",
                if let Some(q) = &quote {
                    figure { class: "bk-quote",
                        Quote { text: q.clone() }
                    }
                    if let Some(h) = hook.filter(|h| !h.text.trim().is_empty()) {
                        HookCard { hook: h }
                    }
                } else if held {
                    p { class: "bk-quote muted", {tx::QUOTE_HELD} }
                }
                if n + content.mcqs + content.cases > 0 {
                    p { class: "bk-made",
                        {tx::MADE}
                        " "
                        span { class: "bk-counts",
                            {
                                let mut parts = Vec::new();
                                if n > 0 { parts.push(count(n, &ITEM)); }
                                if content.mcqs > 0 { parts.push(format!("{} MCQ", count(content.mcqs, &QUESTION))); }
                                if content.cases > 0 { parts.push(count(content.cases, &CASE)); }
                                parts.join(" · ")
                            }
                        }
                    }
                } else {
                    p { class: "muted", {tx::NOTHING_YET} }
                }
            }
            div { class: "bk-foot",
                if n > 0 {
                    button {
                        class: "btn btn-primary",
                        onclick: {
                            let lecture_id = lecture_id.clone();
                            move |_| {
                                *crate::study::START.write() = Some(crate::study::Start {
                                    lectures: lecture_id.clone().map(|l| vec![l]),
                                    items: Some(playable.clone()),
                                    minutes: crate::onboarding::session_minutes(),
                                });
                                close();
                                crate::lecture::close_all();
                                crate::learn::forget();
                                let mut screen = ui.screen;
                                screen.set(Screen::Study);
                            }
                        },
                        "{tx::STUDY} ({n})"
                    }
                }
                if let Some(id) = lecture_id {
                    button {
                        class: "btn btn-tonal",
                        onclick: move |_| {
                            close();
                            crate::lecture::open(ui, &id);
                        },
                        {tx::OPEN_LECTURE}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cvs() -> Scope {
        Scope::Book("cvs".into())
    }

    #[test]
    fn a_book_page_stays_in_its_own_course() {
        // First Aid is one book: a page is looked up in the item's own course.
        let lib = Library::get();
        assert_eq!(scope(lib, "cvs.L01.004"), cvs());
        let other = page_content(lib, 284, &Scope::Book("renal".into()), |_| false);
        assert!(other.playable.iter().all(|id| id.starts_with("renal.")), "{other:?}");
    }

    #[test]
    fn a_page_is_in_its_lecture_and_has_items() {
        let c = page_content(Library::get(), 284, &cvs(), |_| false);
        assert_eq!(c.lecture.as_deref(), Some("cvs.L01"));
        assert!(!c.playable.is_empty());
        assert!(c.playable.iter().all(|id| id.starts_with("cvs.L01.")));
    }

    #[test]
    fn hidden_items_are_left_out() {
        let all = page_content(Library::get(), 284, &cvs(), |_| false);
        let first = all.playable[0].clone();
        let some = page_content(Library::get(), 284, &cvs(), |id| id == first);
        assert_eq!(some.playable.len(), all.playable.len() - 1);
    }

    #[test]
    fn an_empty_id_opens_the_first_course() {
        let lib = Library::get();
        assert_eq!(scope(lib, ""), Scope::Book("bio".into()));
        assert_eq!(page_content(lib, 290, &cvs(), |_| false).lecture.as_deref(), Some("cvs.L03"));
    }
}

/// A book quote laid out as the page has it: a title, paragraphs, and
/// tables drawn as tables (Mohamed, 2026-11-02: "جدول مش نص تحت بعضه").
/// A list with groups becomes a two-column table, the group in the first
/// column beside each of its rows; a titled list without groups, a
/// one-column table; an untitled one, bullets.
#[component]
pub fn Quote(text: String) -> Element {
    use steps_core::quote::{Block, parse};
    let blocks = parse(&text);
    let titled = blocks.iter().any(|b| matches!(b, Block::Title(_)));
    rsx! {
        div { class: "bk-q", lang: "en", dir: "ltr",
            for b in blocks {
                match b {
                    Block::Title(t) => rsx! { p { class: "bk-q-title", "{t}" } },
                    Block::Para(t) => rsx! { p { class: "bk-q-para", "{t}" } },
                    Block::Grid { head, rows } => rsx! {
                        div { class: "bk-q-scroll",
                            table { class: "bk-q-table",
                                thead { tr { for h in head { th { "{h}" } } } }
                                tbody {
                                    for r in rows {
                                        tr { for c in r { td { "{c}" } } }
                                    }
                                }
                            }
                        }
                    },
                    Block::List(rows) if rows.iter().any(|r| r.group) => {
                        rsx! { GroupTable { rows } }
                    }
                    Block::List(rows) if titled => rsx! {
                        table { class: "bk-q-table",
                            tbody { for r in rows { tr { td { class: "d{r.depth.min(3)}", "{r.text}" } } } }
                        }
                    },
                    Block::List(rows) => rsx! {
                        ul { class: "bk-q-list",
                            for r in rows { li { class: "d{r.depth.min(3)}", "{r.text}" } }
                        }
                    },
                }
            }
        }
    }
}

/// A group heading and how many rows it spans.
type GroupCell = Option<(String, usize)>;

/// Group rows of a list as table rows: (group cell, its rows).
fn grouped(rows: &[steps_core::quote::Row]) -> Vec<(GroupCell, Vec<String>)> {
    let mut out: Vec<(GroupCell, Vec<String>)> = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        let r = &rows[i];
        if r.group {
            let mut kids = Vec::new();
            let mut j = i + 1;
            while j < rows.len() && rows[j].depth > r.depth {
                let pad = "\u{2003}".repeat(rows[j].depth - r.depth - 1);
                kids.push(format!("{pad}{}", rows[j].text));
                j += 1;
            }
            out.push((Some((r.text.trim_end_matches(':').to_string(), kids.len())), kids));
            i = j;
        } else {
            out.push((None, vec![r.text.clone()]));
            i += 1;
        }
    }
    out
}

#[component]
fn GroupTable(rows: Vec<steps_core::quote::Row>) -> Element {
    let groups = grouped(&rows);
    rsx! {
        table { class: "bk-q-table grouped",
            tbody {
                for (g, kids) in groups {
                    match g {
                        Some((name, n)) => rsx! {
                            for (k, kid) in kids.iter().enumerate() {
                                tr {
                                    if k == 0 {
                                        th { scope: "rowgroup", rowspan: "{n}", "{name}" }
                                    }
                                    td { "{kid}" }
                                }
                            }
                        },
                        None => rsx! { tr { td { class: "solo", colspan: "2", "{kids[0]}" } } },
                    }
                }
            }
        }
    }
}

/// The تحشيشة under a quote; a mnemonic's letters in a small table.
#[component]
fn HookCard(hook: Hook) -> Element {
    rsx! {
        div { class: "bk-hook",
            p { class: "bk-hook-label", {tx::HOOK} }
            crate::learn::Rich { text: hook.text.clone(), class: "bk-hook-text ls-rich" }
            if !hook.parts.is_empty() {
                table { class: "bk-hook-parts", dir: "ltr", lang: "en",
                    tbody {
                        for (key, text) in hook.parts.iter() {
                            tr { th { "{key}" } td { "{text}" } }
                        }
                    }
                }
            }
        }
    }
}
