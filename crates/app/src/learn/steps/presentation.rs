//! Lesson figures, reveals, mnemonics and instructional asides.

use super::*;

/// Pinch, zoom buttons and double-tap in the full-screen figure viewer.
pub(super) const VIEWER_JS: &str = include_str!("../viewer.js");
#[component]
pub(super) fn Figure(
    title: Option<String>,
    svg: String,
    image: Option<Picture>,
    caption: String,
    alt: String,
) -> Element {
    // A tap opens the figure full screen at every width (#337); in the
    // lesson it stays a plain picture, so a swipe on it turns the page (#338).
    let mut open = use_signal(|| false);
    use_hook(|| {
        document::eval(VIEWER_JS);
    });
    // Open-licence images show their author and licence; a book figure's
    // page chip is the step's own.
    let credit = image
        .as_ref()
        .filter(|p| !p.book)
        .and_then(|p| Some(format!("{} · {}", p.credit.as_deref()?, p.license.as_deref()?)));
    let picture = match &image {
        // Content-made SVG, checked by the validator (no scripts, links or fixed colours).
        None => rsx! { div { class: "ls-fig-svg", dir: "ltr", dangerous_inner_html: "{svg}" } },
        Some(p) => rsx! { img { src: "{p.data}", alt: "{alt}", decoding: "async" } },
    };
    rsx! {
        figure { class: "ls-figure",
            if let Some(t) = title.clone() { h2 { class: "ls-h", Line { text: t } } }
            button {
                class: if image.is_some() { "ls-svg ls-photo" } else { "ls-svg" },
                // English figures read from the top left.
                dir: "ltr",
                aria_label: "{alt}",
                title: tx::ZOOM_IN,
                onclick: move |_| open.set(true),
                {picture.clone()}
                span { class: "ls-zoom-hint", aria_hidden: "true", "⤢" }
            }
            figcaption {
                Rich { text: caption.clone() }
                if let Some(c) = credit {
                    p { class: "ls-credit muted small", bdi { dir: "ltr", lang: "en", "📷 {c}" } }
                }
            }
        }
        if open() {
            div {
                class: "ls-viewer",
                role: "dialog",
                aria_modal: "true",
                aria_label: "{alt}",
                tabindex: "-1",
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        open.set(false);
                    }
                },
                onmounted: move |e: MountedEvent| async move {
                    let _ = e.set_focus(true).await;
                },
                header { class: "ls-viewer-bar",
                    button {
                        class: "ls-close",
                        "data-back": "true",
                        aria_label: tx::ZOOM_CLOSE,
                        title: tx::ZOOM_CLOSE,
                        onclick: move |_| open.set(false),
                        "×"
                    }
                    if let Some(t) = title { strong { class: "ls-viewer-title", Line { text: t } } }
                    div { class: "ls-viewer-zoom", dir: "ltr",
                        button { class: "ls-arrow", "data-zoom": "out", aria_label: tx::ZOOM_OUT, title: tx::ZOOM_OUT, "−" }
                        button { class: "ls-arrow", "data-zoom": "fit", aria_label: tx::ZOOM_FIT, title: tx::ZOOM_FIT, "1:1" }
                        button { class: "ls-arrow", "data-zoom": "in", aria_label: tx::ZOOM_IN, title: tx::ZOOM_IN, "+" }
                    }
                }
                div { class: "ls-viewer-scroll", dir: "ltr",
                    div { class: "ls-viewer-inner", {picture} }
                }
                p { class: "ls-viewer-hint muted small", {tx::ZOOM_HINT} }
            }
        }
    }
}
#[component]
pub(super) fn Reveal(prompt: String, answer: String, on_answer: EventHandler<bool>) -> Element {
    let (saved, put) = use_draft();
    let mut open = use_signal(|| matches!(saved, Draft::Reveal { open: true }));
    rsx! {
        div { class: "ls-reveal",
            p { class: "ls-kicker", {tx::THINK} }
            div { class: "ls-q", Rich { text: prompt } }
            if open() {
                div { class: "ls-answer ls-pop", Rich { text: answer } }
            } else {
                p { class: "muted small", {tx::THINK_HINT} }
                button {
                    class: "btn btn-primary",
                    "aria-keyshortcuts": "Space",
                    onclick: move |_| {
                        open.set(true);
                        put.call(Draft::Reveal { open: true });
                        on_answer.call(true);
                    },
                    {tx::REVEAL}
                }
            }
        }
    }
}
#[component]
pub(super) fn Mnemonic(word: String, parts: Vec<MnemonicPart>, hook: Option<String>) -> Element {
    let n = parts.len();
    let (saved, put) = use_draft();
    let mut shown = use_signal(|| match saved {
        Draft::Mnemonic { shown } if shown.len() == n => shown,
        _ => vec![false; n],
    });
    let all = shown.read().iter().all(|s| *s);
    let mut show = move |v: Vec<bool>| {
        put.call(Draft::Mnemonic { shown: v.clone() });
        shown.set(v);
    };
    rsx! {
        div { class: "ls-mnemonic",
            p { class: "ls-kicker", {tx::MNEMONIC} }
            div { class: "ls-mn-letters", dir: "ltr", lang: "en",
                for (i , p) in parts.iter().enumerate() {
                    button {
                        key: "{i}",
                        class: "ls-mn-key",
                        "aria-pressed": if shown.read()[i] { "true" } else { "false" },
                        onclick: move |_| {
                            let mut v = shown.peek().clone();
                            v[i] = true;
                            show(v);
                        },
                        "{p.key}"
                    }
                }
            }
            p { class: "ls-mn-word", dir: "ltr", lang: "en", "{word}" }
            ol { class: "ls-mn-lines", dir: "ltr", lang: "en",
                for (i , p) in parts.iter().enumerate() {
                    li { key: "{i}", "data-open": if shown.read()[i] { "yes" } else { "no" },
                        span { class: "ls-mn-k", "{p.key}" }
                        if shown.read()[i] {
                            bdi { class: "ls-pop", "{p.text}" }
                        } else {
                            span { class: "muted", "…" }
                        }
                    }
                }
            }
            if !all {
                button { class: "btn-link", onclick: move |_| show(vec![true; n]), {tx::SHOW_ALL} }
            }
            if let Some(h) = hook {
                div { class: "ls-mn-hook", Rich { text: h } }
            }
        }
    }
}
#[component]
pub(super) fn Aside(tone: Tone, text: String) -> Element {
    let (name, label, glyph) = match tone {
        Tone::Tip => ("tip", tx::TIP, "💡"),
        Tone::Exam => ("exam", tx::EXAM, "🎯"),
        Tone::Trap => ("trap", tx::TRAP, "⚠️"),
        Tone::Analogy => ("analogy", tx::ANALOGY, "🔁"),
        Tone::Joke => ("joke", tx::JOKE, "😄"),
    };
    rsx! {
        div { class: "ls-aside", "data-tone": name,
            p { class: "ls-aside-label", span { aria_hidden: "true", "{glyph} " } {label} }
            Rich { text }
        }
    }
}
