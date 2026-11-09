//! Shared parts of the screen browser fixtures (`learn/browser.rs`,
//! `study/browser.rs`): production screens rendered by the test-only SSR
//! renderer inside the app's shell. The production stylesheets are written
//! once (`app.css`) and the real theme and text-size CSS once per preference
//! (`prefs-<theme>-<size>.css`), so the browser job switches them the way the
//! app does instead of keeping a file per combination. Rust callbacks are not
//! hydrated: these fixtures check layout, not taps or saving.

use crate::prefs::{DevicePrefs, TextSize};
use crate::{Screen, Ui};
use dioxus::prelude::*;
use std::path::PathBuf;
use steps_core::theme::Theme;

/// `$APP_BROWSER_FIXTURES/<screen>`, created, with the stylesheets in it.
pub fn out_dir(screen: &str) -> PathBuf {
    let root = std::env::var_os("APP_BROWSER_FIXTURES").expect("APP_BROWSER_FIXTURES");
    let dir = PathBuf::from(root).join(screen);
    std::fs::create_dir_all(&dir).unwrap();
    let app = [
        crate::fonts::css().to_string(),
        crate::MAIN_CSS.text().to_string(),
        crate::SHAPE_CSS.text().to_string(),
        crate::PARTS_CSS.text().to_string(),
        crate::APPEARANCE_CSS.text().to_string(),
        crate::sheet::CSS.text().to_string(),
        // Chrome draws this in every window: panes, divider, focus button.
        crate::desk::DESK_CSS.text().to_string(),
    ]
    .join("\n");
    std::fs::write(dir.join("app.css"), app).unwrap();
    for theme in Theme::ALL {
        for size in [TextSize::Normal, TextSize::Larger] {
            let prefs = DevicePrefs { theme, text_size: size, ..DevicePrefs::default() };
            let file = format!("prefs-{}-{}.css", theme.as_str(), size.as_str());
            std::fs::write(dir.join(file), prefs.css()).unwrap();
        }
    }
    dir
}

/// The app's frame around one screen, as `App` draws it (main.rs): the
/// phone bar and rail are left out, as focus mode hides them anyway.
#[component]
pub fn Shell(screen: Screen, focus: bool, children: Element) -> Element {
    use_context_provider(|| Ui {
        screen: Signal::new(screen),
        prefs: Signal::new(DevicePrefs::default()),
        focus: Signal::new(focus),
    });
    rsx! {
        div { class: "app", lang: "ar", dir: "rtl", "data-appearance": "calm",
            "data-focus": if focus { "true" } else { "false" },
            main { class: "content", "data-screen": "{screen:?}",
                div { class: "content-rev", {children} }
            }
            crate::abbrsheet::AbbrSheet {}
        }
    }
}

/// One rendered state as a page using the shared stylesheets.
pub fn write_page(dir: &std::path::Path, file: &str, dom: &VirtualDom) {
    let html = dioxus::ssr::render(dom);
    let page = format!(
        "<!doctype html><html lang=\"ar\" dir=\"rtl\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <link rel=\"stylesheet\" href=\"app.css\">\
         <link id=\"prefs\" rel=\"stylesheet\" href=\"prefs-system-normal.css\">\
         </head><body>{html}</body></html>"
    );
    std::fs::write(dir.join(file), page).unwrap();
}
