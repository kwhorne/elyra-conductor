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

if [ -n "$DMG" ] && xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
  echo "==> Notarizing $(basename "$DMG")"
  xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
  echo "==> Stapling notarization ticket"
  xcrun stapler staple "$DMG"
  echo "==> Gatekeeper assessment"
  spctl -a -t open --context context:primary-signature -vv "$DMG" 2>&1 || true
else
  echo "!! Skipping notarization (no '$NOTARY_PROFILE' keychain profile, or no DMG)."
  echo "   The DMG is Developer-ID signed but NOT notarized; downloads will warn."
fi

echo "==> Generating latest.json"
node scripts/make-latest-json.mjs

echo
echo "Done. Artifacts in src-tauri/target/release/bundle/ and latest.json"
if [ -n "$DMG" ]; then echo "DMG: $DMG"; fi
