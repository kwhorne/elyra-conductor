# Elyra Conductor — Dev runbook

Runnable notes for working on Conductor. Click **▶ Run** on any shell block to
send it to this project's terminal.

## Install & run

```bash
pnpm install
```

```bash
pnpm tauri dev
```

## Tasks

Click a task chip to run a discovered `package.json` script in this project's terminal:

- [[task:dev|dev]] · [[task:build|build]] · [[task:preview|preview]]

## Build

Frontend only (fast, catches Svelte/JS errors):

```bash
pnpm build
```

Full release binary (no installer bundle):

```bash
pnpm tauri build --no-bundle
```

## Release

Signed release build (DMG + updater tarball + latest.json):

```bash
./scripts/release-build.sh
```

## Key files

Click to open in the editor:

- [[src/App.svelte|App.svelte]] — main UI, tabs, persistence
- [[src/lib/RunbookPanel.svelte]] — this runbook panel
- [[src-tauri/src/projects.rs]] — git status & project scanning
- [[CHANGELOG.md]]

## Git

```bash
git status --porcelain=v2 --branch
```

## Checklist

- [ ] Bump version in `package.json`, `tauri.conf.json`, `Cargo.toml`
- [ ] Update `CHANGELOG.md`
- [ ] Tag `vX.Y.Z` and push
- [ ] Upload assets to the GitHub release

> Stored in `.conductor/notes/` — versioned with git.
