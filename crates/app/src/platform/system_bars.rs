//! Android's status and navigation bars take the theme's background, with
//! icons that read on it, through `app.steps.SystemBars` (android/). With
//! True black the whole OLED screen stays dark. Nothing on the Mac.
// Adapted from kabar-app main: crates/app/src/platform/mod.rs (system_bars).

/// `day` and `night` are `#rrggbb`: the same for a fixed theme.
pub fn apply(day: &str, night: &str) {
    #[cfg(target_os = "android")]
    {
        let argb = |hex: &str| {
            let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
            (0xff00_0000 | rgb) as i32
        };
        if let Err(e) = android::apply(argb(day), argb(night)) {
            crate::diag::report!("System bars: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    let _ = (day, night);
}

/// Whether the device is in dark mode, where the platform can tell
/// (Android); `None` elsewhere, where `prefers-color-scheme` is right.
pub fn device_dark() -> Option<bool> {
    #[cfg(target_os = "android")]
    {
        android::is_night().map_err(|e| crate::diag::report!("Night mode: {e}")).ok()
    }
    #[cfg(not(target_os = "android"))]
    None
}

#[cfg(target_os = "android")]
mod android {
    use jni::objects::JClassLoader;
    use jni::refs::LoaderContext;
    use jni::{JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    /// `SystemBars.apply(activity, day, night)`, colours as ARGB ints.
    pub fn apply(day: i32, night: i32) -> Result<(), String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.SystemBars"), true)?;
            env.call_static_method(
                &class,
                jni_str!("apply"),
                jni_sig!("(Landroid/app/Activity;II)V"),
                &[JValue::Object(activity), JValue::Int(day), JValue::Int(night)],
            )
            .map(drop)
        })
    }

    /// `SystemBars.isNight(activity)`.
    pub fn is_night() -> Result<bool, String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.SystemBars"), true)?;
            env.call_static_method(
                &class,
                jni_str!("isNight"),
                jni_sig!("(Landroid/app/Activity;)Z"),
                &[JValue::Object(activity)],
            )?
            .z()
        })
    }
}
