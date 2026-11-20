//! Files shared to Steps on Android (#836): «Share» or «Open with» on an Excel
//! sheet, a zip, or a JSON or YAML file. `app.steps.ShareActivity` copies the
//! file into the app and keeps its name in `PendingFile`; [`FileInbox`] takes
//! it when Steps comes to the front, opens Settings › بياناتك and hands it to
//! the card that reads it: a Steps backup to the restore card, anything else
//! to «ضيف أسئلة من ملف», which says what it can't read. Nothing on the Mac
//! or the web, where the cards' own file inputs pick files.

use dioxus::prelude::*;
use serde::Deserialize;

use crate::settings::{Section, SettingsNav};
use crate::store::import::{ImportError, parse};

/// A shared file, waiting for its card to read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrived {
    pub name: String,
    /// The file's bytes; `None` when Android couldn't copy it in.
    pub bytes: Option<Vec<u8>>,
}

/// A file for «ضيف أسئلة من ملف».
pub static QUESTIONS: GlobalSignal<Option<Arrived>> = Signal::global(|| None);
/// A Steps backup, for the restore card.
pub static BACKUP: GlobalSignal<Option<Arrived>> = Signal::global(|| None);

/// The cards' ids, to bring the one that got the file into view.
pub const QUESTIONS_CARD: &str = "own-questions";
pub const BACKUP_CARD: &str = "restore";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum For {
    Questions,
    Backup,
}

/// Which card reads a file: a JSON file that is a Steps backup goes to the
/// restore card (one from a newer Steps too, which says so); anything else is
/// a file of questions.
pub fn route(name: &str, bytes: &[u8]) -> For {
    let json = name.rsplit_once('.').is_some_and(|(_, e)| e.eq_ignore_ascii_case("json"));
    let backup = json
        && std::str::from_utf8(bytes).is_ok_and(|t| matches!(parse(t), Ok(_) | Err(ImportError::TooNew(_))));
    if backup { For::Backup } else { For::Questions }
}

/// Hands a file to its card and opens Settings › بياناتك on it.
pub fn receive(arrived: Arrived, settings: SettingsNav) {
    let to = arrived.bytes.as_deref().map_or(For::Questions, |b| route(&arrived.name, b));
    match to {
        For::Questions => *QUESTIONS.write() = Some(arrived),
        For::Backup => *BACKUP.write() = Some(arrived),
    }
    settings.show(Some(Section::Data));
}

/// Brings a card into view once it has the file.
pub fn scroll_to(id: &str) {
    document::eval(&format!(
        r#"document.getElementById("{id}")?.scrollIntoView({{behavior: "smooth", block: "start"}});"#
    ));
}

/// What `PendingFile` keeps: the file's name, and its copy (empty when it
/// couldn't be copied).
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Pending {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn parse_pending(json: &str) -> Option<Pending> {
    serde_json::from_str(json).ok()
}

/// Reads the waiting file's copy and deletes it.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn open_pending(pending: Pending) -> Arrived {
    let bytes = (!pending.path.is_empty()).then(|| std::fs::read(&pending.path)).and_then(|read| {
        let _ = std::fs::remove_file(&pending.path);
        read.map_err(|e| crate::diag::report!("Shared file: {e}")).ok()
    });
    Arrived { name: pending.name, bytes }
}

/// The waiting shared file, if any; it is forgotten once taken.
fn take() -> Option<Arrived> {
    #[cfg(target_os = "android")]
    {
        match android::take() {
            Ok(json) => json.as_deref().and_then(parse_pending).map(open_pending),
            Err(e) => {
                crate::diag::report!("Shared file: {e}");
                None
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    None
}

/// Mounted once in the app: picks up files shared to Steps (Android).
#[component]
pub fn FileInbox() -> Element {
    let settings = use_context::<SettingsNav>();
    use_future(move || async move {
        if !cfg!(target_os = "android") {
            return;
        }
        // A share brings Steps to the front, so look each time the page
        // becomes visible (and once at start, for a cold start).
        let mut eval = document::eval(
            r#"
            const send = () => { if (!document.hidden) dioxus.send(true); };
            document.addEventListener("visibilitychange", send);
            send();
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            if let Some(arrived) = take() {
                receive(arrived, settings);
            }
        }
    });
    rsx! {}
}

// Adapted from crates/app/src/own/share.rs (android::take), for `PendingFile`.
#[cfg(target_os = "android")]
mod android {
    use jni::objects::{JClassLoader, JString};
    use jni::refs::LoaderContext;
    use jni::{JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    pub fn take() -> Result<Option<String>, String> {
        with_activity(|env, activity| {
            let loader = env
                .call_method(
                    activity,
                    jni_str!("getClassLoader"),
                    jni_sig!("()Ljava/lang/ClassLoader;"),
                    &[],
                )?
                .l()?;
            let loader = env.cast_local::<JClassLoader>(loader)?;
            let class =
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.PendingFile"), true)?;
            let v = env
                .call_static_method(
                    &class,
                    jni_str!("take"),
                    jni_sig!("(Landroid/content/Context;)Ljava/lang/String;"),
                    &[JValue::Object(activity)],
                )?
                .l()?;
            if v.is_null() {
                return Ok(None);
            }
            let s = env.cast_local::<JString>(v)?;
            Ok(Some(s.try_to_string(env)?))
        })
    }
}

#[cfg(test)]
#[path = "inbox_tests.rs"]
mod tests;
