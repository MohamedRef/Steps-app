//! What a file of questions does, question by question, like Anki's import
//! log (Mohamed, 2026-11-20): before «ضيفهم» what it would add, change, keep
//! and leave out, and after it what it did. Each question shows its number,
//! lecture, page, the start of its stem, its answer, and whether it has a
//! figure or a video; a changed one says what changed. After the import a
//! question opens in the bank with a tap.

use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;
use steps_core::exam::Set;
use steps_core::ownq::{self, Field, OwnQuestion, Plan};

use crate::content::Library;
use crate::i18n::ownq as tx;

/// Questions listed per group; the rest are counted.
pub const SHOWN: usize = 100;
/// How much of a stem a line shows.
const STEM_CHARS: usize = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    New,
    Changed,
    Same,
    Skipped,
}

impl Group {
    pub const ALL: [Group; 4] = [Group::New, Group::Changed, Group::Same, Group::Skipped];
}

/// One question of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub id: String,
    pub category: String,
    /// The bank item it is (to open it).
    pub item_id: String,
    /// «مناعة › Immunosuppressants», with its topic's title when it has one.
    pub place: String,
    pub page: Option<u32>,
    /// The start of the stem, on one line.
    pub stem: String,
    pub answer: char,
    pub figure: bool,
    pub video: bool,
    /// A changed question: what changed ([`ownq::changes`]).
    pub changes: Vec<Field>,
    /// A changed answer: the letter it was.
    pub was: Option<char>,
}

/// What a file does, by group.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Report {
    pub new: Vec<Line>,
    pub changed: Vec<Line>,
    pub same: Vec<Line>,
    /// Each row left out, with where and why, in words.
    pub skipped: Vec<String>,
    /// The categories it adds to or changes, in name order.
    pub categories: Vec<String>,
}

impl Report {
    pub fn count(&self, group: Group) -> usize {
        match group {
            Group::New => self.new.len(),
            Group::Changed => self.changed.len(),
            Group::Same => self.same.len(),
            Group::Skipped => self.skipped.len(),
        }
    }

    pub fn lines(&self, group: Group) -> &[Line] {
        match group {
            Group::New => &self.new,
            Group::Changed => &self.changed,
            Group::Same => &self.same,
            Group::Skipped => &[],
        }
    }

    /// The banks it adds to or changes, each once, with a category of each
    /// (`MH` and `mh` both go to «أسئلة MH»).
    pub fn sets(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for c in &self.categories {
            let name = set_name(c);
            if !out.iter().any(|(n, _)| *n == name) {
                out.push((name, c.clone()));
            }
        }
        out
    }

    /// The group shown first: before the import, the rows left out (they
    /// are his to fix), then what it adds; after it, what it did.
    pub fn first(&self, done: bool) -> Option<Group> {
        let order: [Group; 4] = if done {
            [Group::New, Group::Changed, Group::Skipped, Group::Same]
        } else {
            [Group::Skipped, Group::New, Group::Changed, Group::Same]
        };
        order.into_iter().find(|g| self.count(*g) > 0)
    }
}

/// The report of `incoming`, planned against `existing`, with `skipped`
/// rows already in words.
pub fn report(
    incoming: &[OwnQuestion],
    existing: &[OwnQuestion],
    plan: &Plan,
    skipped: Vec<String>,
) -> Report {
    let lib = Library::get();
    // By key, once: a file of a few thousand questions stays quick.
    let (incoming, existing) = (by_key(incoming), by_key(existing));
    let lines = |keys: &[String], with_old: bool| -> Vec<Line> {
        keys.iter()
            .filter_map(|key| {
                let q = incoming.get(key)?;
                let old = if with_old { existing.get(key).copied() } else { None };
                Some(line(lib, q, old))
            })
            .collect()
    };
    let categories: BTreeSet<String> = plan
        .new
        .iter()
        .chain(&plan.changed)
        .filter_map(|key| incoming.get(key).map(|q| q.category.clone()))
        .collect();
    Report {
        new: lines(&plan.new, false),
        changed: lines(&plan.changed, true),
        same: lines(&plan.same, false),
        skipped,
        categories: categories.into_iter().collect(),
    }
}

fn by_key(questions: &[OwnQuestion]) -> BTreeMap<String, &OwnQuestion> {
    questions.iter().map(|q| (q.key(), q)).collect()
}

fn line(lib: &Library, q: &OwnQuestion, old: Option<&OwnQuestion>) -> Line {
    let topic = q.topic.as_ref().and_then(|t| {
        lib.lectures.get(&q.lecture)?.topics.iter().find(|x| &x.id == t).map(|x| x.title.clone())
    });
    let mut place = lib.crumb(&q.lecture);
    if let Some(t) = topic {
        place = format!("{place} › {t}");
    }
    let changes = old.map(|o| ownq::changes(o, q)).unwrap_or_default();
    let was = old.filter(|_| changes.contains(&Field::Answer)).map(|o| ownq::letter(o.answer));
    Line {
        id: q.id.clone(),
        category: q.category.clone(),
        item_id: q.item_id(),
        place,
        page: q.page,
        stem: stem_start(&q.stem),
        answer: ownq::letter(q.answer),
        figure: q.image.is_some(),
        video: q.video.is_some(),
        changes,
        was,
    }
}

/// The stem's first characters on one line, with "…" when cut.
pub(super) fn stem_start(stem: &str) -> String {
    let flat = stem.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= STEM_CHARS {
        return flat;
    }
    let cut: String = flat.chars().take(STEM_CHARS).collect();
    format!("{}…", cut.trim_end())
}

/// The bank's name for a category: «أسئلة MH» for MH, else «أسئلة Pathoma».
pub fn set_name(category: &str) -> String {
    match Set::of_category(category) {
        Set::Vq => crate::i18n::vq::SET.to_string(),
        _ => tx::set(category),
    }
}

/// Closes Settings and opens the bank on a category's questions.
pub fn open_category(ui: crate::Ui, settings: crate::settings::SettingsNav, category: &str) {
    settings.close();
    crate::bank::open_set(ui, Set::of_category(category));
}

/// The report: a tile per group (each shows its list), then the list.
/// After the import (`done`), a question opens in the bank.
#[component]
pub fn ReportView(report: Report, done: bool) -> Element {
    // His pick, while it still has questions (another file may come in).
    let mut picked = use_signal(|| None::<Group>);
    let ui = use_context::<crate::Ui>();
    let settings = use_context::<crate::settings::SettingsNav>();
    let several = report.sets().len() > 1;
    let tone = |g: Group| match g {
        Group::New => "good",
        Group::Changed => "accent",
        Group::Same => "",
        Group::Skipped => "bad",
    };
    let group = picked().filter(|g| report.count(*g) > 0).or_else(|| report.first(done));
    let count = group.map_or(0, |g| report.count(g));
    rsx! {
        div { class: "stat-tiles ownq-tiles",
            for g in Group::ALL {
                button {
                    key: "{g:?}",
                    class: "tap stat-tile",
                    "data-tone": tone(g),
                    "aria-pressed": if group == Some(g) { "true" } else { "false" },
                    disabled: report.count(g) == 0,
                    onclick: move |_| picked.set(Some(g)),
                    span { class: "stat-num", "{report.count(g)}" }
                    span { class: "stat-label", {tx::group(g, done)} }
                }
            }
        }
        if let Some(g) = group {
            h3 { class: "label ownq-group",
                {tx::group_title(g, done)}
                " "
                span { class: "num-ltr", "({count})" }
            }
            if g == Group::Skipped {
                ul { class: "ownq-problems", role: "list",
                    for (i, p) in report.skipped.iter().take(SHOWN).enumerate() {
                        li { key: "{i}", class: "small", "{p}" }
                    }
                }
                if count > SHOWN {
                    p { class: "muted small", {tx::MORE_PROBLEMS} }
                }
            } else {
                if done {
                    p { class: "muted small", {tx::TAP_TO_OPEN} }
                }
                ul { class: "tap-list ownq-lines", role: "list",
                    for l in report.lines(g).iter().take(SHOWN).cloned() {
                        li { key: "{l.category}/{l.id}",
                            if done {
                                button {
                                    class: "tap tap-row",
                                    onclick: {
                                        let item = l.item_id.clone();
                                        move |_| {
                                            settings.close();
                                            crate::bank::open_question(ui, &item);
                                        }
                                    },
                                    LineBody { line: l.clone(), several }
                                    span { class: "chev", aria_hidden: "true" }
                                }
                            } else {
                                div { class: "ownq-line",
                                    LineBody { line: l.clone(), several }
                                }
                            }
                        }
                    }
                }
                if count > SHOWN {
                    p { class: "muted small", {tx::more(count - SHOWN)} }
                }
            }
        }
    }
}

/// One question: its number and what it carries, the start of its stem,
/// where it is and its answer; a changed one says what changed.
#[component]
fn LineBody(line: Line, several: bool) -> Element {
    let mut facts = vec![];
    if let Some(p) = line.page {
        facts.push(tx::page(p));
    }
    facts.push(tx::answer(line.answer));
    rsx! {
        span { class: "tap-row-main ownq-line-main",
            span { class: "ownq-line-head",
                bdi { class: "ownq-id", dir: "ltr", strong { "{line.id}" } }
                if several {
                    span { class: "chip", bdi { {set_name(&line.category)} } }
                }
                if line.figure {
                    span { class: "chip", {tx::FIGURE} }
                }
                if line.video {
                    span { class: "chip", {tx::VIDEO} }
                }
            }
            bdi { class: "ownq-stem", dir: "ltr", "{line.stem}" }
            span { class: "muted small",
                bdi { "{line.place}" }
                for f in facts {
                    " · {f}"
                }
            }
            if !line.changes.is_empty() {
                span { class: "small ownq-what", {tx::changed(&line.changes, line.was, line.answer)} }
            }
            if line.changes.contains(&Field::Lecture) {
                span { class: "muted small", {tx::MOVED} }
            }
        }
    }
}
