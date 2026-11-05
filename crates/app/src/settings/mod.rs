//! Settings: one sheet over the current screen, so opening it never loses a
//! study session. The gear in the top bar opens it.
//!
//! A feature chunk that needs settings adds its `Section` variant (in
//! `ALL`, with a title, summary and id) and one line in `SectionView`, and
//! keeps the rows in its own module. Sections with nothing in them yet are
//! simply not listed.
// adapted from batna-app main: crates/app/src/settings/mod.rs

#[cfg(test)]
mod browser;

mod about;
mod data;
mod look;
#[cfg(all(feature = "desktop", not(target_os = "android")))]
pub mod menu;
mod search;

use dioxus::prelude::*;

use crate::i18n::settings as tx;
use crate::look::{Css, css};
use crate::ui::{ActionRow, Card};

const SETTINGS_CSS: Css = css!("assets/settings.css");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Study,
    Reminders,
    Sync,
    Look,
    /// Pen hover and the sidebar, on Android (the Tab S11, the S24 Ultra).
    Tablet,
    /// Mouse hover, the sidebar and the keyboard, on the Mac and desktops.
    Mac,
    Data,
    Updates,
    About,
}

impl Section {
    pub const ALL: [Self; 9] = [
        Self::Study,
        Self::Reminders,
        Self::Sync,
        Self::Look,
        Self::Tablet,
        Self::Mac,
        Self::Data,
        Self::Updates,
        Self::About,
    ];

    /// Stable id, for `STEPS_OPEN_SETTINGS=<id>` previews.
    pub fn id(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Reminders => "reminders",
            Self::Sync => "sync",
            Self::Look => "look",
            Self::Tablet => "tablet",
            Self::Mac => "mac",
            Self::Data => "data",
            Self::Updates => "updates",
            Self::About => "about",
        }
    }

    /// Only the sections that apply on this device are listed.
    pub fn applies(self) -> bool {
        match self {
            Self::Tablet => cfg!(target_os = "android"),
            Self::Mac => !cfg!(target_os = "android"),
            _ => true,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }

    fn title(self) -> &'static str {
        match self {
            Self::Study => crate::i18n::study_settings::SECTION,
            Self::Reminders => crate::i18n::remind::SECTION,
            Self::Sync => crate::i18n::cloud::SECTION,
            Self::Look => tx::LOOK,
            Self::Tablet => crate::i18n::desk::TABLET,
            Self::Mac => crate::i18n::desk::MAC,
            Self::Data => tx::DATA,
            Self::Updates => crate::i18n::update::SECTION,
            Self::About => tx::ABOUT,
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Study => crate::i18n::study_settings::SECTION_SUB,
            Self::Reminders => crate::i18n::remind::SECTION_SUB,
            Self::Sync => crate::i18n::cloud::SECTION_SUB,
            Self::Look => tx::LOOK_SUB,
            Self::Tablet => crate::i18n::desk::TABLET_SUB,
            Self::Mac => crate::i18n::desk::MAC_SUB,
            Self::Data => tx::DATA_SUB,
            Self::Updates => crate::i18n::update::SECTION_SUB,
            Self::About => tx::ABOUT_SUB,
        }
    }
    fn matches(self, query: &str) -> bool {
        let keywords = match self {
            Self::Study => {
                "study review daily limit fsrs sounds vibration onboarding weak marks مذاكرة مراجعة حد يومي صوت اهتزاز ترحيب علامات"
            }
            Self::Reminders => "notifications permission quiet hours اشعارات تنبيهات اذونات هدوء مواعيد",
            Self::Sync => "google sign in account cloud devices جوجل دخول حساب مزامنة أجهزة",
            Self::Look => {
                "theme font text size appearance stepped calm outline tinted glass ثيم خط حجم ألوان شكل نمط درجات"
            }
            Self::Tablet => "pen hover sidebar navigation قلم قائمة تابلت",
            Self::Mac => "keyboard shortcuts mouse hover sidebar كيبورد اختصارات ماوس قائمة",
            Self::Data => {
                "backup restore export import anki diagnostics نسخة احتياطية استرجاع تصدير استيراد تشخيص"
            }
            Self::Updates => "version download install changelog اصدار نسخة تنزيل تثبيت جديد",
            Self::About => "version content book source اصدار نسخة محتوى كتاب مصدر",
        };
        search::matches(query, &format!("{} {} {} {keywords}", self.id(), self.title(), self.summary()))
    }
}

/// Whether the sheet is open and on which section. `section: None` is the
/// section list on the phone; wider windows show the first section beside it.
#[derive(Clone, Copy)]
pub struct SettingsNav {
    pub open: Signal<bool>,
    pub section: Signal<Option<Section>>,
    query: Signal<String>,
}

impl SettingsNav {
    pub fn new() -> Self {
        // STEPS_OPEN_SETTINGS=look (or "" for the list) opens the sheet at launch.
        let start = std::env::var("STEPS_OPEN_SETTINGS").ok();
        Self {
            open: Signal::new(start.is_some()),
            section: Signal::new(start.as_deref().and_then(Section::from_id)),
            query: Signal::new(String::new()),
        }
    }

    pub fn show(mut self, section: Option<Section>) {
        self.query.set(String::new());
        self.section.set(section);
        self.open.set(true);
    }

    pub fn close(mut self) {
        self.open.set(false);
    }
}

/// The Settings sheet; renders nothing while closed.
#[component]
pub fn SettingsSheet() -> Element {
    let nav = use_context::<SettingsNav>();
    if !*nav.open.read() {
        return rsx! {};
    }
    let chosen = nav.section.read().filter(|s| s.applies());
    // Phones show the list until a section is picked; wider windows show both.
    let current = chosen.unwrap_or(Section::ALL[0]);
    let drilled = chosen.is_some();
    let mut section_sig = nav.section;

    rsx! {
        style { {SETTINGS_CSS.text()} }
        div { class: "settings-backdrop", onclick: move |_| nav.close() }
        div {
            class: "settings-window",
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: "settings-title",
            tabindex: "-1",
            "data-drilled": if drilled { "true" } else { "false" },
            onmounted: move |_| {
                document::eval(include_str!("../../assets/settings-dialog.js"));
            },
            header { class: "settings-head",
                h2 { id: "settings-title",
                    span { class: "settings-title-list", {tx::TITLE} }
                    span { class: "settings-title-section", {current.title()} }
                }
                button {
                    class: "icon-button",
                    "data-back": "true",
                    aria_label: tx::CLOSE,
                    title: tx::CLOSE,
                    onclick: move |_| nav.close(),
                    span { aria_hidden: "true", "✕" }
                }
                // Last in the header, so system Back picks it over Close
                // whenever it shows (the phone, inside a section).
                button {
                    class: "icon-button settings-back",
                    "data-back": "true",
                    aria_label: tx::BACK,
                    onclick: move |_| section_sig.set(None),
                    span { aria_hidden: "true", "→" }
                }
            }
            crate::persistence::SaveNotice {}
            div { class: "settings-body",
                SectionList { current }
                div { class: "settings-pane", key: "{current.id()}", role: "region", aria_labelledby: "settings-section-title",
                    header { class: "settings-intro",
                        h3 { id: "settings-section-title", {current.title()} }
                    }
                    SectionView { section: current }
                }
            }
        }
    }
}

const GROUPS: [(&str, &[Section]); 3] = [
    (tx::GROUP_STUDY, &[Section::Study, Section::Reminders]),
    (tx::GROUP_DEVICE, &[Section::Look, Section::Tablet, Section::Mac]),
    (tx::GROUP_DATA, &[Section::Sync, Section::Data, Section::Updates, Section::About]),
];

#[component]
fn SectionList(current: Section) -> Element {
    let nav = use_context::<SettingsNav>();
    let mut section = nav.section;
    let mut query = nav.query;
    let updates = crate::update::view::use_update_status().read().clone();
    let filtered: Vec<Section> =
        Section::ALL.into_iter().filter(|s| s.applies() && s.matches(&query.read())).collect();
    let detail = |s: Section| -> String {
        match s {
            Section::Updates => updates
                .as_ref()
                .and_then(crate::update::view::summary)
                .unwrap_or_else(|| s.summary().to_string()),
            // The version isolated left to right (LRI … PDI): bare in the
            // Arabic line it read "alpha.0-1.0.0" (UX #295).
            Section::About => format!("{} \u{2066}{}\u{2069}", tx::VERSION, env!("CARGO_PKG_VERSION")),
            _ => s.summary().to_string(),
        }
    };
    rsx! {
        aside { class: "settings-index",
            div { class: "settings-tools",
                input {
                    class: "settings-search", r#type: "search",
                    aria_label: tx::SEARCH, placeholder: tx::SEARCH,
                    value: "{query}", oninput: move |event| query.set(event.value()),
                }
            }
            nav { class: "settings-sections", aria_label: tx::SECTIONS,
                for (title, members) in GROUPS {
                    {
                        let shown: Vec<Section> = members.iter().copied().filter(|s| filtered.contains(s)).collect();
                        rsx! {
                            if !shown.is_empty() {
                                div { key: "{title}", class: "section-head", h2 { {title} } }
                                Card { class: "grouped",
                                    for s in shown {
                                        ActionRow {
                                            key: "{s.id()}", title: s.title(), detail: detail(s),
                                            selected: current == s, icon: crate::icons::section(s.id()),
                                            onclick: move |_| section.set(Some(s)),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if filtered.is_empty() {
                    div { class: "settings-empty", role: "status",
                        p { {tx::NO_RESULTS} }
                        button { r#type: "button", class: "btn btn-secondary", onclick: move |_| query.set(String::new()), {tx::CLEAR_SEARCH} }
                    }
                }
            }
        }
    }
}

#[component]
fn SectionView(section: Section) -> Element {
    match section {
        Section::Study => {
            rsx! {
                crate::today::StudySection {}
                crate::learn::ViewSection {}
                crate::weak::WeakSection {}
                crate::sounds::SoundSection {}
                crate::onboarding::ReopenCard {}
            }
        }
        Section::Reminders => rsx! { crate::reminders::RemindSection {} },
        Section::Sync => rsx! { crate::cloud::SyncSection {} },
        Section::Look => rsx! { look::LookSection {} },
        Section::Tablet | Section::Mac => rsx! { crate::desk::DeviceSettings { section } },
        Section::Data => rsx! { data::DataSection {} },
        Section::Updates => rsx! { crate::update::view::UpdatesSection {} },
        Section::About => rsx! { about::AboutSection {} },
    }
}

/// A small pill after a field label: this choice stays on this device.
#[component]
pub fn DeviceOnly() -> Element {
    rsx! { span { class: "chip settings-scope", {tx::THIS_DEVICE} } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_ids_round_trip() {
        for s in Section::ALL {
            assert_eq!(Section::from_id(s.id()), Some(s));
        }
        assert_eq!(Section::from_id("nope"), None);
    }

    #[test]
    fn search_finds_preferences_inside_their_section() {
        assert!(Section::Look.matches("حجم الخط"));
        assert!(Section::Look.matches("FONT size"));
        assert!(Section::Reminders.matches("إِشْعَارات"));
        assert!(Section::Data.matches("نسخة احتياطية"));
        assert!(!Section::ALL.into_iter().any(|s| s.matches("zzzz-no-setting")));
        for section in Section::ALL {
            assert!(section.matches(section.id()));
        }
    }

    #[test]
    fn every_section_is_in_exactly_one_group() {
        for section in Section::ALL {
            assert_eq!(GROUPS.iter().filter(|(_, members)| members.contains(&section)).count(), 1);
        }
    }
}
