//! Colour themes. Each theme is a fixed set of tokens that the app turns into
//! CSS custom properties. Contrast is checked in tests against WCAG AA, so a
//! new theme can't ship unreadable text.
// adapted from kabar-app main: crates/core/src/theme.rs

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
        // Glossy's highlights mix toward white and its page glows with the
        // accent; on True black both turn to black, so its pixels stay off.
        // Liquid Glass (assets/liquid-glass.css) hides its colour field and
        // keeps surfaces opaque there for the same reason.
        out.push_str(if self.is_true_black() {
            "--glass-white:#000000;--glass-glow:transparent;\
             --glass-field-on:0;--glass-alpha:100%;--glass-alpha-2:100%;--glass-alpha-bg:100%;"
        } else {
            "--glass-white:#ffffff;--glass-glow:var(--accent-soft);"
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
}

impl Theme {
    pub const ALL: [Self; 6] =
        [Self::System, Self::SystemBlack, Self::Paper, Self::Night, Self::Black, Self::NightShift];

    /// Stored value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::SystemBlack => "system-black",
            Self::Paper => "paper",
            Self::Night => "night",
            Self::Black => "black",
            Self::NightShift => "night-shift",
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
mod tests {
    use super::*;

    const ALL: [(&str, &Palette); 4] =
        [("paper", &PAPER), ("night", &NIGHT), ("black", &BLACK), ("night-shift", &NIGHT_SHIFT)];

    #[test]
    fn every_theme_meets_wcag_aa_for_text() {
        for (name, p) in ALL {
            let pairs = [
                ("text on bg", p.text, p.bg),
                ("text on surface", p.text, p.surface),
                ("muted on bg", p.muted, p.bg),
                ("muted on surface", p.muted, p.surface),
                ("accent on surface", p.accent, p.surface),
                ("accent on accent-soft", p.accent, p.accent_soft),
                ("good on good-soft", p.good, p.good_soft),
                ("bad on bad-soft", p.bad, p.bad_soft),
                ("text on accent-soft", p.text, p.accent_soft),
                ("text on surface-2", p.text, p.surface_2),
                ("muted on surface-2", p.muted, p.surface_2),
                ("on-accent on accent", p.on_accent, p.accent),
                ("accent-ink on accent-soft", p.accent_ink, p.accent_soft),
                ("text on soft", p.text, p.soft),
                ("muted on soft", p.muted, p.soft),
                ("accent on soft", p.accent, p.soft),
                ("gold on gold-soft", p.gold, p.gold_soft),
                ("text on gold-soft", p.text, p.gold_soft),
                ("good on surface", p.good, p.surface),
                ("bad on surface", p.bad, p.surface),
                ("text on sheet", p.text, p.sheet),
                ("muted on sheet", p.muted, p.sheet),
                ("accent on sheet", p.accent, p.sheet),
                ("good on sheet", p.good, p.sheet),
                ("bad on sheet", p.bad, p.sheet),
            ];
            for (what, fg, bg) in pairs {
                let c = contrast(fg, bg);
                assert!(c >= 4.5, "{name}: {what} is {c:.2}:1, below 4.5:1");
            }
        }
    }

    #[test]
    fn the_info_sheet_stands_out() {
        // The sheet floats over the page and its cards: it must read as a
        // different layer at a glance (adapted from kabar-app main:
        // crates/core/src/theme.rs word_sheet_stands_out).
        let dist = |a: &str, b: &str| {
            let (a, b) = (rgb(a), rgb(b));
            let d = |x: u8, y: u8| (f64::from(x) - f64::from(y)).powi(2);
            (d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)).sqrt()
        };
        for (name, p) in ALL {
            for (what, under) in [("page", p.bg), ("card", p.surface), ("chip", p.surface_2)] {
                let d = dist(p.sheet, under);
                assert!(d >= 24.0, "{name}: sheet vs {what} too close ({d:.0})");
            }
            if p.scheme == Scheme::Dark {
                assert!(
                    luminance(p.sheet) > luminance(p.surface),
                    "{name}: a dark sheet sits above its cards"
                );
            }
        }
    }

    #[test]
    fn night_shift_has_no_blue_light() {
        for (name, hex) in NIGHT_SHIFT.tokens() {
            let (r, _, b) = rgb(hex);
            assert!(b <= r, "night shift {name} {hex} has more blue than red");
        }
    }

    #[test]
    fn themes_round_trip_and_unknown_is_system() {
        for t in Theme::ALL {
            assert_eq!(Theme::parse(t.as_str()), t);
        }
        assert_eq!(Theme::parse("neon"), Theme::System);
        assert!(Theme::System.css().contains("prefers-color-scheme"));
        assert!(Theme::NightShift.css().starts_with(":root{--bg:#110d0a;"));
    }

    #[test]
    fn black_switches_oled_pixels_off() {
        assert_eq!(BLACK.bg, "#000000");
        assert_eq!(BLACK.surface, "#000000");
        // With surface == bg, cards are drawn by their outline alone.
        assert!(contrast(BLACK.line, BLACK.bg) >= 1.5, "card outlines must show on black");
        assert_eq!(Theme::Black.css(), format!(":root{{{}}}", BLACK.css()));
        // Glossy's white highlights and glow turn black too.
        assert!(BLACK.css().contains("--glass-white:#000000;--glass-glow:transparent;"));
        assert!(BLACK.css().contains("--glass-field-on:0;--glass-alpha:100%;"));
        assert!(!PAPER.css().contains("--glass-field-on"));
        assert!(!PAPER.is_true_black() && PAPER.css().contains("--glass-white:#ffffff;"));
        let auto = Theme::SystemBlack.css();
        assert!(auto.contains("prefers-color-scheme: dark){:root{--bg:#000000;"), "{auto}");
        assert!(auto.starts_with(":root{--bg:#f3f6f9;"));
    }

    #[test]
    fn system_follows_the_device_when_the_platform_knows_it() {
        assert_eq!(Theme::System.css_for(Some(true)), format!(":root{{{}}}", NIGHT.css()));
        assert_eq!(Theme::SystemBlack.css_for(Some(true)), format!(":root{{{}}}", BLACK.css()));
        assert_eq!(Theme::SystemBlack.css_for(Some(false)), format!(":root{{{}}}", PAPER.css()));
        assert_eq!(Theme::Night.css_for(Some(false)), Theme::Night.css());
        assert_eq!(Theme::System.css_for(None), Theme::System.css());
    }

    #[test]
    fn the_default_pair_is_the_redesign_palette() {
        // Paper and Night carry the 2026-10 design's steps-light/dark brand;
        // True black shares Night's brand blue.
        assert_eq!((PAPER.bg, PAPER.accent, PAPER.accent_soft), ("#f3f6f9", "#245f7c", "#d8eaf2"));
        assert_eq!((NIGHT.bg, NIGHT.accent, NIGHT.on_accent), ("#11202b", "#9bcde3", "#132d3b"));
        assert_eq!(BLACK.accent, NIGHT.accent);
        let css = PAPER.css();
        for token in ["--on-accent:", "--accent-ink:", "--soft:", "--gold:", "--gold-soft:"] {
            assert!(css.contains(token), "{token} missing from {css}");
        }
    }

    #[test]
    fn contrast_matches_known_values() {
        assert!((contrast("#000000", "#ffffff") - 21.0).abs() < 1e-9);
        assert!((contrast("#777777", "#777777") - 1.0).abs() < 1e-9);
    }
}
