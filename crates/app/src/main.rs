//! Steps: one Dioxus codebase for the Mac, the S24 Ultra and the Tab S11.
//!
//! The same code is the web app (`web/`, docs/WEB.md): built for wasm32, the
//! browser stands in for the OS (`platform::web`).
//!
//! Feature chunks (docs/WORKSTREAMS.md) each own a module directory and register it
//! with one line in `Screen` / `screen_body` below.

// The web build leaves out the Mac/Android-only paths (the sync thread,
// updates over the air, files), so their helpers go unused there.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports, unused_variables))]

mod abbrdict;
mod abbrsheet;
mod anki;
mod back;
mod bank;
#[cfg(test)]
mod browser_fixture;
mod case;
mod catchup;
mod cloud;
mod content;
mod courses;
mod data_export;
mod dead_ends;
mod desk;
mod diag;
mod fonts;
mod gopage;
mod hover;
mod i18n;
mod icons;
mod import;
mod labs;
mod learn;
mod lecture;
mod look;
mod metacog;
mod more;
mod notes;
mod onboarding;
mod oral;
mod order;
mod own;
mod pagesheet;
mod parts;
mod persistence;
mod plan;
mod platform;
mod prefs;
mod qbank;
mod recap;
mod reminders;
#[cfg(test)]
mod render_tests;
mod settings;
mod sheet;
mod shortcuts;
mod sounds;
mod state;
mod stats;
mod store;
mod study;
mod synced;
mod table;
mod today;
mod ui;
mod update;
mod verify;
mod weak;

use crate::look::{Css, css};
use dioxus::prelude::*;

pub use state::Ui;

const MAIN_CSS: Css = css!("assets/main.css");
const SHAPE_CSS: Css = css!("assets/shape.css");
const PARTS_CSS: Css = css!("assets/parts.css");
const APPEARANCE_CSS: Css = css!("assets/appearance.css");
const LIQUID_GLASS_CSS: Css = css!("assets/liquid-glass.css");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Today,
    Learn,
    Study,
    Bank,
    Cases,
    Oral,
    Stats,
    Settings,
}

impl Screen {
    /// Bottom bar on phones (Cases, Oral and Stats under More), side rail
    /// from 600dp. Settings is a sheet opened from the top bar.
    pub const NAV: [Screen; 7] =
        [Self::Today, Self::Learn, Self::Study, Self::Bank, Self::Cases, Self::Oral, Self::Stats];

    pub fn label(self) -> &'static str {
        use i18n::nav;
        match self {
            Self::Today => nav::TODAY,
            Self::Learn => nav::LEARN,
            Self::Study => nav::STUDY,
            Self::Bank => nav::BANK,
            Self::Cases => nav::CASES,
            Self::Oral => nav::ORAL,
            Self::Stats => nav::STATS,
            Self::Settings => nav::SETTINGS,
        }
    }

    fn from_env() -> Option<Screen> {
        // STEPS_OPEN_SCREEN=study opens that screen at launch (previews and screenshots).
        Self::named(&std::env::var("STEPS_OPEN_SCREEN").ok()?)
    }

    /// The screen called `name` (`study`, `Settings`), in any case.
    fn named(name: &str) -> Option<Screen> {
        [Self::NAV.as_slice(), &[Self::Settings]]
            .concat()
            .into_iter()
            .find(|s| format!("{s:?}").eq_ignore_ascii_case(name))
    }
}

fn main() {
    diag::install_panic_hook();
    // Mac, automatic updates: a verified Steps.app staged last time goes in
    // before the window opens (and Steps reopens as the new version).
    #[cfg(target_os = "macos")]
    update::apply_staged_on_launch();
    update::init();
    #[cfg(all(feature = "desktop", not(target_os = "android")))]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        // STEPS_WINDOW_SIZE=412x900 previews the phone layout on the Mac.
        // The Mac opens wide enough for the two-column lesson (adapted from
        // batna-app main: crates/app/src/main.rs).
        let default_size = if cfg!(target_os = "macos") { (1280.0, 820.0) } else { (1100.0, 760.0) };
        let (w, h) = std::env::var("STEPS_WINDOW_SIZE")
            .ok()
            .and_then(|v| {
                let (w, h) = v.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            })
            .unwrap_or(default_size);
        let window = WindowBuilder::new()
            .with_title("Steps")
            .with_inner_size(LogicalSize::new(w, h))
            .with_min_inner_size(LogicalSize::new(360.0, 560.0));
        dioxus::LaunchBuilder::desktop()
            .with_cfg(
                Config::new()
                    .with_window(window)
                    .with_menu(settings::menu::initial())
                    .with_disable_context_menu(true),
            )
            .launch(App);
    }
    // The web: the content pack and the kept database come before the first screen.
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async {
        if platform::web::boot().await {
            dioxus::launch(App);
        }
    });
    #[cfg(not(any(all(feature = "desktop", not(target_os = "android")), target_arch = "wasm32")))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! { AppAt { start: Screen::from_env().unwrap_or(Screen::Today) } }
}

/// The whole app, opening on `start`. `children` draw last inside the app's
/// frame, with every context in reach: the screen render tests use them to
/// open sheets and pages (`render_tests.rs`); the app passes none.
#[component]
fn AppAt(start: Screen, children: Element) -> Element {
    persistence::use_provide_saves();
    let ui = use_context_provider(|| Ui {
        screen: Signal::new(start),
        prefs: Signal::new(prefs::DevicePrefs::load()),
        focus: Signal::new(false),
    });
    let settings = use_context_provider(settings::SettingsNav::new);
    use_context_provider(update::sheet::UpdateSheetNav::new);
    store::use_provide_review();
    // An unfinished Study session from before a restart, checked against
    // the review log, is back before Today first renders.
    study::use_restore();
    cloud::use_provide_cloud();
    #[cfg(target_arch = "wasm32")]
    platform::web::use_saver();
    reminders::use_local_reminders();
    // Android: a renderer reclaimed in the background ends Steps quietly
    // instead of crashing it; earlier crashes show in the diagnostics.
    use_hook(platform::webview_guard::install);
    // Time in another app doesn't count toward a Study answer's duration.
    study::use_foreground_reset();
    let current = *ui.screen.read();
    // System themes follow the device's dark mode. Android's WebView can't
    // be trusted for it (it reports light under a light activity theme), so
    // there the platform is asked directly and re-asked every few seconds;
    // elsewhere `prefers-color-scheme` does the job (device_dark is None).
    let mut device_dark = use_signal(platform::system_bars::device_dark);
    use_future(move || async move {
        if device_dark.peek().is_none() {
            return;
        }
        loop {
            crate::platform::time::sleep(std::time::Duration::from_secs(2)).await;
            let now = platform::system_bars::device_dark();
            if now.is_some() && now != *device_dark.peek() {
                device_dark.set(now);
            }
        }
    });
    let look = ui.prefs.read().css_for(*device_dark.read());
    // Android's status and navigation bars follow the theme, so True black
    // keeps the whole OLED screen dark.
    use_effect(move || {
        device_dark.read();
        let (day, night) = ui.prefs.read().theme.day_and_night();
        platform::system_bars::apply(day.bg, night.bg);
    });
    lecture::use_lecture_page(ui);
    sheet::use_gestures();
    courses::focus::use_close_on_screen_change(ui);
    back::use_back(ui, current);
    update::view::use_android_install(ui);
    update::view::use_content_switch(ui);
    update::view::use_update_notice_taps(settings);
    // A screen opens at its top, not at the last screen's scroll offset (UX #102).
    use_effect(move || {
        let _ = ui.screen.read();
        // A tapped peek belongs to the screen it was on (#322).
        document::eval(
            r#"document.querySelector("main.content")?.scrollTo(0, 0); window.__stepsHover?.hide();"#,
        );
    });
    rsx! {
        document::Title { "Steps" }
        style { {fonts::css()} }
        style { {MAIN_CSS.text()} }
        style { {SHAPE_CSS.text()} }
        style { {PARTS_CSS.text()} }
        style { {APPEARANCE_CSS.text()} }
        style { {sheet::CSS.text()} }
        style { {LIQUID_GLASS_CSS.text()} }
        style { {look} }
        desk::Chrome {}
        div {
            class: "app",
            lang: "ar",
            dir: "rtl",
            // Changing appearance only switches CSS; navigation state, input,
            // and scroll position stay mounted (assets/liquid-glass.css).
            "data-appearance": ui.prefs.read().appearance.as_str(),
            "data-focus": if *ui.focus.read() { "true" } else { "false" },
            header { class: "topbar",
                div { class: "brand",
                    span { class: "brand-name", {i18n::app::NAME} }
                    span { class: "brand-tag", {i18n::app::TAGLINE} }
                    // A narrow bar cut the line inside «USMLE» (UX #584):
                    // there it says only the exam (main.css).
                    bdi { class: "brand-tag brand-tag-short", dir: "ltr", {i18n::app::TAGLINE_SHORT} }
                }
                // Search and Settings as one group; the phone's touch way
                // into quick search (Ctrl/⌘ + K on a keyboard).
                div { class: "topbar-actions",
                    shortcuts::SearchButton {}
                    button {
                        class: "icon-button",
                        aria_label: Screen::Settings.label(),
                        title: Screen::Settings.label(),
                        onclick: move |_| settings.show(None),
                        {icons::nav(Screen::Settings)}
                    }
                }
            }
            nav { class: "nav", aria_label: i18n::nav::LABEL,
                // From 600px the top bar goes and the name heads the rail or
                // sidebar, so the screen gets the full height (nav.css).
                div { class: "nav-brand", aria_hidden: "true",
                    span { class: "brand-name", {i18n::app::NAME} }
                    span { class: "brand-tag", {i18n::app::TAGLINE} }
                }
                for screen in Screen::NAV {
                    NavButton {
                        key: "{screen:?}",
                        screen,
                        class: more::nav_class(screen),
                    }
                }
                // Phones: Cases, Oral and Progress sit under More (more.rs).
                more::More {}
                // From 600px: Courses, References, Abbreviations and a new card.
                more::ToolButtons {}
                // From 600px the rail or sidebar carries Settings at its foot (nav.css).
                button {
                    class: "nav-item nav-foot",
                    aria_label: Screen::Settings.label(),
                    onclick: move |_| settings.show(None),
                    span { class: "glyph", aria_hidden: "true", {icons::nav(Screen::Settings)} }
                    span { class: "label", {Screen::Settings.label()} }
                }
            }
            main { class: "content", "data-screen": "{current:?}",
                if !*settings.open.read() { persistence::SaveNotice {} }
                update::view::UpdateBanner {}
                if current == Screen::Today {
                    update::view::ContentNote {}
                }
                // A lecture's page opens over the screen it came from (lecture.rs).
                // A content update switched in while open draws it all again.
                for rev in [*content::REV.read()] {
                    div { key: "{rev}", class: "content-rev",
                        match lecture::shown(current) {
                            Some(id) => rsx! { lecture::LecturePage { key: "{id}", id } },
                            None => screen_body(current),
                        }
                    }
                }
            }
            settings::SettingsSheet {}
            // After Settings, which opens it, so it draws and takes Back on top.
            shortcuts::KeysSheet {}
            shortcuts::Palette {}
            update::sheet::UpdateSheet {}
            pagesheet::BookSheet {}
            gopage::GoPage {}
            abbrsheet::AbbrSheet {}
            labs::LabSheet {}
            // After the page sheet, which opens it, so it draws on top.
            notes::NoteSheet {}
            courses::Picker {}
            own::ShareInbox {}
            {children}
            onboarding::Onboarding {}
        }
    }
}

/// The body of each screen; Settings is presented as an overlay.
fn screen_body(screen: Screen) -> Element {
    match screen {
        Screen::Today => rsx! { today::Today {} },
        Screen::Learn => rsx! { learn::Learn {} },
        Screen::Study => rsx! { study::Study {} },
        Screen::Bank => rsx! { bank::Bank {} },
        Screen::Cases => rsx! { case::CasesScreen {} },
        Screen::Oral => rsx! { oral::Oral {} },
        Screen::Stats => rsx! { stats::Stats {} },
        // Settings is a sheet over the current screen, not a standalone page.
        Screen::Settings => rsx! { today::Today {} },
    }
}

#[component]
fn NavButton(screen: Screen, class: &'static str) -> Element {
    let mut ui = use_context::<Ui>();
    // A tool page on top (المواد, مراجعك…) takes the mark (UX #583).
    let active = *ui.screen.read() == screen && more::open_tool(screen).is_none();
    rsx! {
        button {
            class,
            "aria-current": if active { "page" } else { "false" },
            "aria-label": screen.label(),
            onclick: move |_| ui.screen.set(screen),
            span { class: "glyph", {icons::nav(screen)} }
            span { class: "label", {screen.label()} }
        }
    }
}
