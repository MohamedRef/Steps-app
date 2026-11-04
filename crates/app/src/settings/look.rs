//! Settings › Look: colour themes, appearance styles and text size, per device.

use dioxus::prelude::*;
use steps_core::theme::Theme;

use crate::ui::Card;

use super::DeviceOnly;
use crate::Ui;
use crate::i18n::settings as tx;
use crate::prefs::{Appearance, TextSize};

fn theme_label(t: Theme) -> (&'static str, &'static str) {
    match t {
        Theme::System => (tx::THEME_SYSTEM, tx::THEME_SYSTEM_SUB),
        Theme::SystemBlack => (tx::THEME_SYSTEM_BLACK, tx::THEME_SYSTEM_BLACK_SUB),
        Theme::Paper => (tx::THEME_PAPER, tx::THEME_PAPER_SUB),
        Theme::Night => (tx::THEME_NIGHT, tx::THEME_NIGHT_SUB),
        Theme::Black => (tx::THEME_BLACK, tx::THEME_BLACK_SUB),
        Theme::NightShift => (tx::THEME_SHIFT, tx::THEME_SHIFT_SUB),
    }
}

fn appearance_label(a: Appearance) -> (&'static str, &'static str) {
    match a {
        Appearance::Calm => (tx::APPEARANCE_CALM, tx::APPEARANCE_CALM_SUB),
        Appearance::Glossy => (tx::APPEARANCE_GLOSSY, tx::APPEARANCE_GLOSSY_SUB),
        Appearance::LiquidGlass => (tx::APPEARANCE_LIQUID_GLASS, tx::APPEARANCE_LIQUID_GLASS_SUB),
    }
}

fn size_label(s: TextSize) -> &'static str {
    match s {
        TextSize::Smaller => tx::TEXT_SMALLER,
        TextSize::Small => tx::TEXT_SMALL,
        TextSize::Normal => tx::TEXT_NORMAL,
        TextSize::Large => tx::TEXT_LARGE,
        TextSize::Larger => tx::TEXT_LARGER,
    }
}

/// Two halves of the swatch: the page and the accent (System themes: the
/// day and the night page).
fn swatch_style(t: Theme) -> String {
    let (a, b) = match t.palette() {
        Some(p) => (p.bg, p.accent),
        None => (t.day_and_night().0.bg, t.day_and_night().1.bg),
    };
    format!("--sw-a:{a};--sw-b:{b};")
}

#[component]
pub fn LookSection() -> Element {
    let ui = use_context::<Ui>();
    let now = *ui.prefs.read();
    rsx! {
        Card {
            h2 { {tx::THEME} DeviceOnly {} }
            div { class: "choice-list", role: "radiogroup", aria_label: tx::THEME,
                for t in Theme::ALL {
                    button {
                        key: "{t.as_str()}",
                        class: "choice",
                        role: "radio",
                        aria_checked: if now.theme == t { "true" } else { "false" },
                        onclick: move |_| ui.update_prefs(|prefs| prefs.theme = t),
                        span { class: "swatch", aria_hidden: "true", style: swatch_style(t) }
                        span { class: "choice-text",
                            span { class: "choice-title", {theme_label(t).0} }
                            span { class: "muted small", {theme_label(t).1} }
                        }
                    }
                }
            }
        }
        Card {
            h2 { {tx::APPEARANCE} DeviceOnly {} }
            div { class: "choice-list", role: "radiogroup", aria_label: tx::APPEARANCE,
                for a in Appearance::ALL {
                    button {
                        key: "{a.as_str()}",
                        class: "choice",
                        role: "radio",
                        aria_checked: if now.appearance == a { "true" } else { "false" },
                        // Only the look changes: the screen under the sheet
                        // keeps its answer, scroll and typing.
                        onclick: move |_| ui.update_prefs(|prefs| prefs.appearance = a),
                        span { class: "appearance-swatch", "data-swatch": a.as_str(), aria_hidden: "true" }
                        span { class: "choice-text",
                            span { class: "choice-title",
                                if matches!(a, Appearance::Glossy | Appearance::LiquidGlass) {
                                    bdi { {appearance_label(a).0} }
                                } else {
                                    {appearance_label(a).0}
                                }
                            }
                            span { class: "muted small", {appearance_label(a).1} }
                        }
                    }
                }
            }
        }
        Card {
            h2 { {tx::TEXT_SIZE} DeviceOnly {} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::TEXT_SIZE,
                for s in TextSize::ALL {
                    button {
                        key: "{s.as_str()}",
                        role: "radio",
                        aria_checked: if now.text_size == s { "true" } else { "false" },
                        onclick: move |_| ui.update_prefs(|prefs| prefs.text_size = s),
                        {size_label(s)}
                    }
                }
            }
            p { class: "text-sample", {tx::TEXT_SAMPLE_AR} " " bdi { {tx::TEXT_SAMPLE_TERM} } "؟" }
        }
    }
}
