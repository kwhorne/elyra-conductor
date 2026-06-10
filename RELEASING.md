# Releasing & auto-update

elyra-conductor ships an in-app updater (Tauri updater plugin) that checks GitHub
Releases for new versions. Updates **must be signed** with the project's private
key, otherwise the app refuses to install them.

## One-time setup

A signing keypair was generated with:

```bash
pnpm tauri signer generate -w ~/.tauri/elyra-conductor.key
```

- **Private key:** `~/.tauri/elyra-conductor.key` — keep secret, never commit.
- **Public key:** embedded in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

> If the private key is lost, existing installs can no longer auto-update — you'd
> have to ship a new signed build manually and rotate the pubkey.

## Cutting a release

1. **Update the changelog.** In [CHANGELOG.md](CHANGELOG.md), rename the
   `[Unreleased]` heading to `[<version>] — <YYYY-MM-DD>`, then add a fresh empty
   `[Unreleased]` section above it. Update the comparison links at the bottom of
   the file (add a `compare/v<prev>...v<version>` line and re-point `[Unreleased]`
   to `compare/v<version>...HEAD`). This section becomes the GitHub release notes
   in step 5.

2. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml` (keep them in sync), then commit (changelog + bump
   together).

3. **Build a signed bundle.** Use the helper script — it first runs the quality
   gate (`pnpm check` — the build aborts on any svelte-check error or warning),
   then loads the signing key, cleans stale DMG mounts, and sets `CI=true` so
   create-dmg skips the flaky AppleScript window-styling step:

   ```bash
   ./scripts/release-build.sh
   # (set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=... first if your key has a password)
   ```

   It runs `pnpm tauri build` and then `make-latest-json.mjs` for you. The build
   produces, in `src-tauri/target/release/bundle/`:
   - `dmg/Elyra Conductor_<version>_aarch64.dmg` — installer for new users
   - `macos/Elyra Conductor.app.tar.gz` — the updater payload
   - `macos/Elyra Conductor.app.tar.gz.sig` — its signature

   > The bundle name comes from `productName` ("Elyra Conductor"). Upload the
   > updater tarball under the stable, space-free name **`elyra-conductor.app.tar.gz`**
   > (that's what `latest.json` points to — the signature is over the file
   > contents, not the name, so renaming is safe).

4. **The update manifest** (`latest.json`) is generated automatically by the
   release script. To regenerate it manually: `node scripts/make-latest-json.mjs`.

5. **Create the GitHub release** for tag `v<version>` (use the new changelog
   section as the release notes) and upload these assets (rename the tarball/sig
   to the stable space-free name):
   - `Elyra Conductor_<version>_aarch64.dmg`
   - `elyra-conductor.app.tar.gz`  (the `Elyra Conductor.app.tar.gz`)
   - `elyra-conductor.app.tar.gz.sig`
   - `latest.json`

   ```bash
   git tag -a v<version> -m "elyra-conductor v<version>" && git push origin v<version>
   # then upload the four assets to the release (gh release upload / API / web UI)
   ```

## How the update check works

- The app's `plugins.updater.endpoints` points at
  `https://github.com/kwhorne/elyra-conductor/releases/latest/download/latest.json`,
  which always resolves to the **latest** (non-prerelease) release's manifest.
- On startup (and via the command palette → "Check for updates…"), the app fetches
  the manifest, compares versions, and if newer shows a toast to install & restart.
- The downloaded `.app.tar.gz` is verified against the embedded public key before
  installing.

## Notes

- Current target is **Apple Silicon (`darwin-aarch64`)**. To support Intel or a
  universal binary, build for that target and add the matching entry under
  `platforms` in `latest.json` (e.g. `darwin-x86_64` or `darwin-universal`).
- The app is **Developer ID code-signed and Apple-notarized**. Signing identity:
  `Developer ID Application: GETS AS (7G383N3VY7)` (set in
  `bundle.macOS.signingIdentity`). `release-build.sh` notarizes the DMG via
  `xcrun notarytool` (keychain profile **`elyra-notary`**) and staples the ticket, so
  downloads open with no Gatekeeper warning.
- One-time notarytool setup on a build machine:
  ```bash
  xcrun notarytool store-credentials "elyra-notary" \
    --apple-id <id> --team-id 7G383N3VY7 --password <app-specific-password>
  ```
  The Developer ID signing key must be in the login keychain (a `.p12` with the private
  key). If the keychain profile is missing, the script still builds and signs but skips
  notarization (and prints a warning).
