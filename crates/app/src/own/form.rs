//! The "add a card" form (a sheet over any screen), the list on the Study
//! screen, and the Android share inbox that opens the form with a draft.

use dioxus::prelude::*;
use steps_core::own::{self as core_own, OwnCard, Problem};

use crate::content::Library;
use crate::i18n::own as tx;
use crate::look::{Css, css};
use crate::store::{self, own::new_id, use_review};

const CSS: Css = css!("src/own/own.css");

/// The card being written or edited; the sheet shows while it's set.
pub static DRAFT: GlobalSignal<Option<Draft>> = Signal::global(|| None);
/// A share that arrived while he was writing a card: it waits, and opens
/// once that sheet closes, so nothing he typed is replaced.
static QUEUED: GlobalSignal<Option<Draft>> = Signal::global(|| None);

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub card: OwnCard,
    pub kind: DraftKind,
    /// A shared passage was longer than a card's back and its end was cut.
    pub trimmed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftKind {
    New,
    Edit,
    Shared,
}

/// Opens the form on an empty card.
pub fn open_new() {
    let card = OwnCard {
        id: new_id(),
        front: String::new(),
        back: String::new(),
        lecture: None,
        page: None,
        source: None,
        created_ms: store::now_ms(),
    };
    *DRAFT.write() = Some(Draft { card, kind: DraftKind::New, trimmed: false });
}

/// Mounted once in the app: picks up text shared from other apps (Android),
/// and shows the form sheet whenever there is a draft.
#[component]
pub fn ShareInbox() -> Element {
    use_future(|| async move {
        if !cfg!(target_os = "android") {
            return;
        }
        // A share brings Steps to the front, so look each time the page
        // becomes visible (and once at start, for a cold start), not on a timer.
        let mut eval = document::eval(
            r#"
            const send = () => { if (!document.hidden) dioxus.send(true); };
            document.addEventListener("visibilitychange", send);
            send();
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            let Some(shared) = super::share::take() else { continue };
            let (card, trimmed) = core_own::draft_from_shared(
                &shared.text,
                shared.source().as_deref(),
                new_id(),
                store::now_ms(),
            );
            let draft = Draft { card, kind: DraftKind::Shared, trimmed };
            if DRAFT.peek().is_some() {
                *QUEUED.write() = Some(draft);
            } else {
                *DRAFT.write() = Some(draft);
            }
        }
    });
    // The sheet closed: the share that waited opens now.
    use_effect(|| {
        if DRAFT.read().is_none()
            && let Some(next) = QUEUED.write().take()
        {
            *DRAFT.write() = Some(next);
        }
    });
    rsx! {
        if let Some(draft) = DRAFT() {
            Sheet { key: "{draft.card.id}", draft }
        }
    }
}

#[component]
fn Sheet(draft: Draft) -> Element {
    let review = use_review();
    let mut card = use_signal(|| draft.card.clone());
    let mut problem = use_signal(|| None::<String>);
    let title = match draft.kind {
        DraftKind::New => tx::FORM_NEW,
        DraftKind::Edit => tx::FORM_EDIT,
        DraftKind::Shared => tx::FORM_SHARED,
    };
    let lib = Library::get();
    let lectures: Vec<(String, String)> =
        lib.index.lectures.iter().map(|e| (e.id.clone(), lib.crumb(&e.id))).collect();
    let mut invalid = use_signal(|| None::<(Field, Problem)>);
    let close = move || *DRAFT.write() = None;
    // A failed check or save keeps every word he typed: the sheet stays
    // open on the same draft, with the problem next to its field.
    let save = move |_| match validate(&card.read()) {
        Err(bad) => {
            invalid.set(Some(bad));
            problem.set(None);
        }
        Ok(c) => {
            invalid.set(None);
            match review.backend.borrow_mut().save_own(&c) {
                Ok(()) => *DRAFT.write() = None,
                Err(e) => problem.set(Some(format!("{} {e}", tx::FAILED))),
            }
        }
    };
    let c = card();
    let bad = invalid();
    let err = move |f: Field| bad.filter(|(at, _)| *at == f).map(|(_, p)| problem_text(p));
    let aria = |f: Field| if err(f).is_some() { "true" } else { "false" };
    rsx! {
        style { {CSS.text()} }
        div { class: "sheet-backdrop own-backdrop", onclick: move |_| close() }
        div {
            class: "sheet own-sheet",
            role: "dialog",
            aria_modal: "true",
            aria_label: title,
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    close();
                }
            },
            div { class: "sheet-head",
                h2 { {title} }
                button {
                    class: "icon-button",
                    "data-back": "true",
                    aria_label: tx::CANCEL,
                    title: tx::CANCEL,
                    onclick: move |_| close(),
                    span { aria_hidden: "true", "✕" }
                }
            }
            if draft.kind == DraftKind::Shared {
                p { class: "muted small", {tx::SHARED_HINT} }
            }
            if draft.trimmed {
                p { class: "muted small", {tx::SHARED_TRIMMED} }
            }
            if QUEUED.read().is_some() {
                p { class: "small own-waiting", role: "status", {tx::SHARE_WAITING} }
            }
            label { class: "field own-field", "data-invalid": aria(Field::Front),
                span { class: "own-label", {tx::FRONT}
                    bdi { class: "own-count", dir: "ltr", "{c.front.chars().count()}/{core_own::MAX_FRONT}" }
                }
                textarea {
                    dir: "auto",
                    lang: "en",
                    rows: 3,
                    autofocus: true,
                    placeholder: tx::FRONT_HINT,
                    "aria-invalid": aria(Field::Front),
                    value: "{c.front}",
                    oninput: move |e| card.write().front = e.value(),
                }
                if let Some(m) = err(Field::Front) {
                    span { class: "field-error", role: "alert", {m} }
                }
            }
            label { class: "field own-field", "data-invalid": aria(Field::Back),
                span { class: "own-label", {tx::BACK}
                    bdi { class: "own-count", dir: "ltr", "{c.back.chars().count()}/{core_own::MAX_BACK}" }
                }
                textarea {
                    dir: "auto",
                    lang: "en",
                    rows: 7,
                    placeholder: tx::BACK_HINT,
                    "aria-invalid": aria(Field::Back),
                    value: "{c.back}",
                    oninput: move |e| card.write().back = e.value(),
                }
                if let Some(m) = err(Field::Back) {
                    span { class: "field-error", role: "alert", {m} }
                }
            }
            div { class: "own-row",
                label { class: "field own-field",
                    span { {tx::LECTURE} }
                    select {
                        onchange: move |e| {
                            let v = e.value();
                            card.write().lecture = (!v.is_empty()).then_some(v);
                        },
                        option { value: "", selected: c.lecture.is_none(), {tx::NO_LECTURE} }
                        for (id , name) in lectures {
                            option {
                                key: "{id}",
                                value: "{id}",
                                selected: c.lecture.as_deref() == Some(id.as_str()),
                                "{id.rsplit('.').next().unwrap_or_default()} · {name}"
                            }
                        }
                    }
                }
                label { class: "field own-field own-page",
                    span { {tx::PAGE} }
                    input {
                        r#type: "number",
                        inputmode: "numeric",
                        min: "1",
                        dir: "ltr",
                        value: c.page.map(|p| p.to_string()).unwrap_or_default(),
                        oninput: move |e| card.write().page = e.value().trim().parse().ok(),
                    }
                }
            }
            label { class: "field own-field", "data-invalid": aria(Field::Source),
                span { {tx::SOURCE} }
                input {
                    dir: "auto",
                    placeholder: tx::SOURCE_HINT,
                    "aria-invalid": aria(Field::Source),
                    value: c.source.clone().unwrap_or_default(),
                    oninput: move |e| card.write().source = Some(e.value()),
                }
                if let Some(m) = err(Field::Source) {
                    span { class: "field-error", role: "alert", {m} }
                }
            }
            // How it will look in a session: the question, then the answer.
            if !(c.front.trim().is_empty() && c.back.trim().is_empty()) {
                details { class: "own-preview",
                    summary { {tx::PREVIEW} }
                    div { class: "own-preview-card",
                        bdi { lang: "en", dir: "auto", class: "own-preview-front", "{c.front.trim()}" }
                        hr {}
                        bdi { lang: "en", dir: "auto", class: "own-preview-back", "{c.back.trim()}" }
                        if c.lecture.is_some() || c.page.is_some() || c.source.as_deref().is_some_and(|s| !s.trim().is_empty()) {
                            span { class: "muted small",
                                if let Some(l) = &c.lecture {
                                    bdi { dir: "ltr", "{l.rsplit('.').next().unwrap_or_default()}" }
                                    " "
                                }
                                if let Some(p) = c.page {
                                    "{crate::i18n::cite::page(p)} "
                                }
                                if let Some(s) = c.source.as_deref().filter(|s| !s.trim().is_empty()) {
                                    "{tx::FROM} " bdi { "{s.trim()}" }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(p) = problem() {
                p { class: "field-error", role: "alert", "{p}" }
            }
            div { class: "own-actions",
                button { class: "btn btn-primary", onclick: save, {tx::SAVE} }
                button { class: "btn btn-secondary", onclick: move |_| close(), {tx::CANCEL} }
            }
        }
    }
}

/// The field a problem belongs to, so the message sits under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Front,
    Back,
    Source,
}

/// The tidied card to save, or the first problem and its field. The draft
/// itself is left as typed.
fn validate(draft: &OwnCard) -> Result<OwnCard, (Field, Problem)> {
    let c = draft.clone().tidy();
    c.check().map(|()| c).map_err(|p| {
        let field = match p {
            Problem::EmptyFront | Problem::FrontTooLong => Field::Front,
            Problem::EmptyBack | Problem::BackTooLong => Field::Back,
            Problem::SourceTooLong => Field::Source,
        };
        (field, p)
    })
}

fn problem_text(p: Problem) -> &'static str {
    match p {
        Problem::EmptyFront => tx::E_FRONT,
        Problem::EmptyBack => tx::E_BACK,
        Problem::FrontTooLong => tx::E_FRONT_LONG,
        Problem::BackTooLong => tx::E_BACK_LONG,
        Problem::SourceTooLong => tx::E_SOURCE_LONG,
    }
}

/// His cards on the Study screen: add, edit, delete, study just these.
#[component]
pub fn OwnCardsCard(on_study: EventHandler<()>) -> Element {
    let review = use_review();
    // Re-read whenever the review store changes (a save, an import, a sync).
    let _ = review.revision.read();
    let cards = super::cards();
    let mut confirm = use_signal(|| None::<String>);
    // A delete that didn't land keeps its confirm row open, with the reason.
    let mut delete_failed = use_signal(|| None::<String>);
    let delete = {
        let review = review.clone();
        move |id: String| match review.backend.borrow_mut().delete_own(&id) {
            Ok(()) => {
                crate::store::write_recovered(&format!("deleting own card {id}"));
                delete_failed.set(None);
                confirm.set(None);
            }
            Err(e) => {
                crate::store::write_failed(&format!("deleting own card {id}"), &e);
                delete_failed.set(Some(e.to_string()));
            }
        }
    };
    rsx! {
        style { {CSS.text()} }
        crate::ui::Card { class: "own",
            div { class: "own-head",
                h2 { {tx::TITLE} }
                if !cards.is_empty() {
                    span { class: "chip", {tx::cards(cards.len())} }
                }
            }
            div { class: "own-actions",
                button { class: "btn btn-primary", onclick: move |_| open_new(), {tx::NEW} }
                if !cards.is_empty() {
                    button { class: "btn btn-secondary", onclick: move |_| on_study.call(()), {tx::STUDY_THEM} }
                }
            }
            if cards.is_empty() {
                p { class: "muted small", {tx::NONE_YET} }
            }
            ul { class: "own-list",
                for c in cards {
                    li { key: "{c.id}",
                        div { class: "own-text",
                            bdi { lang: "en", dir: "auto", class: "own-front", "{c.front}" }
                            span { class: "muted small",
                                if let Some(l) = &c.lecture {
                                    bdi { dir: "ltr", "{l.rsplit('.').next().unwrap_or_default()}" }
                                    " "
                                }
                                if let Some(s) = &c.source {
                                    "{tx::FROM} "
                                    bdi { "{s}" }
                                }
                            }
                        }
                        if confirm().as_deref() == Some(c.id.as_str()) {
                            div { class: "own-actions",
                                span { class: "small", {tx::DELETE_SURE} }
                                button {
                                    class: "btn btn-danger",
                                    onclick: {
                                        let id = c.id.clone();
                                        let mut delete = delete.clone();
                                        move |_| delete(id.clone())
                                    },
                                    {tx::DELETE_YES}
                                }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| {
                                        delete_failed.set(None);
                                        confirm.set(None);
                                    },
                                    {tx::CANCEL}
                                }
                            }
                            if let Some(e) = delete_failed() {
                                crate::persistence::WriteFailed {
                                    message: tx::DELETE_FAILED,
                                    detail: e,
                                    on_retry: {
                                        let id = c.id.clone();
                                        let mut delete = delete.clone();
                                        move |_| delete(id.clone())
                                    },
                                }
                            }
                        } else {
                            div { class: "own-actions",
                                button {
                                    class: "btn btn-secondary",
                                    onclick: {
                                        let c = c.clone();
                                        move |_| *DRAFT.write() = Some(Draft { card: c.clone(), kind: DraftKind::Edit, trimmed: false })
                                    },
                                    {tx::EDIT}
                                }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: {
                                        let id = c.id.clone();
                                        move |_| {
                                            delete_failed.set(None);
                                            confirm.set(Some(id.clone()))
                                        }
                                    },
                                    {tx::DELETE}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(front: &str, back: &str) -> OwnCard {
        OwnCard {
            id: "own.t".into(),
            front: front.into(),
            back: back.into(),
            lecture: None,
            page: Some(12),
            source: Some(" notes ".into()),
            created_ms: 1,
        }
    }

    #[test]
    fn a_failed_check_names_the_field_and_keeps_the_draft() {
        let draft = card("  SAAG ≥ 1.1 means?  ", "   ");
        let before = draft.clone();
        assert_eq!(validate(&draft), Err((Field::Back, Problem::EmptyBack)));
        assert_eq!(draft, before, "nothing he typed is touched");
        let long = card(&"x".repeat(core_own::MAX_FRONT + 1), "y");
        assert_eq!(validate(&long).unwrap_err().0, Field::Front);
        let src = OwnCard { source: Some("s".repeat(core_own::MAX_SOURCE + 1)), ..card("q", "a") };
        assert_eq!(validate(&src).unwrap_err().0, Field::Source);
    }

    #[test]
    fn a_good_card_saves_tidied() {
        let ok = validate(&card("  q  ", " a ")).unwrap();
        assert_eq!(ok, card("q", "a").tidy());
        assert_eq!(ok.page, Some(12));
    }
}
