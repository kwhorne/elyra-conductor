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

1. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml` (keep them in sync), then commit.

2. **Build a signed bundle** (the signing env vars are required because
   `createUpdaterArtifacts` is enabled):

   ```bash
   export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/elyra-conductor.key)"
   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""   # empty if the key has no password
   pnpm tauri build
   ```

   This produces, in `src-tauri/target/release/bundle/`:
   - `dmg/Elyra Conductor_<version>_aarch64.dmg` — installer for new users
   - `macos/Elyra Conductor.app.tar.gz` — the updater payload
   - `macos/Elyra Conductor.app.tar.gz.sig` — its signature

   > The bundle name comes from `productName` ("Elyra Conductor"). Upload the
   > updater tarball under the stable, space-free name **`elyra-conductor.app.tar.gz`**
   > (that's what `latest.json` points to — the signature is over the file
   > contents, not the name, so renaming is safe).

3. **Generate the update manifest:**

   ```bash
   node scripts/make-latest-json.mjs
   ```

   This writes `latest.json` pointing at the release URL for the current version.

4. **Create the GitHub release** for tag `v<version>` and upload these assets
   (rename the tarball/sig to the stable space-free name):
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
- Releases are **unsigned by Apple** (no notarization), so first launch still needs
  right-click → Open. Notarization is independent of the Tauri update signature.
