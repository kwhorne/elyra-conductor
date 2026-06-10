#!/usr/bin/env bash
# Robust signed release build.
#
# Fixes the flaky `bundle_dmg.sh` failure by:
#   1. removing leftover read-write temp DMGs from previous runs,
#   2. detaching any stale mounted "Elyra Conductor" volumes,
#   3. setting CI=true so create-dmg skips the AppleScript/Finder window styling
#      step (the usual cause of the hang/failure on macOS).
#
# Usage:
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD=... ./scripts/release-build.sh
set -euo pipefail
cd "$(dirname "$0")/.."

KEY_FILE="${TAURI_SIGNING_PRIVATE_KEY_FILE:-$HOME/.tauri/elyra-conductor.key}"
if [ ! -f "$KEY_FILE" ]; then
  echo "Signing key not found at $KEY_FILE" >&2
  exit 1
fi

echo "==> Quality gate: svelte-check (type + a11y)"
pnpm check

echo "==> Cleaning stale DMG artifacts / mounts"
rm -f "src-tauri/target/release/bundle/macos/rw."*.dmg 2>/dev/null || true
for v in /Volumes/Elyra\ Conductor*; do
  [ -d "$v" ] || continue
  echo "    detaching $v"
  hdiutil detach "$v" -force 2>/dev/null || diskutil unmount force "$v" 2>/dev/null || true
done

export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_FILE")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
# Skip create-dmg's AppleScript window-styling step (flaky / needs Finder access).
export CI=true

echo "==> Building signed release"
pnpm tauri build "$@"

# ── Notarize + staple the DMG (Developer ID) ───────────────────────────────
# Requires a notarytool keychain profile. Create once with:
#   xcrun notarytool store-credentials "$NOTARY_PROFILE" \
#     --apple-id <id> --team-id <team> --password <app-specific-password>
NOTARY_PROFILE="${NOTARY_PROFILE:-elyra-notary}"
DMG="$(ls -t src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | head -1 || true)"
APP_PATH="$(ls -d src-tauri/target/release/bundle/macos/*.app 2>/dev/null | head -1 || true)"

# Submit to Apple's notary service and poll. Apple's queue occasionally gets
# stuck for an hour+ on a submission (notarytool --wait can even crash). When a
# submission stays "In Progress" past ~12 min we give up on it and resubmit a
# fresh job — in practice the new one is accepted in under a minute.
notarize_with_retry() {
  local file="$1" attempt id status waited
  for attempt in 1 2 3; do
    echo "   submit (attempt $attempt)…"
    id="$(xcrun notarytool submit "$file" --keychain-profile "$NOTARY_PROFILE" 2>/dev/null | awk '/id:/{print $2; exit}')"
    if [ -z "$id" ]; then echo "   submit failed; retrying in 5s"; sleep 5; continue; fi
    echo "   submission id: $id"
    waited=0
    while [ "$waited" -lt 720 ]; do
      status="$(xcrun notarytool info "$id" --keychain-profile "$NOTARY_PROFILE" 2>/dev/null | awk '/status:/{print $2; exit}')"
      case "$status" in
        Accepted) echo "   accepted ✓"; return 0 ;;
        Invalid|Rejected)
          echo "   $status ✗"; xcrun notarytool log "$id" --keychain-profile "$NOTARY_PROFILE" 2>/dev/null | head -40; return 1 ;;
      esac
      sleep 20; waited=$((waited + 20))
    done
    echo "   still In Progress after ${waited}s — Apple queue stuck, resubmitting…"
  done
  echo "!! Notarization did not complete after retries"; return 1
}

if [ -n "$DMG" ] && xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
  echo "==> Notarizing $(basename "$DMG")"
  if notarize_with_retry "$DMG"; then
    echo "==> Stapling notarization ticket"
    xcrun stapler staple "$DMG"
    [ -n "$APP_PATH" ] && xcrun stapler staple "$APP_PATH"
    echo "==> Gatekeeper assessment"
    spctl -a -t open --context context:primary-signature -vv "$DMG" 2>&1 || true
  else
    echo "!! Notarization failed — the DMG is signed but not notarized."
  fi
else
  echo "!! Skipping notarization (no '$NOTARY_PROFILE' keychain profile, or no DMG)."
  echo "   The DMG is Developer-ID signed but NOT notarized; downloads will warn."
fi

echo "==> Generating latest.json"
node scripts/make-latest-json.mjs

echo
echo "Done. Artifacts in src-tauri/target/release/bundle/ and latest.json"
if [ -n "$DMG" ]; then echo "DMG: $DMG"; fi
