//! Settings › بياناتك › «مفتاح بنك VQ»: paste the key and «افتح» opens the
//! questions on this device; «اقفل على الجهاز ده» forgets the key again. A
//! key is checked against the one the questions were sealed with before it
//! is kept, and the library opens again with it, with no restart.

use dioxus::prelude::*;
use steps_core::sealed::Key;

use super::{CSS, vault};
use crate::content::Library;
use crate::i18n::vq as tx;
use crate::ui::Card;

/// What the card last said, after a tap.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Said {
    Empty,
    NotAKey,
    OtherKey,
    NotKept,
    Opened,
    Kept,
    Locked,
}

impl Said {
    fn text(self) -> &'static str {
        match self {
            Self::Empty => tx::SAID_EMPTY,
            Self::NotAKey => tx::SAID_NOT_A_KEY,
            Self::OtherKey => tx::SAID_OTHER_KEY,
            Self::NotKept => tx::SAID_NOT_KEPT,
            Self::Opened => tx::SAID_OPENED,
            Self::Kept => tx::SAID_KEPT,
            Self::Locked => tx::SAID_LOCKED,
        }
    }
}

/// The library again, with or without the questions, and every screen
/// drawn from it again, as a content update does.
fn reopen(review: &crate::store::Review) {
    Library::reload();
    review.backend.borrow_mut().library_replaced();
    *crate::content::REV.write() += 1;
}

#[component]
pub fn KeyCard() -> Element {
    let review = crate::store::use_review();
    let _ = crate::content::REV.read();
    let mut text = use_signal(String::new);
    let mut said = use_signal(|| None::<Said>);
    let bank = &Library::get().vq;
    let stored = vault::load();
    let open = stored.as_ref().is_some_and(|k| bank.fits(k));

    let mut submit = {
        let review = review.clone();
        move || {
            let typed = text.peek().clone();
            let result = if typed.trim().is_empty() {
                Said::Empty
            } else {
                match Key::parse(&typed) {
                    Err(_) => Said::NotAKey,
                    Ok(key) if !Library::get().vq.fits(&key) => Said::OtherKey,
                    Ok(key) if !vault::save(&key) => Said::NotKept,
                    Ok(_) => {
                        text.set(String::new());
                        reopen(&review);
                        if Library::get().vq.opened() > 0 { Said::Opened } else { Said::Kept }
                    }
                }
            };
            said.set(Some(result));
        }
    };
    let forget = move |_| {
        vault::clear();
        reopen(&review);
        said.set(Some(Said::Locked));
    };

    rsx! {
        style { {CSS.text()} }
        Card { class: "vq-key",
            h2 { {tx::TITLE} }
            p { class: "muted small", {tx::INTRO} }
            p { class: "vq-state",
                if open {
                    span { class: "chip", "data-tone": "good", {tx::opened(bank.opened())} }
                } else {
                    span { class: "chip", {tx::LOCKED} }
                }
            }
            if bank.sealed == 0 {
                p { class: "muted small", {tx::NONE_YET} }
            }
            if stored.is_some() && !open {
                p { class: "small", {tx::STORED_OTHER} }
            }
            if !open {
                form {
                    class: "vq-form",
                    onsubmit: move |e: FormEvent| {
                        e.prevent_default();
                        submit();
                    },
                    label { class: "field",
                        span { {tx::FIELD} }
                        input {
                            r#type: "password",
                            name: "vq-key",
                            dir: "ltr",
                            lang: "en",
                            autocomplete: "off",
                            autocapitalize: "characters",
                            spellcheck: "false",
                            placeholder: tx::PLACEHOLDER,
                            value: "{text}",
                            oninput: move |e| {
                                text.set(e.value());
                                said.set(None);
                            },
                        }
                        span { class: "field-hint", {tx::FIELD_HINT} }
                    }
                    div { class: "row",
                        button { class: "btn btn-primary", r#type: "submit", {tx::OPEN} }
                    }
                }
            }
            if stored.is_some() {
                div { class: "row",
                    button { class: "btn btn-secondary", r#type: "button", onclick: forget, {tx::FORGET} }
                }
            }
            if let Some(s) = said() {
                p { class: "small vq-said", role: "status", "data-said": "{s:?}", {s.text()} }
            }
            p { class: "muted small", {tx::DEVICE_ONLY} }
        }
    }
}
