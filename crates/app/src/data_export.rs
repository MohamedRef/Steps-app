//! Settings › Your data › Export everything: one click writes the whole
//! store to `steps-export-YYYY-MM-DD.json`. The document comes from
//! `Store::export`; this module names, writes and reports the file.
//! On Android the file lands in the app's own folder first, then the system
//! "Save to" dialog offers Downloads or Drive (`platform::documents`).
// adapted from kabar-app main: crates/app/src/data_export.rs

use std::io::Write;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use crate::i18n::data as tx;
use crate::platform::documents::{self, DocState};
use crate::store::{self, Store};

/// `steps-export-2026-10-27.json`, or `…-2.json`, `…-3.json` when one exists.
pub fn file_path(dir: &Path, date: &str) -> PathBuf {
    let base = format!("steps-export-{date}");
    let first = dir.join(format!("{base}.json"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{base}-{n}.json"))).find(|p| !p.exists()).unwrap_or(first)
}

/// Today's local date, `YYYY-MM-DD`.
pub fn local_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Writes the export into `dir` (created if missing); returns the path and
/// the number of rows. Written to a temporary file first, so a failure never
/// leaves half a file behind.
pub fn save(store: &Store, dir: &Path, date: &str, now_ms: i64) -> Result<(PathBuf, u64), String> {
    let export = store.export(env!("CARGO_PKG_VERSION"), now_ms).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = file_path(dir, date);
    let tmp = path.with_extension("json.part");
    let write = || -> std::io::Result<()> {
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        serde_json::to_writer_pretty(&mut out, &export.document)?;
        out.write_all(b"\n")?;
        out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        std::fs::rename(&tmp, &path)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })?;
    Ok((path, export.rows))
}

/// The web: the same export as a browser download; returns its name and rows.
#[cfg(target_arch = "wasm32")]
fn download(store: &Store, date: &str, now_ms: i64) -> Result<(String, u64), String> {
    let export = store.export(env!("CARGO_PKG_VERSION"), now_ms).map_err(|e| e.to_string())?;
    let mut text = serde_json::to_vec_pretty(&export.document).map_err(|e| e.to_string())?;
    text.push(b'\n');
    let name = format!("steps-export-{date}.json");
    crate::platform::web::download(&name, "application/json", &text).map(|n| (n, export.rows))
}

/// Android: hands a file the app just wrote to the system "Save to" dialog
/// (Downloads, Drive …), once on its own, then again on a tap. Key it by
/// attempt so each new export opens the dialog.
#[component]
pub fn SaveTo(path: String) -> Element {
    let mut moved = use_signal(|| None::<DocState>);
    let mut save_to = move |path: String| {
        let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        moved.set(Some(DocState::Busy));
        match documents::save(&path, &name) {
            Ok(()) => {
                spawn(async move { moved.set(Some(documents::finished().await)) });
            }
            Err(_) => moved.set(Some(DocState::Failed)),
        }
    };
    use_hook({
        let path = path.clone();
        move || save_to(path)
    });
    match moved() {
        Some(DocState::Busy) => rsx! { p { class: "muted small", {tx::EXPORT_SAVING} } },
        Some(DocState::Saved(Some(name))) => rsx! {
            p { class: "small", "{tx::EXPORT_SAVED_AS} " bdi { dir: "ltr", "{name}" } }
        },
        Some(DocState::Saved(None)) => rsx! { p { class: "small", {tx::EXPORT_SAVED_OUT} } },
        other => rsx! {
            p { class: "muted small",
                if other == Some(DocState::Cancelled) {
                    {tx::EXPORT_SAVE_CANCELLED}
                } else if other.is_some() {
                    {tx::EXPORT_SAVE_FAILED}
                }
                " {tx::EXPORT_ANDROID_HINT}"
            }
            button { class: "verify-btn plain", onclick: move |_| save_to(path.clone()), {tx::EXPORT_SAVE_TO} }
        },
    }
}

#[component]
pub fn ExportCard() -> Element {
    let mut outcome = use_signal(|| None::<Result<(String, u64), String>>);
    // Each export opens the "Save to" dialog afresh.
    let mut attempt = use_signal(|| 0u32);
    let mut run = move || {
        #[cfg(target_arch = "wasm32")]
        let result = download(store::shared(), &local_date(), store::now_ms());
        #[cfg(not(target_arch = "wasm32"))]
        let result = store::export_dir()
            .and_then(|dir| save(store::shared(), &dir, &local_date(), store::now_ms()))
            .map(|(p, rows)| (p.display().to_string(), rows));
        attempt += 1;
        outcome.set(Some(result));
    };
    rsx! {
        crate::ui::Card {
            h2 { {tx::EXPORT_TITLE} }
            button { class: "btn btn-primary", onclick: move |_| run(), {tx::EXPORT_BUTTON} }
            match outcome() {
                Some(Ok((path, rows))) => rsx! {
                    div { role: "status",
                        p { class: "small", {tx::export_saved(rows)} }
                        p { class: "small", bdi { dir: "ltr", "{path}" } }
                        if cfg!(target_os = "android") {
                            SaveTo { key: "{attempt}", path }
                        }
                    }
                },
                // Nothing was written; the same export runs again.
                Some(Err(e)) => rsx! {
                    crate::persistence::WriteFailed { message: tx::EXPORT_FAILED, detail: e, on_retry: move |_| run() }
                },
                None => rsx! {},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_never_overwrite_and_files_are_whole() {
        let dir = std::env::temp_dir().join(format!("steps-export-test-{}", store::now_ms()));
        let s = Store::open_in_memory().unwrap();
        let (a, rows) = save(&s, &dir, "2026-10-27", 1).unwrap();
        let (b, _) = save(&s, &dir, "2026-10-27", 2).unwrap();
        assert!(a.ends_with("steps-export-2026-10-27.json"));
        assert!(b.ends_with("steps-export-2026-10-27-2.json"));
        let doc = crate::store::import::parse(&std::fs::read_to_string(&a).unwrap()).unwrap();
        assert_eq!(doc["row_count"], rows);
        assert!(
            std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().path().to_string_lossy().ends_with(".part"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
