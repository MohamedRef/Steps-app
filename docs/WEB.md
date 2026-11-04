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
| The app | `crates/app` with `--no-default-features --features web` | Every screen, the content pack and the fonts, built in |
| Browser stand-ins | `crates/app/src/platform/web.rs`, `platform/time.rs` | Storage, downloads, prefs, timers |
| Web shell | `web/index.html`, `web/manifest.webmanifest`, `web/_headers` | Loads `steps.js`, installable as an app |
| Build | `scripts/web/build.sh` (`just web`, `just web-release`) | cargo → wasm-bindgen → `target/web/steps` |
| Host | `web/wrangler.toml`, `.github/workflows/web-deploy.yml` | A static-assets Worker on the free plan, same account as Steps Cloud |
| CI gate | `ci.yml` › "Web app compiles (wasm32)" | clippy for wasm32, nightly and by hand |

## What differs in the browser

| Area | Mac / Android | Web |
|---|---|---|
| Database | SQLite file | The same SQLite, compiled to wasm, in memory; its bytes are saved in IndexedDB 1.5 s after a change and when the tab is hidden or closed |
| Two tabs | n/a | The newest tab owns the data; an older one stops saving and says so on Today |
| Device prefs | `device-prefs.txt` | `localStorage` |
| Exports (backup, checks, Anki) | A file in Downloads / Save to | A browser download |
| Restore | File picker / found files | File picker or paste |
| Read aloud | System voice | The browser's voice (Web Speech) |
| Updates | Over the air | Each deploy is the newest build; reload to get it |
| Sync (Steps Cloud) | Live, WebSocket with a Bearer header | Live, the browser's `fetch` and `WebSocket`; see Sync below |
| Sign-in | Google, desktop client (code + PKCE) | Google by redirect, Web client (ID token); session in `localStorage` |
| Reminders | System notifications | Not on the web; the phone or Mac rings |

Nothing is dropped: everything else on the Mac and Android also runs in the browser.

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

It exists ("Steps web", created 2026-11-02, same project as Steps Desktop).
Its client ID is public (it is in every sign-in URL), and the web app keeps no
secret. It is `WEB_GOOGLE_CLIENT_ID` in `crates/app/src/cloud/mod.rs` and the
second entry of `GOOGLE_CLIENT_IDS` in `crates/server/wrangler.toml`, so the hub
accepts its ID tokens. As with the desktop client, only the test users on the
consent screen can sign in.

## Run it

```sh
cargo install wasm-bindgen-cli@<version in Cargo.lock> --locked   # once
just web          # dev build, http://localhost:8080
just web-release  # what Cloudflare serves (wasm-opt runs if installed)
```
