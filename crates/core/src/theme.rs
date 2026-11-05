//! Colour themes. Each theme is a fixed set of tokens that the app turns into
//! CSS custom properties. Contrast is checked in tests against WCAG AA, so a
//! new theme can't ship unreadable text.
// adapted from batna-app main: crates/core/src/theme.rs

/// Light or dark, for the CSS `color-scheme` property (form controls,
/// scrollbars).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// One theme's colours, as `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub scheme: Scheme,
    pub bg: &'static str,
    pub surface: &'static str,
    /// A raised container one step off `surface`: grouped rows, chips, the
    /// floating bar. On True black it is the only lit background.
    pub surface_2: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub line: &'static str,
    /// The brand colour (the design's "primary"): main actions, links, the
    /// current nav item.
    pub accent: &'static str,
    /// Text and icons on a solid `accent` fill (the primary button).
    pub on_accent: &'static str,
    /// The accent container: selected rows, the active nav pill, tonal
    /// buttons.
    pub accent_soft: &'static str,
    /// Text and icons on `accent_soft` when the accent itself is too loud.
    pub accent_ink: &'static str,
    /// A quiet tinted container: notes, flat cards, the lecture header.
    pub soft: &'static str,
    /// Calm green: right answers, "ready".
    pub good: &'static str,
    pub good_soft: &'static str,
    pub bad: &'static str,
    pub bad_soft: &'static str,
    /// Warm supporting notes (a guideline update, a mnemonic), never a grade.
    pub gold: &'static str,
    pub gold_soft: &'static str,
    /// The info sheet (what a tapped "ص N" chip opens): a raised layer in
    /// its own tint, so it never reads as one more card on the page.
    pub sheet: &'static str,
}

impl Palette {
    /// `(css variable, value)` for every token.
    pub fn tokens(&self) -> [(&'static str, &'static str); 18] {
        [
            ("--bg", self.bg),
            ("--surface", self.surface),
            ("--surface-2", self.surface_2),
            ("--text", self.text),
            ("--muted", self.muted),
            ("--line", self.line),
            ("--accent", self.accent),
            ("--on-accent", self.on_accent),
            ("--accent-soft", self.accent_soft),
            ("--accent-ink", self.accent_ink),
            ("--soft", self.soft),
            ("--good", self.good),
            ("--good-soft", self.good_soft),
            ("--bad", self.bad),
            ("--bad-soft", self.bad_soft),
            ("--gold", self.gold),
            ("--gold-soft", self.gold_soft),
            ("--sheet", self.sheet),
        ]
    }

    /// The page and cards are pure black (OLED pixels off).
    pub fn is_true_black(&self) -> bool {
        self.bg == "#000000" && self.surface == "#000000"
    }

    /// The tokens as CSS declarations, plus `color-scheme`.
    pub fn css(&self) -> String {
        let mut out = String::new();
        for (name, value) in self.tokens() {
            out.push_str(name);
            out.push(':');
            out.push_str(value);
            out.push(';');
        }
        // Glass highlights mix toward white; on True black they turn to
        // black, so its pixels stay off. Liquid Glass
        // (assets/liquid-glass.css) hides its colour field and keeps surfaces
        // opaque there for the same reason.
        out.push_str(if self.is_true_black() {
            "--glass-white:#000000;\
             --glass-field-on:0;--glass-alpha:100%;--glass-alpha-2:100%;--glass-alpha-bg:100%;"
        } else {
            "--glass-white:#ffffff;"
        });
        out.push_str("color-scheme:");
        out.push_str(self.scheme.as_str());
        out.push(';');
        out
    }
}

/// Paper: cool paper and petrol blue, the default light theme (the 2026-10
/// redesign's "steps-light"). Muted is one step darker than the design's
/// #5b7080 so it still reads on the raised surface.
pub const PAPER: Palette = Palette {
    scheme: Scheme::Light,
    bg: "#f3f6f9",
    surface: "#ffffff",
    surface_2: "#eaf0f5",
    text: "#193348",
    muted: "#5a6f7f",
    line: "#dce5ec",
    accent: "#245f7c",
    on_accent: "#ffffff",
    accent_soft: "#d8eaf2",
    accent_ink: "#27576b",
    soft: "#e7f0f5",
    good: "#277654",
    good_soft: "#e8f4ec",
    bad: "#ad493c",
    bad_soft: "#fbece7",
    gold: "#8a641d",
    gold_soft: "#fbf1d9",
    sheet: "#fcf4e0",
};

/// Night: the default dark theme ("steps-dark"): deep petrol with a light
/// blue brand, so a primary button is light with dark text.
pub const NIGHT: Palette = Palette {
    scheme: Scheme::Dark,
    bg: "#11202b",
    surface: "#192d3b",
    surface_2: "#213c4d",
    text: "#eef4f8",
    muted: "#adc3d2",
    line: "#435e70",
    accent: "#9bcde3",
    on_accent: "#132d3b",
    accent_soft: "#234656",
    accent_ink: "#c0e0ee",
    soft: "#20394a",
    good: "#a0d7b3",
    good_soft: "#1c3c31",
    bad: "#f1b2a3",
    bad_soft: "#4a2e30",
    gold: "#e3c581",
    gold_soft: "#3d3624",
    sheet: "#2c4f64",
};

/// True black, for OLED screens: the page and cards are `#000000`, so those
/// pixels switch off. Cards are told apart by their `line` outline instead of
/// a grey surface. Text is a soft off-white, which smears less than pure
/// white when an OLED screen scrolls. The brand blue is Night's.
pub const BLACK: Palette = Palette {
    scheme: Scheme::Dark,
    bg: "#000000",
    surface: "#000000",
    surface_2: "#141414",
    text: "#e6ebe7",
    muted: "#9fa9a3",
    line: "#353c38",
    accent: "#9bcde3",
    on_accent: "#0b1c25",
    accent_soft: "#10252f",
    accent_ink: "#c0e0ee",
    soft: "#0c171d",
    good: "#7cc7a0",
    good_soft: "#10251b",
    bad: "#ee8f80",
    bad_soft: "#331a17",
    gold: "#e3c581",
    gold_soft: "#2a2416",
    sheet: "#1c2a33",
};

/// Night shift: for studying late. Very dark and warm, with no colour where
/// blue outweighs red, so the screen stays easy on the eyes before sleep.
/// It keeps its amber accent: the petrol brand blue would break that rule.
pub const NIGHT_SHIFT: Palette = Palette {
    scheme: Scheme::Dark,
    bg: "#110d0a",
    surface: "#1a1511",
    surface_2: "#231c16",
    text: "#e8dccb",
    muted: "#a8998a",
    line: "#3a3029",
    accent: "#e3a552",
    on_accent: "#1a1108",
    accent_soft: "#3a2a16",
    accent_ink: "#efc890",
    soft: "#211a14",
    good: "#c2b56c",
    good_soft: "#2a2614",
    bad: "#ef9373",
    bad_soft: "#3d1f17",
    gold: "#d9b56a",
    gold_soft: "#2e2414",
    sheet: "#3a2c1f",
};

/// Scrubs: clinic teal on a fresh mint page, like a ward in the morning.
pub const SCRUBS: Palette = Palette {
    scheme: Scheme::Light,
    bg: "#f1f7f5",
    surface: "#ffffff",
    surface_2: "#e3efeb",
    text: "#12322c",
    muted: "#4c665f",
    line: "#d3e3de",
    accent: "#0f6b5f",
    on_accent: "#ffffff",
    accent_soft: "#d2ece6",
    accent_ink: "#12584e",
    soft: "#e5f2ee",
    good: "#256d48",
    good_soft: "#e6f3ea",
    bad: "#a8453a",
    bad_soft: "#fbece8",
    gold: "#80601a",
    gold_soft: "#fbf1d9",
    sheet: "#fcf1d8",
};

/// Sepia: warm book paper and brown ink, for long reading. The info sheet is
/// cool blue so it never melts into the warm page.
pub const SEPIA: Palette = Palette {
    scheme: Scheme::Light,
    bg: "#f6f0e4",
    surface: "#fcf8f0",
    surface_2: "#efe6d4",
    text: "#3a2c1d",
    muted: "#65543f",
    line: "#e2d6c0",
    accent: "#8a4b22",
    on_accent: "#ffffff",
    accent_soft: "#f1dfcc",
    accent_ink: "#6e3c1b",
    soft: "#f2e9d9",
    good: "#3b6530",
    good_soft: "#e8efdc",
    bad: "#9c3a2b",
    bad_soft: "#f7e3dc",
    gold: "#6e510f",
    gold_soft: "#f5e7c2",
    sheet: "#dde9f0",
};

/// Indigo: deep blue-violet with a soft periwinkle brand, a calmer dark than
/// Night.
pub const INDIGO: Palette = Palette {
    scheme: Scheme::Dark,
    bg: "#14142b",
    surface: "#1c1d3a",
    surface_2: "#26284b",
    text: "#eeeefa",
    muted: "#b2b4d6",
    line: "#45487a",
    accent: "#b4b8ff",
    on_accent: "#1a1b3d",
    accent_soft: "#2e3163",
    accent_ink: "#d2d4ff",
    soft: "#23254a",
    good: "#9fdcb3",
    good_soft: "#1d3a33",
    bad: "#f4b2a6",
    bad_soft: "#4a2a35",
    gold: "#e6c983",
    gold_soft: "#3b3626",
    sheet: "#383b78",
};

/// High contrast: black on white with a strong blue brand, outlines a step
/// darker, for bright light or tired eyes.
pub const CONTRAST: Palette = Palette {
    scheme: Scheme::Light,
    bg: "#ffffff",
    surface: "#ffffff",
    surface_2: "#eeeeee",
    text: "#000000",
    muted: "#333333",
    line: "#6b6b6b",
    accent: "#003d99",
    on_accent: "#ffffff",
    accent_soft: "#dbe7ff",
    accent_ink: "#002a6b",
    soft: "#f0f0f0",
    good: "#0b5a2e",
    good_soft: "#e3f3e8",
    bad: "#9e1b0f",
    bad_soft: "#fde4e0",
    gold: "#5c4200",
    gold_soft: "#fbefcc",
    sheet: "#fff1c2",
};

/// What the learner picked in Settings › Look.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    /// Paper by day, Night when the device is dark.
    #[default]
    System,
    /// Paper by day, True black when the device is dark.
    SystemBlack,
    Paper,
    Night,
    /// True black (AMOLED).
    Black,
    NightShift,
    Scrubs,
    Sepia,
    Indigo,
    Contrast,
}

impl Theme {
    pub const ALL: [Self; 10] = [
        Self::System,
        Self::SystemBlack,
        Self::Paper,
        Self::Night,
        Self::Black,
        Self::NightShift,
        Self::Scrubs,
        Self::Sepia,
        Self::Indigo,
        Self::Contrast,
    ];

    /// Stored value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::SystemBlack => "system-black",
            Self::Paper => "paper",
            Self::Night => "night",
            Self::Black => "black",
            Self::NightShift => "night-shift",
            Self::Scrubs => "scrubs",
            Self::Sepia => "sepia",
            Self::Indigo => "indigo",
            Self::Contrast => "contrast",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|t| t.as_str() == s).unwrap_or_default()
    }

    /// The fixed palette, or `None` for the System themes (they follow the
    /// device; see [`Theme::day_and_night`]).
    pub fn palette(self) -> Option<&'static Palette> {
        match self {
            Self::System | Self::SystemBlack => None,
            Self::Paper => Some(&PAPER),
            Self::Night => Some(&NIGHT),
            Self::Black => Some(&BLACK),
            Self::NightShift => Some(&NIGHT_SHIFT),
            Self::Scrubs => Some(&SCRUBS),
            Self::Sepia => Some(&SEPIA),
            Self::Indigo => Some(&INDIGO),
            Self::Contrast => Some(&CONTRAST),
        }
    }

    /// The palettes shown by day and when the device is dark: the same one
    /// for a fixed theme.
    pub fn day_and_night(self) -> (&'static Palette, &'static Palette) {
        match self {
            Self::System => (&PAPER, &NIGHT),
            Self::SystemBlack => (&PAPER, &BLACK),
            fixed => {
                let p = fixed.palette().unwrap_or(&PAPER);
                (p, p)
            }
        }
    }

    /// The CSS that applies this theme to the whole page.
    pub fn css(self) -> String {
        self.css_for(None)
    }

    /// Like [`Theme::css`], but when the platform knows the device's dark
    /// mode (`Some`), a System theme takes that side directly instead of
    /// trusting `prefers-color-scheme`: Android's WebView reports light
    /// whenever the activity theme isn't DayNight, even in night mode.
    pub fn css_for(self, device_dark: Option<bool>) -> String {
        if let (None, Some(dark)) = (self.palette(), device_dark) {
            let (day, night) = self.day_and_night();
            return format!(":root{{{}}}", if dark { night } else { day }.css());
        }
        match self.palette() {
            Some(p) => format!(":root{{{}}}", p.css()),
            None => {
                let (day, night) = self.day_and_night();
                format!(
                    ":root{{{}}}@media (prefers-color-scheme: dark){{:root{{{}}}}}",
                    day.css(),
                    night.css()
                )
            }
        }
    }
}

/// WCAG relative luminance of `#rrggbb`.
pub fn luminance(hex: &str) -> f64 {
    let (r, g, b) = rgb(hex);
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// WCAG contrast ratio between two `#rrggbb` colours (1 to 21).
pub fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0);
    (p(0), p(2), p(4))
}

#[cfg(test)]
mod tests;
