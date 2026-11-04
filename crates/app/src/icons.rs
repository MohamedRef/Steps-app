//! Small stroke icons, drawn inline so they follow `currentColor`.
// adapted from kabar-app main: crates/app/src/icons.rs

use dioxus::prelude::*;

use crate::Screen;

/// The path for each screen's nav icon (24×24, stroked).
pub fn nav_path(screen: Screen) -> &'static str {
    match screen {
        // sun over a horizon
        Screen::Today => "M4 18h16M7 18a5 5 0 0 1 10 0M12 6V4M5.6 9.6 4.2 8.2M18.4 9.6l1.4-1.4",
        // a light bulb: understanding first
        Screen::Learn => {
            "M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2.1h5c0-.9.4-1.6 1-2.1A6 6 0 0 0 12 3z"
        }
        // two stacked review cards
        Screen::Study => {
            "M8 4h10a2 2 0 0 1 2 2v10M6 8h9a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z"
        }
        // a list of answer choices, the second one ticked
        Screen::Bank => "M4 6h.01M4 12l1.5 1.5L8 11M4 18h.01M10 6h10M11 12h9M10 18h10",
        // stethoscope
        Screen::Cases => "M6 3v5a4 4 0 0 0 8 0V3M10 12v3a4 4 0 0 0 8 0v-2M18 13a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
        // microphone
        Screen::Oral => "M9 4a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0zM5 11a7 7 0 0 0 14 0M12 18v3M9 21h6",
        // three bars on a baseline
        Screen::Stats => "M4 20h16M7 16v-5M12 16V6M17 16v-8",
        Screen::Settings => GEAR,
    }
}

/// Settings: a cog with eight teeth.
const GEAR: &str = "M12 9a3 3 0 1 0 0 6 3 3 0 1 0 0-6zM10.6 3h2.8l.5 2.3 1.7.9 2.2-.8 1.4 2.4-1.7 1.6v1.9l1.7 1.6-1.4 2.4-2.2-.8-1.7.9-.5 2.3h-2.8l-.5-2.3-1.7-.9-2.2.8-1.4-2.4 1.7-1.6v-1.9L4.8 7.8l1.4-2.4 2.2.8 1.7-.9z";

/// More: three dots in a row.
pub fn more() -> Element {
    stroke(
        "M4 12a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0zM10.5 12a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0zM17 12a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0z",
    )
}

pub fn nav(screen: Screen) -> Element {
    stroke(nav_path(screen))
}

fn stroke(d: &'static str) -> Element {
    rsx! {
        svg {
            class: "icon",
            width: "22",
            height: "22",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d }
        }
    }
}

/// An open book: the lesson view toggle (pages) and References.
const OPEN_BOOK: &str = "M3 6c3-1 6-1 9 1 3-2 6-2 9-1v12c-3-1-6-1-9 1-3-2-6-2-9-1zM12 7v12";

/// Lesson view toggle: an open book (pages).
pub fn book() -> Element {
    stroke(OPEN_BOOK)
}

/// References (More, the rail): the same open book.
pub fn references() -> Element {
    stroke(OPEN_BOOK)
}

/// The abbreviations dictionary: "A" over "Z" with a line between.
pub fn abbreviations() -> Element {
    stroke("M4 11l3-7 3 7M5.3 8h3.4M14 4h6l-6 7h6M4 15h16M7 20h10")
}

/// The MSc map: a graduation cap.
pub fn courses() -> Element {
    stroke("M2 9l10-5 10 5-10 5zM6 11v5c0 1.5 2.7 3 6 3s6-1.5 6-3v-5M22 9v6")
}

/// Opens a list below: a chevron pointing down.
pub fn caret_down() -> Element {
    stroke("M6 9l6 6 6-6")
}

/// A new card: a card with a plus.
pub fn add_card() -> Element {
    stroke("M5 5h14a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zM12 9v6M9 12h6")
}

/// The calculators: a calculator.
pub fn calculator() -> Element {
    stroke(
        "M7 3h10a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM9 7h6M9 12h.01M12 12h.01M15 12h.01M9 16h.01M12 16h.01M15 16h.01",
    )
}

/// Lesson view toggle: stacked cards (steps).
pub fn steps() -> Element {
    stroke("M5 8h14v11H5zM7 5h10M9 2.5h6")
}

/// A magnifier, for search fields.
pub fn search() -> Element {
    stroke("M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM15.5 15.5 20 20")
}

/// A Settings section's icon, by its id (`settings::Section::id`).
pub fn section(id: &str) -> Element {
    stroke(match id {
        // A card stack: review.
        "study" => "M5 8h14v11H5zM7 5h10M9 2.5h6",
        "reminders" => "M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15zM10 20.5a2 2 0 0 0 4 0",
        "sync" => "M7 18a4.5 4.5 0 0 1-.5-8.97A6 6 0 0 1 18 9.5a4 4 0 0 1-1 8.5z",
        "look" => {
            "M12 3a9 9 0 1 0 0 18c1.2 0 1.8-.9 1.4-2l-.4-1a1.6 1.6 0 0 1 1.5-2.2H17a4 4 0 0 0 4-4C21 6.5 17 3 12 3zM7.5 11.5h.01M10 7.5h.01M15 7.5h.01"
        }
        "tablet" => "M6 3h12a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM11 18h2",
        "mac" => "M3 6h18v11H3zM6 9.5h1M9.5 9.5h1M13 9.5h1M16.5 9.5h1M7.5 13.5h9",
        "data" => {
            "M3 6.5a1.5 1.5 0 0 1 1.5-1.5H9l2 2h8.5A1.5 1.5 0 0 1 21 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z"
        }
        "updates" => "M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14",
        _ => "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 11v5M12 8h.01",
    })
}

/// A page-turn arrow. The app is right to left, so "next" points left.
pub fn chevron(next: bool) -> Element {
    stroke(if next { "M15 5l-7 7 7 7" } else { "M9 5l7 7-7 7" })
}

/// Today's greeting: a pulse line.
pub fn pulse() -> Element {
    stroke("M3 12h4l2-5 4 10 2-5h6")
}

/// "Go" on a start button. The app is right to left, so it points left.
pub fn arrow_go() -> Element {
    stroke("M19 12H5M11 6l-6 6 6 6")
}

/// View switcher: cards in a grid.
pub fn layout_cards() -> Element {
    stroke("M4.5 4.5h6v6h-6zM13.5 4.5h6v6h-6zM4.5 13.5h6v6h-6zM13.5 13.5h6v6h-6z")
}

/// View switcher: rows with a second line.
pub fn layout_list() -> Element {
    stroke("M4.5 5h4v4h-4zM11.5 6h8M11.5 8.5h5M4.5 15h4v4h-4zM11.5 16h8M11.5 18.5h5")
}

/// View switcher: one line per item.
pub fn layout_compact() -> Element {
    stroke("M4.5 5h15M4.5 9.67h15M4.5 14.33h15M4.5 19h15")
}

/// The order of a list: an arrow down beside shorter and shorter lines.
pub fn sort() -> Element {
    stroke("M7 4.5v15M4 16.5l3 3 3-3M13 6.5h7M13 11.5h5M13 16.5h3")
}

/// A favourite (the calculators' ☆): a star, filled once pinned.
pub fn star(filled: bool) -> Element {
    rsx! {
        svg {
            class: "icon",
            width: "22",
            height: "22",
            view_box: "0 0 24 24",
            fill: if filled { "currentColor" } else { "none" },
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M12 3.5l2.6 5.3 5.9.9-4.25 4.1 1 5.8L12 16.9l-5.25 2.7 1-5.8L3.5 9.7l5.9-.9z" }
        }
    }
}

/// A tick, for a finished lesson.
pub fn check() -> Element {
    stroke("M5 12.5l4.5 4.5L19 7.5")
}

/// A circular arrow: load again.
pub fn refresh() -> Element {
    stroke("M19.5 12a7.5 7.5 0 1 1-2.2-5.3M19.5 4.5v4h-4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_screen_has_a_path_of_drawing_commands() {
        for s in [Screen::NAV.as_slice(), &[Screen::Settings]].concat() {
            let d = nav_path(s);
            assert!(d.starts_with('M'), "{s:?}");
            assert!(d.chars().all(|c| "MmLlHhVvCcSsQqTtAaZz0123456789.,- ".contains(c)), "{s:?}: {d}");
        }
    }
}
