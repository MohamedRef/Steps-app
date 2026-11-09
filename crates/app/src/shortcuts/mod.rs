//! Keyboard shortcuts for tablet keyboards and the Mac (PARITY.md §6.5):
//! Ctrl/⌘ + 1–7 for the screens, Ctrl/⌘ + , for Settings, Ctrl/⌘ + G for a
//! First Aid page, and on study
//! screens Space to reveal (then Space or Enter rates Good), 1–4 to rate, Z
//! to undo, A–E for MCQ options, Enter for next, ← / → between lesson steps
//! and the questions of a timed paper, F to flag, Esc to close, ? for the
//! list. The key safety rules (no auto-repeat, nothing while typing, a
//! dialog first) are adapted from batna-app main: crates/app/src/shortcuts/mod.rs.
//!
//! Screens don't register handlers: a key presses the visible, enabled button
//! whose `aria-keyshortcuts` names it (`"Space"`, `"1"`, `"A"`, `"Enter"`,
//! `"Escape"`, `"ArrowLeft"`, `"F"`). That is also how screen readers learn the key.
// adapted from batna-app main: crates/app/src/settings/menu.rs ("?" key) and
// crates/app/src/review.rs (keys on visible buttons).

use dioxus::prelude::*;

use crate::i18n::{keys as tx, nav};
use crate::settings::SettingsNav;
use crate::{Screen, Ui};

mod palette;
pub use palette::{Palette, open as open_palette};

static SHEET_OPEN: GlobalSignal<bool> = Signal::global(|| false);

pub fn open_sheet(open: bool) {
    *SHEET_OPEN.write() = open;
}

const KEYS_JS: &str = r#"
if (window.__stepsKeys) document.removeEventListener("keydown", window.__stepsKeys, true);
window.__stepsKeys = (e) => {
  // One press, one action: a held key never runs through several cards.
  if (e.defaultPrevented || e.isComposing || e.repeat) return;
  const t = e.target;
  const typing = t && (t.isContentEditable ||
    t.closest?.('input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="textbox"]'));
  const mod = e.metaKey || e.ctrlKey;
  if (mod && !e.altKey && !e.shiftKey) {
    if (/^Digit[1-7]$/.test(e.code)) { e.preventDefault(); dioxus.send("go:" + e.code.slice(5)); return; }
    if (e.code === "Comma") { e.preventDefault(); dioxus.send("go:settings"); return; }
    if (e.code === "KeyG") { e.preventDefault(); dioxus.send("page"); return; }
    if (e.code === "KeyK") { e.preventDefault(); dioxus.send("quick"); return; }
    return;
  }
  // Quick search (palette.rs): ↑ ↓ move through the results, Enter in the
  // box opens the first one, Tab stays inside.
  const qs = document.querySelector('.qs[role="dialog"]');
  if (qs && !e.altKey) {
    const rows = [...qs.querySelectorAll(".qs-row")];
    const at = rows.indexOf(document.activeElement);
    if ((e.key === "ArrowDown" || e.key === "ArrowUp") && rows.length) {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      rows[at < 0 ? (step > 0 ? 0 : rows.length - 1) : (at + step + rows.length) % rows.length].focus();
      return;
    }
    if (e.key === "Enter" && t?.matches?.(".qs-input") && rows.length) { e.preventDefault(); rows[0].click(); return; }
    if (e.key === "Tab") {
      const all = [...qs.querySelectorAll("input, button")];
      const first = all[0], last = all[all.length - 1];
      if (e.shiftKey ? document.activeElement === first : document.activeElement === last) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
      }
      return;
    }
  }
  if (e.altKey || mod || (e.shiftKey && e.key !== "?" && e.key !== "؟")) return;
  if (e.key === "Escape" && window.__stepsHover?.visible()) {
    window.__stepsHover.hide(); e.preventDefault(); return;
  }
  // Typing an answer, a note or a search never grades, reveals or moves.
  if (typing && e.key !== "Escape") return;
  // The pane divider (desk/resize.js) owns its own arrows, Home and End.
  if (t?.closest?.(".desk-resize") && ["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
  if (e.key === "?" || e.key === "؟") { e.preventDefault(); dioxus.send("help"); return; }
  // By physical key, so an Arabic keyboard layout works the same.
  let key = null;
  if (e.code === "Space") key = "space";
  else if (e.key === "Enter") key = "enter";
  else if (e.key === "Escape") key = "escape";
  else if (/^(Digit|Numpad)[1-9]$/.test(e.code)) key = e.code.slice(-1);
  else if (e.key === "ArrowLeft" || e.key === "ArrowRight") key = e.key.toLowerCase();
  else if (/^Key[A-FZ]$/.test(e.code)) key = e.code.slice(3).toLowerCase();
  if (!key) return;
  // An open dialog takes the keys first; nothing behind it is pressed.
  const modal = document.querySelector('[role="dialog"][aria-modal="true"], [role="alertdialog"][aria-modal="true"]');
  const scope = modal || document;
  const shown = (el) => !el.disabled && !el.closest('[hidden], [inert], [aria-hidden="true"]') &&
    el.getClientRects().length > 0 && getComputedStyle(el).visibility !== "hidden";
  const find = (name) => [...scope.querySelectorAll("[aria-keyshortcuts]")].find((el) =>
    shown(el) && el.getAttribute("aria-keyshortcuts").toLowerCase().split(/\s+/).includes(name));
  let hit = find(key);
  // Like a flashcard deck: with the answer shown, Space or Enter rates Good
  // (3). Only on a Study card, never in a lesson, a dialog or an exam.
  const inStudy = !modal && document.querySelector('main.content[data-screen="Study"] .st-item');
  if (!hit && inStudy && (key === "space" || key === "enter")) hit = find("3");
  if (!hit) return;
  // Enter and Space already press a focused button; don't press twice.
  if ((key === "enter" || key === "space") && t?.closest?.("button, a[href], [role=button]")) return;
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
    Page,
    Quick,
}

fn parse(msg: &str) -> Option<Command> {
    match msg {
        "help" => Some(Command::Help),
        "page" => Some(Command::Page),
        "quick" => Some(Command::Quick),
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
    // After a key (or a screen reader) reveals, grades, undoes or turns a
    // lesson step, focus moves to the new card or step instead of falling
    // back to the page. It never presses anything; mouse clicks are left alone.
    use_future(|| async move {
        let mut eval = document::eval(include_str!("focus.js"));
        while eval.recv::<String>().await.is_ok() {}
    });
}

/// Does what one message from the key script asks.
fn follow(msg: &str, ui: Ui, settings: SettingsNav) {
    let command = parse(msg);
    if command.is_some() && command != Some(Command::Quick) {
        open_palette(false);
    }
    match command {
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
        Some(Command::Page) => {
            open_sheet(false);
            crate::gopage::open(ui);
        }
        Some(Command::Help) => {
            let open = !*SHEET_OPEN.peek();
            open_sheet(open);
        }
        Some(Command::Quick) => {
            open_sheet(false);
            open_palette(!palette::is_open());
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
    let rows: [(&[&str], String); 17] = [
        (&["Ctrl", "K"], tx::QUICK.into()),
        (&["Ctrl", "1–7"], format!("{} ({screens})", tx::SCREENS)),
        (&["Ctrl", ","], tx::SETTINGS.into()),
        (&["Ctrl", "G"], tx::PAGE.into()),
        (&["F"], tx::FOCUS.into()),
        (&["Space"], tx::REVEAL.into()),
        (&["Space", "Enter"], tx::GOOD.into()),
        (&["1", "2", "3", "4"], tx::RATE.into()),
        (&["Z"], tx::UNDO.into()),
        (&["A", "B", "C", "D", "E"], tx::OPTIONS.into()),
        (&["Enter"], tx::NEXT.into()),
        (&["←", "→"], tx::MOVE.into()),
        (&["F"], tx::FLAG.into()),
        (&["←", "→"], tx::SPLIT.into()),
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
    fn review_keys_stay_safe() {
        // The dispatcher runs as script in the web view; pin its safety rules.
        for rule in [
            "e.defaultPrevented || e.isComposing || e.repeat",
            "[role=\"textbox\"]",
            "if (typing && e.key !== \"Escape\") return",
            "const scope = modal || document",
            "[hidden], [inert], [aria-hidden=\"true\"]",
            "if (!hit && inStudy",
            "find(\"3\")",
            "Key[A-FZ]",
        ] {
            assert!(KEYS_JS.contains(rule), "missing key rule: {rule}");
        }
        // Lessons move by the arrows, right-to-left: → back, ← next.
        let player = include_str!("../learn/player.rs");
        assert!(player.contains("\"aria-keyshortcuts\": \"ArrowRight\""));
        assert!(player.contains("\"aria-keyshortcuts\": \"Enter ArrowLeft\""));
        assert!(include_str!("../study/mod.rs").contains("\"aria-keyshortcuts\": \"Z\""));
    }

    #[test]
    fn keyboard_focus_follows_the_task_but_never_presses() {
        let focus = include_str!("focus.js");
        for rule in [
            "e.detail !== 0 || modalOpen()",
            "editable(active)",
            "pointerdown",
            "freshControl(item) || item",
            "data-grade=\"good\"",
        ] {
            assert!(focus.contains(rule), "missing focus rule: {rule}");
        }
        assert!(!focus.contains(".click();"), "focus.js must never press a button");
        // The card and the lesson step can take focus when they have nothing else.
        assert!(include_str!("../study/item.rs").contains(r#"class: "st-item", tabindex: "-1""#));
        assert!(include_str!("../learn/player.rs").contains(r#"class: "ls-stage", tabindex: "-1""#));
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
