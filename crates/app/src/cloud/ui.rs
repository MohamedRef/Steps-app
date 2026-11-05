//! Settings › Sync: Google sign-in, the sync state, and the Devices list
//! with remote sign-out. `use_provide_cloud` starts the engine once (when
//! this build knows a Steps Cloud URL) and mirrors its events into signals.
// adapted from batna-app main: crates/app/src/account.rs and main.rs (use_cloud)

use dioxus::prelude::*;
use steps_protocol::cloud::{DeviceSummary, Platform};
use tokio::sync::mpsc::UnboundedSender;

use super::{CloudStatus, Command, Event, LoginStatus, Problem, SyncState};
use crate::i18n::cloud as tx;

/// Steps Cloud state for the UI, shared through context.
#[derive(Clone, Copy)]
pub struct Cloud {
    pub status: Signal<CloudStatus>,
    pub devices: Signal<Option<Result<Vec<DeviceSummary>, Problem>>>,
    pub signing_in: Signal<bool>,
    pub login: Signal<LoginStatus>,
    commands: Signal<Option<UnboundedSender<Command>>>,
}

impl Cloud {
    pub fn send(&self, command: Command) {
        if let Some(tx) = &*self.commands.peek() {
            tx.send(command).ok();
        }
    }
}

#[cfg(test)]
impl Cloud {
    /// Screen tests: the commands the screen sends arrive here, as the
    /// engine would get them.
    pub fn listen_for_tests(&self) -> tokio::sync::mpsc::UnboundedReceiver<Command> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut commands = self.commands;
        commands.set(Some(tx));
        rx
    }

    /// Screen tests: a stand-in engine. The screen's commands come out of
    /// the receiver; events sent in reach the screen as the engine's would.
    pub fn engine_for_tests(
        &self,
    ) -> (tokio::sync::mpsc::UnboundedReceiver<Command>, UnboundedSender<Event>) {
        let (commands, rx) = tokio::sync::mpsc::unbounded_channel();
        let (events_tx, events) = tokio::sync::mpsc::unbounded_channel();
        follow(*self, super::CloudHandle { commands, events }, consume_context());
        (rx, events_tx)
    }
}

/// Sends the screen's commands to the sync engine and mirrors its events
/// into the signals the screens read.
fn follow(cloud: Cloud, mut handle: super::CloudHandle, review: crate::store::Review) {
    let mut commands = cloud.commands;
    commands.set(Some(handle.commands.clone()));
    spawn(async move {
        let (mut status, mut devices, mut signing_in, mut login) =
            (cloud.status, cloud.devices, cloud.signing_in, cloud.login);
        while let Some(event) = handle.events.recv().await {
            match event {
                Event::Status(s) => {
                    if *status.peek() != s {
                        status.set(s);
                    }
                }
                Event::Devices(d) => devices.set(Some(d)),
                Event::SigningIn(on) => signing_in.set(on),
                Event::Login(l) => login.set(l),
                Event::Changed(_) => {
                    // Another device answered, judged or changed a setting.
                    let _ = crate::store::shared().observe_stored();
                    review.backend.borrow_mut().reload_and_notify();
                }
            }
        }
    });
}

/// Provides [`Cloud`] and, when this build knows a server, starts sync.
/// Call once in `App`, after `store::use_provide_review`.
pub fn use_provide_cloud() {
    let store = crate::store::shared();
    let device_id = store.device();
    let cloud = use_context_provider(|| Cloud {
        status: Signal::new(CloudStatus::not_configured(device_id.clone())),
        devices: Signal::new(None),
        signing_in: Signal::new(false),
        login: Signal::new(LoginStatus::default()),
        commands: Signal::new(None),
    });
    let review = use_context::<crate::store::Review>();
    use_hook(move || {
        let Some(config) = super::CloudConfig::from_env() else { return };
        let device = super::device_info(device_id);
        #[cfg(target_arch = "wasm32")]
        let handle = super::start(config, device);
        #[cfg(not(target_arch = "wasm32"))]
        let handle = {
            // An in-memory store (the data folder couldn't be opened) has nothing to sync.
            let Some(path) = store.path.clone() else { return };
            #[cfg(target_os = "macos")]
            let vault: Box<dyn super::session::SessionVault> = match path.parent() {
                Some(dir) => Box::new(super::session::FileVault::mac(dir)),
                None => Box::new(super::session::KeyringVault::new()),
            };
            #[cfg(not(target_os = "macos"))]
            let vault: Box<dyn super::session::SessionVault> = Box::new(super::session::KeyringVault::new());
            match super::start(config, path, device, vault) {
                Ok(h) => h,
                Err(e) => {
                    crate::diag::report!("sync didn't start: {e}");
                    return;
                }
            }
        };
        follow(cloud, handle, review.clone());
    });
    // Back in front (Android hands a backgrounded app no network): a sign-in
    // Google approved meanwhile finishes now, and the outbox goes up.
    use_future(move || async move {
        let mut eval = document::eval(
            r#"
            const send = () => { if (!document.hidden) dioxus.send(true); };
            document.addEventListener("visibilitychange", send);
            window.addEventListener("focus", send);
            window.addEventListener("pageshow", send);
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            cloud.send(Command::Kick);
        }
    });
    // Every answer and verdict is in the outbox: push it now.
    let revision = use_context::<crate::store::Review>().revision;
    use_effect(move || {
        revision.read();
        cloud.send(Command::Kick);
    });
}

fn problem_text(p: Problem) -> &'static str {
    match p {
        Problem::NotConfigured => tx::P_NOT_CONFIGURED,
        Problem::NotAllowed => tx::P_NOT_ALLOWED,
        Problem::SignInCancelled => tx::P_CANCELLED,
        Problem::SignInFailed => tx::P_FAILED,
        Problem::SignedOutRemotely => tx::P_REMOTE,
        Problem::Network => tx::P_NETWORK,
        Problem::SignInWaiting => tx::P_SIGN_IN_WAITING,
        Problem::SignInInterrupted => tx::P_INTERRUPTED,
        Problem::Server => tx::P_SERVER,
        Problem::Outdated => tx::P_OUTDATED,
        Problem::ClockOff => tx::P_CLOCK,
        Problem::WrongPassword => tx::P_WRONG_PASSWORD,
        Problem::CurrentWrong => tx::P_CURRENT_WRONG,
        Problem::Locked(_) => tx::P_LOCKED,
        Problem::UsernameTaken => tx::P_TAKEN,
        Problem::BadUsername => tx::P_BAD_USERNAME,
        Problem::ShortPassword => tx::P_SHORT_PASSWORD,
        Problem::Mismatch => tx::P_MISMATCH,
    }
}

/// The sentence for a problem, with the wait when the username is locked.
fn problem_line(p: Problem) -> String {
    match p {
        Problem::Locked(minutes) => tx::locked(minutes),
        p => problem_text(p).to_string(),
    }
}

fn state_text(s: SyncState) -> Option<&'static str> {
    match s {
        SyncState::SignedOut => None,
        SyncState::Connecting => Some(tx::CONNECTING),
        SyncState::CatchingUp => Some(tx::CATCHING_UP),
        SyncState::Live => Some(tx::LIVE),
        SyncState::Offline => Some(tx::OFFLINE),
        SyncState::Stopped => Some(tx::STOPPED),
    }
}

/// "من ٣ دقايق" with the Egyptian dual and plural forms.
pub fn ago(then_ms: u64, now_ms: u64) -> String {
    let secs = now_ms.saturating_sub(then_ms) / 1000;
    if secs < 60 {
        return tx::JUST_NOW.to_string();
    }
    use crate::i18n::count::{DAY, HOUR, MINUTE, count};
    let (n, noun) = match secs {
        s if s < 3600 => (s / 60, &MINUTE),
        s if s < 86_400 => (s / 3600, &HOUR),
        s => (s / 86_400, &DAY),
    };
    format!("{} {}", tx::AGO, count(n, noun))
}

fn now_ms() -> u64 {
    crate::store::now_ms() as u64
}

#[component]
pub fn SyncSection() -> Element {
    let cloud = use_context::<Cloud>();
    let status = cloud.status.read().clone();
    let signed_in = status.email.is_some() || status.sync != SyncState::SignedOut;

    // Load the device list when this opens, and again once connected.
    let state = use_memo(move || cloud.status.read().sync);
    use_effect(move || {
        if matches!(state(), SyncState::Live | SyncState::CatchingUp) {
            cloud.send(Command::LoadDevices);
        }
    });

    rsx! {
        section { class: "card",
            h2 { {tx::TITLE} }
            if !status.configured {
                p { class: "muted small", {tx::NOT_CONFIGURED} }
            } else if signed_in {
                SignedIn { status: status.clone() }
            } else {
                if *cloud.signing_in.read() {
                    p { class: "muted", aria_live: "polite", {tx::SIGNING_IN} }
                } else if status.finishing_sign_in {
                    p { class: "muted", aria_live: "polite", {tx::FINISHING} }
                    button { class: "btn btn-primary", onclick: move |_| cloud.send(Command::Kick), {tx::TRY_NOW} }
                } else {
                    button { class: "btn btn-primary", onclick: move |_| cloud.send(Command::SignInGoogle), {tx::SIGN_IN} }
                }
            }
            if let Some(p) = status.problem {
                p { class: "small", role: "status", {problem_line(p)} }
            }
        }
        if status.configured && !signed_in {
            PasswordSignIn {}
        }
        if signed_in {
            LoginCard {}
            DevicesCard {}
        }
    }
}

/// Signed out: username + password, for a device without Google.
#[component]
fn PasswordSignIn() -> Element {
    let cloud = use_context::<Cloud>();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut theirs = use_signal(|| false);
    let busy = *cloud.signing_in.read();
    let ready = !username.read().trim().is_empty() && !password.read().is_empty();
    let mut submit = move || {
        if !ready || busy {
            return;
        }
        cloud.send(Command::SignInPassword {
            username: username.peek().clone(),
            password: password.peek().clone(),
            remember: !*theirs.peek(),
        });
        password.set(String::new());
    };
    rsx! {
        section { class: "card pw-card",
            h2 { {tx::PW_TITLE} }
            p { class: "muted small", {tx::PW_INTRO} }
            form { class: "pw-form",
                onsubmit: move |e| {
                    e.prevent_default();
                    submit();
                },
                label { class: "field",
                    span { {tx::USERNAME} }
                    input {
                        r#type: "text", name: "username", dir: "ltr", lang: "en",
                        autocomplete: "username", autocapitalize: "none", spellcheck: "false",
                        value: "{username}",
                        oninput: move |e| username.set(e.value()),
                    }
                }
                label { class: "field",
                    span { {tx::PASSWORD} }
                    input {
                        r#type: "password", name: "password", dir: "ltr",
                        autocomplete: "current-password",
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                }
                label { class: "pw-theirs",
                    input {
                        r#type: "checkbox",
                        checked: theirs(),
                        onchange: move |e: FormEvent| theirs.set(e.checked()),
                    }
                    span {
                        {tx::NOT_MINE}
                        small { class: "muted", {tx::NOT_MINE_HINT} }
                    }
                }
                div { class: "sync-actions",
                    button { class: "btn btn-primary", r#type: "submit", disabled: !ready || busy,
                        if busy { {tx::PW_CHECKING} } else { {tx::PW_SIGN_IN} }
                    }
                }
            }
        }
    }
}

/// Signed in: the username and password this account can sign in with
/// elsewhere, set or changed here.
#[component]
fn LoginCard() -> Element {
    let cloud = use_context::<Cloud>();
    let login = cloud.login.read().clone();
    let state = use_memo(move || cloud.status.read().sync);
    use_effect(move || {
        if matches!(state(), SyncState::Live | SyncState::CatchingUp) && cloud.login.peek().username.is_none()
        {
            cloud.send(Command::LoadLogin);
        }
    });
    let current = login.username.clone().flatten();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut again = use_signal(String::new);
    // A session that signed in with the password gives the current one to
    // change or remove it (Steps Cloud checks it).
    let mut old = use_signal(String::new);
    let needs_current = login.needs_current && current.is_some();
    let given = move || if needs_current { Some(old.peek().clone()) } else { None };
    let mut mismatch = use_signal(|| false);
    // The form starts from the username the account has.
    let known = current.clone();
    use_effect(use_reactive!(|known| {
        if let Some(k) = known
            && username.peek().is_empty()
        {
            username.set(k);
        }
    }));
    let mut editing = use_signal(|| false);
    let open = editing() || (login.username == Some(None));
    let saved = login.saved;
    use_effect(use_reactive!(|saved| {
        if saved {
            password.set(String::new());
            again.set(String::new());
            old.set(String::new());
            editing.set(false);
        }
    }));
    let ready = !username.read().trim().is_empty()
        && !password.read().is_empty()
        && !again.read().is_empty()
        && (!needs_current || !old.read().is_empty());
    let current_field = rsx! {
        label { class: "field",
            span { {tx::CURRENT_PASSWORD} }
            input {
                r#type: "password", name: "current-password", dir: "ltr", autocomplete: "current-password",
                value: "{old}",
                oninput: move |e| old.set(e.value()),
            }
            span { class: "field-hint", {tx::CURRENT_HINT} }
        }
    };
    let problem = if mismatch() { Some(Problem::Mismatch) } else { login.problem };
    rsx! {
        section { class: "card pw-card",
            h2 { {tx::PW_TITLE} }
            match &login.username {
                None => rsx! { p { class: "muted", {waiting_line(state())} } },
                Some(Some(name)) => rsx! {
                    div { class: "sync-who",
                        span { class: "row-icon", aria_hidden: "true", {crate::icons::section("sync")} }
                        div { class: "row-copy",
                            small { {tx::PW_YOURS} }
                            bdi { class: "sync-email", dir: "ltr", title: "{name}", "{name}" }
                        }
                    }
                    if saved {
                        p { class: "small", role: "status", {tx::PW_SAVED} }
                    }
                    p { class: "muted small", {tx::PW_HOW} }
                },
                Some(None) => rsx! { p { class: "muted small", {tx::PW_NONE} } },
            }
            if open {
                form { class: "pw-form",
                    onsubmit: move |e| {
                        e.prevent_default();
                        if *password.peek() != *again.peek() {
                            mismatch.set(true);
                            return;
                        }
                        mismatch.set(false);
                        cloud.send(Command::SetLogin {
                            username: username.peek().clone(),
                            password: password.peek().clone(),
                            current: given(),
                        });
                    },
                    if needs_current { {current_field.clone()} }
                    label { class: "field",
                        span { {tx::USERNAME} }
                        input {
                            r#type: "text", name: "username", dir: "ltr", lang: "en",
                            autocomplete: "username", autocapitalize: "none", spellcheck: "false",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                        span { class: "field-hint", {tx::USERNAME_HINT} }
                    }
                    label { class: "field",
                        span { {tx::NEW_PASSWORD} }
                        input {
                            r#type: "password", name: "new-password", dir: "ltr", autocomplete: "new-password",
                            value: "{password}",
                            oninput: move |e| { password.set(e.value()); mismatch.set(false); },
                        }
                        span { class: "field-hint", {tx::PASSWORD_HINT} }
                    }
                    label { class: "field",
                        span { {tx::PASSWORD_AGAIN} }
                        input {
                            r#type: "password", name: "new-password-again", dir: "ltr", autocomplete: "new-password",
                            value: "{again}",
                            oninput: move |e| { again.set(e.value()); mismatch.set(false); },
                        }
                    }
                    div { class: "sync-actions",
                        if current.is_some() {
                            button { class: "btn btn-ghost", r#type: "button", onclick: move |_| editing.set(false), {tx::CANCEL} }
                        }
                        button { class: "btn btn-primary", r#type: "submit", disabled: !ready || login.saving,
                            if login.saving { {tx::PW_SAVING} } else { {tx::PW_SAVE} }
                        }
                    }
                }
            } else if current.is_some() {
                if needs_current { {current_field} }
                div { class: "sync-actions",
                    button { class: "btn btn-danger",
                        disabled: login.saving || (needs_current && old.read().is_empty()),
                        onclick: move |_| cloud.send(Command::RemoveLogin { current: given() }),
                        {tx::PW_REMOVE}
                    }
                    button { class: "btn btn-secondary", onclick: move |_| editing.set(true), {tx::PW_CHANGE} }
                }
            }
            if let Some(p) = problem {
                p { class: "small", role: "status", {problem_line(p)} }
            }
        }
    }
}

#[component]
fn SignedIn(status: CloudStatus) -> Element {
    let cloud = use_context::<Cloud>();
    let email = status.email.clone().unwrap_or_default();
    let last = status.last_sync_ms.map(|ms| ago(ms, now_ms()));
    rsx! {
        div { class: "sync-who",
            span { class: "row-icon", aria_hidden: "true", {crate::icons::section("sync")} }
            div { class: "row-copy",
                small { {tx::SIGNED_IN_AS} }
                bdi { class: "sync-email", dir: "ltr", title: "{email}", "{email}" }
            }
        }
        if status.temporary {
            p { class: "small pw-temporary", role: "note", {tx::TEMPORARY} }
        }
        p { class: "sync-state muted small", aria_live: "polite",
            if let Some(state) = state_text(status.sync) {
                span { class: "chip", "data-tone": if status.sync == SyncState::Live { "good" } else { "accent" }, {state} }
            }
            if status.pending > 0 {
                span { {tx::pending(status.pending)} }
            }
            if let Some(last) = last {
                span { "{tx::LAST}: {last}" }
            }
        }
        div { class: "sync-actions",
            button { class: "btn btn-secondary", onclick: move |_| cloud.send(Command::SignOut), {tx::SIGN_OUT} }
        }
    }
}

/// Whether Steps Cloud answers now, so a card can load from it.
fn reachable(state: SyncState) -> bool {
    matches!(state, SyncState::Live | SyncState::CatchingUp | SyncState::Connecting)
}

/// What a card that loads from Steps Cloud says before it has anything:
/// "loading" while it can, else that it waits for the server (UX #581;
/// offline it said «بيحمّل…» forever).
fn waiting_line(state: SyncState) -> &'static str {
    if reachable(state) { tx::LOADING } else { tx::WAITING_SERVER }
}

#[component]
fn DevicesCard() -> Element {
    let cloud = use_context::<Cloud>();
    let devices = cloud.devices.read().clone();
    let now = now_ms();
    let state = cloud.status.read().sync;
    rsx! {
        section { class: "card sync-devices",
            div { class: "section-head",
                h2 { {tx::DEVICES} }
                button {
                    class: "btn btn-ghost",
                    // Nothing to ask while the server can't be reached (UX #581).
                    disabled: !reachable(state),
                    onclick: move |_| cloud.send(Command::LoadDevices),
                    {crate::icons::refresh()}
                    {tx::REFRESH}
                }
            }
            match devices {
                None => rsx! { p { class: "muted", {waiting_line(state)} } },
                Some(Err(p)) => rsx! { p { class: "small", {problem_line(p)} } },
                Some(Ok(list)) => rsx! {
                    ul { class: "device-list",
                        for d in list {
                            li { key: "{d.device_id}", class: if d.revoked { "device-row revoked" } else { "device-row" },
                                span { class: "row-icon", aria_hidden: "true", {platform_icon(d.platform, &d.name)} }
                                div { class: "row-copy",
                                    span { class: "device-title",
                                        bdi { class: "device-name", dir: "auto", "{d.name}" }
                                        if d.current {
                                            span { class: "chip", "data-tone": "accent", {tx::THIS_DEVICE} }
                                        }
                                    }
                                    small {
                                        if d.revoked {
                                            {tx::DEVICE_OUT}
                                        } else if let Some(ms) = d.last_sync_ms {
                                            "{tx::LAST}: {ago(ms, now)}"
                                        } else {
                                            {tx::NEVER_SYNCED}
                                        }
                                    }
                                }
                                if !d.current && !d.revoked {
                                    div { class: "row-end",
                                        button {
                                            class: "btn btn-danger device-out",
                                            onclick: {
                                                let id = d.device_id.clone();
                                                move |_| cloud.send(Command::RevokeDevice(id.clone()))
                                            },
                                            {tx::SIGN_OUT_DEVICE}
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// Galaxy Tab model numbers start SM-X or SM-T; other Android devices are phones.
fn is_tablet(name: &str) -> bool {
    name.starts_with("SM-X") || name.starts_with("SM-T")
}

fn platform_icon(p: Platform, name: &str) -> Element {
    let d = match p {
        Platform::Android if is_tablet(name) => {
            "M6 3h12a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM11 18h2"
        }
        Platform::Android => "M8 3h8v18H8zM11 18h2",
        // laptop
        Platform::Macos => "M5 6h14v9H5zM3 18h18",
        Platform::Web => "M4 5h16v14H4zM4 9h16M7 7h.01M9.5 7h.01",
        Platform::Other => "M4 5h16v11H4zM9 20h6M12 16v4",
    };
    rsx! {
        svg {
            width: "22",
            height: "22",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn galaxy_tabs_get_the_tablet_icon() {
        assert!(is_tablet("SM-X730"));
        assert!(!is_tablet("SM-S928B"));
        assert!(!is_tablet("Mohamed’s MacBook Air"));
    }

    #[test]
    fn relative_times_use_egyptian_number_forms() {
        let now = 100 * 86_400_000;
        assert_eq!(ago(now - 10_000, now), "دلوقتي");
        assert_eq!(ago(now - MIN, now), "من دقيقة");
        assert_eq!(ago(now - 2 * MIN, now), "من دقيقتين");
        assert_eq!(ago(now - 5 * MIN, now), "من 5 دقايق");
        assert_eq!(ago(now - 25 * MIN, now), "من 25 دقيقة");
        assert_eq!(ago(now - 3 * 60 * MIN, now), "من 3 ساعات");
        assert_eq!(ago(now - 2 * 86_400_000, now), "من يومين");
    }
}
