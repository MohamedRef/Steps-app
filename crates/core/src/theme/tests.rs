use super::*;

const ALL: [(&str, &Palette); 8] = [
    ("paper", &PAPER),
    ("night", &NIGHT),
    ("black", &BLACK),
    ("night-shift", &NIGHT_SHIFT),
    ("scrubs", &SCRUBS),
    ("sepia", &SEPIA),
    ("indigo", &INDIGO),
    ("contrast", &CONTRAST),
];

#[test]
fn every_fixed_theme_is_tested() {
    let fixed = Theme::ALL.iter().filter(|t| t.palette().is_some()).count();
    assert_eq!(fixed, ALL.len());
    for (name, p) in ALL {
        assert_eq!(Theme::parse(name).palette(), Some(p), "{name}");
    }
}

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
    // different layer at a glance (adapted from batna-app main:
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
            assert!(luminance(p.sheet) > luminance(p.surface), "{name}: a dark sheet sits above its cards");
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
    // Glass's white highlights turn black too.
    assert!(BLACK.css().contains("--glass-white:#000000;--glass-field-on:0;"));
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
