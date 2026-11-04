//! Local notifications for the daily study reminder: through
//! `app.steps.ReminderBridge` (android/) on Android, and
//! UNUserNotificationCenter from the bundled Steps.app on the Mac. Elsewhere
//! (and from `cargo run`, which has no bundle) every call is a no-op and
//! [`local`] is false, so Settings hides the rows.
// Adapted from kabar-app main: crates/app/src/platform/android.rs and
// crates/app/src/platform/macos.rs (reminders).

/// This device can arm reminders itself.
pub fn local() -> bool {
    #[cfg(target_os = "macos")]
    return mac::bundled();
    #[cfg(not(target_os = "macos"))]
    cfg!(target_os = "android")
}

/// Replaces the armed reminders with `json` (`[]` clears them).
pub fn schedule(json: &str) {
    #[cfg(target_os = "android")]
    report(android::schedule(json));
    #[cfg(target_os = "macos")]
    if let Err(e) = mac::schedule(json) {
        crate::diag::report!("Mac reminders: {e}");
    }
    #[cfg(not(any(target_os = "android", target_os = "macos")))]
    let _ = json;
}

/// The fallback the platform arms once the planned reminders run out
/// (`plan::Repeat` as JSON, `{}` for none).
pub fn repeat(json: &str) {
    #[cfg(target_os = "android")]
    report(android::string_call("repeat", json));
    #[cfg(target_os = "macos")]
    mac::repeat(json);
    #[cfg(not(any(target_os = "android", target_os = "macos")))]
    let _ = json;
}

/// Whether notifications are allowed for Steps (None where unknown). On the
/// Mac the answer comes back later, so the first call says None.
pub fn enabled() -> Option<bool> {
    #[cfg(target_os = "macos")]
    return mac::allowed();
    #[cfg(target_os = "android")]
    return android::flag("enabled").map_err(|e| crate::diag::report!("ReminderBridge: {e}")).ok();
    #[cfg(not(any(target_os = "android", target_os = "macos")))]
    None
}

/// Whether One UI lets Steps's alarms run while it sleeps (None where unknown).
pub fn battery_unrestricted() -> Option<bool> {
    #[cfg(target_os = "android")]
    return android::flag("batteryUnrestricted")
        .map_err(|e| crate::diag::report!("ReminderBridge: {e}"))
        .ok();
    #[cfg(not(target_os = "android"))]
    None
}

/// The system prompt for notifications, if not answered yet (Android 13+,
/// and the Mac).
pub fn request_permission() {
    #[cfg(target_os = "android")]
    report(android::activity_call("requestPermission"));
    #[cfg(target_os = "macos")]
    mac::request();
}

/// Steps's notification page in the system settings.
pub fn open_notification_settings() {
    #[cfg(target_os = "android")]
    report(android::activity_call("openNotificationSettings"));
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
        .spawn();
}

/// Steps's app page in Android's settings (battery: Unrestricted).
pub fn open_app_settings() {
    #[cfg(target_os = "android")]
    report(android::activity_call("openAppSettings"));
}

#[cfg(target_os = "android")]
fn report(result: Result<(), String>) {
    if let Err(e) = result {
        crate::diag::report!("ReminderBridge: {e}");
    }
}

#[cfg(target_os = "android")]
mod android {
    use jni::objects::{JClass, JClassLoader, JObject};
    use jni::refs::LoaderContext;
    use jni::strings::JNIStr;
    use jni::{Env, JValue, jni_sig, jni_str};

    use crate::platform::tts_android::with_activity;

    /// `app.steps.ReminderBridge`, loaded through the Activity's class loader.
    fn bridge<T>(f: impl FnOnce(&mut Env, &JClass, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
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
                LoaderContext::Loader(&loader).load_class(env, jni_str!("app.steps.ReminderBridge"), true)?;
            f(env, &class, activity)
        })
    }

    pub fn schedule(json: &str) -> Result<(), String> {
        string_call("schedule", json)
    }

    /// A `(Context, String) -> Unit` method.
    pub fn string_call(name: &'static str, json: &str) -> Result<(), String> {
        bridge(|env, class, activity| {
            let json = env.new_string(json)?;
            env.call_static_method(
                class,
                method(name),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
                &[JValue::Object(activity), JValue::Object(&json)],
            )
            .map(drop)
        })
    }

    /// A `(Context) -> Boolean` method.
    pub fn flag(name: &'static str) -> Result<bool, String> {
        bridge(|env, class, activity| {
            let name = method(name);
            env.call_static_method(
                class,
                name,
                jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(activity)],
            )?
            .z()
        })
    }

    /// A `(Activity) -> Unit` method.
    pub fn activity_call(name: &'static str) -> Result<(), String> {
        bridge(|env, class, activity| {
            env.call_static_method(
                class,
                method(name),
                jni_sig!("(Landroid/app/Activity;)V"),
                &[JValue::Object(activity)],
            )
            .map(drop)
        })
    }

    fn method(name: &'static str) -> &'static JNIStr {
        match name {
            "schedule" => jni_str!("schedule"),
            "repeat" => jni_str!("repeat"),
            "enabled" => jni_str!("enabled"),
            "batteryUnrestricted" => jni_str!("batteryUnrestricted"),
            "requestPermission" => jni_str!("requestPermission"),
            "openNotificationSettings" => jni_str!("openNotificationSettings"),
            _ => jni_str!("openAppSettings"),
        }
    }
}

/// The bundled Mac app's notifications. UNUserNotificationCenter needs a real
/// bundle (`app.steps`): from `cargo run` there is none and the framework
/// would abort, so everything checks for one first. Calendar triggers fire at
/// the local date and time in the Mac's own zone, so daylight saving never
/// moves a reminder.
///
/// macOS can't wake Steps when the last planned reminder fires (Android's
/// `extend`), so the fallback is armed ahead instead: dated [`plan::Repeat`]
/// reminders after the planned ones, up to macOS's 64 pending. That is two
/// months at one a day and three weeks at three, and every launch of Steps
/// re-plans from today. Weekly repeating triggers would ring twice on the
/// planned days, and on days whose target is already met.
///
/// [`plan::Repeat`]: crate::reminders::plan::Repeat
#[cfg(target_os = "macos")]
mod mac {
    use std::ptr::NonNull;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU8, Ordering};

    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSBundle, NSDateComponents, NSError, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNAuthorizationStatus, UNCalendarNotificationTrigger,
        UNMutableNotificationContent, UNNotificationRequest, UNNotificationSettings, UNNotificationSound,
        UNUserNotificationCenter,
    };
    use serde::Deserialize;

    use crate::reminders::plan::{self, Repeat};

    /// macOS keeps the soonest 64 pending notifications per app.
    const MAX: usize = 64;

    /// The latest fallback from [`repeat`], read by the next [`schedule`].
    static REPEAT: Mutex<Option<Repeat>> = Mutex::new(None);

    /// The last permission answer macOS gave: 0 unknown, 1 denied, 2 allowed.
    static ALLOWED: AtomicU8 = AtomicU8::new(0);

    #[derive(Deserialize)]
    struct Item {
        date: String,
        time: String,
        title: String,
        text: String,
    }

    pub fn bundled() -> bool {
        NSBundle::mainBundle().bundleIdentifier().is_some()
    }

    pub fn request() {
        if !bundled() {
            return;
        }
        let done = RcBlock::new(|granted: Bool, _error: *mut NSError| {
            ALLOWED.store(if granted.as_bool() { 2 } else { 1 }, Ordering::Relaxed);
            if !granted.as_bool() {
                crate::diag::report!("notifications not allowed on this Mac");
            }
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                &done,
            );
    }

    /// The last answer, and asks macOS again for the next call (Settings
    /// polls this while it is open).
    pub fn allowed() -> Option<bool> {
        if !bundled() {
            return None;
        }
        let done = RcBlock::new(|settings: NonNull<UNNotificationSettings>| {
            // SAFETY: macOS passes a valid settings object for the length of
            // the call.
            #[allow(unsafe_code)]
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            let answer = match status {
                UNAuthorizationStatus::Denied => 1,
                UNAuthorizationStatus::NotDetermined => 0,
                _ => 2,
            };
            ALLOWED.store(answer, Ordering::Relaxed);
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&done);
        match ALLOWED.load(Ordering::Relaxed) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }

    /// Keeps the fallback (`{}` for none) for the next [`schedule`].
    pub fn repeat(json: &str) {
        let spec = serde_json::from_str::<Repeat>(json).ok();
        *REPEAT.lock().unwrap_or_else(|e| e.into_inner()) = spec;
    }

    /// Replaces Steps's pending notifications with `json`'s, then the
    /// fallback after them.
    pub fn schedule(json: &str) -> Result<(), String> {
        if !bundled() {
            return Ok(());
        }
        let mut items: Vec<Item> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let spec = REPEAT.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let (Some(r), Some(last)) = (spec, items.last().map(|i| i.date.clone())) {
            let more = plan::after(&last, &r, MAX.saturating_sub(items.len()));
            items.extend(more.into_iter().map(|(date, time)| Item {
                date,
                time,
                title: r.title.clone(),
                text: r.text.clone(),
            }));
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.removeAllPendingNotificationRequests();
        for (i, item) in items.iter().enumerate() {
            let (Some((y, mo, d)), Some((h, mi))) = (ymd(&item.date), hm(&item.time)) else { continue };
            let when = NSDateComponents::new();
            when.setYear(y);
            when.setMonth(mo);
            when.setDay(d);
            when.setHour(h);
            when.setMinute(mi);
            let trigger =
                UNCalendarNotificationTrigger::triggerWithDateMatchingComponents_repeats(&when, false);
            let content = UNMutableNotificationContent::new();
            content.setTitle(&NSString::from_str(&item.title));
            content.setBody(&NSString::from_str(&item.text));
            content.setSound(Some(&UNNotificationSound::defaultSound()));
            let id = NSString::from_str(&format!("steps.reminder.{i}"));
            let request =
                UNNotificationRequest::requestWithIdentifier_content_trigger(&id, &content, Some(&trigger));
            center.addNotificationRequest_withCompletionHandler(&request, None);
        }
        Ok(())
    }

    fn ymd(s: &str) -> Option<(isize, isize, isize)> {
        let mut parts = s.split('-').map(|p| p.parse().ok());
        Some((parts.next()??, parts.next()??, parts.next()??))
    }

    fn hm(s: &str) -> Option<(isize, isize)> {
        let (h, m) = s.split_once(':')?;
        Some((h.parse().ok()?, m.parse().ok()?))
    }
}
