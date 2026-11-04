//! Share-to-Steps on Android: `app.steps.ShareActivity` keeps the shared text
//! in `PendingShare`; [`take`] hands it over once. Nothing on the Mac (there
//! he pastes into the "add a card" form).
// Adapted from batna-app main: crates/app/src/platform/android.rs (take_shared_input).

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Shared {
    pub text: String,
    #[serde(default)]
    pub subject: Option<String>,
    /// The sharing app's package, when Android says (`com.android.chrome`).
    #[serde(default)]
    pub app: Option<String>,
}

impl Shared {
    /// What the card's "from" line starts as: the subject, else the app.
    pub fn source(&self) -> Option<String> {
        let clean =
            |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        clean(&self.subject).or_else(|| clean(&self.app))
    }
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn parse(json: &str) -> Option<Shared> {
    serde_json::from_str::<Shared>(json).ok().filter(|s| !s.text.trim().is_empty())
}

/// The waiting shared text, if any; it is forgotten once taken.
pub fn take() -> Option<Shared> {
    #[cfg(target_os = "android")]
    {
        match android::take() {
            Ok(json) => json.as_deref().and_then(parse),
            Err(e) => {
                crate::diag::report!("Share: {e}");
                None
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    None
}

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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.PendingShare"), true)?;
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
mod tests {
    use super::*;

    #[test]
    fn parses_what_share_kt_writes() {
        let s = parse(r#"{"text":"PFO: 25% of people","subject":null,"app":"com.android.chrome"}"#).unwrap();
        assert_eq!(s.source().as_deref(), Some("com.android.chrome"));
        let s = parse(r#"{"text":"x","subject":" Pathoma ","app":"a"}"#).unwrap();
        assert_eq!(s.source().as_deref(), Some("Pathoma"));
        assert!(parse(r#"{"text":"  "}"#).is_none());
        assert!(parse("not json").is_none());
    }
}
