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

Releases are built by **GitHub Actions**; pushing a tag is the whole process. The
Developer ID certificate and the updater signing key live in repository secrets,
so no signing material sits on a developer machine.

1. **Update the changelog.** In [`CHANGELOG.md`](../CHANGELOG.md), rename `[Unreleased]`
   to `[<version>] — <YYYY-MM-DD>`, add a fresh empty `[Unreleased]`, and update the
   comparison links at the bottom. This section becomes the release notes.
2. **Bump the version** in `package.json`, `src-tauri/tauri.conf.json`,
   `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` — verify with
   `node scripts/check-version-sync.mjs`.
3. **Commit, tag, push:**

   ```bash
   git commit -am "Release v<version>: <summary>"
   git tag -a v<version> -m "elyra-conductor v<version>"
   git push origin main && git push origin v<version>
   ```

CI runs the quality gate, builds, signs, notarizes and staples, generates
`latest.json`, and publishes the release with all four assets. It fails the build
if the result is not actually notarized, rather than shipping something users
cannot open.

`./scripts/release-build.sh` still performs the same build locally (using a
`notarytool` keychain profile instead of the secrets) when you want to test a
bundle before tagging. See [`RELEASING.md`](../RELEASING.md) for the secret list
and the local path.

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
- Releases **are** Developer ID code-signed **and Apple-notarized** (the release
  script submits the DMG via `xcrun notarytool` and staples the ticket), so a
  downloaded build opens without a Gatekeeper warning — no right-click → Open
  needed. Notarization is independent of the Tauri update signature: the former
  is Apple vouching for the binary, the latter is what the in-app updater
  verifies. See [`RELEASING.md`](../RELEASING.md) for the signing setup.

## Related

- [`RELEASING.md`](../RELEASING.md) — the full guide.
- [`CHANGELOG.md`](../CHANGELOG.md) — version history and release notes.
