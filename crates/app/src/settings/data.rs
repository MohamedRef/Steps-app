//! Settings › Your data: where it lives. Export, restore and sync (chunk 21)
//! and the Anki export (chunk 9) add their rows here.

use dioxus::prelude::*;

use crate::ui::Card;

use crate::i18n::settings as tx;

#[component]
pub fn DataSection() -> Element {
    let folder = crate::prefs::data_dir().map(|d| d.display().to_string());
    rsx! {
        style { {crate::today::CSS.text()} }
        style { {crate::verify::CSS.text()} }
        // Three groups: keep a copy, send it elsewhere, and this device.
        div { class: "section-head", h2 { {tx::DATA_GROUP_BACKUP} } }
        crate::data_export::ExportCard {}
        crate::import::ImportCard {}
        div { class: "section-head", h2 { {tx::DATA_GROUP_EXPORT} } }
        crate::anki::AnkiExport {}
        crate::verify::ExportVerdicts {}
        div { class: "section-head", h2 { {tx::DATA_GROUP_DEVICE} } }
        Card {
            h2 { {tx::DATA_FOLDER} }
            match folder {
                Some(f) => rsx! { bdi { class: "path", dir: "ltr", "{f}" } },
                None if cfg!(target_arch = "wasm32") => rsx! { p { class: "muted", {tx::DATA_IN_BROWSER} } },
                None => rsx! { p { class: "muted", {tx::DATA_NOWHERE} } },
            }
            if crate::store::shared().path.is_some() || (cfg!(target_arch = "wasm32") && crate::store::open_error().is_none()) {
            } else {
                p { class: "small", role: "alert", {crate::i18n::study_settings::STORE_MEMORY} }
            }
        }
        crate::diag::DiagnosticsCard {}
    }
}
