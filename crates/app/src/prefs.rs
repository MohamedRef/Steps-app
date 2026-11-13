//! Preferences that belong to one device and never sync: the phone, the
//! tablet and the Mac each keep their own theme and text size. A small
//! `key=value` file in the app's data folder, written whole on every change.
// adapted from batna-app main: crates/app/src/settings/device.rs

#[cfg(not(target_arch = "wasm32"))] // the web keeps prefs in localStorage
mod storage;

use std::collections::BTreeMap;
use std::path::PathBuf;

use steps_core::theme::Theme;

const FILE: &str = "device-prefs.txt";
const THEME: &str = "theme";
const TEXT_SIZE: &str = "text_size";
const NAV_STYLE: &str = "nav_style";
const PEN_HOVER: &str = "pen_hover";
const MOUSE_HOVER: &str = "mouse_hover";
const APPEARANCE: &str = "appearance";
const DESK_SPLIT: &str = "desk_split";
const DESK_FOCUS: &str = "desk_focus";
const READ_SIZE: &str = "read_size";
const READ_SPACING: &str = "read_spacing";
const READ_WIDTH: &str = "read_width";

/// This test thread's data folder (`data_dir` under `cfg(test)`).
#[cfg_attr(not(test), allow(dead_code))]
pub fn test_dir() -> PathBuf {
    let thread = format!("{:?}", std::thread::current().id());
    let n: String = thread.chars().filter(char::is_ascii_digit).collect();
    std::env::temp_dir().join(format!("steps-test-{}-{n}", std::process::id()))
}

/// The app's own data folder: the local database and device files live here.
pub fn data_dir() -> Option<PathBuf> {
    // Previews point the whole app at a scratch folder.
    if let Some(dir) = std::env::var_os("STEPS_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    // Unit tests never touch the real profile: one scratch folder per test
    // thread, so tests running side by side start from their own empty data.
    if cfg!(test) {
        return Some(test_dir());
    }
    native_data_dir(|name| std::env::var_os(name))
}

/// Where this platform keeps an app's data, given how to read an
/// environment variable.
fn native_data_dir(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        // The browser has no folders: the database is in IndexedDB and the
        // device prefs in localStorage (`platform::web`).
        return None;
    }
    if cfg!(target_os = "android") {
        // The app's private files directory (Context.getFilesDir for the
        // device owner). A JNI lookup can replace this once platform/ has one.
        return Some(PathBuf::from("/data/data/app.steps/files"));
    }
    let home = var("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/app.steps"));
    }
    var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")))
        .map(|d| d.join("steps"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSize {
    Smaller,
    Small,
    #[default]
    Normal,
    Large,
    Larger,
}

impl TextSize {
    pub const ALL: [Self; 5] = [Self::Smaller, Self::Small, Self::Normal, Self::Large, Self::Larger];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smaller => "smaller",
            Self::Small => "small",
            Self::Normal => "normal",
            Self::Large => "large",
            Self::Larger => "larger",
        }
    }

    fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }

    /// The root size: type, spacing and control heights are all in rem, so
    /// this scales the whole interface, not only the text.
    pub fn percent(self) -> u32 {
        match self {
            Self::Smaller => 82,
            Self::Small => 90,
            Self::Normal => 100,
            Self::Large => 112,
            Self::Larger => 125,
        }
    }
}

/// The sidebar on wide windows (tablet landscape, the Mac).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavStyle {
    /// Icons with labels.
    #[default]
    Labels,
    /// The narrow icon rail, as on medium widths.
    Slim,
}

impl NavStyle {
    pub const ALL: [Self; 2] = [Self::Labels, Self::Slim];

    /// Stored value and `data-nav-style` on `<html>` (nav.css).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Labels => "labels",
            Self::Slim => "slim",
        }
    }

    fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }
}

// The lesson reading choices: text size, line spacing and line width.
// Idea and values adapted from kabar-app main:
// crates/app/src/settings/device.rs and crates/app/src/reader/reading.rs;
// Steps keeps them in this device file and draws its own controls.

/// The lesson text size, on top of the interface size (Settings › Look):
/// scales the step's text only, so the bar and the buttons stay put.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadSize {
    Small,
    #[default]
    Normal,
    Large,
    Larger,
}

impl ReadSize {
    pub const ALL: [Self; 4] = [Self::Small, Self::Normal, Self::Large, Self::Larger];

    /// Stored value and `data-read-size` on the lesson player.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "s",
            Self::Normal => "m",
            Self::Large => "l",
            Self::Larger => "xl",
        }
    }

    fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }
}

/// The space between a lesson's lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadSpacing {
    Snug,
    #[default]
    Normal,
    Airy,
}

impl ReadSpacing {
    pub const ALL: [Self; 3] = [Self::Snug, Self::Normal, Self::Airy];

    /// Stored value and `data-read-spacing`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Snug => "snug",
            Self::Normal => "normal",
            Self::Airy => "airy",
        }
    }

    fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }
}

/// How wide a lesson's column runs on a tablet or the Mac (a phone is
/// always the screen's width): a narrow column, the book's width, or most
/// of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadWidth {
    Narrow,
    #[default]
    Book,
    Wide,
}

impl ReadWidth {
    pub const ALL: [Self; 3] = [Self::Narrow, Self::Book, Self::Wide];

    /// Stored value and `data-read-width`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Narrow => "narrow",
            Self::Book => "book",
            Self::Wide => "wide",
        }
    }

    fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }
}

/// How lessons read on this device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    pub size: ReadSize,
    pub spacing: ReadSpacing,
    pub width: ReadWidth,
}

/// The surface style (Settings › Look › "نمط الواجهة"), independent of the
/// colour theme. Stepped, the default, is Steps' own look: cards stand on a
/// solid ledge and buttons press down into it. Calm is flat with hairlines;
/// Outline draws cards as clear outlines on the page; Tinted fills them with
/// the theme's soft colour; Frost is Steps' glass: matte frosted panes on
/// ledges over a terrace of the theme's colour, content kept solid. Five minimal styles drop every ledge and
/// decoration: Borderless (space, no lines), Dividers (rules, no boxes),
/// Side line (a brand line at a card's start), Soft fill (grey cards on a
/// plain page) and Mono (ink and paper). `data-appearance` on `.app`
/// (assets/appearance.css, assets/frost.css).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Stepped,
    Calm,
    Outline,
    Tinted,
    Borderless,
    Dividers,
    SideLine,
    SoftFill,
    Mono,
    Frost,
}

impl Appearance {
    pub const ALL: [Self; 10] = [
        Self::Stepped,
        Self::Calm,
        Self::Borderless,
        Self::Dividers,
        Self::SideLine,
        Self::SoftFill,
        Self::Mono,
        Self::Outline,
        Self::Tinted,
        Self::Frost,
    ];

    /// Stored value and `data-appearance`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stepped => "stepped",
            Self::Calm => "calm",
            Self::Outline => "outline",
            Self::Tinted => "tinted",
            Self::Borderless => "borderless",
            Self::Dividers => "dividers",
            Self::SideLine => "side-line",
            Self::SoftFill => "soft-fill",
            Self::Mono => "mono",
            Self::Frost => "frost",
        }
    }

    fn parse(s: &str) -> Self {
        // "glossy" was the gradient style Stepped replaced (2026-11-05).
        if s == "glossy" {
            return Self::Stepped;
        }
        // "liquid-glass" was the glass style Frost replaced (2026-11-13).
        if s == "liquid-glass" {
            return Self::Frost;
        }
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DevicePrefs {
    pub theme: Theme,
    pub text_size: TextSize,
    pub nav_style: NavStyle,
    /// Peeks while the S Pen hovers (Settings › Tablet).
    pub pen_hover: bool,
    /// Peeks when the mouse rests on a word (Settings › Mac).
    pub mouse_hover: bool,
    /// One of [`Appearance::ALL`]. Older files have no line for it and load
    /// as Stepped.
    pub appearance: Appearance,
    /// The question pane's share of a two-pane study card, in percent
    /// (35–70); 0 keeps each item's own default (desk/mod.rs).
    pub desk_split: u8,
    /// Focus mode on the Mac and tablet: the sidebar and the lesson's side
    /// column step aside while a lesson or a study card is open.
    pub desk_focus: bool,
    /// Lesson text size, line spacing and line width. Older files have no
    /// lines for them and load as the defaults.
    pub reading: Reading,
}

impl Default for DevicePrefs {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            text_size: TextSize::default(),
            nav_style: NavStyle::default(),
            pen_hover: true,
            mouse_hover: true,
            appearance: Appearance::default(),
            desk_split: 0,
            desk_focus: false,
            reading: Reading::default(),
        }
    }
}

impl DevicePrefs {
    pub fn load() -> Self {
        #[cfg(target_arch = "wasm32")]
        let text = crate::platform::web::read_pref(FILE).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        let text = data_dir().and_then(|d| storage::read(&d).ok()).unwrap_or_default();
        Self::parse(&text)
    }

    /// Apply immediately, and keep the newest complete device choice for retry.
    pub fn save(&self) {
        let choice = *self;
        crate::persistence::save("device.preferences", move || {
            #[cfg(target_arch = "wasm32")]
            let result = if crate::platform::web::write_pref(FILE, &choice.render()) {
                Ok(())
            } else {
                Err("The browser refused to keep this choice".to_string())
            };
            #[cfg(not(target_arch = "wasm32"))]
            let result = data_dir()
                .ok_or_else(|| "No device data folder is available".to_string())
                .and_then(|dir| storage::write(&dir, &choice.render()).map_err(|e| e.to_string()));
            result
        });
    }

    fn parse(text: &str) -> Self {
        let map: BTreeMap<&str, &str> =
            text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect();
        Self {
            theme: map.get(THEME).map_or_else(Theme::default, |v| Theme::parse(v)),
            text_size: map.get(TEXT_SIZE).map_or_else(TextSize::default, |v| TextSize::parse(v)),
            nav_style: map.get(NAV_STYLE).map_or_else(NavStyle::default, |v| NavStyle::parse(v)),
            pen_hover: map.get(PEN_HOVER) != Some(&"off"),
            mouse_hover: map.get(MOUSE_HOVER) != Some(&"off"),
            appearance: map.get(APPEARANCE).map_or_else(Appearance::default, |v| Appearance::parse(v)),
            desk_split: map
                .get(DESK_SPLIT)
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|v| (35..=70).contains(v))
                .unwrap_or(0),
            desk_focus: map.get(DESK_FOCUS) == Some(&"on"),
            reading: Reading {
                size: map.get(READ_SIZE).map_or_else(ReadSize::default, |v| ReadSize::parse(v)),
                spacing: map.get(READ_SPACING).map_or_else(ReadSpacing::default, |v| ReadSpacing::parse(v)),
                width: map.get(READ_WIDTH).map_or_else(ReadWidth::default, |v| ReadWidth::parse(v)),
            },
        }
    }

    fn render(&self) -> String {
        let on_off = |on: bool| if on { "on" } else { "off" };
        format!(
            "{THEME}={}\n{TEXT_SIZE}={}\n{NAV_STYLE}={}\n{PEN_HOVER}={}\n{MOUSE_HOVER}={}\n{APPEARANCE}={}\n\
             {DESK_SPLIT}={}\n{DESK_FOCUS}={}\n{READ_SIZE}={}\n{READ_SPACING}={}\n{READ_WIDTH}={}\n",
            self.theme.as_str(),
            self.text_size.as_str(),
            self.nav_style.as_str(),
            on_off(self.pen_hover),
            on_off(self.mouse_hover),
            self.appearance.as_str(),
            self.desk_split,
            on_off(self.desk_focus),
            self.reading.size.as_str(),
            self.reading.spacing.as_str(),
            self.reading.width.as_str(),
        )
    }

    /// The page-wide CSS for these choices: colour tokens and root text size.
    pub fn css(&self) -> String {
        self.css_for(None)
    }

    /// [`DevicePrefs::css`] with the device's dark mode when the platform
    /// knows it (Android), so System themes follow it.
    pub fn css_for(&self, device_dark: Option<bool>) -> String {
        format!("{}html{{font-size:{}%;}}", self.theme.css_for(device_dark), self.text_size.percent())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(any(target_os = "macos", target_os = "android", target_arch = "wasm32")))]
    #[test]
    fn linux_keeps_data_in_the_xdg_folder_or_under_home() {
        use std::ffi::OsString;
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| OsString::from(v))
        };
        assert_eq!(
            native_data_dir(env(&[("HOME", "/home/mohamed"), ("XDG_DATA_HOME", "/data")])),
            Some(PathBuf::from("/data/steps"))
        );
        assert_eq!(
            native_data_dir(env(&[("HOME", "/home/mohamed")])),
            Some(PathBuf::from("/home/mohamed/.local/share/steps"))
        );
        assert_eq!(native_data_dir(env(&[])), None);
    }

    #[test]
    fn round_trips_and_ignores_junk() {
        let p = DevicePrefs {
            theme: Theme::NightShift,
            text_size: TextSize::Larger,
            nav_style: NavStyle::Slim,
            pen_hover: false,
            mouse_hover: true,
            appearance: Appearance::Tinted,
            desk_split: 65,
            desk_focus: true,
            reading: Reading { size: ReadSize::Larger, spacing: ReadSpacing::Airy, width: ReadWidth::Narrow },
        };
        assert_eq!(DevicePrefs::parse(&p.render()), p);
        for &size in &ReadSize::ALL {
            for &spacing in &ReadSpacing::ALL {
                for &width in &ReadWidth::ALL {
                    let q = DevicePrefs { reading: Reading { size, spacing, width }, ..p };
                    assert_eq!(DevicePrefs::parse(&q.render()), q);
                }
            }
        }
        let junk = DevicePrefs::parse("read_size=huge\nread_spacing=wide\nread_width=page");
        assert_eq!(junk.reading, Reading::default());
        assert_eq!(DevicePrefs::parse("theme=neon\nnonsense\ntext_size=huge"), DevicePrefs::default());
    }

    #[test]
    fn css_carries_theme_and_size() {
        let p = DevicePrefs { theme: Theme::Paper, text_size: TextSize::Large, ..DevicePrefs::default() };
        let css = p.css();
        assert!(css.contains("--bg:#f3f6f9") && css.ends_with("html{font-size:112%;}"), "{css}");
    }

    #[test]
    fn a_file_from_before_the_redesign_loads_as_stepped() {
        // The exact file an older build wrote: no appearance line.
        let old = "theme=night\ntext_size=small\nnav_style=labels\npen_hover=on\nmouse_hover=off\n";
        let p = DevicePrefs::parse(old);
        assert_eq!(p.appearance, Appearance::Stepped);
        assert_eq!(p.reading, Reading::default());
        assert_eq!((p.theme, p.text_size, p.mouse_hover), (Theme::Night, TextSize::Small, false));
        assert_eq!(DevicePrefs::parse("appearance=sparkly").appearance, Appearance::Stepped);
        // The old gradient style moves to Stepped, the style that replaced it.
        assert_eq!(DevicePrefs::parse("appearance=glossy").appearance, Appearance::Stepped);
        assert_eq!(DevicePrefs::parse("appearance=calm").appearance, Appearance::Calm);
        // The old glass style moves to Frost, Steps' own glass.
        assert_eq!(DevicePrefs::parse("appearance=liquid-glass").appearance, Appearance::Frost);
        for appearance in Appearance::ALL {
            let with_appearance = DevicePrefs { appearance, ..p };
            assert_eq!(DevicePrefs::parse(&with_appearance.render()), with_appearance);
        }
    }

    #[test]
    fn frost_has_solid_accessibility_fallbacks() {
        let css = include_str!("../assets/frost.css");
        for expected in [
            ".app[data-appearance=\"frost\"]",
            "@supports",
            "prefers-reduced-transparency: reduce",
            "prefers-contrast: more",
            "forced-colors: active",
            ".settings-window",
            "--glass-solid: var(--surface);",
            "--surface: color-mix(in srgb, var(--glass-solid)",
            ".nav-item[aria-current=\"page\"] .glyph",
        ] {
            assert!(css.contains(expected), "Missing {expected}");
        }
    }

    #[test]
    fn surface_ink_stays_solid_under_frost() {
        // Frost turns --surface translucent (assets/frost.css).
        // Text, ticks, fills and strokes painted in the surface colour (light
        // ink on a coloured fill) read the solid copy instead, or they fade.
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("readable dir") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "css" || e == "rs") {
                    out.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("assets"), &mut files);
        walk(&root.join("src"), &mut files);
        let needle = ["var(--", "surface)"].concat();
        for path in files {
            let text = std::fs::read_to_string(&path).expect("readable file");
            for (i, _) in text.match_indices(&needle) {
                let before = text[..i].trim_end();
                let ink = ["color:", "fill:", "stroke:", "solid"]
                    .iter()
                    .any(|p| before.ends_with(p) && !before.ends_with("-color:"));
                assert!(
                    !ink || before.ends_with("var(--glass-solid,"),
                    "{}: surface-coloured ink must read var(--glass-solid, var(--surface))",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn desk_split_and_focus_keep_safe_defaults() {
        let old = DevicePrefs::parse("theme=night\n");
        assert_eq!((old.desk_split, old.desk_focus), (0, false));
        assert_eq!(DevicePrefs::parse("desk_split=99\ndesk_focus=yes").desk_split, 0);
        assert_eq!(DevicePrefs::parse("desk_split=34").desk_split, 0);
        let p = DevicePrefs::parse("desk_split=35\ndesk_focus=on");
        assert_eq!((p.desk_split, p.desk_focus), (35, true));
    }

    #[test]
    fn smaller_interface_size_round_trips() {
        let p = DevicePrefs { text_size: TextSize::Smaller, ..DevicePrefs::default() };
        assert_eq!(DevicePrefs::parse(&p.render()), p);
        assert!(p.css().ends_with("html{font-size:82%;}"));
    }
}
