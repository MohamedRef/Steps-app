//! Diagnostics: the one place the app reports something that went wrong
//! but doesn't stop it (a file that couldn't be read, a speech engine that
//! didn't answer).
//!
//! `report!("…", args)` writes the line where it can be read later: logcat
//! on Android (an app's stderr goes nowhere there, `adb logcat -s steps`),
//! stderr on the Mac and in development, and a ring of the last [`KEEP`]
//! lines that Settings › Your data shows and copies. `eprintln!` is
//! disallowed in this crate (clippy.toml), so new code reports through here.
// adapted from kabar-app main: crates/app/src/diag.rs

use std::collections::VecDeque;
use std::fmt;
use std::sync::Mutex;
use web_time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;

use crate::i18n::settings as tx;

/// Lines kept for Settings; older ones drop off.
pub const KEEP: usize = 200;

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// `report!("reading the review log failed: {e}")`: see the module docs.
macro_rules! report {
    ($($arg:tt)*) => {
        $crate::diag::emit(format_args!($($arg)*))
    };
}
pub(crate) use report;

pub fn emit(args: fmt::Arguments<'_>) {
    let line = args.to_string();
    write_out(&line);
    let stamped = format!("{} {line}", utc_time(now_ms()));
    let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    if lines.len() == KEEP {
        lines.pop_front();
    }
    lines.push_back(stamped);
}

/// The kept lines, oldest first.
pub fn lines() -> Vec<String> {
    LINES.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
}

/// Reports a panic before the default hook runs, so a crash on the phone
/// leaves its message in logcat.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        emit(format_args!("panic: {info}"));
        default(info);
    }));
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// `HH:MM:SS` in UTC: enough to line reports up with what happened.
fn utc_time(ms: u64) -> String {
    let s = ms / 1000 % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[cfg(target_os = "android")]
fn write_out(line: &str) {
    android::write(line);
}

/// The web: the browser's console (stderr goes nowhere in wasm).
#[cfg(target_arch = "wasm32")]
fn write_out(line: &str) {
    web_sys::console::warn_1(&format!("steps: {line}").into());
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
#[allow(clippy::disallowed_macros)]
fn write_out(line: &str) {
    eprintln!("steps: {line}");
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod android {
    use std::ffi::{CString, c_char, c_int};

    const ANDROID_LOG_WARN: c_int = 5;

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    /// `adb logcat -s steps` shows these.
    pub fn write(line: &str) {
        let text = CString::new(line.replace('\0', " ")).unwrap_or_default();
        // SAFETY: both pointers are NUL-terminated strings that outlive the call.
        unsafe {
            __android_log_write(ANDROID_LOG_WARN, c"steps".as_ptr(), text.as_ptr());
        }
    }
}

/// A JavaScript string literal for `text`: Rust's debug quoting, with
/// control characters (which it writes as `\u{..}`, not valid JS) blanked.
fn js_string(text: &str) -> String {
    let clean: String =
        text.chars().map(|c| if c.is_control() && c != '\n' && c != '\t' { ' ' } else { c }).collect();
    format!("{clean:?}")
}

/// Settings › Your data: what went wrong lately, to copy into a message.
#[component]
pub fn DiagnosticsCard() -> Element {
    let mut shown = use_signal(lines);
    let mut copied = use_signal(|| false);
    let refresh = move |_| {
        shown.set(lines());
        copied.set(false);
    };
    let copy = move |_| {
        let text = format!("Steps {}\n{}", env!("CARGO_PKG_VERSION"), shown.read().join("\n"));
        document::eval(&format!("navigator.clipboard.writeText({})", js_string(&text)));
        copied.set(true);
    };
    let count = shown.read().len();

    rsx! {
        crate::ui::Card { class: "",
            h2 { {tx::DIAG} }
            if count == 0 {
                p { {tx::DIAG_EMPTY} }
            } else {
                details {
                    summary { {tx::diag_lines(count)} }
                    pre { class: "diag-lines", dir: "ltr",
                        for (i, line) in shown.read().iter().enumerate() {
                            bdi { key: "{i}", "{line}\n" }
                        }
                    }
                }
            }
            div { class: "row",
                button { class: "btn btn-secondary", onclick: refresh, {tx::DIAG_REFRESH} }
                if count > 0 {
                    button { class: "btn btn-secondary", onclick: copy,
                        if copied() { {tx::DIAG_COPIED} } else { {tx::DIAG_COPY} }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two tests that read the ring back don't run side by side.
    static RING: Mutex<()> = Mutex::new(());

    #[test]
    fn a_panic_leaves_its_message_in_the_report() {
        let _ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        install_panic_hook();
        let joined = std::thread::spawn(|| panic!("the valve card has no answer")).join();
        assert!(joined.is_err());
        assert!(
            lines().iter().any(|l| l.contains("panic: ") && l.contains("the valve card has no answer")),
            "{:?}",
            lines()
        );
    }

    #[test]
    fn keeps_the_last_lines() {
        let _ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        for i in 0..KEEP + 5 {
            report!("line {i}");
        }
        let kept = lines();
        assert_eq!(kept.len(), KEEP);
        assert!(kept.last().unwrap().ends_with(&format!("line {}", KEEP + 4)));
    }

    #[test]
    fn utc_time_of_day() {
        assert_eq!(utc_time(1_793_011_856_000), "10:50:56");
    }

    #[test]
    fn js_strings_are_quoted_and_clean() {
        assert_eq!(js_string("a \"b\"\n\u{1b}c"), r#""a \"b\"\n c""#);
    }
}
