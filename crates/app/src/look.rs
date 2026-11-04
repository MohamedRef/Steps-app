//! The app's stylesheets, with the update feed's newer copies on top.
//!
//! Every `.css` file is built in with [`css!`]. A content pack from the feed
//! may carry newer copies (`Pack::look`), which only the build they were made
//! with takes (`update::installed_pack`), so a change to the look reaches
//! devices without a new app build. Screens write a stylesheet as
//! `style { {CSS.text()} }`, which picks the newest copy at each render.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// One stylesheet: its path under `crates/app` and the built-in text.
#[derive(Clone, Copy)]
pub struct Css {
    pub path: &'static str,
    pub text: &'static str,
}

/// `css!("src/lecture/lecture.css")`: a stylesheet under `crates/app`.
macro_rules! css {
    ($path:literal) => {
        $crate::look::Css { path: $path, text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)) }
    };
}
pub(crate) use css;

/// The feed's stylesheets in use (leaked: replaced a few times a session).
static FROM_FEED: RwLock<Option<&'static BTreeMap<String, String>>> = RwLock::new(None);

/// Uses `look` (from a pack) over the built-in stylesheets; empty goes back
/// to the built-in ones.
pub fn set(look: BTreeMap<String, String>) {
    let look = (!look.is_empty()).then(|| &*Box::leak(Box::new(look)));
    *FROM_FEED.write().unwrap_or_else(|e| e.into_inner()) = look;
}

/// Largest stylesheet a pack may carry (the biggest built-in one is a few KB).
const MAX_SHEET_BYTES: usize = 256 * 1024;

/// The first stylesheet in a pack that could reach outside the app, run
/// script or break out of its `<style>`, or is far too big: such a pack is
/// refused whole (#344). Only `data:` and relative `url(…)`s pass.
pub fn unsafe_sheet(look: &BTreeMap<String, String>) -> Option<&str> {
    look.iter()
        .find(|(path, text)| {
            let lower = text.to_ascii_lowercase();
            let bad_url = lower.match_indices("url(").any(|(i, _)| {
                let target = lower[i + 4..].trim_start_matches([' ', '\t', '\n', '"', '\'']);
                let target = &target[..target.find(')').unwrap_or(target.len())];
                !target.starts_with("data:") && (target.starts_with("//") || target.contains(':'))
            });
            !path.ends_with(".css")
                || path.contains("..")
                || text.len() > MAX_SHEET_BYTES
                || bad_url
                || ["@import", "javascript:", "expression(", "</style", "-moz-binding"]
                    .iter()
                    .any(|p| lower.contains(p))
        })
        .map(|(path, _)| path.as_str())
}

impl Css {
    /// The text in use: the feed's copy when it has one.
    pub fn text(self) -> &'static str {
        let feed = *FROM_FEED.read().unwrap_or_else(|e| e.into_inner());
        feed.and_then(|m| m.get(self.path)).map_or(self.text, String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_harmless_stylesheets_come_from_a_pack() {
        fn sheets(dir: &std::path::Path, root: &std::path::Path, out: &mut BTreeMap<String, String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let path = e.path();
                if path.is_dir() {
                    sheets(&path, root, out);
                } else if path.extension().is_some_and(|x| x == "css") {
                    let key = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    out.insert(key, std::fs::read_to_string(&path).unwrap());
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut built_in = BTreeMap::new();
        for d in ["src", "assets"] {
            sheets(&root.join(d), root, &mut built_in);
        }
        assert!(built_in.len() > 5);
        assert_eq!(unsafe_sheet(&built_in), None, "the app's own sheets pass");
        let one = |path: &str, text: &str| BTreeMap::from([(path.to_owned(), text.to_owned())]);
        assert_eq!(
            unsafe_sheet(&one("assets/a.css", ".x { background: url(\"data:image/svg+xml,<svg/>\") }")),
            None
        );
        assert_eq!(unsafe_sheet(&one("assets/a.css", ".x { background: url(img/a.png) }")), None);
        for bad in [
            "@import 'https://x.example/a.css';",
            ".x { background: url( 'https://x.example/t.png') }",
            ".x { background: url(//x.example/t.png) }",
            "</style><script>alert(1)</script>",
            ".x { width: expression(alert(1)) }",
        ] {
            assert_eq!(unsafe_sheet(&one("assets/a.css", bad)), Some("assets/a.css"), "{bad}");
        }
        assert!(unsafe_sheet(&one("../a.css", ".x{}")).is_some());
        assert!(unsafe_sheet(&one("assets/a.css", &"a".repeat(MAX_SHEET_BYTES + 1))).is_some());
    }

    #[test]
    fn the_feed_copy_wins_and_goes_away() {
        const MAIN: Css = css!("assets/main.css");
        assert!(MAIN.text().contains("--"), "the built-in stylesheet is there");
        set(BTreeMap::from([("assets/main.css".to_owned(), ".x { color: red; }".to_owned())]));
        assert_eq!(MAIN.text(), ".x { color: red; }");
        set(BTreeMap::new());
        assert_eq!(MAIN.text(), MAIN.text, "back to the built-in one");
    }
}
