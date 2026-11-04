//! Keyboard shortcuts for tablet keyboards and the Mac (PARITY.md §6.5):
//! Ctrl/⌘ + 1–7 for the screens, Ctrl/⌘ + , for Settings, and on study
//! screens Space to reveal, 1–4 to rate, A–E for MCQ options, Enter for next,
//! ← / → between questions of a timed paper, F to flag, Esc to close, ? for
//! the list.
//!
//! Screens don't register handlers: a key presses the visible, enabled button
//! whose `aria-keyshortcuts` names it (`"Space"`, `"1"`, `"A"`, `"Enter"`,
//! `"Escape"`, `"ArrowLeft"`, `"F"`). That is also how screen readers learn the key.
// adapted from kabar-app main: crates/app/src/settings/menu.rs ("?" key) and
// crates/app/src/review.rs (keys on visible buttons).

use dioxus::prelude::*;

use crate::i18n::{keys as tx, nav};
use crate::settings::SettingsNav;
use crate::{Screen, Ui};

static SHEET_OPEN: GlobalSignal<bool> = Signal::global(|| false);

pub fn open_sheet(open: bool) {
    *SHEET_OPEN.write() = open;
}

const KEYS_JS: &str = r#"
if (window.__stepsKeys) document.removeEventListener("keydown", window.__stepsKeys, true);
window.__stepsKeys = (e) => {
  const t = e.target;
  const typing = t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable);
  const mod = e.metaKey || e.ctrlKey;
  if (mod && !e.altKey && !e.shiftKey) {
    if (/^Digit[1-7]$/.test(e.code)) { e.preventDefault(); dioxus.send("go:" + e.code.slice(5)); return; }
    if (e.code === "Comma") { e.preventDefault(); dioxus.send("go:settings"); return; }
    return;
  }
  if (e.altKey || mod) return;
  if (e.key === "Escape") {
    if (window.__stepsHover && window.__stepsHover.visible()) { window.__stepsHover.hide(); e.preventDefault(); return; }
  } else if (typing) return;
  if (e.key === "?" || e.key === "\u061F") { e.preventDefault(); dioxus.send("help"); return; }
  // By physical key, so an Arabic keyboard layout works the same.
  let key = null;
  if (e.code === "Space") key = "space";
  else if (e.key === "Enter") key = "enter";
  else if (e.key === "Escape") key = "escape";
  else if (/^(Digit|Numpad)[1-9]$/.test(e.code)) key = e.code.slice(-1);
  else if (e.key === "ArrowLeft" || e.key === "ArrowRight") key = e.key.toLowerCase();
  else if (/^Key[A-F]$/.test(e.code)) key = e.code.slice(3).toLowerCase();
  if (!key) return;
  // An open dialog (the shortcut list) takes the keys first.
  const scope = document.querySelector("[role=dialog][aria-modal=true], [role=alertdialog][aria-modal=true]") || document;
  const hit = [...scope.querySelectorAll("[aria-keyshortcuts]")].find((el) =>
    !el.disabled && el.getClientRects().length > 0 &&
    el.getAttribute("aria-keyshortcuts").toLowerCase().split(/\s+/).includes(key));
  if (!hit) return;
  // Enter and Space already press a focused button; don't press twice.
  if ((key === "enter" || key === "space") && t && t.tagName === "BUTTON" && t !== hit) return;
  e.preventDefault();
  hit.click();
};
document.addEventListener("keydown", window.__stepsKeys, true);
await new Promise(() => {});
"#;

/// What a message from the key script asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Go(Screen),
    Settings,
    Help,
}

fn parse(msg: &str) -> Option<Command> {
    match msg {
        "help" => Some(Command::Help),
        "go:settings" => Some(Command::Settings),
        _ => {
            let n: usize = msg.strip_prefix("go:")?.parse().ok()?;
            Screen::NAV.get(n.checked_sub(1)?).copied().map(Command::Go)
        }
    }
}

pub fn use_shortcuts() {
    let ui = use_context::<Ui>();
    let settings = use_context::<SettingsNav>();
    use_future(move || async move {
        let mut eval = document::eval(KEYS_JS);
        while let Ok(msg) = eval.recv::<String>().await {
            follow(&msg, ui, settings);
        }
    });
}

/// Does what one message from the key script asks.
fn follow(msg: &str, ui: Ui, settings: SettingsNav) {
    match parse(msg) {
        Some(Command::Go(screen)) => {
            open_sheet(false);
            settings.close();
            let mut s = ui.screen;
            s.set(screen);
        }
        Some(Command::Settings) => {
            open_sheet(false);
            settings.show(None);
        }
        Some(Command::Help) => {
            let open = !*SHEET_OPEN.peek();
            open_sheet(open);
        }
        None => {}
    }
}

/// The shortcut list: `?`, Help › Keyboard Shortcuts on the Mac, or the
/// button in Settings › Tablet and Mac.
#[component]
pub fn KeysSheet() -> Element {
    if !*SHEET_OPEN.read() {
        return rsx! {};
    }
    let screens = format!("{} … {}", nav::TODAY, nav::STATS);
    let rows: [(&[&str], String); 11] = [
        (&["Ctrl", "1–7"], format!("{} ({screens})", tx::SCREENS)),
        (&["Ctrl", ","], tx::SETTINGS.into()),
        (&["Space"], tx::REVEAL.into()),
        (&["1", "2", "3", "4"], tx::RATE.into()),
        (&["A", "B", "C", "D", "E"], tx::OPTIONS.into()),
        (&["Enter"], tx::NEXT.into()),
        (&["←", "→"], tx::MOVE.into()),
        (&["F"], tx::FLAG.into()),
        (&["Esc"], tx::ESC.into()),
        (&["?"], tx::HELP.into()),
        (&["⌘"], tx::NOTE.into()),
    ];
    rsx! {
        // The shared bounded sheet (parts.css): tap outside, ✕, Esc or
        // Android Back closes it.
        div { class: "sheet-backdrop keys-backdrop", onclick: move |_| open_sheet(false) }
        div {
            class: "sheet keys-sheet",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "keys-title",
            div { class: "sheet-head",
                h2 { id: "keys-title", {tx::TITLE} }
                button {
                    class: "icon-button",
                    "aria-keyshortcuts": "Escape",
                    "aria-label": tx::CLOSE,
                    "data-back": "true",
                    onclick: move |_| open_sheet(false),
                    "✕"
                }
            }
            table { class: "keys-table",
                tbody {
                    for (keys, what) in rows {
                        tr { key: "{what}",
                            td { {what} }
                            td { class: "keys-cell",
                                span { dir: "ltr",
                                    for k in keys.iter() {
                                        kbd { key: "{k}", "{k}" }
                                    }
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

    thread_local! {
        static NAV: std::cell::Cell<Option<(Ui, SettingsNav)>> = const { std::cell::Cell::new(None) };
    }

    fn keep() {
        NAV.with(|n| n.set(Some((consume_context(), consume_context()))));
    }

    #[test]
    fn keys_go_to_screens_settings_and_the_list() {
        let mut app = crate::render_tests::App::with(Screen::Today, keep);
        let (ui, settings) = NAV.with(std::cell::Cell::get).unwrap();
        app.run(move || follow("help", ui, settings));
        app.assert_has(tx::REVEAL);
        app.assert_has("Ctrl");
        app.tap(tx::CLOSE);
        app.assert_lacks(tx::REVEAL);
        app.run(move || follow("help", ui, settings));
        app.tap("keys-backdrop");
        app.assert_lacks(tx::REVEAL);
        app.run(move || follow("go:settings", ui, settings));
        assert!(app.can_tap("settings-backdrop"));
        app.run(move || follow("go:3", ui, settings));
        assert!(!app.can_tap("settings-backdrop"));
        assert!(app.html().contains(r#"data-screen="Study""#));
        app.run(move || follow("nothing", ui, settings));
        assert!(app.html().contains(r#"data-screen="Study""#));
    }

    #[test]
    fn messages_map_to_screens() {
        assert_eq!(parse("go:1"), Some(Command::Go(Screen::Today)));
        assert_eq!(parse("go:7"), Some(Command::Go(Screen::Stats)));
        assert_eq!(parse("go:settings"), Some(Command::Settings));
        assert_eq!(parse("help"), Some(Command::Help));
        assert_eq!(parse("go:0"), None);
        assert_eq!(parse("go:8"), None);
        assert_eq!(parse("other"), None);
    }
}
