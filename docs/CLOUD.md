# Steps Cloud: sync between the Mac, the phone and the tablet

Steps Cloud is a small Rust Cloudflare Worker (`crates/server`) on the **Workers Free** plan. It is Steps's own: its own Worker name (`steps-cloud`), its own Durable Object storage, its own Google OAuth client and its own secrets. Nothing is shared with Kabar Cloud. Parts of the code are adapted from Kabar's `crates/server` and `crates/app/src/cloud` (credited in each file).

It does three things:

1. **Sign-in**: checks your Google ID token against an allowlist (your Gmail only) and issues Steps tokens: a 15-minute access token and a 60-day refresh token that rotates on every use.
2. **Devices**: lists every signed-in device with its last sync, and signs one out remotely (its socket closes at once).
3. **Live sync**: one Durable Object per account is the WebSocket hub (hibernation, so idle sockets cost nothing). It keeps the op log; your reviews and settings arrive on the other devices within seconds.

Cost: **$0**. Nothing here needs a card on file. There is no D1 database, no R2 and no AI.

## 1. What syncs

| Table | Rule | What it holds |
|---|---|---|
| `review_events` | append-only (a delete is a tombstone) | every review: item, grade, time, and the optional confidence (sure / unsure / guess) and error reason |
| `item_status` | last writer wins (HLC) | your ✓ / ✎ / ✗ content checks |
| `settings` | last writer wins (HLC) | retention, new items a day and other study settings |

The list lives in `steps_protocol::sync::TABLES`. **A new synced table must be added there**, or the hub rejects its ops as `unknown_table`. A device only sends rows it wrote itself (its own `device_id`), and a clock more than 5 minutes ahead is rejected.

## 2. Layout

| Piece | Where |
|---|---|
| Worker routes | `crates/server/src/wasm/mod.rs` |
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
1. <https://console.cloud.google.com/> → create a **new** project "Steps" (not Kabar's).
2. **Google Auth Platform → Branding/Audience**: External, keep it in *Testing*, add your Gmail as a test user.
3. **Clients → Create client → Desktop app** "Steps desktop". The Mac and Android both use it (browser sign-in with PKCE and a loopback redirect). On Android the redirect lands while Chrome is in front and Android keeps a backgrounded Steps off the network, so the app only waits for the code there; the code exchange and the Steps Cloud sign-in run once Steps is back in front (the page's "ارجع لـ Steps" button opens `app.steps://signed-in`), retrying for up to nine minutes.
4. Put its client id in `crates/server/wrangler.toml`:
   ```toml
   GOOGLE_CLIENT_IDS = "1234-desktop.apps.googleusercontent.com"
   ```
   The id is not a secret; commit it.

### 3.3 Deploy
**From GitHub (no Mac needed).** The `Deploy Steps Cloud` workflow builds and deploys the Worker.
1. In Cloudflare, open **Workers & Pages** once so the account gets its `workers.dev` subdomain.
2. At **My Profile › API Tokens**, create a token from the **Edit Cloudflare Workers** template.
3. Add repo secrets (**Settings › Secrets and variables › Actions**):
   - `CLOUDFLARE_API_TOKEN`: the token.
   - `STEPS_ALLOWED_EMAILS`: your Gmail.
4. Run **Actions › Deploy Steps Cloud › Run workflow**. The run summary shows the Worker URL.

The first run makes the token signing key inside Cloudflare, so `just cloud-setup` isn't needed. After that, every push to `main` that touches `crates/server` or `crates/protocol` redeploys the hub.

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
| Secret | `STEPS_GOOGLE_CLIENT_SECRET` | the Desktop client's secret |

`steps-android-preview.yml` passes them to the build on pushes to `main` only; a PR's build gets none and its APK hides sign-in. The next preview build (and the update it ships) can sign in. GitHub masks the secret in logs, and nothing writes it to the repo. Like every installed-app secret, it can be read out of the APK. That is only acceptable because the client is a **Desktop app** (installed-app) OAuth client, which Google treats as public: its "secret" grants nothing without the user's own consent and PKCE. Never put a Web-application client's secret here, because that one is truly confidential.

## 4. API

All JSON. Authenticated routes take `Authorization: Bearer <access_token>`. Errors are `{ "code": "...", "message": "..." }` (`ErrorCode` in `crates/protocol/src/cloud.rs`).

| Route | What |
|---|---|
| `GET /v1/health` | `{ name, protocol }` |
| `POST /v1/auth/google` | `{ id_token, device }` → `Session` |
| `POST /v1/auth/refresh` | `{ refresh_token }` → new `Session` (the old refresh token stops working) |
| `POST /v1/auth/signout` | signs this device out |
| `GET /v1/devices` | `{ devices: [DeviceSummary] }` |
| `DELETE /v1/devices/{id}` | remote sign-out |
| `GET /v1/sync` | WebSocket; messages in `crates/protocol/src/sync.rs` |

Sync in one paragraph: the device sends `hello { last_seq }`, gets the missing ops in pages and `caught_up`, then pushes its outbox as `push { batch, ops ≤ 50 }` and gets an `ack` listing what can leave the outbox. Other devices' ops arrive as `ops` the moment they're stored. Merging is idempotent, so reading the log again never doubles a review.

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
