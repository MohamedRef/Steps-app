//! «Export to Anki»: builds `steps.apkg` from the content inside the app (the
//! same deck `steps-tools anki` writes, via `steps-anki`) and hands it to
//! Anki, so Mohamed never moves files by hand.
//!
//! - Android: saved to `Download/Steps/steps.apkg` through MediaStore (no
//!   permission and no FileProvider needed), then opened with an
//!   `ACTION_VIEW` intent, which AnkiDroid answers with its import screen.
//! - Mac and other desktops: saved to `~/Downloads/steps.apkg` and opened with
//!   the default app (Anki registers `.apkg`).
//!
//! His ✓/✎/✗ from verify-while-study apply: ✗ items are left out, ✎ items
//! carry his edited text, ✓ items are tagged `approved`.
//!
//! The app holds merged lectures only; the shared-folder file can also carry
//! drafts. Both use the same note ids, so either import updates the other.

use dioxus::prelude::*;

use std::collections::{BTreeMap, BTreeSet};

use steps_core::content::{Item, Lecture};
use steps_core::review::verify::{self, Check, Judgement, Verdict};

use crate::content::Library;
use crate::i18n::anki as tx;
use crate::store::use_review;

#[cfg(target_os = "android")]
mod android;

const FILE_NAME: &str = "steps.apkg";

const CSS: &str = "
.anki-export .btn:disabled { cursor: progress; }
.anki-export .result { margin-block-start: var(--sp-3); }
";

#[derive(Clone, PartialEq)]
enum State {
    Idle,
    Working,
    Done { notes: usize, place: String },
    Failed(String),
}

#[component]
pub fn AnkiExport() -> Element {
    let mut state = use_signal(|| State::Idle);
    let review = use_review();
    let lib = Library::get();
    let lectures = lib.lectures.len();
    let items: usize = lib.lectures.values().map(|l| l.items.len()).sum();
    let busy = *state.read() == State::Working;

    let export = move || {
        state.set(State::Working);
        // Let the button show "working" before the (sub-second) build runs.
        let review = review.clone();
        spawn(async move {
            futures_yield().await;
            let backend = review.backend.borrow();
            state.set(match export_now(|id| backend.judgement(id)) {
                Ok((notes, place)) => State::Done { notes, place },
                Err(e) => State::Failed(e),
            });
        });
    };

    rsx! {
        style { {CSS} }
        crate::ui::Card { class: "anki-export",
            h2 { {tx::TITLE} }
            p { class: "muted small", {tx::contents(lectures, items)} }
            button { class: "btn btn-primary", disabled: busy,
                onclick: {
                    let mut export = export.clone();
                    move |_| export()
                },
                if busy { {tx::WORKING} } else { {tx::BUTTON} }
            }
            match &*state.read() {
                State::Done { notes, place } => rsx! {
                    p { class: "note result", {tx::done(*notes)} " {tx::SAVED_IN} " bdi { dir: "ltr", "{place}" } }
                },
                State::Failed(e) => rsx! {
                    crate::persistence::WriteFailed {
                        message: tx::FAILED,
                        detail: e.clone(),
                        on_retry: {
                            let mut export = export.clone();
                            move |_| export()
                        },
                    }
                },
                _ => rsx! {},
            }
        }
    }
}

/// Yields once to the UI so the "working" label paints.
async fn futures_yield() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if yielded {
            std::task::Poll::Ready(())
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .await
}

/// What his verdicts mean for the deck: ✗ ids, ✎ items with the edit
/// applied, and ✓ ids. A verdict counts only while it matches the item's
/// current text (`Judgement::current`), the same rule review uses.
#[derive(Default)]
struct Verdicts {
    wrong: BTreeSet<String>,
    edits: BTreeMap<String, Item>,
    approved: BTreeSet<String>,
}

fn verdicts<'a>(lectures: &[Lecture], judgement: impl Fn(&str) -> Option<&'a Judgement>) -> Verdicts {
    let mut out = Verdicts::default();
    for item in lectures.iter().flat_map(|l| &l.items) {
        let j = judgement(&item.id);
        if verify::check(item, j) == Check::Hidden {
            out.wrong.insert(item.id.clone());
            continue;
        }
        match j.filter(|j| j.current(item)).map(|j| j.verdict) {
            Some(Verdict::Edited) => {
                out.edits.insert(item.id.clone(), verify::effective(item, j));
            }
            Some(Verdict::Approved) => {
                out.approved.insert(item.id.clone());
            }
            _ => {}
        }
    }
    out
}

/// Builds the deck from the embedded content and his verdicts; returns the
/// note count and where the file went.
fn export_now<'a>(judgement: impl Fn(&str) -> Option<&'a Judgement>) -> Result<(usize, String), String> {
    let mut lectures: Vec<_> = Library::get().lectures.values().cloned().collect();
    // His own cards ride along as `Steps::My own cards`.
    lectures.extend(crate::own::lecture().cloned());
    let v = verdicts(&lectures, judgement);
    let opts = steps_anki::Options {
        approved: Some(&v.approved),
        wrong: Some(&v.wrong),
        edits: Some(&v.edits),
        drafts: None,
        program: Some(&crate::content::Library::get().program),
    };
    let deck = steps_anki::build(&lectures, &opts);
    let now_ms = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let bytes = steps_anki::to_apkg(&deck, now_ms)?;
    let place = save_and_open(&bytes)?;
    Ok((deck.notes.len(), place))
}

#[cfg(target_os = "android")]
fn save_and_open(bytes: &[u8]) -> Result<String, String> {
    android::save_and_open(FILE_NAME, bytes)
}

/// The web: the deck downloads; the browser offers to open it in Anki.
#[cfg(target_arch = "wasm32")]
fn save_and_open(bytes: &[u8]) -> Result<String, String> {
    crate::platform::web::download(FILE_NAME, "application/octet-stream", bytes)
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn save_and_open(bytes: &[u8]) -> Result<String, String> {
    let home =
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).ok_or("no home folder")?;
    let downloads = std::path::Path::new(&home).join("Downloads");
    let dir = if downloads.is_dir() { downloads } else { std::path::PathBuf::from(home) };
    let path = dir.join(FILE_NAME);
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    // Opening is a convenience: the file is saved either way.
    let _ = std::process::Command::new(opener).arg(&path).spawn();
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_embedded_content_exports() {
        let lectures: Vec<_> = super::Library::get().lectures.values().cloned().collect();
        let deck = steps_anki::build(&lectures, &steps_anki::Options::default());
        let bytes = steps_anki::to_apkg(&deck, 0).unwrap();
        assert!(bytes.starts_with(b"PK"), "an .apkg is a zip");
        assert_eq!(deck.per_lecture.len(), lectures.len());
    }
}
