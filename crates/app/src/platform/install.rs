//! Android's side of updates over the air: `app.steps.UpdateInstaller`
//! (android/), reached through JNI. It checks a downloaded APK the way
//! Android will (same package, same signing key, a versionCode no lower than
//! this one) and hands it to PackageInstaller. Rust starts it and polls.
//!
//! Adapted from batna-app main: crates/app/src/platform/android.rs (the
//! `UpdateInstaller` calls).

#![allow(unsafe_code)]

use jni::objects::{JClass, JClassLoader, JObject, JString};
use jni::refs::LoaderContext;
use jni::{Env, JValue, jni_sig, jni_str};

use crate::platform::tts_android::with_activity;

/// What `UpdateInstaller` is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallState {
    Idle,
    Working,
    /// Android's confirmation screen is up.
    WaitingUser,
    Failed(String),
    Cancelled,
}

/// Runs `f` with `app.steps.UpdateInstaller`, loaded through the Activity's
/// class loader (`FindClass` from a native thread only sees system classes).
fn installer<T>(f: impl FnOnce(&mut Env, &JClass, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    with_activity(|env, activity| {
        let loader = env
            .call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?
            .l()?;
        let loader = env.cast_local::<JClassLoader>(loader)?;
        let class =
            LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.UpdateInstaller"), true)?;
        f(env, &class, activity)
    })
}

/// A static `(Activity) -> boolean` on UpdateInstaller.
fn ask(name: &'static jni::strings::JNIStr) -> Result<bool, String> {
    installer(|env, class, activity| {
        env.call_static_method(
            class,
            name,
            jni_sig!("(Landroid/app/Activity;)Z"),
            &[JValue::Object(activity)],
        )?
        .z()
    })
}

/// A static `(Activity) -> void` on UpdateInstaller.
fn tell(name: &'static jni::strings::JNIStr) -> Result<(), String> {
    installer(|env, class, activity| {
        env.call_static_method(
            class,
            name,
            jni_sig!("(Landroid/app/Activity;)V"),
            &[JValue::Object(activity)],
        )
        .map(drop)
    })
}

/// "Install unknown apps" is on for Steps.
pub fn can_request_installs() -> Result<bool, String> {
    ask(jni_str!("canRequestInstalls"))
}

/// Android 12+ and Steps installed itself last time: no confirmation screen.
pub fn can_install_quietly() -> Result<bool, String> {
    ask(jni_str!("canInstallQuietly"))
}

/// Mobile data or Data Saver.
pub fn network_metered() -> Result<bool, String> {
    ask(jni_str!("isMetered"))
}

/// Opens Steps's "Install unknown apps" switch in Android Settings.
pub fn open_install_permission() -> Result<(), String> {
    tell(jni_str!("openInstallPermission"))
}

/// The confirmation screen that arrived while Steps was in the background.
pub fn show_pending_confirm() -> Result<(), String> {
    tell(jni_str!("showPendingConfirm"))
}

/// Checks the APK and hands it to PackageInstaller. Returns at once; poll
/// [`install_state`].
pub fn install_apk(
    path: &str,
    version_code: u64,
    done_title: &str,
    done_body: &str,
    ready_title: &str,
    channel: (&str, &str),
) -> Result<(), String> {
    installer(|env, class, activity| {
        let (path, title, body) =
            (env.new_string(path)?, env.new_string(done_title)?, env.new_string(done_body)?);
        let ready = env.new_string(ready_title)?;
        let (channel_name, channel_description) = (env.new_string(channel.0)?, env.new_string(channel.1)?);
        env.call_static_method(
            class,
            jni_str!("install"),
            jni_sig!(
                "(Landroid/app/Activity;Ljava/lang/String;JLjava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"
            ),
            &[
                JValue::Object(activity),
                JValue::Object(&path),
                JValue::Long(i64::try_from(version_code).unwrap_or(0)),
                JValue::Object(&title),
                JValue::Object(&body),
                JValue::Object(&ready),
                JValue::Object(&channel_name),
                JValue::Object(&channel_description),
            ],
        )
        .map(drop)
    })
}

/// `UpdateInstaller.state()`; a finished state is reset once read.
pub fn install_state() -> Result<InstallState, String> {
    installer(|env, class, _| {
        let state = env.call_static_method(class, jni_str!("state"), jni_sig!("()I"), &[])?.i()?;
        let out = match state {
            1 => return Ok(InstallState::Working),
            2 => return Ok(InstallState::WaitingUser),
            3 => {
                let msg = env
                    .call_static_method(class, jni_str!("message"), jni_sig!("()Ljava/lang/String;"), &[])?
                    .l()?;
                let msg = if msg.is_null() {
                    String::new()
                } else {
                    env.cast_local::<JString>(msg)?.mutf8_chars(env)?.to_string()
                };
                InstallState::Failed(msg)
            }
            4 => InstallState::Cancelled,
            _ => InstallState::Idle,
        };
        env.call_static_method(class, jni_str!("reset"), jni_sig!("()V"), &[])?;
        Ok(out)
    })
}
