# Elyra Conductor Documentation

**Elyra Conductor** is a local project conductor — a single window where you switch
projects, run terminals, split panes, browse files, quick-edit, and launch your real
editor. It is cross-platform Rust + web: the heavy lifting (PTYs, filesystem, process
launching, git) lives in [Tauri](https://tauri.app/)/Rust, while the UI is built with
[Svelte 5](https://svelte.dev/), [`xterm.js`](https://xtermjs.org/), and
[Monaco](https://microsoft.github.io/monaco-editor/).

## Quick start

Conductor is a desktop app. From a clone of the repository:

```bash
pnpm install
pnpm tauri dev
```

On first launch it scans `~/Code`. Click a project to open a terminal tab rooted at
that folder, then split with `⌘D`, browse files with `⌘B`, and jump anywhere with the
command palette (`⌘K`).

For the full first-run flow, see [Quickstart](quickstart.md).

## Design principle

> **Conductor orchestrates; it does not reason.**

Conductor is a launcher, terminal multiplexer, and host UI — never an AI agent. All
intelligence lives in external tools (chiefly the [Elyra](https://elyracode.com) CLI),
which Conductor runs like any other process. It never calls an LLM, stores API keys,
or defines prompts/tools/models. See [Architecture & boundaries](architecture.md).

## Start here

- [Quickstart](quickstart.md) — install, build, and open your first project.
- [Using Conductor](usage.md) — day-to-day workflow and the main UI areas.

## Features

- [Projects & sidebar](projects.md) — project switcher, git status, pinned projects.
- [Terminals & panes](terminals.md) — PTY terminals, splitting, search, titles, rings.
- [Workspaces](workspaces.md) — save and restore named layouts.
- [Tasks](tasks.md) — run `package.json` / `Makefile` / `justfile` / `composer.json` tasks.
- [Files & editor](files-and-editor.md) — the file tree and inline Monaco quick-edit.
- [Git](git.md) — status in the sidebar and the in-app commit dialog.
- [Command palette](command-palette.md) — the `⌘K` fuzzy launcher.
- [Keyboard shortcuts](keybindings.md) — every shortcut, and how editor focus changes them.

## Elyra integration

- [Elyra agent](elyra-agent.md) — host a native agent panel driven by `elyra --mode rpc`.

## Architecture

- [Architecture & boundaries](architecture.md) — layering and the host-not-agent rule.
- [Tauri commands](tauri-commands.md) — the full Rust ↔ webview IPC surface.
- [State & persistence](persistence.md) — how sessions, workspaces, and scrollback persist.

## Operations

- [Releasing & auto-update](releasing.md) — cut a signed release the in-app updater consumes.
- [Development](development.md) — local setup, project structure, and debugging.
- [Troubleshooting](troubleshooting.md) — common issues and fixes.
