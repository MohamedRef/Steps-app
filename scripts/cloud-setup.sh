#!/usr/bin/env bash
# One-time Steps Cloud setup on your Cloudflare account (Workers Free plan).
# Safe to re-run: it skips what already exists. Needs `wrangler login` first.
# Adapted from batna-app main: scripts/cloud-setup.sh (no D1: Steps keeps everything
# in the account's Durable Object). Nothing here is shared with another app's cloud.
set -euo pipefail

cd "$(dirname "$0")/../crates/server"
WRANGLER="${WRANGLER:-wrangler}"

echo "→ Checking you're logged in to Cloudflare"
"$WRANGLER" whoami >/dev/null

echo "→ Token signing key (Ed25519 seed, never leaves Cloudflare)"
if "$WRANGLER" secret list 2>/dev/null | grep -q TOKEN_SIGNING_KEY; then
  echo "  already set"
else
  openssl rand -base64 32 | tr -d '\n' | "$WRANGLER" secret put TOKEN_SIGNING_KEY
fi

echo "→ Allowlist: the Google account(s) allowed to sign in"
if "$WRANGLER" secret list 2>/dev/null | grep -q ALLOWED_EMAILS; then
  echo "  already set (change with: wrangler secret put ALLOWED_EMAILS)"
else
  read -r -p "  Your Gmail address: " email
  printf '%s' "$email" | "$WRANGLER" secret put ALLOWED_EMAILS
fi

cat <<'MSG'

Done. Next:
  1. Put Steps's Google OAuth client id(s) in the repo variable STEPS_GOOGLE_CLIENT_IDS
     (docs/CLOUD.md §3.2 shows how to create them).
  2. Run Actions › Deploy Steps Cloud, or: just cloud-deploy (then set
     GOOGLE_CLIENT_IDS in the Cloudflare dashboard).
MSG
