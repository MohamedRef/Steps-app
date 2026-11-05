# Steps Cloud: sync between the Mac, the phone and the tablet

Steps Cloud is a small Rust Cloudflare Worker (`crates/server`) on the **Workers Free** plan. It is Steps's own: its own Worker name (`steps-cloud`), its own Durable Object storage, its own Google OAuth client and its own secrets. Nothing is shared with another app's cloud. Parts of the code are adapted from Batna's `crates/server` and `crates/app/src/cloud` (credited in each file).

It does three things:

1. **Sign-in**: with Google, or with a username and password on a device that doesn't have your Gmail (§4.1). Google ID tokens are checked against an allowlist (your Gmail only). Either way Steps issues its own tokens: a 15-minute access token and a 60-day refresh token that rotates on every use.
2. **Devices**: lists every signed-in device with its last sync, and signs one out remotely (its socket closes at once).
3. **Live sync**: one Durable Object per account is the WebSocket hub (hibernation, so idle sockets cost nothing). It keeps the op log; your reviews and settings arrive on the other devices within seconds.

Cost: **$0**. Nothing here needs a card on file. There is no D1 database and no R2.

## 1. What syncs

| Table | Rule | What it holds |
|---|---|---|
| `review_events` | append-only (a delete is a tombstone) | every review: item, grade, time, and the optional confidence (sure / unsure / guess) and error reason |
| `item_status` | last writer wins (HLC) | your ✓ / ✎ / ✗ content checks |
| `settings` | last writer wins (HLC) | retention, new items a day and other study settings; lesson marks; where each unfinished lesson stands (`lessonpos.<id>`: step, answers, notes, score) |
| `own_cards` | last writer wins (HLC, a delete is a tombstone) | the cards you write yourself |
| `self_assessments` | last writer wins (HLC, a delete is a tombstone) | your NBME / UWSA / Free 120 scores |

Places (`Store::put_place`) sync too, so a new device opens a lesson at the step you left; a place too big for one op stays on its device. A row written again before it went up keeps only its newest op in the outbox, so a lesson's place costs one op per push however many answers. The open timed bank paper stays on its device on purpose: two devices restoring the same paper could count its answers twice.

The list lives in `steps_protocol::sync::TABLES`. **A new synced table must be added there**, or the hub rejects its ops as `unknown_table`. A device only sends rows it wrote itself (its own `device_id`), and a clock more than 5 minutes ahead is rejected.

## 2. Layout

| Piece | Where |
|---|---|
| Worker routes | `crates/server/src/wasm/mod.rs` |
| Durable Object `Login`, one per username (password sign-in) | `crates/server/src/wasm/login.rs`, `src/login.rs` |
| Durable Object `Account` (devices + op log in DO SQLite) | `crates/server/src/wasm/account.rs`, `src/account.rs`, `src/devices.rs` |
| Sync hub logic (tested natively against SQLite) | `crates/server/src/hub.rs` |
| Tokens, Google ID-token check | `crates/server/src/tokens.rs`, `src/google.rs` |
| Wire types, merge rules, client state machine | `crates/protocol/src/{sync,cloud,merge,client}.rs` |
| App engine (sign-in, session vault, WebSocket, backoff) | `crates/app/src/cloud/` |
| Outbox and remote merge | `crates/app/src/store/sync.rs` (`sync_outbox`, migration 2) |
| Settings screen | Settings › المزامنة (`crates/app/src/cloud/ui.rs`) |

## 3. Setup (once, about 15 minutes)

### 3.1 Cloudflare
1. Use your free Cloudflare account (or create one at <https://dash.cloudflare.com/sign-up>; no card).
2. On the Mac:
   ```bash
   just cloud-tools      # wrangler 4 + worker-build
   wrangler login        # opens the browser
   just cloud-setup      # token signing key + your Gmail on the allowlist (Worker secrets)
   ```

### 3.2 Google sign-in (Google Cloud Console, free)
1. <https://console.cloud.google.com/> → create a **new** project "Steps" (not another app's).
2. **Google Auth Platform → Branding/Audience**: External, keep it in *Testing*, add your Gmail as a test user.
3. **Clients → Create client → Desktop app** "Steps desktop". The Mac and Android both use it (browser sign-in with PKCE and a loopback redirect). On Android the redirect lands while Chrome is in front and Android keeps a backgrounded Steps off the network, so the app only waits for the code there; the code exchange and the Steps Cloud sign-in run once Steps is back in front (the page's "ارجع لـ Steps" button opens `app.steps://signed-in`), retrying for up to nine minutes.
4. Put its client id in the repo variable **`STEPS_GOOGLE_CLIENT_IDS`** (Settings › Secrets and variables › Actions › Variables), with the web client's id after a comma once there is one ([WEB.md](WEB.md) § Sync): `1234-desktop.apps.googleusercontent.com,5678-web.apps.googleusercontent.com`. The ids are not secret. The deploy passes them to the Worker (`wrangler deploy --var GOOGLE_CLIENT_IDS:…`); `crates/server/wrangler.toml` keeps the value empty, and a Cloudflare dashboard edit would be overwritten by the next deploy. Without the variable the deploy fails and says so, because Google sign-in can't work (#102). Put the desktop id in `STEPS_GOOGLE_CLIENT_ID` too: the preview and release builds build it into the apps (§3.5). For `just cloud-deploy` from the Mac, set it in the Cloudflare dashboard or pass `--var` yourself.

### 3.3 Deploy
**From GitHub (no Mac needed).** The `Deploy Steps Cloud` workflow builds and deploys the Worker.
1. In Cloudflare, open **Workers & Pages** once so the account gets its `workers.dev` subdomain.
2. At **My Profile › API Tokens**, create a token from the **Edit Cloudflare Workers** template.
3. Add repo secrets (**Settings › Secrets and variables › Actions**):
   - `CLOUDFLARE_API_TOKEN`: the token.
   - `STEPS_ALLOWED_EMAILS`: your Gmail.
4. Run **Actions › Deploy Steps Cloud › Run workflow**. The run summary shows the Worker URL.

The first run makes the token signing key inside Cloudflare, so `just cloud-setup` isn't needed. After that, the hub redeploys when CI passes on `main` and `crates/server` or `crates/protocol` changed since the last deploy (a commit that fails CI never reaches it). Each deploy then checks that sign-in answers on the live hub.

**From the Mac:**
```bash
just cloud-deploy
curl https://steps-cloud.<your-subdomain>.workers.dev/v1/health
```

Deploy the hub from `main` at PR #134 or later. An older hub rejects the `relay` ops that carry a restored backup to the other devices (as `wrong_device`), and the app then drops them for good. Whenever `steps_protocol::sync` changes, redeploy the hub before installing the new app. Refresh tokens rotate on use: an old one keeps working until the device presents its replacement, so a lost response or a failed session save never signs a device out, and a token that comes back after its replacement was used signs that device out. A signed-out device can't list or sign out the others, even while its access token lasts.

### 3.4 Connect the app
1. `cp .env.example .env` and fill in `STEPS_CLOUD_URL` (the URL above) and the Desktop client's id and secret. `.env` is git-ignored; `just` loads it and the app reads it at build time.
2. `just run` → **Settings › المزامنة › سجّل دخول بجوجل**. The browser opens, you pick your Gmail, and the page says to go back to Steps.
3. Build the APK the same way (`.env` in place) and sign in on the S24 Ultra and the Tab S11.
4. Review a card on one device: the review shows on the others within seconds. Until you sign in, everything stays on the device, exactly as before.

### 3.5 The preview APK (GitHub Actions)
The auto-updating preview APK is built on GitHub, where there is no `.env`. Give it the same values once, in the repo's **Settings › Secrets and variables › Actions**:

| Kind | Name | Value |
|---|---|---|
| Variable | `STEPS_GOOGLE_CLIENT_ID` | the Desktop client's id |
| Secret | `STEPS_GOOGLE_CLIENT_SECRET` | the Desktop client's secret |

`steps-android-preview.yml` passes them to the nightly and hand-started builds of `main` only; a PR's build gets none and its APK hides sign-in. The next preview build (and the update it ships) can sign in. GitHub masks the secret in logs, and nothing writes it to the repo. Like every installed-app secret, it can be read out of the APK. That is only acceptable because the client is a **Desktop app** (installed-app) OAuth client, which Google treats as public: its "secret" grants nothing without the user's own consent and PKCE. Never put a Web-application client's secret here, because that one is truly confidential.

## 4. API

All JSON. Authenticated routes take `Authorization: Bearer <access_token>`. Errors are `{ "code": "...", "message": "..." }` (`ErrorCode` in `crates/protocol/src/cloud.rs`).

| Route | What |
|---|---|
| `GET /v1/health` | `{ name, protocol }` |
| `POST /v1/auth/google` | `{ id_token, device }` → `Session` |
| `POST /v1/auth/password` | `{ username, password_key, device }` → `Session` (§4.1) |
| `GET /v1/login` | `{ username }` (or `null`) for this account |
| `POST /v1/login` | `{ username, password_key }`: set or change; → `{ username }` |
| `DELETE /v1/login` | turn password sign-in off; → `{ username: null }` |
| `POST /v1/auth/refresh` | `{ refresh_token }` → new `Session` (the old refresh token stops working) |
| `POST /v1/auth/signout` | signs this device out |
| `GET /v1/devices` | `{ devices: [DeviceSummary] }` |
| `DELETE /v1/devices/{id}` | remote sign-out |
| `GET /v1/sync` | WebSocket; messages in `crates/protocol/src/sync.rs` |

Sync in one paragraph: the device sends `hello { last_seq }`, gets the missing ops in pages and `caught_up`, then pushes its outbox as `push { batch, ops ≤ 50 }` and gets an `ack` listing what can leave the outbox. Other devices' ops arrive as `ops` the moment they're stored. Merging is idempotent, so reading the log again never doubles a review.

### 4.1 Username and password

For a borrowed laptop or a library computer, where your Gmail isn't. Set a username and password once from a signed-in device (Settings › المزامنة › «الدخول باسم مستخدم وباسورد»); then on any device, signed out, type them in the same screen. It is the same account as Google, so everything syncs down, including where each lesson stands. Tick «الجهاز ده مش بتاعي» on someone else's device: the session then lives only in that browser tab (`sessionStorage`) or, in the Mac and Android apps, in memory, and ends when it closes. Sign out there when you're done, or from your phone's «أجهزتك». A session that signed in with the password can change or turn off the password only by giving the current one (the Worker checks it against the username's Login object, so wrong ones count towards its lock). A Google-signed-in device needs none, so a forgotten password is reset from the phone. Changing or turning off the password signs out every other device that signed in with it (#540), and devices from before the hub kept track count as password ones (#541).

How the password is kept safe:

- **It never leaves the device.** The app stretches it with Argon2id (19 MiB, 2 passes, 1 lane; salt `steps-login-v1:<username>`; `steps_protocol::cloud::PASSWORD_PARAMS`) and sends only the 32-byte result. The Worker's 10 ms CPU budget can't run a slow hash; the device can (about a quarter of a second). Changing these parameters changes every key, so they are pinned by a test.
- **The server keeps no key.** The username's `Login` object stores a random salt and `SHA-256(salt, key)`, compared in constant time. A copy of its storage still costs the full Argon2id work per guess.
- **Wrong passwords lock the username**: 5 in a row lock it for a minute, each further one doubles the lock, up to an hour (about 24 guesses a day at most). A wrong password and an unknown username get the same answer (`wrong_password`); a lock answers `too_many_tries` with `retry_after_ms`. Setting a new password lifts a lock.
- Usernames are 3–32 of `a-z 0-9 . _ -`, matched case-insensitively. Passwords are at least 10 characters.
- The allowlist still applies: the username carries the Google account's email, checked on every sign-in and refresh.

## 5. Local development

```bash
just cloud-dev    # http://127.0.0.1:8787 (local workerd)
just cloud-logs   # live logs from the deployed Worker
cargo test -p steps-server -p steps-protocol   # hub, tokens, devices and merge rules, no network
```

## 6. Privacy and limits

- Only your reviews, checks and settings leave the device. No patient data, no book text.
- A phone moved with Smart Switch or another copy of the app's data is a new device to the hub. Android backup is off in the manifest, and on Android 12+, where a device-to-device transfer ignores that, the app notices it was copied (a marker in `no_backup/` is missing) and takes a new `device_id` with an empty outbox (`crates/app/src/store/install.rs`). Its reviews are already on the hub, so it simply syncs as a new device. Sign in again on the new phone.
- Secrets (`TOKEN_SIGNING_KEY`, `ALLOWED_EMAILS`) are Worker secrets, never in git.
- Free-plan limits (Workers 100,000 requests/day, Durable Objects 100,000 requests/day) are far above one learner's use. The Free plan has no card on file, so Cloudflare pauses at a limit instead of billing.
- On Linux (development and CI only) the session is kept in memory, so you sign in on every launch. Android keeps it in the Keystore. The Mac keeps it in `cloud-session.json` in Steps's data folder, readable by your account only: the self-signed Steps.app has no Apple team ID, so the Keychain trusted only the exact build that saved the session and asked for the login password after every new build, every 15 minutes (each token refresh). The first launch with this change moves a Keychain session over, so the Keychain may ask one last time.
