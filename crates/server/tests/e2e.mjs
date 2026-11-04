// Local Steps Cloud integration check: fake Google, two devices, auth,
// refresh, WebSocket sync, and remote sign-out. No real Google calls.
// Run: node crates/server/tests/e2e.mjs (Node 22+, wrangler 4, worker-build).
import { spawn } from "node:child_process";
import { generateKeyPairSync, createSign, randomBytes } from "node:crypto";
import { createServer } from "node:http";
import { writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const serverDir = join(here, "..");
const PORT = 8787;
const KEY_PORT = 8790;
const BASE = `http://127.0.0.1:${PORT}`;
const CLIENT_ID = "steps-e2e.apps.googleusercontent.com";
const EMAIL = "learner@example.com";
const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const jwk = { ...publicKey.export({ format: "jwk" }), kid: "local-key", alg: "RS256", use: "sig" };
const b64 = (v) => Buffer.from(typeof v === "string" ? v : JSON.stringify(v)).toString("base64url");
function idToken(email) {
  const now = Math.floor(Date.now() / 1000);
  const data = `${b64({ alg: "RS256", kid: "local-key" })}.${b64({
    iss: "https://accounts.google.com", aud: CLIENT_ID, sub: "110169484474386276334",
    email, email_verified: true, iat: now, exp: now + 3600,
  })}`;
  return `${data}.${createSign("RSA-SHA256").update(data).sign(privateKey).toString("base64url")}`;
}
const jwksServer = createServer((_req, res) => {
  res.writeHead(200, { "content-type": "application/json", "cache-control": "max-age=60" });
  res.end(JSON.stringify({ keys: [jwk] }));
});
await new Promise((resolve) => jwksServer.listen(KEY_PORT, "127.0.0.1", resolve));
const vars = join(serverDir, ".dev.vars");
const state = join(serverDir, ".wrangler", "e2e-state");
writeFileSync(vars, [
  `ALLOWED_EMAILS=${EMAIL}`,
  `TOKEN_SIGNING_KEY=${randomBytes(32).toString("base64")}`,
  `GOOGLE_CLIENT_IDS=${CLIENT_ID}`,
  `INSECURE_DEV_JWKS_URL=http://127.0.0.1:${KEY_PORT}/certs`, "",
].join("\n"));
rmSync(state, { recursive: true, force: true });
const dev = spawn(process.env.WRANGLER ?? "wrangler", [
  "dev", "--port", String(PORT), "--ip", "127.0.0.1", "--persist-to", state,
], { cwd: serverDir, stdio: ["ignore", "inherit", "inherit"] });
function cleanup() {
  dev.kill("SIGTERM");
  jwksServer.close();
  rmSync(vars, { force: true });
}
process.on("exit", cleanup);
async function api(method, path, { token, body } = {}) {
  const res = await fetch(BASE + path, {
    method, headers: { "content-type": "application/json", ...(token ? { authorization: `Bearer ${token}` } : {}) },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const raw = await res.text();
  return { status: res.status, json: raw ? JSON.parse(raw) : null };
}
async function signIn(device_id, name, email = EMAIL) {
  return api("POST", "/v1/auth/google", {
    body: { id_token: idToken(email), device: { device_id, name, model: null, platform: "android" } },
  });
}
class Socket {
  constructor(token) {
    this.inbox = [];
    this.waiters = [];
    this.ws = new WebSocket(`ws://127.0.0.1:${PORT}/v1/sync`, {
      headers: { authorization: `Bearer ${token}` },
    });
    this.open = new Promise((resolve, reject) => { this.ws.onopen = resolve; this.ws.onerror = reject; });
    this.closed = new Promise((resolve) => { this.ws.onclose = resolve; });
    this.ws.onmessage = ({ data }) => {
      this.inbox.push(JSON.parse(data));
      for (const wake of this.waiters.splice(0)) wake();
    };
  }
  send(value) { this.ws.send(JSON.stringify(value)); }
  async next(type, timeout = 5000) {
    const end = Date.now() + timeout;
    for (;;) {
      const index = this.inbox.findIndex((m) => m.type === type);
      if (index >= 0) return this.inbox.splice(index, 1)[0];
      const left = end - Date.now();
      if (left <= 0) throw Error(`No ${type}; messages: ${JSON.stringify(this.inbox)}`);
      await new Promise((resolve) => {
        const timer = setTimeout(resolve, left);
        this.waiters.push(() => { clearTimeout(timer); resolve(); });
      });
    }
  }
}
for (let i = 0; i < 90; i++) {
  try { if ((await fetch(BASE + "/v1/health")).ok) break; }
  catch {}
  if (i === 89) throw Error("Steps Worker did not start");
  await new Promise((resolve) => setTimeout(resolve, 1000));
}
assert.equal((await signIn("intruder", "Other", "other@example.com")).status, 403);
const phone = await signIn("s24u", "Galaxy S24 Ultra");
const tablet = await signIn("tabs11", "Galaxy Tab S11");
assert.equal(phone.status, 200, JSON.stringify(phone.json));
assert.equal(tablet.status, 200, JSON.stringify(tablet.json));
assert.equal(phone.json.account_id, tablet.json.account_id);
const a = new Socket(phone.json.access_token);
const b = new Socket(tablet.json.access_token);
await Promise.all([a.open, b.open]);
a.send({ type: "hello", protocol: 1, last_seq: 0 });
b.send({ type: "hello", protocol: 1, last_seq: 0 });
await Promise.all([a.next("caught_up"), b.next("caught_up")]);
const op = {
  id: "steps-e2e-1", table: "settings", row_id: "e2e",
  kind: "upsert", hlc: { wall_ms: Date.now(), counter: 0, device: "s24u" },
  payload: { value: "true", deleted: 0 }, relay: false,
};
a.send({ type: "push", batch: 1, ops: [op] });
const ack = await a.next("ack");
assert.deepEqual(ack.done, [op.id], JSON.stringify(ack));
const synced = await b.next("ops");
assert.equal(synced.ops[0].row_id, op.row_id);
const devices = await api("GET", "/v1/devices", { token: tablet.json.access_token });
assert.equal(devices.status, 200);
assert.deepEqual(devices.json.devices.map((d) => d.name).sort(), ["Galaxy S24 Ultra", "Galaxy Tab S11"]);
const refreshed = await api("POST", "/v1/auth/refresh", { body: { refresh_token: tablet.json.refresh_token } });
assert.equal(refreshed.status, 200);
assert.notEqual(refreshed.json.refresh_token, tablet.json.refresh_token);
assert.equal((await api("GET", "/v1/devices")).status, 401);
assert.equal((await api("DELETE", "/v1/devices/s24u", { token: refreshed.json.access_token })).status, 200);
assert.equal((await a.next("revoked")).type, "revoked");
assert.equal((await a.closed).code, 4001);
const blocked = await api("POST", "/v1/auth/refresh", { body: { refresh_token: phone.json.refresh_token } });
assert.equal(blocked.status, 401);
b.ws.close();
console.log("Steps Cloud e2e: sign-in, refresh, sync, devices and revoke passed");
process.exit(0);
