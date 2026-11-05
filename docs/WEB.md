# Steps on the web

Steps's web app is the same Rust code as the Mac and Android apps, compiled to
WebAssembly. It follows the shape of the ArtCraft "Crafting Apps" (PdfCraft,
WordCraft…): one Rust codebase for every platform, a thin web shell, a static
site any host can serve, and CI that keeps the wasm build compiling. Steps keeps
its Dioxus screens (HTML/CSS, right-to-left) instead of their egui toolkit
(Mohamed, 2026-11-02).

## Shape

| Piece | Where | What it does |
|---|---|---|
| The app | `crates/app` with `--no-default-features --features web` | Every screen and the fonts, built in |
| Content pack | `content.pack`, next to the wasm | The same pack the Mac and Android build in, fetched alongside the wasm (see Opening fast) |
| Service worker | `web/sw.js` | Keeps the newest build on the device (see Opening fast) |
| Browser stand-ins | `crates/app/src/platform/web.rs`, `platform/time.rs` | Storage, downloads, prefs, timers |
| Web shell | `web/index.html`, `web/manifest.webmanifest`, `web/_headers` | Loads `steps.js`, installable as an app |
| Build | `scripts/web/build.sh` (`just web`, `just web-release`) | cargo → wasm-bindgen → `target/web/steps` |
| Host | `web/wrangler.toml`, `.github/workflows/web-deploy.yml` | A static-assets Worker on the free plan, same account as Steps Cloud |
| CI gate | `nightly.yml` › "Web app compiles (wasm32)" | clippy for wasm32, nightly and by hand |

## What differs in the browser

| Area | Mac / Android | Web |
|---|---|---|
| Database | SQLite file | The same SQLite, compiled to wasm, in memory; its bytes are saved in IndexedDB 1.5 s after a change and when the tab is hidden or closed |
| Two tabs | n/a | The newest tab owns the data; an older one stops saving and says so on Today |
| Device prefs | `device-prefs.txt` | `localStorage` |
| Exports (backup, checks, Anki) | A file in Downloads / Save to | A browser download |
| Restore | File picker / found files | File picker or paste |
| Read aloud | System voice | The browser's voice (Web Speech) |
| Updates | Over the air | Each deploy downloads in the background and is used from the next open |
| Sync (Steps Cloud) | Live, WebSocket with a Bearer header | Live, the browser's `fetch` and `WebSocket`; see Sync below |
| Sign-in | Google, desktop client (code + PKCE) | Google by redirect, Web client (ID token); session in `localStorage` |
| Reminders | System notifications | Not on the web; the phone or Mac rings |

Nothing is dropped: everything else on the Mac and Android also runs in the browser.

## Opening fast

The first version shipped one 17 MB wasm with the content pack inside, and
most merges change the content, so a phone re-downloaded all of it after
nearly every merge and took 9–19 s to open (simulated phone, 2026-11-04).
Now:

| What | How |
|---|---|
| Smaller code | `[profile.web-release]`: `opt-level = "z"`, fat LTO, then `wasm-opt -Oz` (19 → 8.5 MB before wasm-opt) |
| Content apart from code | On wasm32, `build.rs` doesn't embed the pack; `scripts/web/build.sh` copies it from steps-app's `OUT_DIR` (named in cargo's JSON messages) to `content.pack`. `index.html` starts fetching it alongside the wasm and `platform::web::boot` waits for it, so a content merge doesn't re-send the code |
| Opens from the device | `web/sw.js`, filled in by `build.sh` with the build id (a hash of every file) and the file list. It keeps one build per cache (`steps-<id>`), so `steps.js` and its wasm always match; answers this origin's GETs from it (every navigation is `./`); a new deploy installs in the background and the old cache goes once it activates. Steps Cloud never goes through it |

Simulated phone (4× slower CPU): the first visit downloads everything
(~8 s on 30 Mbps); every open after that takes ~3.5 s, whatever the network.
A new deploy shows on the open after the one that fetched it.

## Sync

The web app syncs through the same Steps Cloud as the Mac and Android, with
the same engine (`cloud/engine.rs`); only the transport differs.

| Piece | Mac / Android | Web |
|---|---|---|
| API calls | `ureq` on a blocking thread | `fetch` (`platform/web.rs`) |
| Live socket | `tokio-tungstenite`, token in the `authorization` header | Browser `WebSocket` (`cloud/socket.rs`); browsers can't set headers, so the token rides in `?access_token=`, which the hub accepts on `/v1/sync` only |
| Keep-alive | Protocol ping | An app-level `Ping` message |
| Google sign-in | `cloud/oauth.rs`: loopback / deep link, code + PKCE | `cloud/oauth_web.rs`: the page goes to Google and comes back with an ID token in the `#` fragment (`response_type=id_token`, checked `state`); the fragment is cleared at once |
| Session | Mac: private file in the data folder; Android: Keystore | `localStorage` (`cloud-session`), like any web app's session |
| Device kind | `mac`, `android` | `web` (older apps read it as "other") |

Steps Cloud answers the browser only from the origins in `WEB_ORIGINS`
(`crates/server/wrangler.toml`): CORS headers and preflights go to those and
nothing else.

### Google Web client (once)

Google sign-in needs a **Web application** OAuth client in the same Google
Cloud project as the desktop one:

- Authorized JavaScript origin: `https://steps-web.hamorefaat85.workers.dev`
- Authorized redirect URI: `https://steps-web.hamorefaat85.workers.dev/`

It exists since 2026-11-05: Google Cloud project "Steps" (`steps-513803`,
Testing mode), next to the desktop client. Its client ID is public (it is in
every sign-in URL), and the web app keeps no secret. The repo variable
`STEPS_WEB_GOOGLE_CLIENT_ID` holds it; `web-deploy.yml` passes it to
`scripts/web/build.sh`, which builds it in (without it the web app hides
sign-in; `WEB_GOOGLE_CLIENT_ID` in `crates/app/src/cloud/mod.rs` is the
fallback), and the hub accepts
its ID tokens once it is in the repo variable `STEPS_GOOGLE_CLIENT_IDS` (or
`STEPS_WEB_GOOGLE_CLIENT_ID`), which the deploy passes to the Worker
([CLOUD.md](CLOUD.md) §3.2). As with the desktop client, only the test users on the
consent screen can sign in.

## Run it

```sh
cargo install wasm-bindgen-cli@<version in Cargo.lock> --locked   # once
just web          # dev build, http://localhost:8080
just web-release  # what Cloudflare serves (wasm-opt runs if installed)
```
