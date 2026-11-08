//! The tablet and Mac desk (PARITY.md §6): the labelled sidebar, two panes on
//! wide windows, and the per-device switches for it, hover peeks and the
//! keyboard. Phones get none of it in the way: everything here keys on window
//! width, never on the device.
//!
//! Feature screens opt in without depending on this module's internals:
//! - [`Panes`]: the question on one side, the explanation on the other (≥840).
//! - [`ListDetail`]: a list beside the open item (≥840).
//! - `"data-peek": text` on any element: its text shows on pen or mouse hover
//!   (a book page's `src.quote`, an MCQ option's `why` after answering).
//!   [`crate::hover::BookPage`] does it for the page chip.
//! - `"aria-keyshortcuts": "Space"` (or `"1"`, `"A"`, `"Enter"`, `"Escape"`)
//!   on a button: that key presses it (see [`crate::shortcuts`]).

use dioxus::prelude::*;

use crate::Ui;
use crate::i18n::{desk as tx, hover as htx, keys as ktx};
use crate::look::{Css, css};
use crate::prefs::NavStyle;
use crate::settings::{DeviceOnly, Section};

const DESK_CSS: Css = css!("assets/desk.css");
const NAV_CSS: Css = css!("assets/nav.css");
const RESIZE_JS: &str = include_str!("resize.js");

/// The width a `split:N` message from resize.js saves: 35–70, or 0 to go
/// back to each item's default.
fn split_width(msg: &str) -> Option<u8> {
    let n: u8 = msg.strip_prefix("split:")?.parse().ok()?;
    (n == 0 || (35..=70).contains(&n)).then_some(n)
}

/// Rendered once by the app root: the desk's styles, the stored device
/// switches, the hover script, the keyboard and (on the Mac) the menu.
#[component]
pub fn Chrome() -> Element {
    let ui = use_context::<Ui>();
    // The device switches reach nav.css and hover.js as data-* on <html>,
    // and the saved pane width as --desk-split (0: each item's default).
    use_effect(move || {
        let p = *ui.prefs.read();
        let on_off = |on: bool| if on { "on" } else { "off" };
        document::eval(&format!(
            "const r = document.documentElement, d = r.dataset; d.navStyle = {:?}; d.penHover = {:?}; d.mouseHover = {:?}; d.deskFocus = {:?}; {}",
            p.nav_style.as_str(),
            on_off(p.pen_hover),
            on_off(p.mouse_hover),
            on_off(p.desk_focus),
            if p.desk_split == 0 {
                "r.style.removeProperty('--desk-split');".to_string()
            } else {
                format!("r.style.setProperty('--desk-split', '{}%');", p.desk_split)
            },
        ));
    });
    // The pane divider's drag and keys (resize.js) report the width they
    // settle on; it is kept per device.
    use_future(move || async move {
        let mut script = document::eval(RESIZE_JS);
        while let Ok(msg) = script.recv::<String>().await {
            if let Some(width) = split_width(&msg) {
                ui.update_prefs(|p| p.desk_split = width);
            }
        }
    });
    // A content pack switched in while open restyles the nav and desk too.
    let _ = crate::content::REV.read();
    crate::hover::use_hover();
    crate::shortcuts::use_shortcuts();
    // Not under unit tests: the menu needs a real desktop window.
    #[cfg(all(feature = "desktop", not(target_os = "android"), not(test)))]
    crate::settings::menu::use_menu_events();
    rsx! {
        style { {NAV_CSS.text()} }
        style { {DESK_CSS.text()} }
    }
}

/// Two panes from 840dp: `primary` (the question) and `secondary` (the
/// explanation, `why` lines, guideline update and book page). Narrower, the
/// secondary pane follows the primary one. With no `secondary` yet (before
/// the answer), wide windows keep the place with a quiet hint.
///
/// `wide_side` gives the secondary pane the wide column, for long answers.
/// `footer` (the grade row) sits under both panes at full width.
///
/// Where the two panes sit side by side, a divider between them resizes
/// them (drag, arrows, Home/End; a double click restores the default). The
/// width is kept per device ([`crate::prefs::DevicePrefs::desk_split`]).
#[component]
pub fn Panes(
    primary: Element,
    secondary: Option<Element>,
    #[props(default)] wide_side: bool,
    footer: Option<Element>,
) -> Element {
    let default = if wide_side { 40 } else { 60 };
    let saved = try_use_context::<Ui>().map_or(0, |ui| ui.prefs.read().desk_split);
    let width = if saved == 0 { default } else { saved };
    rsx! {
        // The panes split by their own width, not the window's (UX #580):
        // at 840 the side rail left each pane ~130–260px.
        div { class: "desk-panes",
            div { class: "desk-split", "data-wide": if wide_side { "side" } else { "main" },
                div { class: "desk-main", {primary} }
                div {
                    class: "desk-resize",
                    role: "separator",
                    tabindex: "0",
                    aria_orientation: "vertical",
                    aria_valuemin: "35",
                    aria_valuemax: "70",
                    aria_valuenow: "{width}",
                    aria_valuetext: "{width}%",
                    aria_label: tx::SPLIT_HANDLE,
                    title: tx::SPLIT_HELP,
                    "data-default": "{default}",
                    span { class: "desk-resize-grip", aria_hidden: "true" }
                }
                match secondary {
                    Some(side) => rsx! {
                        aside { class: "desk-side", aria_label: tx::SIDE_LABEL, {side} }
                    },
                    None => rsx! {
                        aside { class: "desk-side desk-waiting", aria_label: tx::SIDE_LABEL,
                            p { class: "muted small", {tx::SIDE_WAITING} }
                        }
                    },
                }
            }
        }
        if let Some(f) = footer {
            div { class: "desk-footer", {f} }
        }
    }
}

/// Focus mode for the Mac and the tablet: one press hides the sidebar (and
/// a lesson's side column) while a lesson or a study card is open, so the
/// task gets the whole window. Kept per device; F or
/// the button again brings everything back. Phones already give a task the
/// whole screen, so the button shows from 600dp only (desk.css). Idea from
/// batna-app main: crates/app/src/desk/mod.rs (#741).
#[component]
pub fn FocusToggle() -> Element {
    let ui = use_context::<Ui>();
    let on = ui.prefs.read().desk_focus;
    let label = if on { tx::FOCUS_LEAVE } else { tx::FOCUS_ENTER };
    rsx! {
        button {
            r#type: "button",
            class: "desk-focus",
            aria_pressed: if on { "true" } else { "false" },
            "aria-keyshortcuts": "F",
            aria_label: label,
            title: "{label} (F)",
            onclick: move |_| ui.update_prefs(|p| p.desk_focus = !p.desk_focus),
            {if on { crate::icons::focus_leave() } else { crate::icons::focus_enter() }}
        }
    }
}

/// A list beside the open item from 840dp (the question bank). Narrower,
/// only one shows: the item when there is one, else the list.
#[component]
pub fn ListDetail(list: Element, detail: Option<Element>) -> Element {
    let open = detail.is_some();
    rsx! {
        div { class: "desk-list-detail", "data-open": if open { "yes" } else { "no" },
            nav { class: "desk-list", aria_label: tx::LIST_LABEL, {list} }
            if let Some(d) = detail {
                section { class: "desk-detail", {d} }
            }
        }
    }
}

/// Settings › Tablet (Android: pen hover) and Settings › Mac (mouse hover):
/// the sidebar style, the hover peek and the keyboard shortcut list.
#[component]
pub fn DeviceSettings(section: Section) -> Element {
    let ui = use_context::<Ui>();
    let now = *ui.prefs.read();
    let terms = crate::hover::term_count();
    let set = move |f: &dyn Fn(&mut crate::prefs::DevicePrefs)| ui.update_prefs(f);
    let tablet = section == Section::Tablet;
    rsx! {
        crate::ui::Card { class: "desk-settings",
            h2 { {tx::NAV_STYLE} DeviceOnly {} }
            div { class: "desk-segment", role: "radiogroup", aria_label: tx::NAV_STYLE,
                for (style, label) in [(NavStyle::Labels, tx::NAV_LABELS), (NavStyle::Slim, tx::NAV_SLIM)] {
                    button {
                        key: "{style.as_str()}",
                        role: "radio",
                        aria_checked: if now.nav_style == style { "true" } else { "false" },
                        onclick: move |_| set(&|p| p.nav_style = style),
                        {label}
                    }
                }
            }
        }
        crate::ui::Card { class: "desk-settings",
            h2 { {htx::TITLE} DeviceOnly {} }
            if tablet {
                Toggle {
                    label: htx::PEN,
                    note: htx::PEN_NOTE,
                    on: now.pen_hover,
                    onchange: move |v: bool| set(&|p| p.pen_hover = v),
                }
            } else {
                Toggle {
                    label: htx::MOUSE,
                    note: htx::MOUSE_NOTE,
                    on: now.mouse_hover,
                    onchange: move |v: bool| set(&|p| p.mouse_hover = v),
                }
            }
            p { class: "muted small", bdi { "{terms}" } " " {htx::TERMS} }
        }
        // The whole card opens the shortcut sheet (the button's click bubbles up).
        crate::ui::Card { class: "card-action desk-settings",
            onclick: move |_| crate::shortcuts::open_sheet(true),
            h2 { {ktx::TITLE} }
            button {
                class: "desk-keys-button",
                "⌨ " {ktx::OPEN}
            }
        }
    }
}

#[component]
fn Toggle(label: &'static str, note: &'static str, on: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        label { class: "desk-toggle",
            span { class: "desk-toggle-text",
                span { {label} }
                span { class: "muted small", {note} }
            }
            input {
                r#type: "checkbox",
                role: "switch",
                checked: on,
                onchange: move |e| onchange.call(e.checked()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_divider_saves_only_widths_it_allows() {
        assert_eq!(split_width("split:35"), Some(35));
        assert_eq!(split_width("split:70"), Some(70));
        assert_eq!(split_width("split:0"), Some(0));
        for bad in ["split:34", "split:71", "split:x", "width:50", ""] {
            assert_eq!(split_width(bad), None, "{bad}");
        }
        for needed in
            ["setPointerCapture", "pointercancel", "dblclick", "ArrowLeft", "Home", "rtl", "split:0"]
        {
            assert!(RESIZE_JS.contains(needed), "resize.js needs {needed}");
        }
        let css = DESK_CSS.text();
        assert!(css.contains("var(--desk-split, 60%)") && css.contains("data-desk-focus=\"on\""));
    }
}
