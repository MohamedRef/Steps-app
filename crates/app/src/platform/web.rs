//! The web build (`web/`, wasm32): what the browser stands in for.
//!
//! - **The database.** SQLite runs in wasm, in memory. Its bytes are kept in
//!   IndexedDB (`steps` › `files` › `steps.db`): loaded once before the app
//!   starts, and written back a moment after anything changes, and when the
//!   tab is hidden or closed. See docs/WEB.md.
//! - **One tab writes.** The newest tab owns the database; an older one stops
//!   saving and says so, so two tabs never overwrite each other's work.
//! - **Device prefs** live in `localStorage` (`prefs/storage.rs`).
//! - **Exports** are browser downloads.
//! - **The content pack** is its own file, `content.pack`, which
//!   `index.html` starts fetching while the wasm downloads; `boot` waits for
//!   it. A service worker (`web/sw.js`) keeps every file of the newest build,
//!   so the app opens from the device and a new deploy arrives in the
//!   background, ready for the next open.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
const DB = "steps", STORE = "files";
function open() {
  return new Promise((ok, fail) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => ok(req.result);
    req.onerror = () => fail(req.error);
  });
}
export async function idb_get(key) {
  const db = await open();
  return await new Promise((ok, fail) => {
    const req = db.transaction(STORE, "readonly").objectStore(STORE).get(key);
    req.onsuccess = () => ok(req.result instanceof Uint8Array ? req.result : null);
    req.onerror = () => fail(req.error);
  });
}
export async function idb_put(key, bytes) {
  // `bytes` is a view into wasm memory: copy it before anything awaits (memory
  // may grow and detach it), and so IndexedDB stores only these bytes, not the
  // whole memory the view sits in.
  const copy = bytes.slice();
  const db = await open();
  await new Promise((ok, fail) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(copy, key);
    tx.oncomplete = () => ok();
    tx.onerror = () => fail(tx.error);
    tx.onabort = () => fail(tx.error);
  });
}
export function content_pack() {
  return globalThis.stepsPack ?? Promise.reject(new Error("content.pack wasn't requested"));
}
export function boot_failed(msg) {
  const img = document.querySelector('#boot img');
  if (img) img.style.animation = "none";
  const p = document.getElementById("boot-msg");
  if (p) p.textContent = msg;
}
export function persist() {
  if (navigator.storage && navigator.storage.persist) navigator.storage.persist();
}
export function download(name, mime, bytes) {
  const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
  const a = document.createElement("a");
  a.href = url; a.download = name; a.style.display = "none";
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}
export async function http(method, url, bearer, body) {
  const headers = {};
  if (bearer) headers["authorization"] = bearer;
  if (body !== undefined && body !== null) headers["content-type"] = "application/json";
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), 20000);
  try {
    const res = await fetch(url, { method, headers, body: body ?? undefined, signal: ctl.signal });
    return [res.status, await res.text()];
  } finally { clearTimeout(timer); }
}
export function browser_name() {
  const ua = navigator.userAgent;
  const browser = /Edg\//.test(ua) ? "Edge" : /Firefox\//.test(ua) ? "Firefox"
    : /Chrome\//.test(ua) ? "Chrome" : /Safari\//.test(ua) ? "Safari" : "Browser";
  const os = /Android/.test(ua) ? "Android" : /iPhone|iPad/.test(ua) ? "iOS"
    : /Mac OS X/.test(ua) ? "Mac" : /Windows/.test(ua) ? "Windows" : /Linux/.test(ua) ? "Linux" : "";
  return os ? `${browser} on ${os}` : browser;
}
export function ls_get(key) { try { return localStorage.getItem(key); } catch (_) { return null; } }
export function ls_set(key, value) {
  try { localStorage.setItem(key, value); return true; } catch (_) { return false; }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn idb_get(key: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn idb_put(key: &str, bytes: &[u8]) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = content_pack)]
    async fn js_content_pack() -> Result<JsValue, JsValue>;
    fn boot_failed(msg: &str);
    fn persist();
    #[wasm_bindgen(js_name = download)]
    fn js_download(name: &str, mime: &str, bytes: &[u8]);
    #[wasm_bindgen(catch, js_name = http)]
    async fn js_http(
        method: &str,
        url: &str,
        bearer: Option<String>,
        body: Option<String>,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = browser_name)]
    fn js_browser_name() -> String;
    fn ls_get(key: &str) -> Option<String>;
    fn ls_set(key: &str, value: &str) -> bool;
}

const DB_KEY: &str = "steps.db";
/// Which tab owns the database (`localStorage`).
const OWNER_KEY: &str = "steps:tab";

/// The content pack `boot` fetched, kept for the app's life (the library
/// borrows it and reloads from it).
static PACK: std::sync::OnceLock<&'static [u8]> = std::sync::OnceLock::new();

thread_local! {
    static SNAPSHOT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    static SAVING: Cell<bool> = const { Cell::new(true) };
    /// SQLite's change count at the last save.
    static SAVED_AT: Cell<Option<u64>> = const { Cell::new(None) };
    static TAB: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Before the app starts: waits for the content pack, reads the kept
/// database and claims this tab. Without the pack nothing can show, so it
/// says why on the boot screen and returns false. A browser that can't read
/// IndexedDB starts empty and doesn't save, so it never overwrites what it
/// couldn't read.
pub async fn boot() -> bool {
    match js_content_pack().await {
        Ok(v) => {
            let bytes: &'static [u8] = Box::leak(js_sys::Uint8Array::new(&v).to_vec().into_boxed_slice());
            let _ = PACK.set(bytes);
        }
        Err(e) => {
            let why =
                e.as_string().or_else(|| js_sys::Error::from(e).message().as_string()).unwrap_or_default();
            boot_failed(&format!("{} ({why})", crate::i18n::web::NO_CONTENT));
            return false;
        }
    }
    persist();
    // Google's answer to a sign-in comes back in the address (`cloud::oauth`).
    crate::cloud::oauth::catch_redirect();
    let tab = format!("{:x}", js_sys::Date::now() as u64 ^ (js_sys::Math::random() * 1e12) as u64);
    ls_set(OWNER_KEY, &tab);
    TAB.with(|t| *t.borrow_mut() = tab);
    match idb_get(DB_KEY).await {
        Ok(v) if v.is_null() || v.is_undefined() => {}
        Ok(v) => SNAPSHOT.with(|s| *s.borrow_mut() = Some(js_sys::Uint8Array::new(&v).to_vec())),
        Err(e) => {
            stop_saving();
            crate::store::write_failed("browser storage", &format!("{e:?}"));
        }
    }
    true
}

/// The content pack `boot` fetched.
pub fn content_pack() -> &'static [u8] {
    PACK.get().copied().unwrap_or_default()
}

/// The database bytes `boot` found (empty for a first visit). Once.
pub fn take_snapshot() -> Vec<u8> {
    SNAPSHOT.with(|s| s.borrow_mut().take()).unwrap_or_default()
}

pub fn stop_saving() {
    SAVING.with(|s| s.set(false));
}

fn owns_tab() -> bool {
    TAB.with(|t| ls_get(OWNER_KEY).is_none_or(|owner| owner == *t.borrow()))
}

/// Writes the database to IndexedDB when it changed since the last save.
pub async fn save_now() {
    if !SAVING.with(Cell::get) {
        return;
    }
    if !owns_tab() {
        stop_saving();
        crate::store::write_failed("browser storage", &OTHER_TAB);
        return;
    }
    let store = crate::store::shared();
    if SAVED_AT.with(Cell::get) == Some(store.changes()) {
        return;
    }
    match store.snapshot() {
        Ok((changes, bytes)) => match idb_put(DB_KEY, &bytes).await {
            Ok(_) => {
                SAVED_AT.with(|s| s.set(Some(changes)));
                crate::store::write_recovered("browser storage");
            }
            Err(e) => crate::store::write_failed("browser storage", &format!("{e:?}")),
        },
        Err(e) => crate::store::write_failed("browser storage", &e),
    }
}

/// Shown on Today when another tab took over.
const OTHER_TAB: &str = "Steps is open in another tab; reload this one to keep studying here";

/// Keeps the database saved: shortly after changes, and when the tab is
/// hidden or closed. Call once in `App`.
pub fn use_saver() {
    use_future(|| async {
        // The first visit saves at once, so the device id sticks.
        save_now().await;
        loop {
            crate::platform::time::sleep(Duration::from_millis(1500)).await;
            save_now().await;
        }
    });
    use_future(|| async {
        let mut eval = document::eval(
            r#"
            const send = () => dioxus.send(true);
            document.addEventListener("visibilitychange", () => { if (document.hidden) send(); });
            window.addEventListener("pagehide", send);
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<bool>().await.is_ok() {
            save_now().await;
        }
    });
}

/// One HTTP request (Steps Cloud's API): the status and the body text, or
/// why it never got an answer.
pub async fn fetch(
    method: &str,
    url: &str,
    bearer: Option<&str>,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let pair = js_http(method, url, bearer.map(String::from), body.map(String::from))
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    let pair = js_sys::Array::from(&pair);
    let status = pair.get(0).as_f64().unwrap_or_default() as u16;
    Ok((status, pair.get(1).as_string().unwrap_or_default()))
}

/// "Chrome on Mac": what the Devices list calls this browser.
pub fn browser_name() -> String {
    js_browser_name()
}

/// Hands `bytes` to the browser as a download called `name`.
pub fn download(name: &str, mime: &str, bytes: &[u8]) -> Result<String, String> {
    js_download(name, mime, bytes);
    Ok(name.to_string())
}

/// Device prefs (`prefs/storage.rs`).
pub fn read_pref(file: &str) -> Option<String> {
    ls_get(&format!("steps:{file}"))
}

pub fn write_pref(file: &str, text: &str) -> bool {
    ls_set(&format!("steps:{file}"), text)
}
