//! Hover peeks with the S Pen or the mouse (PARITY.md §6.3, §6.4): a glossary
//! word's expansion, a book page's quote, an MCQ option's `why`. The
//! pointer work is in `assets/hover.js`; this side hands it the glossary and
//! gives feature screens [`BookPage`].

use std::sync::OnceLock;

use dioxus::prelude::*;
use steps_core::glossary::Glossary;

use crate::i18n::hover as tx;

const HOVER_JS: &str = include_str!("../../assets/hover.js");
const GLOSSARY_YAML: &str = include_str!("../../../../content/glossary.yaml");

/// The glossary bundled with the app: `content/glossary.yaml` plus every
/// `content/glossary/<course>.yaml`. A file that doesn't parse is left out
/// (CI's `steps-tools validate` fails on it first).
pub fn glossary() -> &'static Glossary {
    static G: OnceLock<Glossary> = OnceLock::new();
    G.get_or_init(|| merged(None))
}

/// The glossary as a course's lessons read it: a term the course's own file
/// explains keeps that meaning (`ER` is estrogen receptor in repro), and
/// every other term comes from the shared file and the other courses (#271,
/// #314). Built the first time a course asks and kept.
pub fn glossary_for(course: &str) -> &'static Glossary {
    static BY_COURSE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static Glossary>>> =
        OnceLock::new();
    if !crate::content::GLOSSARY_PARTS.iter().any(|(c, _)| *c == course) {
        return glossary();
    }
    let mut map = BY_COURSE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    map.entry(course.to_string()).or_insert_with(|| Box::leak(Box::new(merged(Some(course)))))
}

/// `course`'s file first (when given), then `content/glossary.yaml`, then
/// the other course files in name order; the first file to explain a term
/// gives its meaning ([`Glossary::absorb`]).
fn merged(course: Option<&str>) -> Glossary {
    let parse = |yaml: &str| serde_yaml_ng::from_str::<Glossary>(yaml).ok();
    let empty = || Glossary { schema: String::new(), entries: Vec::new(), plain: Vec::new() };
    let parts = crate::content::GLOSSARY_PARTS;
    let own = course.and_then(|c| parts.iter().find(|(name, _)| *name == c));
    let mut g = match own {
        Some((_, yaml)) => {
            let mut g = parse(yaml).unwrap_or_else(empty);
            if let Some(shared) = parse(GLOSSARY_YAML) {
                g.absorb(shared);
            }
            g
        }
        None => parse(GLOSSARY_YAML).unwrap_or_else(empty),
    };
    for (name, yaml) in parts {
        if Some(*name) == course {
            continue;
        }
        if let Some(p) = parse(yaml) {
            g.absorb(p);
        }
    }
    g
}

pub fn term_count() -> usize {
    glossary().entries.len()
}

/// While a course's lesson is open, the hover peeks use that course's
/// meanings; the shared table comes back when it closes.
pub fn use_course_hover(course: &str) {
    let course = course.to_string();
    use_hook(move || set_table(glossary_for(&course)));
    use_drop(|| set_table(glossary()));
}

fn set_table(g: &Glossary) {
    document::eval(&format!(
        "window.__stepsHover && window.__stepsHover.setGlossary({}, {:?});",
        g.peek_table(),
        tx::UNVERIFIED
    ));
}

/// Each course's own meanings, where they differ from the shared table:
/// `{ course: { exact: {...}, lower: {...} } }`. hover.js reads these first
/// for a word inside an element marked `data-course` (an item in review, a
/// question, a case, an oral prompt), so a repro card shows `ER` as estrogen
/// receptor wherever it is studied (#318, #271).
pub fn course_overrides() -> serde_json::Value {
    let shared = glossary().peek_table();
    let mut out = serde_json::Map::new();
    for (course, _) in crate::content::GLOSSARY_PARTS {
        let own = glossary_for(course).peek_table();
        let mut diff = serde_json::Map::new();
        for key in ["exact", "lower"] {
            let base = shared.get(key).and_then(|v| v.as_object());
            let mut m = serde_json::Map::new();
            for (term, v) in own.get(key).and_then(|v| v.as_object()).into_iter().flatten() {
                if base.and_then(|b| b.get(term)) != Some(v) {
                    m.insert(term.clone(), v.clone());
                }
            }
            if !m.is_empty() {
                diff.insert(key.to_string(), m.into());
            }
        }
        if !diff.is_empty() {
            out.insert(course.to_string(), diff.into());
        }
    }
    out.into()
}

/// Installs the hover script once and gives it the glossary.
pub fn use_hover() {
    use_hook(|| {
        let table = glossary().peek_table();
        document::eval(&format!(
            "{HOVER_JS}\nwindow.__stepsHover.setGlossary({table}, {note:?});\nwindow.__stepsHover.setCourses({courses});",
            note = tx::UNVERIFIED,
            courses = course_overrides()
        ));
    });
}

/// The "ص 14" chip. Hovering it shows the item's short `src.quote` (when
/// there is one), and a tap or click opens the page's sheet
/// ([`crate::book`]): the quote, the lecture, and a session on that page.
#[component]
pub fn BookPage(
    page: u32,
    quote: Option<String>,
    /// The item has a quote, kept back until he answers.
    #[props(default)]
    held: bool,
    /// The item, lesson step or lecture citing it; a course that cites
    /// slides shows "سلايد N" and opens that lecture's slide. Empty: the
    /// primary book (First Aid).
    #[props(default)]
    of: String,
    /// The تحشيشة of the list the quote holds, shown with it in the sheet.
    #[props(default)]
    hook: Option<crate::pagesheet::Hook>,
) -> Element {
    let peek = quote.as_deref().map(steps_core::quote::plain).unwrap_or_default();
    let slides = crate::content::Library::get().program.slides(&of);
    let label = if slides { crate::i18n::cite::slide(page) } else { crate::i18n::cite::page(page) };
    let hint = if slides { crate::i18n::book::SLIDE_HINT } else { crate::i18n::book::CHIP_HINT };
    rsx! {
        button {
            class: "chip page tap-chip",
            "data-peek": peek,
            aria_label: "{hint} {page}",
            aria_haspopup: "dialog",
            onclick: move |e| {
                // Inside a card or row that has its own action, the chip only opens the page.
                e.stop_propagation();
                crate::pagesheet::open(page, &of, quote.clone(), held, hook.clone());
            },
            bdi { {label} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_glossary_parses() {
        assert!(term_count() > 50);
        assert!(glossary().find("SAAG").is_some());
        // From content/glossary/cvs.yaml.
        assert!(glossary().find("PFO").is_some());
    }

    #[test]
    fn a_course_reads_its_own_meaning_first() {
        let shared = glossary().find("ER").unwrap().text.clone();
        assert!(shared.starts_with("Endoplasmic"), "{shared}");
        let repro = glossary_for("repro").find("ER").unwrap();
        assert!(repro.text.starts_with("Estrogen receptor"), "{}", repro.text);
        // Terms only other files explain are still there, and the cache holds.
        assert!(glossary_for("repro").find("PFO").is_some());
        assert!(std::ptr::eq(glossary_for("repro"), glossary_for("repro")));
        // An unknown course reads the shared glossary.
        assert!(std::ptr::eq(glossary_for("nope"), glossary()));
    }

    #[test]
    fn hover_gets_each_course_its_own_meanings() {
        let o = course_overrides();
        let er = &o["repro"]["exact"]["ER"][0];
        assert!(er.as_str().is_some_and(|t| t.starts_with("Estrogen receptor")), "{er}");
        // Only what differs from the shared table is sent.
        assert!(o["repro"]["exact"].get("SAAG").is_none());
    }
}
