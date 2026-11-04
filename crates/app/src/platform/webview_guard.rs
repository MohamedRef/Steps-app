//! Android: keeps a lost WebView renderer from crashing Steps, and reports
//! how recent runs ended (`app.steps.WebViewGuard`, android/). Android
//! reclaims a backgrounded app's renderer under memory pressure; unhandled,
//! that kills Steps as a crash ("Steps is crashing frequently"). It also
//! routes the Back key through the page (`app.steps.BackKey`). Nothing
//! elsewhere.

/// Guards the WebView once it exists and reports earlier runs' endings to
/// the diagnostics (Settings › Your data). Call once at start.
pub fn install() {
    #[cfg(target_os = "android")]
    match android::install() {
        Ok(lines) => {
            for line in lines.lines().filter(|l| !l.is_empty()) {
                crate::diag::report!("{line}");
            }
        }
        Err(e) => crate::diag::report!("WebView guard: {e}"),
    }
}

#[cfg(target_os = "android")]
mod android {
    use jni::objects::{JClassLoader, JString};
    use jni::refs::LoaderContext;
    use jni::{JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    /// `WebViewGuard.install(activity)`: the exit report, one line each.
    pub fn install() -> Result<String, String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.WebViewGuard"), true)?;
            // Back goes through the page first, never WebView history (BackKey.kt).
            let back = LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.BackKey"), true)?;
            env.call_static_method(
                &back,
                jni_str!("install"),
                jni_sig!("(Landroid/app/Activity;)V"),
                &[JValue::Object(activity)],
            )?;
            let s = env
                .call_static_method(
                    &class,
                    jni_str!("install"),
                    jni_sig!("(Landroid/app/Activity;)Ljava/lang/String;"),
                    &[JValue::Object(activity)],
                )?
                .l()?;
            if s.is_null() {
                return Ok(String::new());
            }
            let s = env.cast_local::<JString>(s)?;
            Ok(s.mutf8_chars(env)?.to_string())
        })
    }
}
