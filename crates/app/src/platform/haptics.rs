//! A light tap of vibration with answer feedback, through `app.steps.Haptics`
//! (android/). It follows the phone's own touch-feedback setting. Nothing on
//! the Mac.
// Adapted from batna-app main: crates/app/src/platform/android.rs (vibrate).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Haptic {
    Right,
    Wrong,
    SessionEnd,
}

impl Haptic {
    /// The constant in `Haptics.kt`.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn code(self) -> i32 {
        match self {
            Self::Right => 0,
            Self::Wrong => 1,
            Self::SessionEnd => 2,
        }
    }
}

/// Whether this device has a vibration motor (Settings shows the switch only then).
pub fn available() -> bool {
    #[cfg(target_os = "android")]
    {
        static HAS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *HAS.get_or_init(|| android::available().unwrap_or(false))
    }
    #[cfg(not(target_os = "android"))]
    false
}

pub fn vibrate(kind: Haptic) {
    #[cfg(target_os = "android")]
    if let Err(e) = android::play(kind.code()) {
        crate::diag::report!("Haptics: {e}");
    }
    #[cfg(not(target_os = "android"))]
    let _ = kind;
}

#[cfg(target_os = "android")]
mod android {

    use jni::objects::{JClass, JClassLoader, JObject};
    use jni::refs::LoaderContext;
    use jni::{Env, JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    /// `app.steps.Haptics`, loaded through the Activity's class loader.
    fn haptics<T>(
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.Haptics"), true)?;
            f(env, &class, activity)
        })
    }

    pub fn available() -> Result<bool, String> {
        haptics(|env, class, activity| {
            env.call_static_method(
                class,
                jni_str!("available"),
                jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(activity)],
            )?
            .z()
        })
    }

    pub fn play(code: i32) -> Result<(), String> {
        haptics(|env, class, activity| {
            env.call_static_method(
                class,
                jni_str!("play"),
                jni_sig!("(Landroid/content/Context;I)V"),
                &[JValue::Object(activity), JValue::Int(code)],
            )
            .map(drop)
        })
    }
}
