//! Settings → "Export my checks": every ✓ / ✎ / ✗ as `steps.verdicts/v1`
//! YAML. `steps-tools` reads it back to print a fix list per lecture and to
//! add his approvals to `content/approved.txt`.

use dioxus::prelude::*;
use steps_core::review::verify::Verdict;

use crate::i18n::verify as tx;
use crate::store::{export_dir, now_ms, use_review};

#[component]
pub fn ExportVerdicts() -> Element {
    let review = use_review();
    let mut result = use_signal(|| None::<Result<(String, String), String>>);
    // Each export opens the Android "Save to" dialog afresh.
    let mut attempt = use_signal(|| 0u32);
    let counts = review.backend.borrow().verdict_counts();
    let count = |v| counts.get(&v).copied().unwrap_or(0);
    let total: usize = counts.values().sum();

    let export = move || {
        let now = now_ms();
        let verdicts = review.backend.borrow().verdicts(now);
        let yaml = match serde_yaml_ng::to_string(&verdicts) {
            Ok(y) => y,
            Err(e) => return result.set(Some(Err(e.to_string()))),
        };
        // His local date, not UTC: an export at 1 am must not overwrite yesterday's (#36).
        let date = crate::store::zone().calendar_day(now).iso();
        #[cfg(target_arch = "wasm32")]
        let saved = crate::platform::web::download(
            &format!("steps-verdicts-{date}.yaml"),
            "application/yaml",
            yaml.as_bytes(),
        );
        #[cfg(not(target_arch = "wasm32"))]
        let saved = export_dir().and_then(|dir| {
            let path = dir.join(format!("steps-verdicts-{date}.yaml"));
            std::fs::write(&path, &yaml).map(|_| path.display().to_string()).map_err(|e| e.to_string())
        });
        attempt += 1;
        result.set(Some(saved.map(|p| (p, yaml))));
    };

    rsx! {
        crate::ui::Card { class: "",
            h2 { {tx::EXPORT_TITLE} }
            p { class: "small",
                "✓ {count(Verdict::Approved)} · ✎ {count(Verdict::Edited)} · ✗ {count(Verdict::Wrong)}"
            }
            button { class: "btn btn-primary", disabled: total == 0, onclick: {
                    let mut export = export.clone();
                    move |_| export()
                },
                {tx::EXPORT_BUTTON}
            }
            match result() {
                Some(Ok((path, yaml))) => rsx! {
                    p { class: "small", role: "status", "{tx::EXPORT_SAVED} " bdi { dir: "ltr", "{path}" } }
                    if cfg!(target_os = "android") {
                        crate::data_export::SaveTo { key: "{attempt}", path: path.clone() }
                    }
                    details {
                        summary { class: "small", {tx::EXPORT_SHOW} }
                        textarea { class: "export-text", dir: "ltr", readonly: true, rows: 10, value: "{yaml}" }
                    }
                },
                Some(Err(e)) => rsx! {
                    crate::persistence::WriteFailed {
                        message: tx::EXPORT_FAILED,
                        detail: e,
                        on_retry: {
                            let mut export = export.clone();
                            move |_| export()
                        },
                    }
                },
                None => rsx! {},
            }
        }
    }
}
