//! Study sounds, synthesized by `scripts/assets/synth_sounds.py`: a soft
//! chime for a right answer and a calm tone when a session ends, three
//! variations each, picked in Settings › Study. WAV, so every web view plays
//! them. Phones also give a light tap of vibration with each cue
//! ([`crate::platform::haptics`]). Adapted from Kabar's
//! `crates/app/src/sounds.rs` (main).

use std::collections::HashMap;
use std::sync::LazyLock;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// What just happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// A right answer.
    Right,
    /// A wrong answer: never a sound, at most a barely-there tap.
    Wrong,
    /// The session summary.
    SessionEnd,
}

/// Settings › Study, stored under the synced `sound` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundPrefs {
    pub on: bool,
    /// Right-answer chime, 1 to 3.
    pub chime: u8,
    /// End-of-session tone, 1 to 3.
    pub calm: u8,
    /// Vibrate with each cue (shown only where the device has a motor).
    pub vibrate: bool,
}

impl Default for SoundPrefs {
    fn default() -> Self {
        Self { on: true, chime: 1, calm: 1, vibrate: true }
    }
}

impl SoundPrefs {
    const KEY: &str = "sound";

    pub fn load() -> Self {
        crate::store::shared().setting(Self::KEY).unwrap_or_default()
    }

    /// Saves and applies at once.
    pub fn save(self) {
        crate::store::save_setting(Self::KEY, &self);
        *PREFS.write() = self;
    }
}

/// The current prefs, read by every [`play`].
pub static PREFS: GlobalSignal<SoundPrefs> = Signal::global(SoundPrefs::load);

/// Re-reads the prefs after a sync or restore (see `crate::synced`).
pub fn reload() {
    let fresh = SoundPrefs::load();
    if *PREFS.peek() != fresh {
        *PREFS.write() = fresh;
    }
}

pub const VARIATIONS: u8 = 3;

const FILES: [(&str, &[u8]); 6] = [
    ("chime-1", include_bytes!("../../../assets/sounds/chime-1.wav")),
    ("chime-2", include_bytes!("../../../assets/sounds/chime-2.wav")),
    ("chime-3", include_bytes!("../../../assets/sounds/chime-3.wav")),
    ("calm-1", include_bytes!("../../../assets/sounds/calm-1.wav")),
    ("calm-2", include_bytes!("../../../assets/sounds/calm-2.wav")),
    ("calm-3", include_bytes!("../../../assets/sounds/calm-3.wav")),
];

const RIGHT_VOLUME: f32 = 0.5;
const END_VOLUME: f32 = 0.6;

static URLS: LazyLock<HashMap<&'static str, String>> = LazyLock::new(|| {
    FILES.iter().map(|&(id, bytes)| (id, format!("data:audio/wav;base64,{}", base64(bytes)))).collect()
});

/// The file stem and volume a cue plays with these prefs, if any.
fn sound_for(cue: Cue, prefs: &SoundPrefs) -> Option<(String, f32)> {
    let pick = |n: u8| n.clamp(1, VARIATIONS);
    match cue {
        _ if !prefs.on => None,
        Cue::Right => Some((format!("chime-{}", pick(prefs.chime)), RIGHT_VOLUME)),
        Cue::SessionEnd => Some((format!("calm-{}", pick(prefs.calm)), END_VOLUME)),
        Cue::Wrong => None,
    }
}

fn eval_play(id: &str, volume: f32) {
    let Some(url) = URLS.get(id) else { return };
    document::eval(&format!(
        "try {{ const a = new Audio({url:?}); a.volume = {volume}; a.play().catch(() => {{}}); }} catch (e) {{}}"
    ));
}

/// The one call grading and the session summary make.
pub fn play(cue: Cue) {
    let prefs = *PREFS.peek();
    if prefs.vibrate {
        use crate::platform::haptics::{Haptic, vibrate};
        vibrate(match cue {
            Cue::Right => Haptic::Right,
            Cue::Wrong => Haptic::Wrong,
            Cue::SessionEnd => Haptic::SessionEnd,
        });
    }
    play_with(cue, &prefs);
}

fn play_with(cue: Cue, prefs: &SoundPrefs) {
    if let Some((id, volume)) = sound_for(cue, prefs) {
        eval_play(&id, volume);
    }
}

/// Settings: hear a variation before choosing it, even with sounds off.
pub fn preview(cue: Cue, variation: u8) {
    let prefs = SoundPrefs { on: true, chime: variation, calm: variation, vibrate: false };
    play_with(cue, &prefs);
}

/// Settings › Study: sounds on or off, which chime and tone (each choice
/// plays as it is picked), and vibration where the device has a motor.
#[component]
pub fn SoundSection() -> Element {
    use crate::i18n::sound as tx;
    let now = PREFS();
    let set = move |p: SoundPrefs| p.save();
    rsx! {
        crate::ui::Card { class: "",
            h2 { {tx::TITLE} }
            h3 { {tx::SOUNDS} }
            div { class: "segmented", role: "radiogroup", aria_label: tx::SOUNDS,
                for (on , label) in [(true, tx::ON), (false, tx::OFF)] {
                    button {
                        key: "{on}",
                        role: "radio",
                        "aria-checked": now.on == on,
                        onclick: move |_| set(SoundPrefs { on, ..now }),
                        {label}
                    }
                }
            }
            if now.on {
                for (cue , title) in [(Cue::Right, tx::RIGHT), (Cue::SessionEnd, tx::END)] {
                    h3 { key: "{title}", {title} }
                    div { class: "segmented", role: "radiogroup", aria_label: title,
                        for v in 1..=VARIATIONS {
                            button {
                                key: "{v}",
                                role: "radio",
                                "aria-checked": if cue == Cue::Right { now.chime == v } else { now.calm == v },
                                onclick: move |_| {
                                    preview(cue, v);
                                    set(if cue == Cue::Right { SoundPrefs { chime: v, ..now } } else { SoundPrefs { calm: v, ..now } });
                                },
                                "{tx::VARIATION} {v}"
                            }
                        }
                    }
                }
            }
            if crate::platform::haptics::available() {
                h3 { {tx::VIBRATE} }
                div { class: "segmented", role: "radiogroup", aria_label: tx::VIBRATE,
                    for (on , label) in [(true, tx::ON), (false, tx::OFF)] {
                        button {
                            key: "{on}",
                            role: "radio",
                            "aria-checked": now.vibrate == on,
                            onclick: move |_| set(SoundPrefs { vibrate: on, ..now }),
                            {label}
                        }
                    }
                }
            }
        }
    }
}

/// Standard base64, enough for embedding the WAVs as data URLs.
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ABC[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        let cases = [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")];
        for (plain, coded) in cases {
            assert_eq!(base64(plain.as_bytes()), coded);
        }
    }

    #[test]
    fn every_variation_has_a_wav() {
        for v in 1..=VARIATIONS {
            for cue in [Cue::Right, Cue::SessionEnd] {
                let prefs = SoundPrefs { chime: v, calm: v, ..SoundPrefs::default() };
                let (id, _) = sound_for(cue, &prefs).unwrap();
                assert!(URLS[id.as_str()].starts_with("data:audio/wav;base64,UklGR"), "{id}");
            }
        }
        assert_eq!(URLS.len(), FILES.len());
    }

    #[test]
    fn sounds_off_or_a_wrong_answer_plays_nothing() {
        let off = SoundPrefs { on: false, ..SoundPrefs::default() };
        assert_eq!(sound_for(Cue::Right, &off), None);
        assert_eq!(sound_for(Cue::Wrong, &SoundPrefs::default()), None);
        let odd = SoundPrefs { chime: 9, ..SoundPrefs::default() };
        assert_eq!(sound_for(Cue::Right, &odd).unwrap().0, "chime-3");
    }

    #[test]
    fn old_or_partial_settings_fall_back_to_defaults() {
        let p: SoundPrefs = serde_json::from_str(r#"{"chime":2}"#).unwrap();
        assert_eq!(p, SoundPrefs { chime: 2, ..SoundPrefs::default() });
    }
}
