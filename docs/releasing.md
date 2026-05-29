# Releasing & auto-update

Conductor ships an in-app updater (the Tauri updater plugin) that checks GitHub Releases
for new versions. Updates **must be signed** with the project's private key, or the app
refuses to install them.

> This is a summary. The authoritative, step-by-step guide lives in
> [`RELEASING.md`](../RELEASING.md) at the repo root.

## Signing keys (one-time)

A signing keypair is generated once:

```bash
pnpm tauri signer generate -w ~/.tauri/elyra-conductor.key
```

- **Private key:** `~/.tauri/elyra-conductor.key` — keep secret, never commit.
- **Public key:** embedded in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

If the private key is lost, existing installs can no longer auto-update.

## Cutting a release

1. **Update the changelog.** In [`CHANGELOG.md`](../CHANGELOG.md), rename `[Unreleased]`
   to `[<version>] — <YYYY-MM-DD>`, add a fresh empty `[Unreleased]`, and update the
   comparison links at the bottom. This section becomes the GitHub release notes.
2. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml` (keep them in sync); commit the changelog + bump together.
3. **Build a signed bundle:**

   ```bash
   ./scripts/release-build.sh
   # set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=… first if your key has a password
   ```

   It runs `pnpm tauri build` and `make-latest-json.mjs`, producing (in
   `src-tauri/target/release/bundle/`) the `.dmg` installer, the `.app.tar.gz` updater
   payload, and its `.sig`.
4. **Manifest:** `latest.json` is generated automatically (regenerate manually with
   `node scripts/make-latest-json.mjs`).
5. **Create the GitHub release** for tag `v<version>` (use the new changelog section as
   the notes) and upload the four assets, renaming the tarball/sig to the stable,
   space-free name `elyra-conductor.app.tar.gz`:

   ```bash
   git tag -a v<version> -m "elyra-conductor v<version>" && git push origin v<version>
   ```

## How the update check works

- `plugins.updater.endpoints` points at the repo's
  `releases/latest/download/latest.json`, which always resolves to the latest
  non-prerelease manifest.
- On startup (and via `⌘K` → **Check for updates…**), the app fetches the manifest,
  compares versions, and offers a one-click install & restart for newer builds.
- The downloaded `.app.tar.gz` is verified against the embedded public key before
  installing.

## Notes

- Current target is **Apple Silicon (`darwin-aarch64`)**. To support Intel or a universal
  binary, build for that target and add the matching `platforms` entry in `latest.json`.
- Releases are **not notarized** by Apple, so the first launch still needs
  right-click → Open. Notarization is independent of the Tauri update signature.

## Related

- [`RELEASING.md`](../RELEASING.md) — the full guide.
- [`CHANGELOG.md`](../CHANGELOG.md) — version history and release notes.
