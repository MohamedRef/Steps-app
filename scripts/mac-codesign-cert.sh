#!/usr/bin/env bash
# Adapted from kabar-app main: scripts/mac-codesign-cert.sh.
# Once, on the Mac: a free, self-signed "Steps Code Signing" certificate in
# the login Keychain. `just bundle-mac` signs Steps.app with it, so every
# build keeps the same code identity: the Keychain keeps trusting Steps after
# an update, and the updater checks a new Steps.app comes from the same signer (docs/RELEASE.md §5). No Apple
# Developer account needed. macOS asks for your password once to trust it.
# The identity is also kept as ~/.steps/steps-codesign.p12 (password in the
# Keychain, service "steps-codesign"), so scripts/release-secrets.sh can hand
# the same signer to the release workflow. Back it up with the other keys.
set -euo pipefail
NAME="Steps Code Signing"
P12="$HOME/.steps/steps-codesign.p12"

if security find-identity -v -p codesigning | grep -q "\"$NAME\""; then
  echo "\"$NAME\" is already in the Keychain."
  exit 0
fi
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/cert.cnf" <<CNF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
CNF
openssl req -x509 -newkey rsa:2048 -nodes -days 7300 -config "$tmp/cert.cnf" \
  -keyout "$tmp/key.pem" -out "$tmp/cert.pem" 2>/dev/null
pass="$(openssl rand -hex 16)"
openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -name "$NAME" \
  -out "$tmp/steps.p12" -passout "pass:$pass" 2>/dev/null
mkdir -p "$(dirname "$P12")"
cp "$tmp/steps.p12" "$P12" && chmod 600 "$P12"
security add-generic-password -U -s steps-codesign -a "$USER" -w "$pass"
security import "$tmp/steps.p12" -k "$HOME/Library/Keychains/login.keychain-db" -P "$pass" -T /usr/bin/codesign
security add-trusted-cert -r trustRoot -p codeSign -k "$HOME/Library/Keychains/login.keychain-db" "$tmp/cert.pem"
security find-identity -v -p codesigning | grep "\"$NAME\""
echo "Done. Next 'just bundle-mac' signs Steps.app as \"$NAME\"."
