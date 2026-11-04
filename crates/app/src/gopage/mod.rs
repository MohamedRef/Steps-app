//! Go to a First Aid page: type the printed page number he has open in the
//! book (or a range like 300–305) and get that page's sheet, the same one a
//! "ص N" chip opens: its lecture, what Steps made from it, a session on it.
//! From More, the tools rail, or Ctrl/⌘ + G. Inspired by Lecturio's
//! Bookmatcher and Boards and Beyond's search by First Aid page.

use dioxus::prelude::*;

use crate::content::Library;
use crate::i18n::gopage as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/gopage/gopage.css");

static OPEN: GlobalSignal<bool> = Signal::global(|| std::env::var_os("STEPS_GO_PAGE").is_some());
static TEXT: GlobalSignal<String> = Signal::global(String::new);

pub fn open(_ui: crate::Ui) {
    crate::pagesheet::close();
    *TEXT.write() = String::new();
    *OPEN.write() = true;
}

pub fn close() {
    if *OPEN.peek() {
        *OPEN.write() = false;
    }
}

/// What a typed page leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A printed page inside a lecture's range, with that lecture.
    Page { page: u32, lecture: String },
    /// Outside every lecture: the closest page that has one.
    Nearest { asked: u32, page: u32, lecture: String },
    /// Not a page number.
    Invalid,
}

/// The first number of "300", "300-305" or "ص ٣٠٠–٣٠٥" (Arabic digits too);
/// a range opens its first page.
pub fn parse(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .map(|c| match c {
            '٠'..='٩' => char::from_digit(c as u32 - '٠' as u32, 10).unwrap_or(c),
            _ => c,
        })
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok().filter(|n| *n > 0)
}

/// Where a printed page sits in the index: its lecture, or the nearest page
/// that belongs to one.
pub fn target(lib: &Library, text: &str) -> Target {
    let Some(page) = parse(text) else { return Target::Invalid };
    let ranges = lib.index.lectures.iter().map(|e| (e.pages, e.id.as_str()));
    if let Some((_, id)) = ranges.clone().find(|(p, _)| (p[0]..=p[1]).contains(&page)) {
        return Target::Page { page, lecture: id.to_string() };
    }
    let nearest = ranges.flat_map(|(p, id)| [(p[0], id), (p[1], id)]).min_by_key(|(p, _)| p.abs_diff(page));
    match nearest {
        Some((p, id)) => Target::Nearest { asked: page, page: p, lecture: id.to_string() },
        None => Target::Invalid,
    }
}

fn go(page: u32, lecture: &str) {
    close();
    crate::pagesheet::open(page, lecture, None, false, None);
}

/// The page box, drawn once by the shell; empty while closed.
#[component]
pub fn GoPage() -> Element {
    if !OPEN() {
        return rsx! {};
    }
    let lib = Library::get();
    let text = TEXT();
    let found = if text.trim().is_empty() { None } else { Some(target(lib, &text)) };
    let can_go = matches!(found, Some(Target::Page { .. }));
    let submit = move |e: FormEvent| {
        e.prevent_default();
        match target(Library::get(), &TEXT.peek()) {
            Target::Page { page, lecture } => go(page, &lecture),
            Target::Nearest { .. } | Target::Invalid => {}
        }
    };
    rsx! {
        style { {CSS.text()} }
        crate::sheet::Sheet {
            label_id: "gp-title",
            on_close: move |_| close(),
            head: rsx! {
                h2 { id: "gp-title", class: "gp-title", {tx::TITLE} }
            },
            form { class: "gp-form", onsubmit: submit,
                label { class: "muted small", r#for: "gp-input", {tx::HINT} }
                div { class: "gp-row",
                    input {
                        id: "gp-input",
                        class: "gp-input",
                        r#type: "text",
                        inputmode: "numeric",
                        dir: "ltr",
                        autocomplete: "off",
                        placeholder: tx::PLACEHOLDER,
                        value: "{text}",
                        onmounted: move |e: MountedEvent| async move {
                            let _ = e.set_focus(true).await;
                        },
                        oninput: move |e| *TEXT.write() = e.value(),
                    }
                    button { class: "btn btn-primary", r#type: "submit", disabled: !can_go,
                        {tx::GO}
                    }
                }
                match found {
                    Some(Target::Page { lecture, .. }) => rsx! {
                        p { class: "small gp-where", bdi { "{lib.crumb(&lecture)}" } }
                    },
                    Some(Target::Nearest { asked, page, lecture }) => rsx! {
                        div { class: "note gp-note", role: "status",
                            p { class: "small", {tx::outside(asked)} }
                            button {
                                class: "btn btn-tonal",
                                r#type: "button",
                                onclick: move |_| go(page, &lecture),
                                {tx::nearest(page)}
                            }
                        }
                    },
                    Some(Target::Invalid) => rsx! {
                        p { class: "muted small", role: "status", {tx::INVALID} }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_text_reads_numbers_ranges_and_arabic_digits() {
        assert_eq!(parse("300"), Some(300));
        assert_eq!(parse(" 300–305 "), Some(300));
        assert_eq!(parse("ص ٣٠٠"), Some(300));
        assert_eq!(parse("p. 42"), Some(42));
        assert_eq!(parse("0"), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn printed_pages_find_their_lecture() {
        let lib = Library::get();
        // First Aid's printed page 300 opens Cardiovascular › Pathology.
        let cvs = lib.index.lectures.iter().find(|e| e.id == "cvs.L04").unwrap();
        assert_eq!(
            target(lib, &cvs.pages[0].to_string()),
            Target::Page { page: cvs.pages[0], lecture: "cvs.L04".into() }
        );
        assert_eq!(target(lib, "284"), Target::Page { page: 284, lecture: "cvs.L01".into() });
        assert_eq!(target(lib, "nope"), Target::Invalid);
        // Before Section II: the nearest page that has a lecture.
        let first = &lib.index.lectures[0];
        assert_eq!(
            target(lib, "5"),
            Target::Nearest { asked: 5, page: first.pages[0], lecture: first.id.clone() }
        );
    }

    #[component]
    fn Open(text: String) -> Element {
        use_hook(|| {
            *OPEN.write() = true;
            *TEXT.write() = text.clone();
        });
        rsx! { GoPage {} }
    }

    fn render(text: &str) -> String {
        let mut dom = VirtualDom::new_with_props(Open, OpenProps { text: text.into() });
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    #[test]
    fn the_box_names_the_lecture_or_offers_the_nearest_page() {
        let html = render("284");
        assert!(html.contains(tx::TITLE) && html.contains(tx::GO));
        assert!(html.contains("Embryology"), "{html}");
        let html = render("5");
        assert!(html.contains(&tx::outside(5)));
        assert!(html.contains(&tx::nearest(Library::get().index.lectures[0].pages[0])));
        assert!(render("x").contains(tx::INVALID));
    }
}
