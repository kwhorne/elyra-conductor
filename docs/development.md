# Development

How to set up, run, and work on Conductor locally.

## Prerequisites

- **Node.js** ≥ 20 (developed on 22) and **pnpm** ≥ 9
- **Rust** stable (developed on 1.95) via [rustup](https://rustup.rs/)
- Tauri platform build deps — see the
  [Tauri prerequisites guide](https://tauri.app/start/prerequisites/). On macOS:
  `xcode-select --install`.

## Run

```bash
pnpm install
pnpm tauri dev     # Vite + the Tauri shell (Rust)
```

Vite serves the frontend on port `1420`. The first `tauri dev` compiles Rust deps
(~30 s); later runs are fast.

### Frontend-only checks

```bash
pnpm build         # type/compile the Svelte + Vite frontend
pnpm preview       # preview the built frontend
```

### Rust-only checks

```bash
cd src-tauri
cargo check        # fast type-check of the Rust core
cargo build        # full build
```

## Where things live

See the full tree in [Architecture & boundaries](architecture.md). The short version:

- **`src/`** — Svelte 5 frontend. `App.svelte` owns layout, tabs, splits, shortcuts, and
  persistence; `src/lib/` holds the components and the pure `layout.js` split-tree logic.
- **`src-tauri/src/`** — the Rust core: `pty.rs` (terminals), `projects.rs` (scan + git +
  editors), `fs.rs` (files + tasks), `agent.rs` (Elyra RPC bridge), and `lib.rs` (the
  command registry).

## Adding features

- **A new Rust command:** write a `#[tauri::command]` and register it in `lib.rs`'s
  `generate_handler!`. See [Tauri commands](tauri-commands.md).
- **New persisted state:** extend `serialize()` / `applySnapshot()` in `App.svelte`, and
  document the key in [State & persistence](persistence.md).
- **A new palette action:** push an entry into the derived `commands` list in
  `App.svelte` (see [Command palette](command-palette.md) for the groups).

Keep every feature within the [boundary](architecture.md): Conductor is a host/launcher,
never an AI agent — no model calls, no API keys, no prompts/tools.

## Debugging

- **Webview console:** use the dev window's web inspector for frontend logs and errors.
- **Rust logs:** output from `pnpm tauri dev` shows Rust `println!`/`eprintln!` and panic
  traces.
- **Terminal issues:** check the PTY event wiring (`pty://data/<id>` /
  `pty://exit/<id>`) and that the pane's `termId` is dot-free (Tauri event names reject
  `.`).

## Conventions

- Match the existing code style (Svelte 5 runes, small pure helpers, Rust commands that
  return `Result<_, String>`).
- Update the docs and [`CHANGELOG.md`](../CHANGELOG.md) (`[Unreleased]`) with notable
  changes.

### Checks before you push

```bash
pnpm check                        # svelte-check (errors *and* warnings) + security suite
cd src-tauri
cargo test                        # includes the SQL-escaping and locking regressions
cargo clippy --all-targets        # expected to be warning-free
cargo fmt                         # the tree is rustfmt-clean as of 0.9.2 — keep it that way
```

`pnpm check` also runs [`scripts/check-security.mjs`](../scripts/check-security.mjs),
which asserts that HTML sanitising still strips attacks while preserving runbook
`cfile:`/`ctask:` links, and that scrollback redaction masks credentials without touching
ordinary output. If you change `lib/sanitize.js` or `lib/redact.js`, expect that suite to
have an opinion.

> Keep formatting commits separate from behavioural ones. A reviewer should never have to
> pick a logic change out of a few hundred reflowed lines.

## Related

- [Architecture & boundaries](architecture.md)
- [Releasing & auto-update](releasing.md)
