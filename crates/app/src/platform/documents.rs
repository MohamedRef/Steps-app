//! Android's system document dialogs for Settings › Your data: "Save to" for
//! an export, and the file picker for a restore (`app.steps.Documents` in
//! android/). The Mac writes to ~/Downloads and uses the WebView's own file
//! input, so these are Android-only; elsewhere they report `Failed`.
// Adapted from kabar-app main: crates/app/src/platform/android.rs (save_document, save_state).

use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub enum DocState {
    /// The dialog is up (or the copy is running).
    Busy,
    Failed,
    Cancelled,
    /// Saved; the name the learner chose, when the provider tells it.
    Saved(Option<String>),
    /// Picked: the file's name, and the app-cache copy to read.
    Opened {
        name: String,
        path: String,
    },
}

/// How often the UI asks while a dialog is up.
pub const POLL: Duration = Duration::from_millis(300);

/// Shows "Save to" for the file at `path`, suggesting `name`.
pub fn save(path: &str, name: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return android::save(path, name);
    #[cfg(not(target_os = "android"))]
    {
        let _ = (path, name);
        Err("not on Android".into())
    }
}

/// Shows the system file picker.
pub fn open() -> Result<(), String> {
    #[cfg(target_os = "android")]
    return android::open();
    #[cfg(not(target_os = "android"))]
    Err("not on Android".into())
}

/// The dialog's state; a finished state is handed out once.
pub fn state() -> DocState {
    #[cfg(target_os = "android")]
    return android::state().unwrap_or_else(|e| {
        crate::diag::report!("Documents: {e}");
        DocState::Failed
    });
    #[cfg(not(target_os = "android"))]
    DocState::Failed
}

/// Polls until the dialog finishes.
pub async fn finished() -> DocState {
    loop {
        match state() {
            DocState::Busy => crate::platform::time::sleep(POLL).await,
            done => return done,
        }
    }
}

#[cfg(target_os = "android")]
mod android {
    use jni::objects::{JClass, JClassLoader, JObject, JString};
    use jni::refs::LoaderContext;
    use jni::{Env, JValue, jni_sig, jni_str};

    use super::DocState;
    use crate::platform::tts_android::with_activity;

    /// `app.steps.Documents`, loaded through the Activity's class loader.
    fn documents<T>(
        f: impl FnOnce(&mut Env, &JClass, &JObject) -> jni::errors::Result<T>,
    ) -> Result<T, String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.Documents"), true)?;
            f(env, &class, activity)
        })
    }

    fn string(env: &mut Env, obj: JObject) -> jni::errors::Result<Option<String>> {
        if obj.is_null() {
            return Ok(None);
        }
        let s = env.cast_local::<JString>(obj)?;
        Ok(Some(s.try_to_string(env)?))
    }

    pub fn save(path: &str, name: &str) -> Result<(), String> {
        documents(|env, class, activity| {
            let (path, name) = (env.new_string(path)?, env.new_string(name)?);
            env.call_static_method(
                class,
                jni_str!("save"),
                jni_sig!("(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)V"),
                &[JValue::Object(activity), JValue::Object(&path), JValue::Object(&name)],
            )
            .map(drop)
        })
    }

    pub fn open() -> Result<(), String> {
        documents(|env, class, activity| {
            env.call_static_method(
                class,
                jni_str!("open"),
                jni_sig!("(Landroid/app/Activity;)V"),
                &[JValue::Object(activity)],
            )
            .map(drop)
        })
    }

    pub fn state() -> Result<DocState, String> {
        documents(|env, class, _| {
            let code = env.call_static_method(class, jni_str!("state"), jni_sig!("()I"), &[])?.i()?;
            if code == 1 {
                return Ok(DocState::Busy);
            }
            let name = env
                .call_static_method(class, jni_str!("name"), jni_sig!("()Ljava/lang/String;"), &[])?
                .l()?;
            let name = string(env, name)?.filter(|n| !n.is_empty());
            let path = env
                .call_static_method(class, jni_str!("path"), jni_sig!("()Ljava/lang/String;"), &[])?
                .l()?;
            let path = string(env, path)?;
            env.call_static_method(class, jni_str!("reset"), jni_sig!("()V"), &[])?;
            Ok(match (code, path) {
                (3, _) => DocState::Saved(name),
                (5, Some(path)) => DocState::Opened { name: name.unwrap_or_default(), path },
                (4, _) | (0, _) => DocState::Cancelled,
                _ => DocState::Failed,
            })
        })
    }
}
