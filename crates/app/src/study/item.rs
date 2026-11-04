//! The frame around every item: what kind it is, where it comes from in the
//! book, a guideline-update note when there is one, and the renderer.

use dioxus::prelude::*;
use steps_core::content::{Body, Item, Status};
use steps_core::review::Grade;

use super::card::CardItem;
use super::cloze::ClozeItem;
use super::enumerate::EnumerateItem;
use super::essay::EssayItem;
use crate::i18n::study as tx;

/// Set once the item's answer is out (its `GradeRow` is shown), so the frame
/// can keep the book quote, which often is the answer, hidden until then.
#[derive(Clone, Copy)]
pub struct Revealed(pub Signal<bool>);

/// The item came back through Undo: he has seen its answer already, so a
/// recall card opens with it shown.
#[derive(Clone, Copy)]
pub struct Reopened(pub bool);

/// Item kinds this player can show; the session composer picks only these.
/// A chunk that adds a renderer adds its kind here and an arm below.
pub const PLAYABLE: &[&str] = &["card", "cloze", "enumerate", "essay", "table", "order"];

#[component]
pub fn ItemView(
    item: Item,
    lecture_title: String,
    topic_title: Option<String>,
    fresh: bool,
    retry: bool,
    #[props(default)] reopened: bool,
    on_grade: EventHandler<Grade>,
) -> Element {
    let revealed = use_context_provider(|| Revealed(Signal::new(false))).0;
    use_context_provider(|| Reopened(reopened));
    let badge = if retry {
        tx::BADGE_RETRY
    } else if fresh {
        tx::BADGE_NEW
    } else {
        tx::BADGE_REVIEW
    };
    rsx! {
        crate::ui::Card { element: crate::ui::CardElement::Article, class: "st-item", "data-kind": item.body.kind(),
            // Screen readers only: a heading to jump to for each item; the
            // chips below show the same on screen.
            h1 { class: "st-sr-only", "{kind_label(&item.body)} · {lecture_title}" }
            header { class: "st-item-head",
                span { class: "chip st-badge", "data-badge": if retry { "retry" } else if fresh { "new" } else { "review" }, {badge} }
                span { class: "chip", {kind_label(&item.body)} }
                if item.status == Status::Unverified {
                    span { class: "chip st-unverified", title: tx::UNVERIFIED_HINT, {tx::UNVERIFIED} }
                }
            }
            p { class: "st-where muted small",
                bdi {
                    match &topic_title {
                        Some(t) => format!("{lecture_title} · {t}"),
                        None => lecture_title.clone(),
                    }
                }
            }
            div { class: "st-body",
                {render(&item, on_grade)}
            }
            footer { class: "st-item-foot",
                crate::learn::ExplainLink { item_id: item.id.clone() }
                // His note stays out of sight until the answer is (it may hold it).
                crate::notes::NoteChip {
                    anchor: steps_core::note::Anchor::Item(item.id.clone()),
                    page: (item.src.page > 0).then_some(item.src.page),
                    show_text: revealed(),
                }
                // His own cards may have no page.
                if item.src.page > 0 {
                    crate::hover::BookPage {
                        page: item.src.page,
                        quote: if revealed() { item.src.quote.clone() } else { None },
                        held: !revealed() && item.src.quote.is_some(),
                        of: item.id.clone(),
                        hook: match &item.body {
                            steps_core::content::Body::Enumerate { hook: Some(h), .. } => {
                                Some(crate::pagesheet::Hook { text: h.clone(), parts: Vec::new() })
                            }
                            _ => None,
                        },
                    }
                }
                if let Some(q) = item.src.quote.as_ref().filter(|_| revealed()) {
                    details { class: "st-quote",
                        summary { {tx::BOOK_QUOTE} }
                        crate::pagesheet::Quote { text: q.clone() }
                    }
                }
            }
            if let Some(update) = &item.update {
                details { class: "st-update",
                    summary {
                        strong { {tx::UPDATE_TITLE} }
                        span { class: "muted small", " · {tx::UPDATE_AFTER}" }
                    }
                    p { lang: "en", dir: "ltr", "{update.text}" }
                    p { class: "muted small", lang: "en", dir: "ltr", "{update.reference}" }
                    p { class: "small", {tx::UPDATE_BOOK_WINS} }
                }
            }
        }
    }
}

fn render(item: &Item, on_grade: EventHandler<Grade>) -> Element {
    let item = item.clone();
    match &item.body {
        Body::Card { .. } => rsx! { CardItem { item, on_grade } },
        Body::Cloze { .. } => rsx! { ClozeItem { item, on_grade } },
        Body::Enumerate { .. } => rsx! { EnumerateItem { item, on_grade } },
        Body::Essay { .. } => rsx! { EssayItem { item, on_grade } },
        Body::Table { .. } => rsx! { crate::table::TableItem { item, on_grade } },
        Body::Order { .. } => rsx! { crate::order::OrderItem { item, on_grade } },
        // Not in PLAYABLE yet: other chunks bring these screens.
        _ => rsx! {
            p { class: "muted", {tx::NOT_HERE_YET} }
            button { class: "btn btn-secondary btn-lg", onclick: move |_| on_grade.call(Grade::Good), {tx::SKIP} }
        },
    }
}

fn kind_label(body: &Body) -> &'static str {
    match body {
        Body::Card { .. } => tx::KIND_CARD,
        Body::Cloze { .. } => tx::KIND_CLOZE,
        Body::Enumerate { .. } => tx::KIND_ENUMERATE,
        Body::Essay { .. } => tx::KIND_ESSAY,
        Body::Mcq { .. } => tx::KIND_MCQ,
        Body::Case { .. } => tx::KIND_CASE,
        Body::Table { .. } => tx::KIND_TABLE,
        Body::Order { .. } => tx::KIND_ORDER,
    }
}

/// One line that names an item in a list (the summary's "coming back" list).
pub fn short_text(item: &Item) -> String {
    let s = match &item.body {
        Body::Card { front, .. } => front.clone(),
        Body::Cloze { text, .. } => {
            let mut out = String::new();
            let mut rest = text.as_str();
            while let (Some(a), Some(b)) = (rest.find("{{"), rest.find("}}")) {
                if b < a {
                    break;
                }
                out.push_str(&rest[..a]);
                out.push_str("___");
                rest = &rest[b + 2..];
            }
            out.push_str(rest);
            out
        }
        Body::Mcq { stem, .. } | Body::Case { stem, .. } => stem.clone(),
        Body::Enumerate { prompt, .. } | Body::Essay { prompt, .. } | Body::Order { prompt, .. } => {
            prompt.clone()
        }
        Body::Table { title, .. } => title.clone(),
    };
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > 90 { format!("{}…", s.chars().take(88).collect::<String>()) } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_hides_cloze_answers() {
        let item: Item = serde_yaml_ng::from_str(
            "{ id: x.1, kind: cloze, text: 'PFO is seen in {{c1::25%}} of people', src: { page: 1 }, status: unverified }",
        )
        .unwrap();
        assert_eq!(short_text(&item), "PFO is seen in ___ of people");
    }

    #[test]
    fn every_kind_has_a_label_and_a_short_line() {
        let lec: steps_core::content::Lecture =
            serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap();
        let mut kinds = std::collections::BTreeSet::new();
        for item in &lec.items {
            assert!(!kind_label(&item.body).is_empty());
            let line = short_text(item);
            assert!(!line.is_empty() && line.chars().count() <= 89, "{line}");
            kinds.insert(item.body.kind());
        }
        assert!(kinds.contains("case") && kinds.contains("essay") && kinds.contains("mcq"));
        // A long stem is cut to one line.
        let long: Item = serde_yaml_ng::from_str(&format!(
            "{{ id: x.2, kind: card, front: '{}', back: b, src: {{ page: 1 }}, status: unverified }}",
            "word ".repeat(40)
        ))
        .unwrap();
        assert!(short_text(&long).ends_with('…'));
    }
}
