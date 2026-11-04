//! Phones: the bottom bar keeps four screens (Today, Lessons, Review,
//! Questions) and a "More" tab. More opens a bounded sheet above the bar,
//! grouped as in the 2026-10 redesign: practice (Cases, Oral, Calculators)
//! and your references and tools (References, a new card, Progress,
//! Settings). From 600dp the rail and sidebar show every screen and the
//! tools ([`ToolButtons`]), so the tab and the sheet hide (shape.css).

use dioxus::prelude::*;

use crate::{Screen, Ui, i18n, icons, settings};

/// Screens that sit under More on phones, in the order they appear there.
pub const SCREENS: [Screen; 3] = [Screen::Cases, Screen::Oral, Screen::Stats];

/// Something that opens over the current screen rather than being a tab:
/// label, icon, opener.
#[derive(Clone, Copy)]
pub struct Tool {
    pub label: &'static str,
    pub icon: fn() -> Element,
    pub open: fn(Ui),
}

pub const CALCULATORS: Tool =
    Tool { label: i18n::calc::TITLE, icon: icons::calculator, open: crate::case::open_calculators };
pub const REFERENCES: Tool =
    Tool { label: i18n::refs::TITLE, icon: icons::references, open: crate::lecture::open_references };
pub const COURSES: Tool =
    Tool { label: i18n::courses::TITLE, icon: icons::courses, open: crate::courses::open_map };
pub const ABBREVIATIONS: Tool = Tool {
    label: i18n::abbr::DICT_TITLE,
    icon: icons::abbreviations,
    open: |ui| crate::abbrdict::open(ui, None),
};
pub const ADD_CARD: Tool =
    Tool { label: i18n::refs::ADD_CARD, icon: icons::add_card, open: |_| crate::own::open_new() };

/// The tools the rail and sidebar list under the screens from 600dp (More
/// holds them on phones).
pub const TOOLS: [Tool; 5] = [COURSES, REFERENCES, ABBREVIATIONS, ADD_CARD, CALCULATORS];

/// One row in More.
#[derive(Clone, Copy)]
pub enum Row {
    /// A screen of the bar (its label and icon).
    Screen(Screen),
    /// A tool that opens over the current screen.
    Tool(Tool),
    /// The Settings sheet.
    Settings,
}

impl Row {
    fn label(self) -> &'static str {
        match self {
            Row::Screen(screen) => screen.label(),
            Row::Tool(t) => t.label,
            Row::Settings => Screen::Settings.label(),
        }
    }

    fn is(self, screen: Screen) -> bool {
        matches!(self, Row::Screen(s) if s == screen)
    }
}

/// More's groups: a heading and its rows, in order.
const GROUPS: [(&str, &[Row]); 2] = [
    (
        i18n::nav::MORE_PRACTICE,
        &[Row::Screen(Screen::Cases), Row::Screen(Screen::Oral), Row::Tool(CALCULATORS)],
    ),
    (
        i18n::nav::MORE_TOOLS,
        &[
            Row::Tool(COURSES),
            Row::Tool(REFERENCES),
            Row::Tool(ABBREVIATIONS),
            Row::Tool(ADD_CARD),
            Row::Screen(Screen::Stats),
            Row::Settings,
        ],
    ),
];

pub fn holds(screen: Screen) -> bool {
    SCREENS.contains(&screen)
}

/// The bar class for a screen: More's screens hide from the phone bar.
pub fn nav_class(screen: Screen) -> &'static str {
    if holds(screen) { "nav-item nav-extra" } else { "nav-item" }
}

/// The More tab and its sheet.
#[component]
pub fn More() -> Element {
    let mut ui = use_context::<Ui>();
    let settings = use_context::<settings::SettingsNav>();
    let mut open = use_signal(|| false);
    let current = *ui.screen.read();
    rsx! {
        button {
            class: "nav-item nav-more",
            "aria-current": if holds(current) { "page" } else { "false" },
            "aria-expanded": if open() { "true" } else { "false" },
            "aria-haspopup": "dialog",
            "aria-label": i18n::nav::MORE,
            onclick: move |_| open.toggle(),
            span { class: "glyph", aria_hidden: "true", {icons::more()} }
            span { class: "label", {i18n::nav::MORE} }
        }
        if open() {
            div { class: "more-backdrop", onclick: move |_| open.set(false) }
            div {
                class: "more-sheet",
                role: "dialog",
                aria_label: i18n::nav::MORE,
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        open.set(false);
                    }
                },
                for (title, rows) in GROUPS {
                    div { key: "{title}", class: "more-group", role: "group", aria_label: title,
                        h2 { class: "more-group-title", {title} }
                        for (k, &row) in rows.iter().enumerate() {
                            button {
                                key: "{k}",
                                class: "more-row",
                                "aria-current": if row.is(current) { "page" } else { "false" },
                                onclick: move |_| {
                                    open.set(false);
                                    match row {
                                        Row::Screen(screen) => ui.screen.set(screen),
                                        Row::Tool(t) => (t.open)(ui),
                                        Row::Settings => settings.show(None),
                                    }
                                },
                                span { class: "row-icon", aria_hidden: "true",
                                    match row {
                                        Row::Screen(screen) => icons::nav(screen),
                                        Row::Tool(t) => (t.icon)(),
                                        Row::Settings => icons::nav(Screen::Settings),
                                    }
                                }
                                span { class: "label", {row.label()} }
                                span { class: "chev", aria_hidden: "true" }
                            }
                        }
                    }
                }
                // Back (Android) or a tap here closes the sheet.
                button {
                    class: "btn btn-ghost more-close",
                    "data-back": "true",
                    onclick: move |_| open.set(false),
                    {i18n::settings::CLOSE}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_bar_is_today_lessons_review_questions_more() {
        let bar: Vec<Screen> = Screen::NAV.into_iter().filter(|s| !holds(*s)).collect();
        assert_eq!(bar, [Screen::Today, Screen::Learn, Screen::Study, Screen::Bank]);
    }

    #[test]
    fn every_screen_is_reachable_on_a_phone() {
        // Nothing is dropped: a screen off the bar is in one of More's groups.
        for screen in Screen::NAV.into_iter().filter(|s| holds(*s)) {
            assert!(
                GROUPS.iter().any(|(_, rows)| rows.iter().any(|r| r.is(screen))),
                "{screen:?} has no row in More"
            );
        }
        let grouped: Vec<Screen> = GROUPS
            .iter()
            .flat_map(|(_, rows)| rows.iter())
            .filter_map(|r| if let Row::Screen(s) = r { Some(*s) } else { None })
            .collect();
        assert_eq!(grouped, SCREENS);
        assert!(
            GROUPS.iter().any(|(_, rows)| rows.iter().any(|r| matches!(r, Row::Settings))),
            "Settings is in More"
        );
        // Every tool the rail lists is in More too, so phones reach it.
        for tool in TOOLS {
            assert!(
                GROUPS.iter().any(|(_, rows)| rows.iter().any(|r| r.label() == tool.label)),
                "{} has no row in More",
                tool.label
            );
        }
    }
}

/// From 600dp: the tools under the screens in the rail and sidebar (phones
/// reach them through More).
#[component]
pub fn ToolButtons() -> Element {
    let ui = use_context::<Ui>();
    rsx! {
        div { class: "nav-tools", role: "group", aria_label: i18n::nav::MORE_TOOLS,
            for tool in TOOLS {
                button {
                    key: "{tool.label}",
                    class: "nav-item nav-tool",
                    aria_label: tool.label,
                    title: tool.label,
                    onclick: move |_| (tool.open)(ui),
                    span { class: "glyph", aria_hidden: "true", {(tool.icon)()} }
                    span { class: "label", {tool.label} }
                }
            }
        }
    }
}
