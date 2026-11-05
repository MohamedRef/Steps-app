//! Boards and Beyond videos and UWorld question IDs per First Aid lecture
//! (docs/QBANK.md): a browsable index of every course, lecture and B&B
//! video with a copy button on each, a basket that mixes any of them (and
//! QIDs he pastes) into custom tests of at most 40, a strip at the top of
//! each lesson, a lecture page section, a mark for the videos he has done,
//! and a lookup from a QID he missed back to its lecture. Numbers and
//! video names only, never question text.
//!
//! Content: `content/qbank/uworld.yaml` (`steps_core::qbank`), refreshed by
//! `scripts/qbank-sync.sh`.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use dioxus::prelude::*;
use steps_core::qbank::{LectureSets, MAX_TEST, QBank, Set, Split};

use crate::Ui;
use crate::content::Library;
use crate::i18n::qbank as tx;
use crate::look::{Css, css};

const CSS: Css = css!("src/qbank/qbank.css");
/// Synced, one key per video: `qbank.done.<video>` = true.
const DONE: &str = "qbank.done.";
/// Synced test settings.
const PREFS: &str = "qbank.tests";
/// The page id of the index on the lecture page stack.
pub const PAGE: &str = "uworld";

pub fn data() -> &'static QBank {
    static DATA: OnceLock<QBank> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_yaml_ng::from_str(include_str!("../../../../content/qbank/uworld.yaml"))
            .expect("content/qbank/uworld.yaml parses (steps-tools validate)")
    })
}

fn load_done() -> BTreeSet<String> {
    crate::store::shared()
        .settings_with_prefix::<bool>(DONE)
        .into_iter()
        .filter(|(_, on)| *on)
        .filter_map(|(k, _)| k.strip_prefix(DONE).map(str::to_string))
        .collect()
}

static DONE_SET: GlobalSignal<BTreeSet<String>> = Signal::global(load_done);

fn set_done(video: &str, on: bool) {
    crate::store::save_setting(&format!("{DONE}{video}"), &on);
    let mut done = DONE_SET.write();
    if on {
        done.insert(video.to_string());
    } else {
        done.remove(video);
    }
}

/// After a sync or restore.
pub fn reload() {
    let fresh = load_done();
    if *DONE_SET.peek() != fresh {
        *DONE_SET.write() = fresh;
    }
    let prefs = load_prefs();
    if *TEST_PREFS.peek() != prefs {
        *TEST_PREFS.write() = prefs;
    }
}

/// QIDs as UWorld's custom test takes them: comma separated.
pub fn qid_list<'a>(sets: impl IntoIterator<Item = &'a Set>) -> String {
    sets.into_iter().flat_map(|s| s.qids.iter()).map(u32::to_string).collect::<Vec<_>>().join(", ")
}

/// The B&B video name without its course: `Atherosclerosis`.
pub fn video_name(set: &Set) -> &str {
    set.video.split_once(" › ").map_or(set.video.as_str(), |(_, v)| v)
}

fn copy_text(text: &str) {
    document::eval(&format!(
        "navigator.clipboard && navigator.clipboard.writeText({})",
        serde_json::to_string(text).unwrap_or_default()
    ));
}

/// Opens the index over the current screen.
pub fn open(ui: Ui) {
    crate::pagesheet::close();
    crate::lecture::open(ui, PAGE);
}

/// How he cuts tests: synced as one setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TestPrefs {
    pub size: usize,
    pub split: Split,
    pub shuffle: bool,
    /// Leave out videos he marked done.
    pub skip_done: bool,
}

impl Default for TestPrefs {
    fn default() -> Self {
        Self { size: MAX_TEST, split: Split::Even, shuffle: true, skip_done: true }
    }
}

fn load_prefs() -> TestPrefs {
    let mut p: TestPrefs = crate::store::shared().setting(PREFS).unwrap_or_default();
    p.size = p.size.clamp(1, MAX_TEST);
    p
}

static TEST_PREFS: GlobalSignal<TestPrefs> = Signal::global(load_prefs);

fn save_prefs(p: TestPrefs) {
    crate::store::save_setting(PREFS, &p);
    *TEST_PREFS.write() = p;
}

/// The shuffle, kept until he asks for another so the tests he is copying
/// one by one don't change under him.
static SEED: GlobalSignal<u64> = Signal::global(|| crate::store::now_ms().unsigned_abs());
/// What he ticked: `c:<course>`, `l:<lecture>` or `v:<video>`. Kept for
/// the session.
static PICKED: GlobalSignal<BTreeSet<String>> = Signal::global(BTreeSet::new);
/// QIDs he pasted into the basket.
static PASTED: GlobalSignal<String> = Signal::global(String::new);

/// The QIDs of `sets` he still has to do (all of them when he keeps done
/// videos in), in book order.
pub fn left<'a>(
    sets: impl IntoIterator<Item = &'a Set>,
    done: &BTreeSet<String>,
    skip_done: bool,
) -> Vec<u32> {
    sets.into_iter()
        .filter(|s| !(skip_done && done.contains(&s.video)))
        .flat_map(|s| s.qids.iter().copied())
        .collect()
}

fn course_lectures(course: &str) -> impl Iterator<Item = &'static LectureSets> + '_ {
    data().lectures.iter().filter(move |l| steps_core::program::course_of(&l.id) == course)
}

/// The basket: every ticked course, lecture and video in book order, then
/// the pasted QIDs. Repeats go when the tests are cut.
pub fn basket(picked: &BTreeSet<String>, pasted: &str, done: &BTreeSet<String>, skip_done: bool) -> Vec<u32> {
    let sets = data().lectures.iter().flat_map(|l| {
        let whole = picked.contains(&format!("l:{}", l.id))
            || picked.contains(&format!("c:{}", steps_core::program::course_of(&l.id)));
        l.sets.iter().filter(move |s| whole || picked.contains(&format!("v:{}", s.video)))
    });
    let mut ids = left(sets, done, skip_done);
    ids.extend(steps_core::qbank::parse_qids(pasted));
    ids
}

fn cut(ids: &[u32], p: TestPrefs, seed: u64) -> Vec<Vec<u32>> {
    steps_core::qbank::tests(ids, p.size, p.split, p.shuffle.then_some(seed))
}

/// The test settings: size, how to cut the rest, shuffle.
#[component]
fn PrefsRow() -> Element {
    let p = TEST_PREFS();
    rsx! {
        div { class: "qb-prefs",
            label { class: "qb-size",
                span { {tx::SIZE} " " b { class: "num-ltr", "{p.size}" } }
                input {
                    r#type: "range",
                    min: "5",
                    max: "{MAX_TEST}",
                    step: "1",
                    value: "{p.size}",
                    aria_label: tx::SIZE,
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<usize>() {
                            save_prefs(TestPrefs { size: v.clamp(1, MAX_TEST), ..TEST_PREFS() });
                        }
                    },
                }
            }
            div { class: "qb-split", role: "group",
                button {
                    class: "chip tap-chip",
                    aria_pressed: p.split == Split::Even,
                    onclick: move |_| save_prefs(TestPrefs { split: Split::Even, ..TEST_PREFS() }),
                    {tx::EVEN}
                }
                button {
                    class: "chip tap-chip",
                    aria_pressed: p.split == Split::Rest,
                    onclick: move |_| save_prefs(TestPrefs { split: Split::Rest, ..TEST_PREFS() }),
                    {tx::rest(p.size)}
                }
            }
            label { class: "qb-check",
                input {
                    r#type: "checkbox",
                    checked: p.shuffle,
                    onchange: move |e| save_prefs(TestPrefs { shuffle: e.checked(), ..TEST_PREFS() }),
                }
                {tx::SHUFFLE}
            }
            if p.shuffle {
                button { class: "btn-link qb-reshuffle", onclick: move |_| *SEED.write() += 1, "↻ " {tx::RESHUFFLE} }
            }
        }
    }
}

/// `ids` cut into tests, one copy button each.
#[component]
fn Tests(node: String, ids: Vec<u32>, copied: Signal<Option<String>>) -> Element {
    let key = node;
    let tests = cut(&ids, TEST_PREFS(), SEED());
    let n: usize = tests.iter().map(Vec::len).sum();
    rsx! {
        p { class: "muted small qb-tests-line", {tx::tests_line(n, tests.len())} }
        div { class: "qb-tests",
            for (i , t) in tests.into_iter().enumerate() {
                {
                    let mark = format!("{key}#{i}");
                    let text = t.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
                    let shown = copied().as_deref() == Some(mark.as_str());
                    rsx! {
                        button {
                            key: "{mark}",
                            class: if i == 0 { "btn btn-tonal qb-test" } else { "chip tap-chip qb-test" },
                            title: "{text}",
                            onclick: move |_| {
                                copy_text(&text);
                                copied.set(Some(mark.clone()));
                            },
                            if shown { {tx::COPIED} } else { {tx::test(i + 1, t.len())} }
                        }
                    }
                }
            }
        }
    }
}

/// Copies `ids` at once when they fit one test; otherwise opens the tests
/// under its row (`panel`).
#[component]
fn CopyButton(
    node: String,
    ids: Vec<u32>,
    copied: Signal<Option<String>>,
    panel: Signal<Option<String>>,
) -> Element {
    let key = node;
    if ids.is_empty() {
        return rsx! { span { class: "muted small qb-none", {tx::ALL_DONE} } };
    }
    let n = ids.len();
    let fits = n <= TEST_PREFS().size;
    let open = panel().as_deref() == Some(key.as_str());
    let text = qid_text(&ids);
    let shown = copied().as_deref() == Some(key.as_str());
    rsx! {
        button {
            class: "chip tap-chip qb-copy",
            aria_expanded: if fits { None } else { Some(open.to_string()) },
            title: if fits { text.clone() } else { String::new() },
            onclick: move |_| {
                if fits {
                    copy_text(&text);
                    copied.set(Some(key.clone()));
                } else {
                    panel.set(if open { None } else { Some(key.clone()) });
                }
            },
            if shown { {tx::COPIED} } else { {tx::copy(n)} }
        }
    }
}

fn qid_text(ids: &[u32]) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
}

/// The ✓ that puts a node in the basket.
#[component]
fn Pick(node: String, label: String, under: bool) -> Element {
    let key = node;
    let on = under || PICKED.read().contains(&key);
    rsx! {
        input {
            r#type: "checkbox",
            class: "qb-pick",
            checked: on,
            disabled: under,
            aria_label: "{tx::PICK}: {label}",
            title: tx::PICK,
            onchange: move |e| {
                let mut p = PICKED.write();
                if e.checked() {
                    p.insert(key.clone());
                } else {
                    p.remove(&key);
                }
            },
        }
    }
}

/// A lecture page's section: its B&B videos and their UWorld QIDs.
#[component]
pub fn LectureSection(lecture: String) -> Element {
    let ui = use_context::<Ui>();
    let copied = use_signal(|| None::<String>);
    let panel = use_signal(|| None::<String>);
    let Some(lec) = data().lecture(&lecture) else {
        return rsx! {};
    };
    let done = DONE_SET();
    let ids = left(&lec.sets, &done, TEST_PREFS().skip_done);
    let key = format!("l:{lecture}");
    let open = panel().as_deref() == Some(key.as_str());
    rsx! {
        style { {CSS.text()} }
        section { class: "lec-section qb", aria_labelledby: "qb-title",
            h2 { id: "qb-title",
                {tx::TITLE}
                span { class: "muted small", " · {tx::summary(lec.sets.len(), lec.count())}" }
            }
            p { class: "muted small qb-how", {tx::HOW} }
            div { class: "qb-all", CopyButton { node: key.clone(), ids: ids.clone(), copied, panel } }
            if open {
                div { class: "qb-panel", PrefsRow {} Tests { node: key, ids, copied } }
            }
            ul { class: "qb-sets",
                for set in lec.sets.iter() {
                    SetRow { key: "{set.video}", set: set.clone(), done: done.contains(&set.video), copied, pick: false }
                }
            }
            button { class: "btn-link qb-find", onclick: move |_| open_index(ui), {tx::FIND} }
        }
    }
}

/// The index, from a lecture page or a lesson.
fn open_index(ui: Ui) {
    crate::lecture::close();
    open(ui);
}

#[component]
fn SetRow(
    set: Set,
    done: bool,
    copied: Signal<Option<String>>,
    pick: bool,
    #[props(default)] under: bool,
) -> Element {
    let n = set.qids.len();
    let qids = qid_list([&set]);
    let video = set.video.clone();
    let key = format!("v:{}", set.video);
    let shown = copied().as_deref() == Some(key.as_str());
    rsx! {
        li { class: "qb-set", "data-done": done,
            if pick {
                Pick { node: key.clone(), label: video_name(&set).to_string(), under }
            }
            label { class: "qb-done",
                input {
                    r#type: "checkbox",
                    checked: done,
                    aria_label: tx::DONE,
                    title: tx::DONE,
                    onchange: move |e| set_done(&video, e.checked()),
                }
                span { class: "qb-video",
                    bdi { lang: "en", dir: "ltr", {video_name(&set)} }
                }
            }
            if n > 0 {
                button {
                    class: "chip tap-chip qb-copy",
                    title: "{qids}",
                    onclick: move |_| {
                        copy_text(&qids);
                        copied.set(Some(key.clone()));
                    },
                    if shown { {tx::COPIED} } else { {tx::copy(n)} }
                }
            } else {
                span { class: "muted small qb-none", {tx::NONE} }
            }
        }
    }
}

/// The top of a lesson: its lecture's B&B videos and UWorld QIDs, to copy
/// before or after the lesson.
#[component]
pub fn LessonStrip(lecture: String) -> Element {
    let copied = use_signal(|| None::<String>);
    let panel = use_signal(|| None::<String>);
    let mut list = use_signal(|| false);
    let Some(lec) = data().lecture(&lecture) else {
        return rsx! {};
    };
    let done = DONE_SET();
    let ids = left(&lec.sets, &done, TEST_PREFS().skip_done);
    let key = format!("l:{lecture}");
    let open = panel().as_deref() == Some(key.as_str());
    rsx! {
        style { {CSS.text()} }
        div { class: "qb-strip",
            div { class: "qb-strip-row",
                span { class: "qb-strip-text",
                    {tx::videos(lec.sets.len())}
                    " "
                    bdi { lang: "en", dir: "ltr", "B&B" }
                    " · "
                    {tx::questions(lec.count())}
                    " "
                    bdi { lang: "en", dir: "ltr", "UWorld" }
                }
                CopyButton { node: key.clone(), ids: ids.clone(), copied, panel }
                button {
                    class: "btn-link qb-strip-list",
                    aria_expanded: "{list()}",
                    onclick: move |_| list.toggle(),
                    {if list() { tx::CLOSE } else { tx::VIDEOS }}
                }
            }
            if open {
                div { class: "qb-panel", PrefsRow {} Tests { node: key, ids, copied } }
            }
            if list() {
                ul { class: "qb-sets",
                    for set in lec.sets.iter() {
                        SetRow { key: "{set.video}", set: set.clone(), done: done.contains(&set.video), copied, pick: false }
                    }
                }
            }
        }
    }
}

/// Where a typed QID is filed: its lecture and video.
pub fn lookup(text: &str) -> Option<(&'static str, &'static Set)> {
    let t = text.trim().trim_start_matches('#').trim();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    data().find(t.parse().ok()?)
}

/// Does `q` (lower case) name this course, lecture or video?
fn hit(q: &str, text: &str) -> bool {
    q.is_empty() || text.to_lowercase().contains(q)
}

/// More › UWorld: every course, lecture and B&B video with its QIDs, a
/// copy button on each, a basket for mixed tests, and the QID lookup.
#[component]
pub fn IndexPage() -> Element {
    let ui = use_context::<Ui>();
    let mut text = use_signal(String::new);
    let mut opened = use_signal(BTreeSet::<String>::new);
    let copied = use_signal(|| None::<String>);
    let panel = use_signal(|| None::<String>);
    let lib = Library::get();
    let done = DONE_SET();
    let prefs = TEST_PREFS();
    let typed = text.read().trim().to_string();
    let number = !typed.is_empty() && typed.trim_start_matches('#').chars().all(|c| c.is_ascii_digit());
    let found = lookup(&typed);
    let retired =
        number && typed.trim_start_matches('#').parse::<u32>().is_ok_and(|q| data().retired.contains(&q));
    let q = if number { String::new() } else { typed.to_lowercase() };
    let total: usize = data().lectures.iter().map(LectureSets::count).sum();
    let picked = PICKED();
    let pasted = PASTED();
    let in_basket = basket(&picked, &pasted, &done, prefs.skip_done);
    let courses: Vec<_> = lib
        .program
        .courses
        .iter()
        .filter_map(|c| {
            let lectures: Vec<&LectureSets> = course_lectures(&c.id).filter(|l| l.count() > 0).collect();
            (!lectures.is_empty()).then_some((c, lectures))
        })
        .collect();
    let mut shown_any = false;
    rsx! {
        style { {crate::lecture::CSS.text()} }
        style { {CSS.text()} }
        div { class: "lec-page qb-page",
            div { class: "qb-head",
                button {
                    class: "icon-button lec-back",
                    "data-back": "true",
                    aria_label: crate::i18n::lecture::BACK,
                    title: crate::i18n::lecture::BACK,
                    onclick: move |_| crate::lecture::close(),
                    span { aria_hidden: "true", "→" }
                }
                header { class: "page-head",
                    h1 { {tx::PAGE_TITLE} }
                    p { class: "page-sub", {tx::PAGE_SUB} }
                }
            }
            label { class: "search",
                {crate::icons::search()}
                input {
                    r#type: "search",
                    aria_label: tx::PLACEHOLDER,
                    placeholder: tx::PLACEHOLDER,
                    value: "{text}",
                    oninput: move |e| text.set(e.value()),
                }
            }
            if let Some((lecture, set)) = found {
                div { class: "qb-found",
                    p {
                        {tx::FOUND}
                        " "
                        bdi { lang: "en", dir: "ltr", {lib.crumb(lecture)} }
                    }
                    p { class: "muted small",
                        {tx::FROM_VIDEO}
                        " "
                        bdi { lang: "en", dir: "ltr", {video_name(set)} }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            crate::lecture::close();
                            crate::lecture::open(ui, lecture);
                        },
                        {tx::OPEN_LECTURE}
                    }
                }
            } else if retired {
                p { class: "note", {tx::RETIRED} }
            } else if number {
                p { class: "note", {tx::NOT_FOUND} }
            }
            if !number {
                ul { class: "qb-tree",
                    for (c , lectures) in courses.into_iter() {
                        {
                            let course_hit = hit(&q, &format!("{} {}", c.title, c.short));
                            let lectures: Vec<&LectureSets> = lectures
                                .into_iter()
                                .filter(|l| {
                                    course_hit || hit(&q, lib.title(&l.id)) || l.sets.iter().any(|s| hit(&q, &s.video))
                                })
                                .collect();
                            if lectures.is_empty() {
                                rsx! {}
                            } else {
                                shown_any = true;
                                let ckey = format!("c:{}", c.id);
                                let open_c = !q.is_empty() || opened.read().contains(&ckey);
                                let sets: Vec<&Set> = lectures.iter().flat_map(|l| l.sets.iter()).collect();
                                let all: usize = sets.iter().map(|s| s.qids.len()).sum();
                                let did: usize = sets.iter().filter(|s| done.contains(&s.video)).map(|s| s.qids.len()).sum();
                                let ids = left(sets.iter().copied(), &done, prefs.skip_done);
                                let c_picked = picked.contains(&ckey);
                                let toggle_c = ckey.clone();
                                rsx! {
                                    li { key: "{ckey}", class: "qb-node qb-course",
                                        div { class: "qb-row",
                                            Pick { node: ckey.clone(), label: c.title.clone(), under: false }
                                            button {
                                                class: "qb-name",
                                                aria_expanded: "{open_c}",
                                                onclick: move |_| {
                                                    let mut o = opened.write();
                                                    if !o.remove(&toggle_c) {
                                                        o.insert(toggle_c.clone());
                                                    }
                                                },
                                                span { class: "qb-title",
                                                    "{c.short} "
                                                    bdi { lang: "en", dir: "ltr", class: "muted small", "{c.title}" }
                                                }
                                                span { class: "muted small", {tx::course_line(lectures.len(), all)} }
                                                Meter { did, all }
                                            }
                                            CopyButton { node: ckey.clone(), ids: ids.clone(), copied, panel }
                                        }
                                        if panel().as_deref() == Some(ckey.as_str()) {
                                            div { class: "qb-panel", PrefsRow {} Tests { node: ckey.clone(), ids, copied } }
                                        }
                                        if open_c {
                                            ul { class: "qb-kids",
                                                for l in lectures.into_iter() {
                                                    LectureNode {
                                                        key: "{l.id}",
                                                        lecture: l.id.clone(),
                                                        q: q.clone(),
                                                        open_all: !q.is_empty() && !course_hit && !hit(&q, lib.title(&l.id)),
                                                        under: c_picked,
                                                        opened,
                                                        copied,
                                                        panel,
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
                if !shown_any {
                    p { class: "note", {tx::NO_MATCH} }
                }
            }
            p { class: "qb-note muted small", {tx::SOURCE} " " {tx::summary(data().lectures.iter().map(|l| l.sets.len()).sum(), total)} }
            Basket { ids: in_basket, copied }
        }
    }
}

#[component]
fn Meter(did: usize, all: usize) -> Element {
    let pct = if all == 0 { 0.0 } else { did as f64 * 100.0 / all as f64 };
    rsx! {
        span { class: "qb-meter", aria_hidden: "true",
            span { style: "inline-size: {pct:.1}%" }
        }
    }
}

#[component]
fn LectureNode(
    lecture: String,
    q: String,
    open_all: bool,
    under: bool,
    opened: Signal<BTreeSet<String>>,
    copied: Signal<Option<String>>,
    panel: Signal<Option<String>>,
) -> Element {
    let lib = Library::get();
    let Some(l) = data().lecture(&lecture) else {
        return rsx! {};
    };
    let done = DONE_SET();
    let key = format!("l:{lecture}");
    let open = open_all || opened.read().contains(&key);
    let did: usize = l.sets.iter().filter(|s| done.contains(&s.video)).map(|s| s.qids.len()).sum();
    let ids = left(&l.sets, &done, TEST_PREFS().skip_done);
    let picked = under || PICKED.read().contains(&key);
    let pages = lib.index.lectures.iter().find(|e| e.id == lecture).map(|e| e.pages);
    let toggle = key.clone();
    rsx! {
        li { class: "qb-node",
            div { class: "qb-row",
                Pick { node: key.clone(), label: lib.title(&lecture).to_string(), under }
                button {
                    class: "qb-name",
                    aria_expanded: "{open}",
                    onclick: move |_| {
                        let mut o = opened.write();
                        if !o.remove(&toggle) {
                            o.insert(toggle.clone());
                        }
                    },
                    span { class: "qb-title",
                        bdi { lang: "en", dir: "ltr", {lib.title(&lecture)} }
                    }
                    span { class: "muted small",
                        {tx::summary(l.sets.len(), l.count())}
                        if let Some([a, b]) = pages { span { class: "num-ltr", " · p. {a}–{b}" } }
                    }
                    Meter { did, all: l.count() }
                }
                CopyButton { node: key.clone(), ids: ids.clone(), copied, panel }
            }
            if panel().as_deref() == Some(key.as_str()) {
                div { class: "qb-panel", PrefsRow {} Tests { node: key.clone(), ids, copied } }
            }
            if open {
                ul { class: "qb-sets qb-kids",
                    for set in l.sets.iter().filter(|s| !open_all || hit(&q, &s.video)) {
                        SetRow { key: "{set.video}", set: set.clone(), done: done.contains(&set.video), copied, pick: true, under: picked }
                    }
                }
            }
        }
    }
}

/// The basket, held at the bottom of the index: what he ticked and pasted,
/// cut into tests. Only its head line stays on screen until he opens it.
#[component]
fn Basket(ids: Vec<u32>, copied: Signal<Option<String>>) -> Element {
    let mut open = use_signal(|| false);
    let n = ids.iter().copied().collect::<BTreeSet<u32>>().len();
    let p = TEST_PREFS();
    let tests = n.div_ceil(p.size.max(1));
    rsx! {
        section { class: "qb-basket", aria_label: tx::BASKET, "data-open": open(),
            div { class: "qb-basket-head",
                b { if n > 0 { {tx::in_basket_tests(n, tests)} } else { {tx::BASKET} } }
                button {
                    class: if n > 0 { "btn btn-tonal qb-basket-toggle" } else { "btn-link qb-basket-toggle" },
                    aria_expanded: "{open()}",
                    onclick: move |_| open.toggle(),
                    {if open() { tx::CLOSE } else if n > 0 { tx::SHOW_TESTS } else { tx::SHOW_BASKET }}
                }
            }
            if open() {
                label { class: "search qb-paste",
                    span { aria_hidden: "true", "#" }
                    input {
                        r#type: "text",
                        inputmode: "numeric",
                        dir: "ltr",
                        aria_label: tx::PASTE,
                        placeholder: "{tx::PASTE}: {tx::PASTE_HINT}",
                        value: "{PASTED}",
                        oninput: move |e| *PASTED.write() = e.value(),
                    }
                }
                div { class: "qb-basket-opts",
                    label { class: "qb-check",
                        input {
                            r#type: "checkbox",
                            checked: p.skip_done,
                            onchange: move |e| save_prefs(TestPrefs { skip_done: e.checked(), ..TEST_PREFS() }),
                        }
                        {tx::SKIP_DONE}
                    }
                    if n > 0 {
                        button {
                            class: "btn-link",
                            onclick: move |_| {
                                PICKED.write().clear();
                                PASTED.write().clear();
                            },
                            {tx::CLEAR}
                        }
                    }
                }
                if n > 0 {
                    PrefsRow {}
                    Tests { node: "basket".to_string(), ids, copied }
                } else {
                    p { class: "muted small", {tx::BASKET_EMPTY} }
                }
            } else if n == 0 {
                p { class: "muted small", {tx::BASKET_EMPTY} }
            }
        }
    }
}

#[cfg(test)]
mod tests;
