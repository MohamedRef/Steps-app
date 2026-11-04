//! Settings › Updates, the gentle "new Steps is ready" card above every
//! screen, and the hooks that follow the updater from the UI.
//!
//! Adapted from batna-app main: crates/app/src/update/view.rs (one
//! language, Steps's settings rows and screens).

use dioxus::prelude::*;
use steps_core::update::Channel;

use super::sheet::{SheetMode, UpdateSheetNav};
use super::{ContentCheck, Failure, Mode, Off, Offer, Phase, Status, Updater};
use crate::i18n::update as tx;
use crate::settings::{DeviceOnly, Section, SettingsNav};
use crate::{Screen, Ui};

pub(super) const UPDATE_CSS: &str = r#"
.update-state { display: grid; gap: var(--sp-2); padding: var(--sp-3);
  border-radius: var(--radius); background: var(--accent-soft); }
.update-state progress { inline-size: 100%; accent-color: var(--accent); }
.update-notes { white-space: pre-line; }
.update-banner { flex-direction: row; flex-wrap: wrap; align-items: center; gap: var(--sp-2) var(--sp-3);
  border-inline-start: 4px solid var(--accent); margin-block-end: var(--sp-3); }
.update-banner .ub-text { flex: 1 1 14rem; min-inline-size: 0; display: grid; gap: var(--sp-1); }
.content-note { flex-direction: row; flex-wrap: wrap; align-items: center; gap: var(--sp-2);
  border-inline-start: 4px solid var(--accent); margin-block-end: var(--sp-3); padding-block: var(--sp-2); }
/* The news gets a line of its own when narrow (the buttons go under it,
   side by side) and at most two lines. */
.content-note p { flex: 1 1 14rem; min-inline-size: 0; margin: 0;
  display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden; }
.content-note .btn, .content-note .icon-button { flex: none; }
.ur-card { padding: var(--sp-1) 0; gap: 0; }
.ur-row { display: flex; align-items: center; gap: var(--sp-3); inline-size: 100%; min-block-size: var(--tap-lg);
  padding: var(--sp-2) var(--sp-3); background: none; border: 0; color: inherit; text-align: start; font: inherit; cursor: pointer; }
.ur-row + .ur-row { border-block-start: 1px solid var(--line); }
.ur-row:disabled { cursor: default; }
.ur-row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.ur-text { flex: 1; min-inline-size: 0; display: grid; gap: 2px; }
.ur-title { font-weight: 600; }
.ur-sub { color: var(--muted); font-size: var(--fs-sm); overflow-wrap: anywhere; }
.update-toggle { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-3); }
.update-toggle-text { display: grid; gap: var(--sp-1); }
"#;

/// The updater's status, following its thread.
pub fn use_update_status() -> Signal<Option<Status>> {
    let mut status = use_signal(|| super::get().map(Updater::status));
    use_future(move || async move {
        let Some(u) = super::get() else { return };
        let mut rx = u.subscribe();
        while rx.changed().await.is_ok() {
            let now = rx.borrow_and_update().clone();
            if status.peek().as_ref() != Some(&now) {
                status.set(Some(now));
            }
        }
    });
    status
}

/// Runs a blocking updater call off the UI thread.
fn work(f: impl FnOnce(&'static Updater) + Send + 'static) {
    if let Some(u) = super::get() {
        let _ = std::thread::Builder::new().name("steps-update".into()).spawn(move || f(u));
    }
}

/// What's new in `o`: up to `max` of its changes (new things first), or
/// the feed's notes from before the changelog.
fn notes(o: &Offer, max: usize) -> String {
    if o.changes.is_empty() {
        super::notes_of(o)
    } else {
        super::changelog::lines_upto(&o.changes, tx::notice_more, max)
    }
}

pub(super) fn megabytes(bytes: u64) -> String {
    format!("{:.1} {}", bytes as f64 / (1024.0 * 1024.0), tx::MB)
}

fn channel_label(c: Channel) -> &'static str {
    match c {
        Channel::Preview => tx::PREVIEW,
        Channel::Alpha => tx::ALPHA,
        Channel::Beta => tx::BETA,
        Channel::Stable => tx::STABLE,
    }
}

fn mode_label(m: Mode) -> (&'static str, &'static str) {
    match m {
        Mode::Auto => (tx::MODE_AUTO, tx::MODE_AUTO_SUB),
        Mode::Notify => (tx::MODE_NOTIFY, tx::MODE_NOTIFY_SUB),
        Mode::Off => (tx::MODE_OFF, tx::MODE_OFF_SUB),
    }
}

/// The button that moves an offer forward, for the card and the banner.
pub(super) fn primary_action(phase: &Phase) -> Option<(&'static str, fn())> {
    fn download() {
        work(Updater::download);
    }
    fn install() {
        work(Updater::install);
    }
    fn restart() {
        #[cfg(target_os = "macos")]
        super::restart();
    }
    match phase {
        Phase::Available(_) => Some((tx::DOWNLOAD, download)),
        Phase::Ready(_) if cfg!(target_os = "macos") => Some((tx::INSTALL_RESTART, install)),
        Phase::Ready(_) => Some((tx::INSTALL_NOW, install)),
        Phase::NeedsPermission(_) => Some((tx::TRY_AGAIN, install)),
        Phase::Installed(_) => Some((tx::RESTART_NOW, restart)),
        Phase::Failed { offer: Some(_), why: Failure::Install(_) } => Some((tx::TRY_AGAIN, install)),
        Phase::Failed { offer: Some(_), why: Failure::Network | Failure::Verify(_) } => {
            Some((tx::TRY_AGAIN, download))
        }
        _ => None,
    }
}

/// Settings › Updates.
#[component]
pub fn UpdatesSection() -> Element {
    let status = use_update_status();
    let s = status.read().clone();
    let build = super::build_number();
    let commit = super::commit();
    let busy = s.as_ref().is_some_and(|s| {
        matches!(s.phase, Phase::Checking | Phase::Downloading { .. } | Phase::Installing(_))
    });
    // Updates that are off for a reason this build already knows (no data
    // folder at all counts as a development build).
    let off = s.as_ref().map_or(Some(Off::DevBuild), |s| s.off);
    let has_offer = s.as_ref().is_some_and(|s| {
        matches!(
            s.phase,
            Phase::Available(_)
                | Phase::Downloading { .. }
                | Phase::Ready(_)
                | Phase::NeedsPermission(_)
                | Phase::Installed(_)
        )
    });
    let sheet = try_use_context::<UpdateSheetNav>();

    // The content row opens what the content update brought while it is
    // new to him, else the whole changelog.
    let content_news = s.as_ref().is_some_and(Status::content_news);
    let open_changes = move |mode: SheetMode| {
        move |_| {
            if let Some(sheet) = sheet {
                sheet.open(mode);
            }
        }
    };
    let content_mode = if content_news { SheetMode::Updated } else { SheetMode::History };
    let has_content_sub = s.as_ref().and_then(Status::content_ahead).is_some();
    let content_sub = match s.as_ref().and_then(Status::content_ahead) {
        Some(c) => {
            let when = s.as_ref().and_then(|s| s.content_at).map(|at| {
                let now = super::now_ms();
                format!(" · {} {}", tx::CONTENT_PUBLISHED, crate::cloud::ago((at.max(0) as u64) * 1000, now))
            });
            rsx! {
                {tx::BUILD} " "
                bdi { dir: "ltr", "{c}" }
                {when.unwrap_or_default()}
            }
        }
        None => rsx! {},
    };
    let last_check = s.as_ref().and_then(|s| s.prefs.last_check_ms);
    let feed_down = s.as_ref().is_some_and(|s| {
        matches!(s.phase, Phase::Failed { why: Failure::Network, offer: None })
            && s.content_check == Some(ContentCheck::Failed)
            && !s.content_waiting
    });

    rsx! {
        style { {UPDATE_CSS} }
        // What is on this device: the app, and the lessons/questions/look
        // apart from it, since a content-only update changes only the second.
        div { class: "section-head", h2 { id: "updates-title", {tx::GROUP_HAVE} } }
        crate::ui::Card { class: "ur-card", aria_labelledby: "updates-title",
            button {
                class: "ur-row",
                disabled: sheet.is_none(),
                onclick: open_changes(SheetMode::History),
                span { class: "row-icon", aria_hidden: "true", {crate::icons::section("updates")} }
                span { class: "ur-text",
                    span { class: "ur-title", {tx::ROW_APP} }
                    span { class: "ur-sub",
                        // Each Latin piece isolated on its own, so the Arabic
                        // word "build" doesn't reorder the numbers around it.
                        bdi { dir: "ltr", {env!("CARGO_PKG_VERSION")} }
                        if build > 0 {
                            " · {tx::BUILD} "
                            bdi { dir: "ltr", "{build}" }
                        }
                        if let Some(c) = commit {
                            " · "
                            bdi { dir: "ltr", "{c}" }
                        }
                    }
                }
                span { class: "chev", aria_hidden: "true" }
            }
            button {
                class: "ur-row",
                disabled: sheet.is_none(),
                onclick: open_changes(content_mode),
                span { class: "row-icon", aria_hidden: "true", {crate::icons::book()} }
                span { class: "ur-text",
                    span { class: "ur-title", {tx::ROW_CONTENT} }
                    if has_content_sub {
                        span { class: "ur-sub", {content_sub} }
                    }
                }
                if content_news {
                    span { class: "chip", "data-tone": "good", {tx::BADGE_NEW} }
                }
                span { class: "chev", aria_hidden: "true" }
            }
        }
        div { class: "section-head", h2 { {tx::GROUP_CHECK} } }
        crate::ui::Card {
            if let Some(off) = off {
                p { class: "muted small", role: "status",
                    {match off {
                        Off::DevBuild => tx::OFF_DEV,
                        Off::NoKey => tx::OFF_KEY,
                        Off::NoPlatform => tx::OFF_PLATFORM,
                        Off::NotInBundle => tx::OFF_BUNDLE,
                    }}
                }
            } else if let Some(s) = s.clone() {
                // The feed itself out of reach is one line, not two.
                if feed_down {
                    PhaseView { phase: s.phase.clone(), auto: s.prefs.mode == Mode::Auto, label: false }
                } else {
                    PhaseView { phase: s.phase.clone(), auto: s.prefs.mode == Mode::Auto, label: true }
                    ContentResult { status: s.clone() }
                }
                p { class: "muted small ur-last",
                    match last_check {
                        Some(t) => rsx! { {tx::LAST_CHECK} " " {crate::cloud::ago(t, super::now_ms())} },
                        None => rsx! { {tx::NEVER_CHECKED} },
                    }
                }
                div { class: "row",
                    button {
                        class: "btn btn-primary",
                        disabled: busy,
                        onclick: move |_| work(|u| u.check(false)),
                        {tx::CHECK_NOW}
                    }
                    if let Some(sheet) = sheet {
                        if has_offer {
                            button { class: "btn btn-secondary", onclick: move |_| sheet.open(SheetMode::Offer), {tx::WHATS_NEW_ASK} }
                        }
                        button { class: "btn btn-ghost", onclick: move |_| sheet.open(SheetMode::History), {tx::HISTORY} }
                    }
                }
            }
        }
        if let Some(s) = s.filter(|s| s.off.is_none()) {
            crate::ui::Card {
                h2 { {tx::MODE} DeviceOnly {} }
                div { class: "choice-list", role: "radiogroup", aria_label: tx::MODE,
                    for m in Mode::ALL {
                        button {
                            key: "{m:?}",
                            class: "choice",
                            role: "radio",
                            aria_checked: if s.prefs.mode == m { "true" } else { "false" },
                            onclick: move |_| {
                                if let Some(u) = super::get() {
                                    u.set_mode(m);
                                }
                            },
                            span { class: "choice-text",
                                span { class: "choice-title", {mode_label(m).0} }
                                span { class: "muted small", {mode_label(m).1} }
                            }
                        }
                    }
                }
            }
            if s.prefs.mode != Mode::Off {
                crate::ui::Card {
                    label { class: "update-toggle",
                        span { class: "update-toggle-text",
                            span { {tx::NOTIFY} DeviceOnly {} }
                        }
                        input {
                            r#type: "checkbox",
                            role: "switch",
                            checked: s.prefs.notify,
                            onchange: move |e: FormEvent| {
                                if let Some(u) = super::get() {
                                    u.set_notify(e.checked());
                                }
                            },
                        }
                    }
                }
            }
            crate::ui::Card {
                h2 { {tx::CHANNEL} DeviceOnly {} }
                div { class: "segmented", role: "radiogroup", aria_label: tx::CHANNEL,
                    style: "grid-template-columns: repeat(4, 1fr);",
                    for c in Channel::ALL {
                        button {
                            key: "{c.as_str()}",
                            role: "radio",
                            aria_checked: if s.channel == c { "true" } else { "false" },
                            disabled: busy,
                            onclick: move |_| work(move |u| u.set_channel(c)),
                            {channel_label(c)}
                        }
                    }
                }
                if s.channel == Channel::Preview {
                    p { class: "muted small", {tx::PREVIEW_CHANNEL_NOTE} }
                }
            }
        }
    }
}

/// The Settings list's line for Updates: what is going on, or when it last
/// looked. `None` keeps the section's own summary (updates off, never looked).
pub fn summary(s: &Status) -> Option<String> {
    if s.off.is_some() {
        return None;
    }
    let line = match &s.phase {
        Phase::Checking => tx::CHECKING,
        Phase::Available(_) | Phase::Downloading { .. } => tx::AVAILABLE,
        Phase::Ready(_) | Phase::NeedsPermission(_) => tx::READY,
        _ if s.content_waiting => tx::SUMMARY_CONTENT_WAITING,
        _ if s.content_news() => tx::CONTENT_NOTE,
        Phase::Failed { .. } => tx::SUMMARY_FAILED,
        _ => {
            let t = s.prefs.last_check_ms?;
            return Some(format!("{} {}", tx::LAST_CHECK, crate::cloud::ago(t, super::now_ms())));
        }
    };
    Some(line.to_owned())
}

/// What the last check found for the lessons and questions. A waiting
/// pack says so whatever came after; nothing shows before the first check.
#[component]
fn ContentResult(status: Status) -> Element {
    let (mark, line) = match (status.content_waiting, status.content_check) {
        (true, _) | (_, Some(ContentCheck::New)) => ("", tx::CONTENT_NEW),
        (false, Some(ContentCheck::Current)) => ("✓ ", tx::CONTENT_CURRENT),
        (false, Some(ContentCheck::Failed)) => ("", tx::CONTENT_FAILED),
        (false, None) => return rsx! {},
    };
    // The same box as the app's half above, so the two read as a pair.
    rsx! {
        div { class: "update-state", role: "status", aria_live: "polite",
            p {
                strong { {tx::RESULT_CONTENT} ": " }
                strong { "{mark}" {line} }
            }
        }
    }
}

#[component]
fn PhaseView(phase: Phase, auto: bool, label: bool) -> Element {
    let action = primary_action(&phase);
    let offer: Option<Offer> = match &phase {
        Phase::Available(o)
        | Phase::Ready(o)
        | Phase::Installing(o)
        | Phase::NeedsPermission(o)
        | Phase::Installed(o)
        | Phase::Downloading { offer: o, .. } => Some(o.clone()),
        Phase::Failed { offer, .. } => offer.clone(),
        _ => None,
    };
    let headline = match &phase {
        Phase::Idle => return rsx! {},
        Phase::Checking => tx::CHECKING,
        Phase::UpToDate => tx::UP_TO_DATE,
        Phase::NotForThisDevice => tx::NOT_FOR_DEVICE,
        Phase::Available(_) => tx::AVAILABLE,
        Phase::Downloading { .. } => tx::DOWNLOADING,
        Phase::Ready(_) => tx::READY,
        Phase::Installing(_) => tx::INSTALLING,
        Phase::NeedsPermission(_) => tx::NEEDS_PERMISSION,
        Phase::Installed(_) => tx::INSTALLED,
        Phase::Failed { why: Failure::Network, .. } => tx::FAILED_NETWORK,
        Phase::Failed { why: Failure::Verify(_), .. } => tx::FAILED_VERIFY,
        Phase::Failed { why: Failure::Install(_), .. } => tx::FAILED_INSTALL,
        Phase::Failed { why: Failure::TooNew, .. } => tx::TOO_NEW,
        Phase::Failed { why: Failure::KeyChanged, .. } => tx::KEY_CHANGED,
    };
    let failed = matches!(phase, Phase::Failed { .. });
    let detail = match &phase {
        Phase::Failed { why: Failure::Verify(d) | Failure::Install(d), .. } => Some(d.clone()),
        _ => None,
    };
    let progress = match &phase {
        Phase::Downloading { offer, done } => Some((*done, offer.asset.size.max(1))),
        _ => None,
    };
    let what_new = offer.as_ref().map(|o| notes(o, 3)).filter(|n| !n.is_empty() && !failed);
    // Changelog lines are bullets already; old notes get a "What's new:".
    let prefix = offer.as_ref().is_some_and(|o| o.changes.is_empty());
    rsx! {
        div { class: "update-state", role: "status", aria_live: "polite",
            p {
                // The content's half shows right under it.
                if label {
                    strong { {tx::RESULT_APP} ": " }
                }
                strong {
                    if matches!(phase, Phase::UpToDate) { "✓ " }
                    {headline}
                }
                if let Some(o) = offer.as_ref().filter(|_| !failed) {
                    " "
                    bdi { dir: "ltr", "{o.label()}" }
                    if matches!(phase, Phase::Available(_)) {
                        span { class: "muted", " · " bdi { dir: "ltr", "{megabytes(o.asset.size)}" } }
                    }
                }
            }
            if let Some((done, total)) = progress {
                progress { max: "{total}", value: "{done}" }
            }
            if matches!(phase, Phase::Installing(_)) && cfg!(target_os = "android") {
                p { class: "muted small", {tx::INSTALLING_ANDROID} }
            }
            if matches!(phase, Phase::Available(_)) && auto && cfg!(target_os = "android") {
            }
            if matches!(phase, Phase::Installed(_)) {
                p { class: "muted small", {tx::NEXT_LAUNCH} }
            }
            if let Some(n) = what_new {
                p { class: "update-notes",
                    if prefix {
                        span { class: "muted", {tx::WHATS_NEW} ": " }
                    }
                    "{n}"
                }
            }
            if let Some(d) = detail {
                p { class: "muted small", dir: "ltr", bdi { "{d}" } }
            }
            if matches!(phase, Phase::Failed { why: Failure::KeyChanged, .. }) {
                ol { class: "small",
                    for (i, step) in tx::KEY_CHANGED_STEPS.iter().enumerate() {
                        li { key: "{i}", {*step} }
                    }
                }
                div { class: "row",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            if let Err(e) = crate::cloud::open_url(&preview_apk_url()) {
                                crate::diag::report!("{e}");
                            }
                        },
                        {tx::GET_PREVIEW}
                    }
                }
            }
            if let Some((label, run)) = action {
                div { class: "row",
                    button { class: "btn btn-primary", onclick: move |_| run(), {label} }
                    if matches!(phase, Phase::NeedsPermission(_)) {
                        button { class: "btn btn-secondary", onclick: move |_| open_install_permission(), {tx::OPEN_PERMISSION} }
                    }
                }
            }
        }
    }
}

/// The newest preview APK, next to the preview feed.
fn preview_apk_url() -> String {
    let feed = Channel::Preview.feed_url();
    let base = feed.rsplit_once('/').map_or(feed.as_str(), |(b, _)| b);
    format!("{base}/steps-preview.apk")
}

fn open_install_permission() {
    #[cfg(target_os = "android")]
    if let Err(e) = crate::platform::install::open_install_permission() {
        crate::diag::report!("{e}");
    }
}

/// The card above the screen once there's something to do: a download to
/// start ("tell me first"), an install, or a restart. "Later" hides it
/// until the next version.
#[component]
pub fn UpdateBanner() -> Element {
    let status = use_update_status();
    let Some(s) = status.read().clone() else { return rsx! {} };
    let Some(offer) = s.banner().cloned() else { return rsx! {} };
    let Some((label, run)) = primary_action(&s.phase) else { return rsx! {} };
    let headline = match s.phase {
        Phase::Available(_) => tx::AVAILABLE,
        Phase::Installed(_) => tx::INSTALLED,
        Phase::NeedsPermission(_) => tx::NEEDS_PERMISSION,
        _ => tx::READY,
    };
    // The banner stays short: two lines, the sheet has the rest.
    let what_new = notes(&offer, 2);
    let sheet = try_use_context::<UpdateSheetNav>();
    rsx! {
        style { {UPDATE_CSS} }
        crate::ui::Card { class: "update-banner", role: "status",
            div { class: "ub-text",
                p {
                    strong { {headline} }
                    " "
                    bdi { dir: "ltr", "{offer.label()}" }
                }
                if !what_new.is_empty() {
                    p { class: "muted small update-notes", "{what_new}" }
                }
            }
            div { class: "row",
                button { class: "btn btn-primary", onclick: move |_| run(), {label} }
                if let Some(sheet) = sheet {
                    button { class: "btn btn-secondary", onclick: move |_| sheet.open(SheetMode::Offer), {tx::WHATS_NEW_ASK} }
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        if let Some(u) = super::get() {
                            u.dismiss();
                        }
                    },
                    {tx::LATER}
                }
            }
        }
    }
}

/// After a content-only update: one quiet line at the top of Today with the
/// update's first change, instead of the sheet opening by itself. A tap
/// opens "what's new"; ✕ marks it seen (#336).
#[component]
pub fn ContentNote() -> Element {
    let status = use_update_status();
    let Some(s) = status.read().clone() else { return rsx! {} };
    if !s.content_only_news() {
        return rsx! {};
    }
    let first = s
        .whats_new()
        .iter()
        .flat_map(|b| b.changes.iter())
        .map(|c| if c.text.ar_eg.is_empty() { c.text.en.clone() } else { c.text.ar_eg.clone() })
        .next();
    let sheet = try_use_context::<UpdateSheetNav>();
    rsx! {
        style { {UPDATE_CSS} }
        crate::ui::Card { class: "content-note", role: "status",
            p {
                strong { {tx::CONTENT_NOTE} }
                if let Some(f) = first {
                    ": {f}"
                }
            }
            if let Some(sheet) = sheet {
                button { class: "btn btn-secondary", onclick: move |_| sheet.open(SheetMode::Updated), {tx::WHATS_NEW_ASK} }
            }
            button {
                class: "icon-button",
                aria_label: "{tx::CLOSE}",
                onclick: move |_| {
                    if let Some(u) = super::get() {
                        u.close_whats_new();
                    }
                },
                span { aria_hidden: "true", "✕" }
            }
        }
    }
}

/// Follows a tap on the "new version" notification (Android): the "what's
/// new" sheet opens, and the notification's button starts the download or
/// the install. Mounted once in the shell.
///
/// The tap's trampoline activity is translucent, so a tap while Steps is
/// open may not hide the page at all: besides visibilitychange, it looks on
/// focus, pageshow and every two seconds while visible (Steps-app#165). The
/// Kotlin side drops taps over a minute old.
pub fn use_update_notice_taps(nav: SettingsNav) {
    let sheet = try_use_context::<UpdateSheetNav>();
    use_future(move || async move {
        if !cfg!(target_os = "android") || super::get().is_none() {
            return;
        }
        let mut eval = document::eval(
            r#"
            const send = () => { if (!document.hidden) dioxus.send(true); };
            document.addEventListener("visibilitychange", send);
            window.addEventListener("focus", send);
            window.addEventListener("pageshow", send);
            setInterval(send, 2000);
            send();
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            let Some(tap) = crate::platform::update_notice::take_tap() else { continue };
            // The sheet says what's new and holds the button; without it
            // (never in the shell), Settings › Updates does.
            match sheet {
                Some(sheet) => sheet.open_offer(),
                None => nav.show(Some(Section::Updates)),
            }
            match tap.as_str() {
                "download" => work(Updater::download),
                "install" => work(Updater::install),
                _ => {}
            }
        }
    });
}

/// Whether installing now could cut a study session short: never in the
/// middle of studying, a bank block, a case or oral mode.
fn calm(screen: Screen) -> bool {
    matches!(screen, Screen::Today | Screen::Stats | Screen::Settings)
}

/// Whether something open over a calm screen would be lost by the restart:
/// a lecture he is reading (and where he scrolled to), a card he is
/// writing, or the first-run questions.
fn overlay_open() -> bool {
    crate::lecture::is_open() || crate::own::drafting() || *crate::onboarding::OPEN.peek()
}

/// The startup guard must not certify a pack until the process has opened it.
/// The first tick after a real load marks two seconds of stable operation.
fn should_settle_content(loaded: bool, settled_ticks: u32) -> bool {
    loaded && settled_ticks == 0
}

/// Switches to a content pack that arrived while Steps is open, once he is
/// between sessions (the calm moments a quiet install waits for too): the
/// lectures, lessons and look change under him with no install, and the
/// screens are drawn again. Mounted once in the shell.
pub fn use_content_switch(ui: Ui) {
    let status = use_update_status();
    let review = try_use_context::<crate::store::Review>();
    use_future(move || {
        let review = review.clone();
        async move {
            // Ticks since the pack in use was opened: once Steps has drawn
            // its first screen and run a couple of seconds on it, the
            // crash-loop guard starts counting again (#344).
            let mut loaded_for = 0u32;
            loop {
                crate::platform::time::sleep(std::time::Duration::from_secs(2)).await;
                // Count calm time only after the library is genuinely loaded.
                // Otherwise a slow first render can mark a bad pack as good.
                if should_settle_content(crate::content::Library::is_initialized(), loaded_for) {
                    loaded_for += 1;
                    super::pack_settled();
                }
                if super::get().is_none() {
                    continue;
                }
                let waiting = status.peek().as_ref().is_some_and(|s| s.content_waiting);
                if waiting
                    && calm(*ui.screen.peek())
                    && !overlay_open()
                    && super::get().is_some_and(|u| u.apply_content())
                {
                    if let Some(r) = &review {
                        r.backend.borrow_mut().library_replaced();
                    }
                    *crate::content::REV.write() += 1;
                    loaded_for = 0;
                }
            }
        }
    });
}

/// Android: follows an install in progress, picks up after "Install unknown
/// apps" was allowed, and, in automatic mode, installs a ready update
/// quietly while Steps is in the background (never mid-session, and only
/// where Android needs no confirmation). Mounted once in the shell.
pub fn use_android_install(ui: Ui) {
    let status = use_update_status();
    use_future(move || async move {
        if !cfg!(target_os = "android") || super::get().is_none() {
            return;
        }
        let mut eval = document::eval(
            r#"
            const send = () => dioxus.send(document.hidden);
            document.addEventListener("visibilitychange", send);
            setInterval(() => dioxus.send(null), 1000);
            await new Promise(() => {});
            "#,
        );
        while let Ok(hidden) = eval.recv::<Option<bool>>().await {
            let Some(s) = status.peek().clone() else { continue };
            match (&s.phase, hidden) {
                (Phase::Installing(_), _) => {
                    #[cfg(target_os = "android")]
                    if let Some(u) = super::get() {
                        // Back in front: a confirmation that arrived in the
                        // background (shown as a notification) opens now.
                        if hidden == Some(false) {
                            let _ = crate::platform::install::show_pending_confirm();
                        }
                        u.poll_install();
                    }
                }
                (Phase::NeedsPermission(_), Some(false)) if install_allowed() => work(Updater::install),
                (Phase::Ready(o), Some(true)) if s.prefs.mode == Mode::Auto => {
                    let tried = s.prefs.auto_failed.as_deref() == Some(o.key().as_str());
                    if calm(*ui.screen.peek()) && !overlay_open() && !tried && install_quietly() {
                        work(Updater::install);
                    }
                }
                _ => {}
            }
        }
    });
}

fn install_allowed() -> bool {
    #[cfg(target_os = "android")]
    {
        crate::platform::install::can_request_installs().unwrap_or(false)
    }
    #[cfg(not(target_os = "android"))]
    {
        false
    }
}

fn install_quietly() -> bool {
    #[cfg(target_os = "android")]
    {
        install_allowed() && crate::platform::install::can_install_quietly().unwrap_or(false)
    }
    #[cfg(not(target_os = "android"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_installs_wait_for_a_screen_without_a_session() {
        assert!(calm(Screen::Today));
        assert!(calm(Screen::Stats));
        for busy in [Screen::Study, Screen::Bank, Screen::Cases, Screen::Oral] {
            assert!(!calm(busy), "{busy:?}");
        }
    }
}

#[cfg(test)]
mod settle_tests {
    use super::should_settle_content;

    #[test]
    fn a_timer_before_library_load_never_settles_the_pack() {
        assert!(!should_settle_content(false, 0));
        assert!(!should_settle_content(false, 1));
        assert!(should_settle_content(true, 0));
        assert!(!should_settle_content(true, 1));
    }
}
