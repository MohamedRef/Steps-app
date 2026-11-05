//! The "what's new" sheet: what an update brings before it's installed,
//! what it brought right after (once, by itself), and the whole changelog
//! from Settings › Updates. One sheet for all three, laid out the
//! way Morphe Manager's update dialog is (ideas only, none of its code):
//!
//! - a head with the state ("فيه نسخة جديدة", "Steps اتحدّثت") and
//!   `Steps · v0.1.0 (412)`;
//! - a compact installed → available build summary that leaves room for the timeline;
//! - a quiet timeline: newest build open, earlier builds folded, each kind
//!   in a card with a count and a small coloured icon;
//! - "Earlier builds" immediately after the newest one, a few at a time;
//! - GitHub and a close or update action pinned at the foot.
//!
//! On a phone it covers the whole screen, with Steps's icon heading it; from
//! 600px it is a centred dialog. Morphe's filled pills: blue for new, amber
//! for fixes, green for improvements.
//!
//! Adapted from batna-app main: `crates/app/src/update/sheet.rs` (one
//! language, Steps's colours and buttons).

use dioxus::prelude::*;
use steps_core::update::{BuildChanges, ChangeKind};

use super::Phase;
use super::changelog::{self, between, civil, day_of, label, line, of_kind, tally};
use super::view::{UPDATE_CSS, megabytes, primary_action, use_update_status};
use crate::i18n::update as tx;

const SHEET_CSS: &str = include_str!("sheet.css");

/// Which sheet is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetMode {
    /// What the offered update brings (banner, card, notification).
    Offer,
    /// What the update just installed brought: opens once by itself.
    Updated,
    /// Every build so far (Settings › Updates).
    History,
}

/// Opens and closes the sheet; provided once in the shell.
#[derive(Clone, Copy)]
pub struct UpdateSheetNav(pub Signal<Option<SheetMode>>);

impl UpdateSheetNav {
    pub fn new() -> Self {
        Self(Signal::new(None))
    }

    pub fn open(mut self, mode: SheetMode) {
        self.0.set(Some(mode));
    }

    /// What the offer brings, or the changelog when nothing is offered
    /// right now (a notification tapped after Steps restarted).
    pub fn open_offer(self) {
        let offered = super::get().is_some_and(|u| {
            matches!(
                u.status().phase,
                Phase::Available(_)
                    | Phase::Downloading { .. }
                    | Phase::Ready(_)
                    | Phase::Installing(_)
                    | Phase::NeedsPermission(_)
                    | Phase::Installed(_)
            )
        });
        self.open(if offered { SheetMode::Offer } else { SheetMode::History });
    }
}

/// Lines a kind card shows before "Show all"; it folds only past `FOLD_AT`.
const PREVIEW: usize = 5;
const FOLD_AT: usize = 6;
/// Earlier builds shown at a time.
const EARLIER_STEP: usize = 4;
/// Builds the changelog lists before "Earlier builds".
const HISTORY_FIRST: usize = 1;

/// The full history labels every release after the newest as "Earlier".
/// An update offer instead leaves the installed build above that separator.
fn earlier_divider(shown: &[(bool, BuildChanges)], mode: SheetMode, installed_key: &str) -> Option<usize> {
    shown
        .iter()
        .position(|(earlier, build)| *earlier && (mode == SheetMode::History || build.key() != installed_key))
}

fn kind_label(k: ChangeKind) -> &'static str {
    match k {
        ChangeKind::New => tx::KIND_NEW,
        ChangeKind::Improved => tx::KIND_IMPROVED,
        ChangeKind::Fixed => tx::KIND_FIXED,
    }
}

fn kind_id(k: ChangeKind) -> &'static str {
    match k {
        ChangeKind::New => "new",
        ChangeKind::Improved => "improved",
        ChangeKind::Fixed => "fixed",
    }
}

/// Our own glyphs: a four-point spark, a rising line, a bug.
fn kind_icon(k: ChangeKind) -> Element {
    let path = match k {
        ChangeKind::New => "M12 3l1.9 5.6L19.5 10l-5.6 1.9L12 17.5l-1.9-5.6L4.5 10l5.6-1.4z",
        ChangeKind::Improved => "M4 16l5-5 4 4 7-7M15 8h5v5",
        ChangeKind::Fixed => {
            "M8 10h8v4a4 4 0 0 1-8 0zM9.5 10a2.5 2.5 0 0 1 5 0M12 10v8M4 13h4M16 13h4M5 8l3 2M19 8l-3 2M5 19l3-2M19 19l-3-2"
        }
    };
    let fill = if k == ChangeKind::New { "currentColor" } else { "none" };
    rsx! {
        svg { view_box: "0 0 24 24", fill: "{fill}", stroke: "currentColor", stroke_width: "2.2",
            stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
            path { d: "{path}" }
        }
    }
}

fn glyph(view_box: &'static str, d: &'static str, filled: bool) -> Element {
    let (fill, stroke) = if filled { ("currentColor", "none") } else { ("none", "currentColor") };
    rsx! {
        svg { view_box: "{view_box}", fill: "{fill}", stroke: "{stroke}", stroke_width: "2",
            stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
            path { d: "{d}" }
        }
    }
}

fn chevron() -> Element {
    glyph("0 0 24 24", "M6 9l6 6 6-6", false)
}

fn arrow() -> Element {
    glyph("0 0 24 24", "M5 12h14M13 6l6 6-6 6", false)
}

fn download_icon() -> Element {
    glyph("0 0 24 24", "M12 4v11M7 10l5 5 5-5M5 20h14", false)
}

/// GitHub's mark, for the button that opens the releases page.
fn github_icon() -> Element {
    glyph(
        "0 0 16 16",
        "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z",
        true,
    )
}

/// Steps's mark (assets/src/icon/full.svg), as the sheet's app icon.
fn app_mark() -> Element {
    rsx! {
        svg { view_box: "0 0 108 108", "aria-hidden": "true",
            defs {
                linearGradient { id: "us-petrol", x1: "0", y1: "0", x2: "1", y2: "1",
                    stop { offset: "0", stop_color: "#2e7899" }
                    stop { offset: "1", stop_color: "#123a4f" }
                }
            }
            rect { width: "108", height: "108", fill: "url(#us-petrol)" }
            g { transform: "translate(-4 0)",
                path { d: "M72 32 H44 a8 8 0 0 0 -8 8 V48 a8 8 0 0 0 8 8 H64 a8 8 0 0 1 8 8 V68 a8 8 0 0 1 -8 8 H36", fill: "none", stroke: "#f4f8fb", stroke_width: "9", stroke_linecap: "round", stroke_linejoin: "round" }
                circle { cx: "80", cy: "32", r: "5.5", fill: "#f2c45a" }
            }
        }
    }
}

/// The releases page of the channel this build follows.
fn releases_url() -> String {
    let base = steps_core::update::FEED_BASE.replace("/releases/download", "/releases/tag");
    match super::get() {
        Some(u) => format!("{base}/{}", u.installed().channel.release_tag()),
        None => base.trim_end_matches("/tag").to_owned(),
    }
}

/// The summary's two chips: `#165 → #190` when the version is the same
/// (preview and alpha builds), else `v0.1.0 (12) → v0.2.0 (30)`.
fn jump_labels(from: &str, to: &str) -> (String, String) {
    match (from.split_once(" ("), to.split_once(" (")) {
        (Some((fv, fb)), Some((tv, tb))) if fv == tv => {
            (format!("#{}", fb.trim_end_matches(')')), format!("#{}", tb.trim_end_matches(')')))
        }
        _ => (format!("v{from}"), format!("v{to}")),
    }
}

/// "Today", "2 days ago" within a week, then "28 أكتوبر".
fn when(published: &str, today: i64) -> Option<String> {
    let day = day_of(published)?;
    let ago = today - day;
    Some(match ago {
        ..=0 => tx::TODAY.to_owned(),
        1 => tx::YESTERDAY.to_owned(),
        2 => tx::TWO_DAYS_AGO.to_owned(),
        3..=6 => tx::DAYS_AGO.replace("{n}", &ago.to_string()),
        _ => {
            let (y, m, d) = civil(day);
            let (ty, _, _) = civil(today);
            let month = tx::MONTHS[m as usize - 1];
            if y == ty { format!("{d} {month}") } else { format!("{d} {month} {y}") }
        }
    })
}

/// Today's day number (UTC: close enough for "today" and "yesterday").
fn today() -> i64 {
    (super::now_ms() as i64).div_euclid(86_400_000)
}

/// The sheet; mounted once in the shell. Opens by itself once after an app
/// update that brought something to tell; after a content-only update a
/// quiet card on Today offers it instead (`view::ContentNote`, #336).
#[component]
pub fn UpdateSheet() -> Element {
    let UpdateSheetNav(mut mode) = use_context::<UpdateSheetNav>();
    let status = use_update_status();
    // The update "What's new" last opened for (a content update can bring
    // another while Steps stays open).
    let mut opened_after_update = use_signal(|| None::<String>);
    let mut open_builds = use_signal(Vec::<String>::new);
    let mut open_kinds = use_signal(Vec::<String>::new);
    let mut earlier_shown = use_signal(|| EARLIER_STEP);

    use_effect(move || {
        let from = status
            .read()
            .as_ref()
            .filter(|s| !s.whats_new().is_empty() && !s.content_only_news())
            .and_then(|s| s.prefs.whats_new_from.clone());
        if from.is_some() && *opened_after_update.peek() != from && mode.peek().is_none() {
            opened_after_update.set(from);
            mode.set(Some(SheetMode::Updated));
        }
    });

    let Some(m) = *mode.read() else { return rsx! {} };
    let Some(s) = status.read().clone() else { return rsx! {} };
    let close = move |_| {
        if *mode.peek() == Some(SheetMode::Updated)
            && let Some(u) = super::get()
        {
            u.close_whats_new();
        }
        mode.set(None);
        open_builds.set(Vec::new());
        open_kinds.set(Vec::new());
        earlier_shown.set(EARLIER_STEP);
    };

    let offer = match &s.phase {
        Phase::Available(o)
        | Phase::Ready(o)
        | Phase::Installing(o)
        | Phase::NeedsPermission(o)
        | Phase::Installed(o)
        | Phase::Downloading { offer: o, .. } => Some(o.clone()),
        Phase::Failed { offer, .. } => offer.clone(),
        _ => None,
    };
    let current = s.current();
    // The app build he runs; a content-only update on top is named apart
    // ("بناء 40 · محتوى 45"), so the sheet and Settings agree (#336).
    let this_label = label(&s.version, s.build);
    let content_line =
        s.content_ahead().map(|c| format!("{} {} · {} {c}", tx::BUILD, s.build, tx::CONTENT_SHORT));
    let content_news = s.content_only_news();
    let latest_key = s.history.first().map(BuildChanges::key);

    // What the timeline shows first, the jump the summary names, and what
    // goes under "Earlier builds".
    let (title, top, from_label, to_label, earlier): (String, Vec<BuildChanges>, _, _, Vec<BuildChanges>) =
        match m {
            SheetMode::Offer => {
                let head = match &s.phase {
                    Phase::Downloading { .. } => tx::DOWNLOADING,
                    Phase::Ready(_) => tx::READY,
                    Phase::Installing(_) => tx::INSTALLING,
                    Phase::NeedsPermission(_) => tx::NEEDS_PERMISSION,
                    Phase::Installed(_) => tx::INSTALLED,
                    _ => tx::AVAILABLE,
                };
                let top = offer.as_ref().map(|o| o.changes.clone()).unwrap_or_default();
                let earlier = current
                    .map(|c| between(&s.history, None, c).into_iter().cloned().collect())
                    .unwrap_or_default();
                let to = offer.as_ref().map(|o| o.label());
                (head.to_owned(), top, Some(this_label.clone()), to, earlier)
            }
            SheetMode::Updated if content_news => (
                // Not a new version: no jump, the content line says where he is.
                tx::CONTENT_UPDATED_TITLE.to_owned(),
                s.whats_new(),
                None,
                None,
                Vec::new(),
            ),
            SheetMode::Updated => {
                let top = s.whats_new();
                let from = s.prefs.whats_new_from.as_deref().map(|k| {
                    let (v, b) = k.split_once('+').unwrap_or((k, "0"));
                    label(v, b.parse().unwrap_or(0))
                });
                (tx::UPDATED_TITLE.to_owned(), top, from, Some(this_label.clone()), Vec::new())
            }
            SheetMode::History => {
                let (top, rest) = s.history.split_at(s.history.len().min(HISTORY_FIRST));
                (tx::HISTORY.to_owned(), top.to_vec(), None, None, rest.to_vec())
            }
        };
    let earlier: Vec<BuildChanges> =
        earlier.into_iter().filter(|e| !top.iter().any(|t| t.key() == e.key())).collect();
    let progress = match &s.phase {
        Phase::Downloading { offer, done } if m == SheetMode::Offer => Some((*done, offer.asset.size.max(1))),
        _ => None,
    };
    let action = if m == SheetMode::Offer { primary_action(&s.phase) } else { None };
    let today = today();
    let badge_of = |b: &BuildChanges| -> Option<(&'static str, &'static str)> {
        let o = changelog::order(&b.version, b.build);
        // The app build he runs, and apart from it the content-only update
        // in use, so a (45) row never says "yours" under a (40) header (#349).
        if b.key() == format!("{}+{}", s.version, s.build) {
            Some(("installed", tx::BADGE_INSTALLED))
        } else if s.content_ahead().is_some_and(|c| b.key() == format!("{}+{c}", s.version)) {
            Some(("installed", tx::BADGE_CONTENT_IN_USE))
        } else if latest_key.as_deref() == Some(b.key().as_str()) {
            Some(("latest", tx::BADGE_LATEST))
        } else if o.is_some() && o > current {
            Some(("new", tx::BADGE_NEW))
        } else {
            None
        }
    };
    // The newest build starts open; the learner's taps flip any build.
    let first_open = top.first().map(BuildChanges::key);
    let is_open = move |key: &str| {
        let flipped = open_builds.read().iter().any(|k| k == key);
        (first_open.as_deref() == Some(key)) != flipped
    };
    let summary_builds = if m == SheetMode::History { Vec::new() } else { top.clone() };
    let pills = tally(summary_builds.iter());
    let show_earlier = (*earlier_shown.read()).min(earlier.len());
    let shown: Vec<(bool, BuildChanges)> = top
        .iter()
        .cloned()
        .map(|b| (false, b))
        .chain(earlier.iter().take(show_earlier).cloned().map(|b| (true, b)))
        .collect();
    // History separates the newest from every older build; an update offer
    // keeps the installed build beside the offered ones before the divider.
    let this_key = format!("{}+{}", s.version, s.shown_build());
    let first_earlier = earlier_divider(&shown, m, &this_key);
    let more_earlier = earlier.len() > show_earlier;
    let empty = shown.is_empty();
    let fallback_notes = offer
        .as_ref()
        .filter(|o| m == SheetMode::Offer && o.changes.is_empty())
        .map(super::notes_of)
        .filter(|n| !n.is_empty());

    rsx! {
        style { {UPDATE_CSS} }
        style { {SHEET_CSS} }
        div { class: "us-backdrop", onclick: close }
        div { class: "us-sheet", role: "dialog", aria_modal: "true", aria_labelledby: "us-title",
            div { class: "us-head",
                span { class: "us-logo", {app_mark()} }
                div { class: "us-head-text",
                    h2 { id: "us-title", {crate::i18n::app::NAME},
                    span { class: "us-sr-only", " · {title}" }
                }
                    p {
                        bdi { dir: "ltr", class: "latin",
                            "v{offer.as_ref().filter(|_| m == SheetMode::Offer).map_or(this_label.clone(), |o| o.label())}"
                        }
                    }
                }
                button { class: "icon-button", "data-back": "true", aria_label: "{tx::CLOSE}", onclick: close,
                    span { aria_hidden: "true", "✕" }
                }
            }
            div { class: "us-body",
                if m != SheetMode::History {
                    p { class: "us-context-title", "{title}" }
                }
                if m != SheetMode::History && (from_label.is_some() || !pills.is_empty() || progress.is_some()) {
                    div { class: "us-summary",
                        if let Some(c) = content_line.clone().filter(|_| m == SheetMode::Updated) {
                            p { class: "us-content-line", "data-badge": "content", "{c}" }
                        }
                        if let (Some(f), Some(t)) = (from_label.clone(), to_label.clone()) {
                            {
                                let (f, t) = jump_labels(&f, &t);
                                rsx! {
                                    div { class: "us-jump",
                                        span { class: "us-ver", bdi { dir: "ltr", class: "latin", "{f}" } }
                                        span { class: "us-arrow", {arrow()} }
                                        span { class: "us-ver to", bdi { dir: "ltr", class: "latin", "{t}" } }
                                    }
                                }
                            }
                        }
                        if !top.is_empty() {
                            p { class: "us-count",
                                if top.iter().all(|b| b.content_only) {
                                    "{tx::content_updates_count(top.len())}"
                                } else {
                                    "{tx::builds_count(top.len())}"
                                }
                            }
                        }
                        if !pills.is_empty() {
                            div { class: "us-pills",
                                for (k, n) in pills {
                                    span { key: "{kind_id(k)}", class: "us-pill", "data-kind": "{kind_id(k)}",
                                        {kind_icon(k)}
                                        "{n} {kind_label(k)}"
                                    }
                                }
                            }
                        }
                        if let Some((done, total)) = progress {
                            progress { max: "{total}", value: "{done}" }
                            p { class: "us-mb num-ltr", "{megabytes(done)} / {megabytes(total)}" }
                        }
                    }
                }
                if let Some(n) = fallback_notes {
                    p { class: "update-notes", "{n}" }
                }
                if empty {
                    p { class: "muted", {if m == SheetMode::History { tx::HISTORY_EMPTY } else { tx::NO_DETAILS }} }
                }
                ol { class: "us-timeline",
                    for (i, (is_earlier, b)) in shown.into_iter().enumerate() {
                        if Some(i) == first_earlier {
                            li { key: "{i}-earlier", class: "us-divider", role: "separator", "{tx::EARLIER}" }
                        }
                        {
                            let key = b.key();
                            let open = is_open(&key);
                            let badge = badge_of(&b);
                            let ahead = current.is_some() && changelog::order(&b.version, b.build) > current;
                            let counts = tally([&b]);
                            let mut meta: Vec<String> = when(&b.published_at, today).into_iter().collect();
                            meta.extend(counts.iter().map(|(k, n)| format!("{n} {}", kind_label(*k))));
                            let meta = meta.join(" · ");
                            let toggle_key = key.clone();
                            let _ = is_earlier;
                            rsx! {
                                li { key: "{key}", class: "us-release", "data-badge": badge.map_or("", |b| b.0), "data-ahead": "{ahead}",
                                    button {
                                        class: "us-rel-head",
                                        aria_expanded: if open { "true" } else { "false" },
                                        onclick: move |_| {
                                            let mut v = open_builds.write();
                                            if let Some(i) = v.iter().position(|k| *k == toggle_key) {
                                                v.remove(i);
                                            } else {
                                                v.push(toggle_key.clone());
                                            }
                                        },
                                        span { class: "us-rel-title",
                                            bdi { dir: "ltr", class: "latin", "v{label(&b.version, b.build)}" }
                                            if let Some((id, u)) = badge {
                                                span { class: "us-badge", "data-badge": "{id}", "{u}" }
                                            }
                                            if b.content_only {
                                                span { class: "us-badge", "data-badge": "content", {tx::BADGE_CONTENT} }
                                            }
                                        }
                                        span { class: "us-rel-meta", "{meta}" }
                                        span { class: "us-chevron", {chevron()} }
                                    }
                                    if open {
                                        div { class: "us-kinds",
                                            for k in ChangeKind::ALL {
                                                {
                                                    let items = of_kind(&b, k);
                                                    let kkey = format!("{key}:{}", kind_id(k));
                                                    let all = open_kinds.read().contains(&kkey);
                                                    let folds = items.len() > FOLD_AT;
                                                    let n = items.len();
                                                    let all_label = tx::SHOW_ALL.replace("{n}", &n.to_string());
                                                    let show: Vec<(String, bool)> = items
                                                        .into_iter()
                                                        .take(if folds && !all { PREVIEW } else { usize::MAX })
                                                        .map(line)
                                                        .collect();
                                                    rsx! {
                                                        if n > 0 {
                                                            section { key: "{kkey}", class: "us-kind", "data-kind": "{kind_id(k)}",
                                                                div { class: "us-kind-head",
                                                                    span { class: "us-kind-icon", {kind_icon(k)} }
                                                                    h3 { "{kind_label(k)}" }
                                                                    span { class: "us-chip", "{n}" }
                                                                }
                                                                ul { class: "us-items",
                                                                    for (j, (text, latin)) in show.into_iter().enumerate() {
                                                                        li { key: "{j}",
                                                                            if latin {
                                                                                bdi { dir: "ltr", class: "latin", "{text}" }
                                                                            } else {
                                                                                "{text}"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                if folds {
                                                                    button {
                                                                        class: "us-more",
                                                                        onclick: move |_| {
                                                                            let mut v = open_kinds.write();
                                                                            if let Some(i) = v.iter().position(|x| *x == kkey) {
                                                                                v.remove(i);
                                                                            } else {
                                                                                v.push(kkey.clone());
                                                                            }
                                                                        },
                                                                        if all {
                                                                            "{tx::SHOW_LESS}"
                                                                        } else {
                                                                            "{all_label}"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if more_earlier {
                    button { class: "us-more", onclick: move |_| earlier_shown += EARLIER_STEP, "{tx::SHOW_OLDER}" }
                }
            }
            div { class: "us-foot",
                button {
                    class: "us-btn",
                    onclick: move |_| {
                        let _ = crate::cloud::open_url(&releases_url());
                    },
                    {github_icon()}
                    bdi { dir: "ltr", class: "latin", "GitHub" }
                }
                match action {
                    Some((label, run)) => rsx! {
                        button { class: "us-btn us-go", onclick: move |_| run(),
                            if matches!(s.phase, Phase::Available(_)) {
                                {download_icon()}
                            }
                            "{label}"
                        }
                    },
                    None if m == SheetMode::Updated => rsx! {
                        button { class: "us-btn us-go", onclick: close, "{tx::GOT_IT}" }
                    },
                    None => rsx! {
                        button { class: "us-btn", onclick: close, "{tx::CLOSE}" }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildChanges, SheetMode, earlier_divider};

    fn release(build: u64) -> BuildChanges {
        BuildChanges {
            version: "1.0.0".to_owned(),
            build,
            published_at: "2026-10-30T00:00:00Z".to_owned(),
            changes: Vec::new(),
            content_only: false,
        }
    }

    #[test]
    fn history_labels_the_first_older_release_even_when_it_is_installed() {
        let shown = vec![(false, release(3)), (true, release(2)), (true, release(1))];
        assert_eq!(earlier_divider(&shown, SheetMode::History, "1.0.0+2"), Some(1));
        assert_eq!(earlier_divider(&shown, SheetMode::History, "1.0.0+3"), Some(1));
    }

    #[test]
    fn offer_places_installed_build_before_earlier_history() {
        let shown = vec![(false, release(3)), (true, release(2)), (true, release(1))];
        assert_eq!(earlier_divider(&shown, SheetMode::Offer, "1.0.0+2"), Some(2));
        assert_eq!(earlier_divider(&shown, SheetMode::Offer, "1.0.0+3"), Some(1));
        assert_eq!(earlier_divider(&shown[..1], SheetMode::Offer, "1.0.0+2"), None);
    }
}
