//! A VQ question's figure, above its options: at most as wide as the card,
//! and a tap opens it full size over the screen. Its educational objective,
//! with the explanation. And the `VQ-0001` chip a question-bank item shows
//! where a First Aid item shows its page.

use dioxus::prelude::*;

use super::CSS;
use crate::content::Library;
use crate::i18n::vq as tx;

/// Nothing when the question has no figure (or isn't a VQ question).
#[component]
pub fn Figure(item_id: String) -> Element {
    let Some(src) = Library::get().vq.image(&item_id).map(str::to_owned) else { return rsx! {} };
    let name = super::name(&item_id).unwrap_or_default();
    let alt = tx::figure_alt(&name);
    let mut big = use_signal(|| false);
    rsx! {
        style { {CSS.text()} }
        button {
            class: "vq-figure",
            r#type: "button",
            title: tx::FIGURE_ZOOM,
            aria_label: "{tx::FIGURE_ZOOM}: {alt}",
            onclick: move |e| {
                // Inside an answer or a row with its own action, only the figure opens.
                e.stop_propagation();
                big.set(true);
            },
            img { src: "{src}", alt: "{alt}", decoding: "async" }
        }
        if big() {
            div {
                class: "vq-full",
                role: "dialog",
                aria_modal: "true",
                aria_label: "{alt}",
                tabindex: "-1",
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        big.set(false);
                    }
                },
                onmounted: move |e: MountedEvent| async move {
                    let _ = e.set_focus(true).await;
                },
                header { class: "vq-full-bar",
                    button {
                        class: "icon-button",
                        r#type: "button",
                        "data-back": "true",
                        "aria-keyshortcuts": "Escape",
                        aria_label: tx::FIGURE_CLOSE,
                        title: tx::FIGURE_CLOSE,
                        onclick: move |_| big.set(false),
                        "×"
                    }
                    bdi { class: "vq-full-name", dir: "ltr", "{name}" }
                }
                div { class: "vq-full-scroll", dir: "ltr",
                    img { src: "{src}", alt: "{alt}" }
                }
            }
        }
    }
}

/// The question's educational objective, after the why lines of its
/// explanation; nothing when it has none. English, as the bank wrote it.
#[component]
pub fn Objective(item_id: String) -> Element {
    let Some(text) = Library::get().vq.objective(&item_id).map(str::to_owned) else { return rsx! {} };
    rsx! {
        style { {CSS.text()} }
        aside { class: "note vq-objective", lang: "en", dir: "ltr",
            h3 { class: "vq-objective-head", bdi { {tx::OBJECTIVE} } }
            p { bdi { "{text}" } }
        }
    }
}

/// `VQ-0001` in place of a page chip. The bank has no page to open, so it
/// is a label, not a button.
#[component]
pub fn SourceChip(label: String) -> Element {
    rsx! {
        span { class: "chip vq-chip", bdi { dir: "ltr", lang: "en", "{label}" } }
    }
}
