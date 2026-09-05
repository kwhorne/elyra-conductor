#!/usr/bin/env bash
# Add (or change) the password on the updater signing key WITHOUT rotating it.
#
# 1. generates a password and stores it in the login keychain item that
#    release-build.sh reads ("elyra-conductor-signing"),
# 2. re-encrypts ~/.tauri/elyra-conductor.key under it (same keypair),
# 3. proves the result end to end: `tauri signer sign` with the new password
#    produces a signature the ORIGINAL public key verifies, and signing with the
#    old (empty) password now fails,
# 4. only then removes the backup of the old key file.
#
# Usage:  OLD_PW='' ./scripts/rekey-signing-key.sh
#         (OLD_PW defaults to empty — a key generated without a password.)
set -euo pipefail
cd "$(dirname "$0")/.."

KEY="${TAURI_SIGNING_PRIVATE_KEY_FILE:-$HOME/.tauri/elyra-conductor.key}"
PUB="$KEY.pub"
ITEM="${TAURI_SIGNING_KEYCHAIN_ITEM:-elyra-conductor-signing}"
TOOL_DIR="scripts/rekey-signing-key"
TOOL="$TOOL_DIR/target/release/rekey-signing-key"
[ -f "$KEY" ] || { echo "no key at $KEY" >&2; exit 1; }
[ -f "$PUB" ] || { echo "no public key at $PUB" >&2; exit 1; }

echo "==> Building the re-encryption tool"
(cd "$TOOL_DIR" && cargo build --release --quiet)

if security find-generic-password -s "$ITEM" -w >/dev/null 2>&1; then
  echo "==> Keychain item '$ITEM' already exists — reusing its password as NEW_PW"
  NEW_PW="$(security find-generic-password -s "$ITEM" -w)"
else
  echo "==> Generating a password and storing it in keychain item '$ITEM'"
  NEW_PW="$(openssl rand -base64 32)"
  security add-generic-password -a "$USER" -s "$ITEM" -w "$NEW_PW" -U \
    -j "Password for the elyra-conductor updater signing key (~/.tauri/elyra-conductor.key)"
fi
export NEW_PW OLD_PW="${OLD_PW:-}"

STAMP="$(date +%Y%m%d-%H%M%S)"
BACKUP="$KEY.before-rekey-$STAMP"
OUT="$KEY.rekeyed-$STAMP"
umask 077
echo "==> Backing up $KEY -> $BACKUP"
cp -p "$KEY" "$BACKUP"; chmod 600 "$BACKUP"

echo "==> Re-encrypting (verifies keypair + round-trip before writing)"
"$TOOL" rekey "$KEY" "$PUB" "$OUT"
chmod 600 "$OUT"
mv "$OUT" "$KEY"
chmod 600 "$KEY"

echo "==> End-to-end check with Tauri's own signer"
PROBE="$(mktemp)"; echo "rekey probe $STAMP" > "$PROBE"
TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$NEW_PW" \
  pnpm --silent tauri signer sign "$PROBE" >/dev/null
"$TOOL" verify "$PUB" "$PROBE" "$PROBE.sig"
if TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
     pnpm --silent tauri signer sign "$PROBE" >/dev/null 2>&1; then
  echo "!! the empty password still signs — restoring backup" >&2
  mv "$BACKUP" "$KEY"; exit 1
fi
echo "   empty password refused ✓"
rm -f "$PROBE" "$PROBE.sig"

echo "==> Removing the unprotected backup"
rm -f "$BACKUP"
echo
echo "Done. $KEY is now protected by the password in keychain item '$ITEM'."
echo "Save that password in your password manager:  security find-generic-password -s $ITEM -w"
