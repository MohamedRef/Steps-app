//! Daily study reminders: planned here from the review forecast, armed as
//! local notifications by the platform, so they work offline. Only one
//! device rings (the phone by default); a met target keeps today quiet.

pub mod plan;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform::reminders as native;
use crate::store::{now_ms, shared, use_review, utc_offset_minutes};

/// At most this many reminder times a day.
pub const MAX_TIMES: usize = 3;

/// Settings › Reminders, synced as the `remind` setting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemindPrefs {
    pub on: bool,
    /// Minutes after local midnight, soonest first.
    pub times: Vec<u16>,
}

impl Default for RemindPrefs {
    fn default() -> Self {
        Self { on: true, times: vec![19 * 60] }
    }
}

impl RemindPrefs {
    const KEY: &str = "remind";

    fn load() -> Self {
        shared().setting(Self::KEY).unwrap_or_default()
    }

    pub fn save(mut self) {
        self.times.sort_unstable();
        self.times.dedup();
        self.times.truncate(MAX_TIMES);
        crate::store::save_setting(Self::KEY, &self);
        *PREFS.write() = self;
    }
}

pub static PREFS: GlobalSignal<RemindPrefs> = Signal::global(RemindPrefs::load);

/// Re-reads the reminder times after a sync or restore (see `crate::synced`).
pub fn reload() {
    let fresh = RemindPrefs::load();
    if *PREFS.peek() != fresh {
        *PREFS.write() = fresh;
    }
}

/// This device rings. Device-only, never synced. Until he chooses, devices
/// with a vibration motor ring (his phone), and tablets and the Mac stay
/// quiet; it's a guess about the hardware, not the device's name.
pub static HERE: GlobalSignal<bool> =
    Signal::global(|| shared().local_setting(HERE_KEY).unwrap_or_else(crate::platform::haptics::available));

const HERE_KEY: &str = "remind_here";

pub fn set_here(here: bool) {
    crate::persistence::save("local.remind_here", move || {
        shared().put_local_setting(HERE_KEY, &here).map_err(|e| e.to_string())
    });
    *HERE.write() = here;
}

/// Called once in `App`: re-plans this device's reminders whenever an
/// answer is logged or the reminder or planner settings change.
pub fn use_local_reminders() {
    let review = use_review();
    use_effect(move || {
        let _ = (review.revision)();
        let prefs = PREFS.read().clone();
        let here = *HERE.read();
        let weekdays = crate::plan::SETTINGS.read().weekdays;
        if !native::local() {
            return;
        }
        let now = now_ms();
        let (due, answered) = {
            let b = review.backend.borrow();
            (b.due_forecast(14, now), b.answers_today(now))
        };
        let times = if prefs.on { prefs.times } else { Vec::new() };
        let inputs = plan::Inputs {
            here,
            times: &times,
            weekdays,
            due: &due,
            // He studied today and nothing is left due: today's done.
            target_met: answered > 0 && due.first().is_none_or(|&d| d == 0),
            now_ms: now.max(0) as u64,
            utc_offset_min: utc_offset_minutes(),
        };
        // The fallback first, so a reminder firing in between finds it.
        let repeat = plan::repeat(&inputs).and_then(|r| serde_json::to_string(&r).ok());
        native::repeat(repeat.as_deref().unwrap_or("{}"));
        let items = plan::schedule(&inputs);
        native::schedule(&serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()));
    });
}

pub(crate) fn hhmm(t: u16) -> String {
    format!("{:02}:{:02}", t / 60, t % 60)
}

pub(crate) fn parse_hhmm(s: &str) -> Option<u16> {
    let (h, m) = s.split_once(':')?;
    let (h, m): (u16, u16) = (h.parse().ok()?, m.get(..2)?.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Settings › Reminders.
#[component]
pub fn RemindSection() -> Element {
    use crate::i18n::remind as tx;
    use crate::settings::DeviceOnly;
    let prefs = PREFS();
    let here = HERE();
    // Android: re-read when the sheet re-renders (e.g. after he returns from
    // its settings). The Mac answers later, so it is polled while open.
    let mut mac_allowed = use_signal(native::enabled);
    use_future(move || async move {
        if cfg!(target_os = "macos") {
            loop {
                crate::platform::time::sleep(std::time::Duration::from_secs(1)).await;
                let now = native::enabled();
                if *mac_allowed.peek() != now {
                    mac_allowed.set(now);
                }
            }
        }
    });
    let allowed = if cfg!(target_os = "macos") { mac_allowed() } else { native::enabled() };
    let unrestricted = native::battery_unrestricted();
    let on_off = |on: bool| if on { tx::ON } else { tx::OFF };

    if !native::local() {
        return rsx! {
            crate::ui::Card {
                h2 { {tx::SECTION} }
                p { class: "muted", {tx::PHONE_ONLY} }
            }
        };
    }
    rsx! {
        crate::ui::Card {
            h2 { {tx::DAILY} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::DAILY,
                for on in [true, false] {
                    button {
                        key: "{on}",
                        role: "radio",
                        "aria-checked": prefs.on == on,
                        onclick: {
                            let prefs = prefs.clone();
                            move |_| {
                                if on {
                                    native::request_permission();
                                }
                                RemindPrefs { on, ..prefs.clone() }.save();
                            }
                        },
                        {on_off(on)}
                    }
                }
            }
            if prefs.on {
                h3 { {tx::TIMES} }
                div { class: "remind-times",
                    for (i , t) in prefs.times.iter().copied().enumerate() {
                        div { key: "{i}-{t}", class: "row",
                            input {
                                r#type: "time",
                                value: hhmm(t),
                                aria_label: tx::TIME,
                                onchange: {
                                    let prefs = prefs.clone();
                                    move |e: FormEvent| {
                                        if let Some(v) = parse_hhmm(&e.value()) {
                                            let mut p = prefs.clone();
                                            p.times[i] = v;
                                            p.save();
                                        }
                                    }
                                },
                            }
                            if prefs.times.len() > 1 {
                                button {
                                    class: "btn btn-secondary",
                                    onclick: {
                                        let prefs = prefs.clone();
                                        move |_| {
                                            let mut p = prefs.clone();
                                            p.times.remove(i);
                                            p.save();
                                        }
                                    },
                                    {tx::REMOVE}
                                }
                            }
                        }
                    }
                    if prefs.times.len() < MAX_TIMES {
                        button {
                            class: "btn btn-secondary",
                            onclick: {
                                let prefs = prefs.clone();
                                move |_| {
                                    let mut p = prefs.clone();
                                    let last = p.times.last().copied().unwrap_or(17 * 60);
                                    p.times.push((last + 120).min(23 * 60));
                                    p.save();
                                }
                            },
                            {tx::ADD}
                        }
                    }
                }
            }
        }
        if prefs.on {
            crate::ui::Card {
                h2 { {tx::HERE} DeviceOnly {} }
                div { class: "segmented", role: "radiogroup", aria_label: tx::HERE,
                    for on in [true, false] {
                        button {
                            key: "{on}",
                            role: "radio",
                            "aria-checked": here == on,
                            onclick: move |_| {
                                if on {
                                    native::request_permission();
                                }
                                set_here(on);
                            },
                            {on_off(on)}
                        }
                    }
                }
                if here && allowed == Some(false) {
                    p { {tx::BLOCKED} }
                    div { class: "row",
                        // macOS asks only once; after a no, only System Settings can change it.
                        if !cfg!(target_os = "macos") {
                            button { class: "btn btn-primary", onclick: move |_| native::request_permission(), {tx::ALLOW} }
                        }
                        button { class: "btn btn-secondary", onclick: move |_| native::open_notification_settings(), {tx::OPEN_NOTIFICATIONS} }
                    }
                }
                if here && cfg!(target_os = "macos") && allowed != Some(false) {
                    p { class: "muted small", {tx::MAC_HINT} }
                    button { class: "btn btn-secondary", onclick: move |_| native::open_notification_settings(), {tx::OPEN_NOTIFICATIONS} }
                }
                if here && unrestricted == Some(false) {
                    p { class: "muted small", {tx::BATTERY} }
                    button { class: "btn btn-secondary", onclick: move |_| native::open_app_settings(), {tx::OPEN_APP} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_round_trip_through_the_time_input() {
        assert_eq!(parse_hhmm("19:00"), Some(19 * 60));
        assert_eq!(parse_hhmm("07:45:00"), Some(7 * 60 + 45));
        assert_eq!(hhmm(7 * 60 + 5), "07:05");
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("nope"), None);
    }

    #[test]
    fn old_or_partial_settings_fall_back_to_defaults() {
        let p: RemindPrefs = serde_json::from_str(r#"{"on":false}"#).unwrap();
        assert_eq!(p, RemindPrefs { on: false, ..RemindPrefs::default() });
    }
}
