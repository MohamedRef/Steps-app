//! The version people read (docs/RELEASE.md §1). Since 2026-11-09 (Mohamed)
//! a release is `YEAR.MONTH.N` (`2026.11.1`) and every build, preview or
//! release, carries one rising build number: "2026.11.1 · بناء 812".

use crate::i18n::update as tx;

/// The workspace version this build was made from: the release's own for a
/// release build, the latest release's for a preview (`scripts/version.sh`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The build number CI baked in (`STEPS_BUILD`); 0 for a local build.
pub fn build() -> u64 {
    parse_build(option_env!("STEPS_BUILD"))
}

fn parse_build(raw: Option<&str>) -> u64 {
    raw.and_then(|b| b.trim().parse().ok()).unwrap_or(0)
}

/// "2026.11.1 · بناء 812" for one Arabic line, each Latin piece isolated
/// (LRI…PDI) so the numbers keep their order; a local build shows the
/// version alone.
pub fn line() -> String {
    line_for(VERSION, build())
}

fn line_for(version: &str, build: u64) -> String {
    if build > 0 {
        format!("\u{2066}{version}\u{2069} · {} \u{2066}{build}\u{2069}", tx::BUILD)
    } else {
        format!("\u{2066}{version}\u{2069}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_names_the_version_and_its_build() {
        assert_eq!(line_for("2026.11.1", 812), "\u{2066}2026.11.1\u{2069} · بناء \u{2066}812\u{2069}");
        assert_eq!(line_for("2026.11.1", 0), "\u{2066}2026.11.1\u{2069}");
        assert_eq!(parse_build(Some(" 812\n")), 812);
        assert_eq!(parse_build(Some("x")), 0);
        assert_eq!(parse_build(None), 0);
    }
}
