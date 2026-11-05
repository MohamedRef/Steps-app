// adapted from batna-app main: crates/app/src/settings/browser.rs
//! Browser fixtures render production Settings with the app's own preferences.
//! Rust callbacks are not hydrated; this suite checks layout and the production
//! keyboard controller, not application click or persistence integration.

use super::*;
use crate::{
    Screen, Ui,
    prefs::{Appearance, DevicePrefs, TextSize},
};
use std::cell::RefCell;
use steps_core::theme::Theme;

#[derive(Clone, Copy)]
struct Case {
    theme: Theme,
    appearance: Appearance,
    size: TextSize,
}
thread_local! { static CASE: RefCell<Option<Case>> = const { RefCell::new(None) }; }

fn fixture() -> Element {
    let case = CASE.with(|c| c.borrow().unwrap());
    crate::persistence::use_provide_saves();
    let prefs = DevicePrefs {
        theme: case.theme,
        appearance: case.appearance,
        text_size: case.size,
        ..DevicePrefs::default()
    };
    use_context_provider(|| Ui {
        screen: Signal::new(Screen::Today),
        prefs: Signal::new(prefs),
        focus: Signal::new(false),
    });
    use_context_provider(|| SettingsNav {
        open: Signal::new(true),
        section: Signal::new(Some(Section::Look)),
        query: Signal::new(String::new()),
    });
    rsx! {
        style { {crate::fonts::css()} }
        style { {crate::MAIN_CSS.text()} }
        style { {crate::SHAPE_CSS.text()} }
        style { {crate::PARTS_CSS.text()} }
        style { {crate::APPEARANCE_CSS.text()} }
        style { {crate::LIQUID_GLASS_CSS.text()} }
        style { {prefs.css()} }
        div { class: "app", lang: "ar", dir: "rtl", "data-appearance": case.appearance.as_str(),
            main { id: "background", button { id: "opener", r#type: "button", "Settings" } }
            SettingsSheet {}
        }
    }
}

#[test]
#[ignore = "exports production HTML for the dedicated browser CI job"]
fn export_browser_fixtures() {
    let dir =
        std::path::PathBuf::from(std::env::var_os("APP_BROWSER_FIXTURES").expect("APP_BROWSER_FIXTURES"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut manifest = Vec::new();
    for width in [320, 384, 600, 840, 1280] {
        // Every theme in the default style, and every style on one light, one
        // dark and the true-black theme: the pairs that differ, without
        // multiplying the run by every combination.
        let cases = Theme::ALL.into_iter().map(|t| (t, Appearance::default())).chain(
            [Theme::Paper, Theme::Night, Theme::Black].into_iter().flat_map(|t| {
                Appearance::ALL.into_iter().filter(|a| *a != Appearance::default()).map(move |a| (t, a))
            }),
        );
        for (theme, appearance) in cases {
            for size in [TextSize::Normal, TextSize::Larger] {
                let case = Case { theme, appearance, size };
                CASE.with(|c| *c.borrow_mut() = Some(case));
                let mut dom = VirtualDom::new(fixture);
                dom.rebuild_in_place();
                let html = dioxus::ssr::render(&dom);
                assert!(html.contains("settings-window") && html.contains("text-sample"));
                let file =
                    format!("{width}-{}-{}-{}.html", theme.as_str(), appearance.as_str(), size.as_str());
                std::fs::write(dir.join(&file), format!("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"></head><body>{html}</body></html>")).unwrap();
                manifest.push(serde_json::json!({"file":file,"width":width,"dir":"rtl","theme":theme.as_str(),"appearance":appearance.as_str(),"size":size.as_str()}));
            }
        }
    }
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}
