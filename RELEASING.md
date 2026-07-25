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

Releases are built by GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml)).
Pushing a tag is the whole process — the signing certificate and the updater key
live in repository secrets, not on a laptop.

1. **Update the changelog.** In [CHANGELOG.md](CHANGELOG.md), rename the
   `[Unreleased]` heading to `[<version>] — <YYYY-MM-DD>`, then add a fresh empty
   `[Unreleased]` section above it. Update the comparison links at the bottom of
   the file (add a `compare/v<prev>...v<version>` line and re-point `[Unreleased]`
   to `compare/v<version>...HEAD`). This section becomes the release notes.

2. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`,
   `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`. Check they agree:

   ```bash
   node scripts/check-version-sync.mjs
   ```

3. **Commit, tag and push.** That is the release:

   ```bash
   git commit -am "Release v<version>: <summary>"
   git tag -a v<version> -m "elyra-conductor v<version>"
   git push origin main && git push origin v<version>
   ```

The workflow then runs the quality gate (`pnpm check` + `cargo test`), builds,
signs with Developer ID, notarizes and staples, generates `latest.json`, and
publishes the release with the four assets. It **fails loudly** if the bundle
turns out not to be notarized — Tauri only warns and carries on when the signing
secrets are missing, which would otherwise ship a release nobody can open.

### Required repository secrets

| Secret | What it is |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application `.p12`, base64-encoded (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: GETS AS (7G383N3VY7)` |
| `APPLE_ID` | Apple ID used for notarisation |
| `APPLE_PASSWORD` | an **app-specific password**, not the account password |
| `APPLE_TEAM_ID` | `7G383N3VY7` |
| `TAURI_SIGNING_PRIVATE_KEY` | contents of `~/.tauri/elyra-conductor.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | its passphrase (empty for the current key) |

### Building locally instead

[`scripts/release-build.sh`](scripts/release-build.sh) still does the whole thing
on your own machine — useful for testing a bundle before tagging, or if Actions
is unavailable. It notarizes through a local `notarytool` keychain profile
(`elyra-notary`) rather than the secrets above:

```bash
./scripts/release-build.sh
```

It produces, in `src-tauri/target/release/bundle/`:
- `dmg/Elyra Conductor_<version>_aarch64.dmg` — installer for new users
- `macos/Elyra Conductor.app.tar.gz` — the updater payload
- `macos/Elyra Conductor.app.tar.gz.sig` — its signature

> The bundle name comes from `productName` ("Elyra Conductor"). The updater
> tarball is uploaded under the stable, space-free name
> **`elyra-conductor.app.tar.gz`** — that's what `latest.json` points to, and the
> signature covers the file contents, not the name, so renaming is safe.

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
