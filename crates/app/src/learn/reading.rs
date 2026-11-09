//! How a lesson reads on this device: text size, line spacing and line
//! width. An «Aa» button in the lesson's bar opens the choices in a small
//! popover over the step (never pushing the text down); Settings › Look
//! shows the same choices. Kept with the device's other look choices
//! (prefs.rs), so the phone and the Mac can each read their own way.
// Idea adapted from kabar-app main: crates/app/src/reader/reading.rs (the
// Reader's settings popover); the controls are Steps' segmented rows.

use dioxus::prelude::*;

use crate::Ui;
use crate::i18n::learn as tx;
use crate::prefs::{ReadSize, ReadSpacing, ReadWidth, Reading};

fn size_label(s: ReadSize) -> &'static str {
    match s {
        ReadSize::Small => tx::READ_SIZE_SMALL,
        ReadSize::Normal => tx::READ_SIZE_NORMAL,
        ReadSize::Large => tx::READ_SIZE_LARGE,
        ReadSize::Larger => tx::READ_SIZE_LARGER,
    }
}

fn spacing_label(s: ReadSpacing) -> &'static str {
    match s {
        ReadSpacing::Snug => tx::READ_SPACING_SNUG,
        ReadSpacing::Normal => tx::READ_SPACING_NORMAL,
        ReadSpacing::Airy => tx::READ_SPACING_AIRY,
    }
}

fn width_label(w: ReadWidth) -> &'static str {
    match w {
        ReadWidth::Narrow => tx::READ_WIDTH_NARROW,
        ReadWidth::Book => tx::READ_WIDTH_BOOK,
        ReadWidth::Wide => tx::READ_WIDTH_WIDE,
    }
}

fn set(ui: Ui, next: Reading) {
    ui.update_prefs(|prefs| prefs.reading = next);
}

/// The `data-read-*` attributes the lesson player's CSS keys on.
pub fn attrs(r: Reading) -> [(&'static str, &'static str); 3] {
    [
        ("data-read-size", r.size.as_str()),
        ("data-read-spacing", r.spacing.as_str()),
        ("data-read-width", r.width.as_str()),
    ]
}

/// The three choices with a preview that follows them. Shared by the
/// lesson's popover and Settings › Look.
#[component]
pub fn ReadingChoices() -> Element {
    let ui = use_context::<Ui>();
    let r = ui.prefs.read().reading;
    let [(_, size), (_, spacing), (_, width)] = attrs(r);
    rsx! {
        div { class: "ls-read-choices",
            p {
                class: "ls-read-preview",
                "data-read-size": size,
                "data-read-spacing": spacing,
                "data-read-width": width,
                aria_hidden: "true",
                {tx::READ_SAMPLE[0]}
                bdi { class: "ls-nw", dir: "ltr", lang: "en", {tx::READ_SAMPLE[1]} }
                {tx::READ_SAMPLE[2]}
                bdi { class: "ls-nw", dir: "ltr", lang: "en", {tx::READ_SAMPLE[3]} }
                {tx::READ_SAMPLE[4]}
            }
            div { class: "ls-read-field",
                span { class: "ls-read-label", {tx::READ_SIZE} }
                div { class: "segmented ls-read-sizes", role: "radiogroup", aria_label: tx::READ_SIZE,
                    for s in ReadSize::ALL {
                        button {
                            key: "{s.as_str()}",
                            role: "radio",
                            "data-size": s.as_str(),
                            aria_checked: if r.size == s { "true" } else { "false" },
                            aria_label: size_label(s),
                            title: size_label(s),
                            onclick: move |_| set(ui, Reading { size: s, ..r }),
                            span { class: "ls-read-glyph", aria_hidden: "true", "أ" }
                        }
                    }
                }
            }
            div { class: "ls-read-field",
                span { class: "ls-read-label", {tx::READ_SPACING} }
                div { class: "segmented", role: "radiogroup", aria_label: tx::READ_SPACING,
                    for s in ReadSpacing::ALL {
                        button {
                            key: "{s.as_str()}",
                            role: "radio",
                            aria_checked: if r.spacing == s { "true" } else { "false" },
                            onclick: move |_| set(ui, Reading { spacing: s, ..r }),
                            {spacing_label(s)}
                        }
                    }
                }
            }
            div { class: "ls-read-field ls-read-width",
                span { class: "ls-read-label",
                    {tx::READ_WIDTH}
                    span { class: "muted small", " · {tx::READ_WIDTH_HINT}" }
                }
                div { class: "segmented", role: "radiogroup", aria_label: tx::READ_WIDTH,
                    for w in ReadWidth::ALL {
                        button {
                            key: "{w.as_str()}",
                            role: "radio",
                            aria_checked: if r.width == w { "true" } else { "false" },
                            onclick: move |_| set(ui, Reading { width: w, ..r }),
                            {width_label(w)}
                        }
                    }
                }
            }
        }
    }
}

/// The «Aa» button in the lesson's bar and, while open, the popover under
/// it. While open, the button is the Back and Escape target, so Android
/// Back and Escape close the popover before they close the lesson.
#[component]
pub fn ReadingButton(open: Signal<bool>) -> Element {
    let mut open = open;
    let shown = open();
    rsx! {
        div { class: "ls-read",
            button {
                class: "ls-read-aa",
                "data-back": if shown { "true" } else { "false" },
                "aria-keyshortcuts": if shown { Some("Escape") } else { None },
                aria_expanded: if shown { "true" } else { "false" },
                aria_controls: "ls-read-pop",
                aria_label: tx::READING_OPEN,
                title: tx::READING_OPEN,
                onclick: move |_| open.set(!shown),
                span { dir: "ltr", aria_hidden: "true", "Aa" }
            }
            if shown {
                // A tap anywhere outside the popover closes it.
                div { class: "ls-read-scrim", onclick: move |_| open.set(false) }
                div { id: "ls-read-pop", class: "ls-read-pop", role: "group", aria_label: tx::READING,
                    h2 { class: "ls-read-title", {tx::READING} }
                    ReadingChoices {}
                    button { class: "btn btn-secondary ls-read-done", onclick: move |_| open.set(false), {tx::READ_DONE} }
                }
            }
        }
    }
}

/// Settings › Look: the same choices, with what they change.
#[component]
pub fn ReadingSection() -> Element {
    rsx! {
        crate::ui::Card {
            h2 { {tx::READING} crate::settings::DeviceOnly {} }
            ReadingChoices {}
        }
    }
}
