//! The "new version" notification ([`crate::update::Notice`]) and Android's
//! periodic background check that sends it while Steps is closed
//! (`app.steps.UpdateCheckJob`, android/). No-ops elsewhere for now: the
//! Mac's notifications come with UNUserNotificationCenter (Steps-app#50).
//!
//! Adapted from kabar-app: crates/app/src/platform/{mod,android}.rs (the
//! `update_notice` calls and `Java_app_kabar_UpdateCheckJob_check`).

use crate::update::Notice;

/// Shows `n` now; true once it's shown. Android skips it while Steps is in
/// front (the banner says it there) or notifications are off.
pub fn post(n: &Notice) -> bool {
    #[cfg(target_os = "android")]
    {
        android::post(n).map_err(|e| crate::diag::report!("update notification: {e}")).unwrap_or(false)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = n;
        false
    }
}

/// Removes a shown notification (the offer was closed).
pub fn clear() {
    #[cfg(target_os = "android")]
    if let Err(e) = android::context_call(jni::jni_str!("clear")) {
        crate::diag::report!("update notification: {e}");
    }
}

/// Android: starts (or, with `false`, stops) the background check.
pub fn schedule(on: bool) {
    #[cfg(target_os = "android")]
    if let Err(e) = android::schedule(on) {
        crate::diag::report!("background update check: {e}");
    }
    #[cfg(not(target_os = "android"))]
    let _ = on;
}

/// What a tap on the notification asked for, once: `open`, `download` or
/// `install`.
pub fn take_tap() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        android::take_tap().map_err(|e| crate::diag::report!("update notification: {e}")).ok().flatten()
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

#[cfg(target_os = "android")]
mod android {
    #![allow(unsafe_code)]

    use std::path::Path;

    use jni::objects::{JClass, JClassLoader, JObject, JString};
    use jni::refs::LoaderContext;
    use jni::strings::JNIStr;
    use jni::{Env, JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    /// Runs `f` with `app.steps.UpdateNotice`, loaded through the Activity's
    /// class loader.
    fn notice<T>(f: impl FnOnce(&mut Env, &JClass, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.UpdateNotice"), true)?;
            f(env, &class, activity)
        })
    }

    pub fn post(n: &crate::update::Notice) -> Result<bool, String> {
        let json = serde_json::to_string(n).map_err(|e| e.to_string())?;
        notice(|env, class, activity| {
            let json = env.new_string(&json)?;
            env.call_static_method(
                class,
                jni_str!("show"),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;Z)Z"),
                &[JValue::Object(activity), JValue::Object(&json), JValue::Bool(true)],
            )?
            .z()
        })
    }

    /// A static `(Context) -> void` on UpdateNotice.
    pub fn context_call(name: &'static JNIStr) -> Result<(), String> {
        notice(|env, class, activity| {
            env.call_static_method(
                class,
                name,
                jni_sig!("(Landroid/content/Context;)V"),
                &[JValue::Object(activity)],
            )
            .map(drop)
        })
    }

    pub fn schedule(on: bool) -> Result<(), String> {
        notice(|env, class, activity| {
            env.call_static_method(
                class,
                jni_str!("scheduleChecks"),
                jni_sig!("(Landroid/content/Context;Z)V"),
                &[JValue::Object(activity), JValue::Bool(on)],
            )
            .map(drop)
        })
    }

    pub fn take_tap() -> Result<Option<String>, String> {
        notice(|env, class, activity| {
            let s = env
                .call_static_method(
                    class,
                    jni_str!("takeTap"),
                    jni_sig!("(Landroid/content/Context;)Ljava/lang/String;"),
                    &[JValue::Object(activity)],
                )?
                .l()?;
            if s.is_null() {
                return Ok(None);
            }
            let s = env.cast_local::<JString>(s)?;
            Ok(Some(s.mutf8_chars(env)?.to_string()))
        })
    }

    /// `UpdateCheckJob.check(dataDir, metered, canNotify)`: the background
    /// check, from a JobScheduler thread in a process that may have no
    /// Activity (so nothing here may use `with_activity`). Returns the
    /// notification's JSON, or null.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_steps_UpdateCheckJob_check<'caller>(
        mut unowned_env: jni::EnvUnowned<'caller>,
        _this: JObject<'caller>,
        data_dir: JString<'caller>,
        metered: jni::sys::jboolean,
        can_notify: jni::sys::jboolean,
    ) -> JString<'caller> {
        unowned_env
            .with_env(|env| -> jni::errors::Result<JString<'caller>> {
                let dir = data_dir.mutf8_chars(env)?.to_string();
                let notice = crate::update::background_check(Path::new(&dir), metered, can_notify);
                match notice.and_then(|n| serde_json::to_string(&n).ok()) {
                    Some(json) => JString::from_str(env, json),
                    None => Ok(JString::default()),
                }
            })
            .resolve::<jni::errors::LogErrorAndDefault>()
    }

    /// `UpdateCheckJob.shown(dataDir, key)`: Android showed the notification
    /// `check` returned, so it isn't sent again (Steps-app#165: one that
    /// failed to show is tried again at the next run).
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_steps_UpdateCheckJob_shown<'caller>(
        mut unowned_env: jni::EnvUnowned<'caller>,
        _this: JObject<'caller>,
        data_dir: JString<'caller>,
        key: JString<'caller>,
    ) {
        unowned_env
            .with_env(|env| -> jni::errors::Result<()> {
                let dir = data_dir.mutf8_chars(env)?.to_string();
                let key = key.mutf8_chars(env)?.to_string();
                crate::update::background_shown(Path::new(&dir), &key);
                Ok(())
            })
            .resolve::<jni::errors::LogErrorAndDefault>()
    }
}
