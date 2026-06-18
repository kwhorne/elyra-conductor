<div align="center">

# Elyra Conductor

**A local project conductor — switch projects, run terminals, split panes, browse files, and quick-edit, all in one window.**

<img src="assets/screenshot.png" alt="Elyra Conductor — project sidebar, a deploy running in a terminal, and the file explorer" width="100%" />

</div>

---

## Overview

Elyra Conductor gives you a single cockpit for all your local projects. Pick a
project from the sidebar and it opens in a terminal tab rooted at that folder.
Split the terminal into as many panes as you like, browse the project's files,
quick-edit a file inline with Monaco, or launch the whole project in your real
editor (Zed / VS Code / Cursor).

Run several Elyra agents at once — each isolated in its own git worktree — and see
at a glance which are working and which are waiting on you, with each branch's pull
request and CI status right there.

It is **not** a Ghostty wrapper or a native Swift app — it's cross-platform
Rust + web, so the heavy lifting (PTYs, filesystem, process launching) lives in
Rust while the UI uses battle-tested web components (`xterm.js`, Monaco).

## Design principles & non-goals

**Elyra Conductor orchestrates; it does not reason.** It is a launcher, terminal
multiplexer, and host UI — never an AI agent. AI lives entirely in external tools
(e.g. the [Elyra](https://elyracode.com) CLI), which Conductor treats
like any other process (Zed, iTerm, git).

Conductor will **never**:
- call an LLM or bundle an AI SDK
- store or manage API keys
- define prompts, system prompts, tools, or models
- contain agent/reasoning logic or built-in "AI features"

Rule of thumb: **if a feature needs an API key or a model call, it belongs in
Elyra, not in Conductor.** See [ARCHITECTURE.md](ARCHITECTURE.md) for the full
boundary (including how the planned RPC integration stays a *host*, not an agent).

## Features

- 📁 **Project switcher** — scans a folder (default `~/Code`), shows the current
  git branch, dirty state, and ahead/behind counts per repository, with instant
  fuzzy search.
- 📌 **Pinned projects** — pin favourites to a sticky section at the top of the
  sidebar; they persist (even across folder changes) until you unpin them.
- 🪟 **Split panes** — split any terminal horizontally or vertically, nest freely,
  and drag the dividers to resize. Panes never lose their session on relayout.
- 🖥️ **Real terminals** — one PTY per pane, powered by `portable-pty` and rendered
  with `xterm.js`. Full resize handling and your login shell.
- 🔍 **Command palette (⌘K)** — jump between projects, tabs, and actions from one
  fuzzy-searchable list.
- 🗂️ **File sidebar (⌘B)** — a lazy-loaded recursive file tree of the active
  project. Click a file to open it in the editor. On by default.
- ▶️ **Run files** — right-click a file in the sidebar to run `./<file>` either in
  an in-app terminal modal or in your external terminal (iTerm2 / Terminal.app).
- 🔔 **Notification rings** — a tab pulses green when a background terminal
  produces output, and clears when you switch to it. Tabs stay alive in the
  background, so nothing is lost when you switch.
- ✓ **Finished-command notifications** — get a native notification (with sound) when a
  long-running command in a background tab completes (build/test/deploy done) while
  you're elsewhere. Toggleable; only fires for commands ≥ 8s and never for the tab you're
  watching.
- ⚡ **Port dashboard** — see which local ports are listening (with process & PID); click
  to open `http://localhost:<port>` or stop the process. Each project also shows a green
  **⚡port** badge in the sidebar when it has a server running.
- 🎨 **Per-project tab colour** — each tab is tinted by its project so you always know
  which project you're looking at.
- 🔎 **Terminal search** — `⌘F` finds in the focused terminal; **`⇧⌘F` searches *all*
  open terminals at once** and jumps to the match.
- 🧭 **Panes** — `⌘⌥`+arrows to move between split panes, `⌘⌥Z` to zoom one full-screen.
- 🌓 **Light / dark theme** — toggle in the toolbar or palette; persisted.
- ↔️ **Drag-to-reorder tabs** — drag a tab sideways to rearrange; jump straight to any
  tab with `⌘1`–`⌘9`.
- ⏎ **Modified Enter in terminals** — `⇧↵` inserts a newline (via the Kitty `CSI u`
  sequence) instead of submitting, so TUIs like the Elyra CLI behave as expected.
- ⎇ **Git commit dialog** — review changes, write a message, commit (and push)
  without leaving the app (`⌘↵` to commit).
- 🗄️ **Workspaces** — save the whole layout (projects, tabs, split panes, open
  file, theme) as a named workspace and reopen it later from the palette
  (**Save workspace…** / **Load workspace: …**). Global across project folders.
- ▶️ **Per-project tasks** — Conductor scans `package.json`, `composer.json`,
  `Makefile`, and `justfile` for the active project and lists them in the palette
  (**Run: …**); pick one to launch it in a fresh terminal tab. Just a launcher —
  the command runs in your shell, no interpretation.
- ▶️ **Start project (⌘R)** — one action to run any project's dev command, whatever the
  stack (`npm run dev`, `pnpm dev`, `composer run dev`, `make dev`…). Picks the best
  detected task, asks when several tie (and remembers), with a per-project override.
- ⎁ **Broadcast input** — toggle **⌁ Sync** (toolbar or palette) to mirror your
  keystrokes to every pane in the active tab, tmux `synchronize-panes`-style.
- 💾 **Scrollback restore** — each pane's recent output is persisted and replayed
  as read-only history on restart (a fresh shell starts beneath it; live ptys
  can't be revived without a daemon).
- ⬆️ **Auto-update** — checks GitHub Releases on startup (and via the palette) and
  installs signed updates with one click. See [RELEASING.md](RELEASING.md).
- ✏️ **Inline quick-edit** — a Monaco editor panel for fast edits (⌘S to save),
  without leaving the app.
- 📓 **Runbooks** — runnable, project-scoped markdown notes in `.conductor/notes/`:
  **▶ Run** shell blocks in the project terminal, `[[file]]` links open in the editor,
  and `[[task:name]]` chips run discovered tasks. Record a terminal session into a
  runbook draft, or **✓ Verify** one — run every step headless and flag the ones that
  no longer work, with a freshness banner (mark always-on steps ` ```bash no-verify `).
  Local files, versionable with git.
- 🗄️ **Database browser** — connect to a project's databases (MySQL / PostgreSQL /
  ClickHouse / SQLite), **several at once** (e.g. MySQL + ClickHouse). Browse tables,
  sort/order/filter (incl. per-column) and page a data grid, edit cells inline, inspect
  structure, run SQL, and **export to Excel**. Connections are saved securely in the OS
  keychain (never committed); saved queries are private per project. ClickHouse uses its
  native protocol. Just a tool — no model calls.
- 🚀 **Open in your editor** — auto-detects installed editors and launches the
  project in Zed, VS Code, or Cursor.
- 🤖 **Elyra agent** — if the [Elyra](https://elyracode.com) coding agent CLI is
  installed, open a **native agent panel** in a tab (sidebar button, palette, or
  right-click → "Ask Elyra about this file"). Conductor drives `elyra --mode rpc`
  over JSON-RPC: stream replies, see tool activity, answer confirm/select/input
  prompts, and get a notification ring when the agent needs you. All AI stays in
  Elyra — Conductor is only the host UI (see [ARCHITECTURE.md](ARCHITECTURE.md)).
- 🌳 **Git worktrees — parallel branches, one agent each** — create isolated
  worktrees (separate checkouts sharing the repo's `.git`) and open each as a
  terminal or an Elyra agent, so several agents can work different branches at once
  without colliding. Worktrees live in a sibling `<repo>.worktrees/<branch>` folder.
- 🤖 **Agent command center** — each agent reports a coarse state, shown as a per-tab
  dot (working / waiting on you / exited) and a pill in the tab strip counting how many
  agents are **working** vs **waiting for your input**. Click to jump straight to one;
  a notification fires when an agent starts waiting while you're looking elsewhere.
- ✅ **GitHub PR status** — with an authenticated `gh`, each worktree shows its open
  PR (number, CI check rollup, review state) linking to GitHub. Open PRs without a
  worktree are listed too, so you can **check a PR out as a worktree** (terminal or
  agent) in one click — PR branches are tracked from `origin` with their real contents.
- ⚡ **"Fix it" — self-healing terminal** — when a command fails in a shell-integrated
  terminal, a quiet toast offers a one-click handoff to an Elyra agent with the full
  context (command, exit code, output tail, git branch) and a fix-oriented prompt. Never
  nags: interrupts are ignored and the offer expires on its own.
- 🌅 **Morning brief** — open Conductor after a real break and a welcome-back card shows
  where you left off: last project with git state, container health, and your last
  commands (failures flagged). One click to resume or have Elyra plan your day.
- 🕘 **Command timeline & history** — a flight recorder of what ran, where, and how long
  (real commands + exit codes via zsh shell integration, on by default). Jump back to any
  pane, or hand a failed command's context to Elyra to ask "why did this fail?". Every
  command is **persisted to a local SQLite store**, so you can **search across every
  session** (“how did I fix this last time?” — matching the command *and* its output).
- 📊 **Insights** — the timeline's Insights tab aggregates your history over Today / 7 days /
  All time: how many commands ran, how many failed, total time spent waiting, and the
  biggest time sinks per command (runs, average, failures). All local — no telemetry.

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘K` | Open command palette |
| `⌘D` | Split active pane right |
| `⇧⌘D` | Split active pane down |
| `⌘W` | Close active pane (or close the editor when it is focused) |
| `⌘B` | Toggle file sidebar |
| `⌘S` | Save file (when editor is focused) |
| `⌘F` | Find in terminal (when a terminal is focused) |
| `⌘↵` | Commit (in the commit dialog) |
| `⌘/` | Show keyboard-shortcuts help |

> On Linux/Windows, `Ctrl` substitutes for `⌘`.

While the editor is focused it owns all the usual editor keys — `⌘F` find,
`⌘/` toggle comment, `⌘D` multi-cursor, `⌘K` chords, and so on — and the app's
pane shortcuts step aside. Close the editor with `⌘W` or the **✕** button.

Each pane also has hover controls (split right / split down / close), and the
dividers between panes are drag-to-resize.

## Tech stack

| Layer | Choice |
|-------|--------|
| Shell | [Tauri 2](https://tauri.app/) (Rust core + system webview) |
| Frontend | [Svelte 5](https://svelte.dev/) (runes) + Vite |
| Terminal | [`xterm.js`](https://xtermjs.org/) ↔ [`portable-pty`](https://crates.io/crates/portable-pty) |
| Editor | [Monaco](https://microsoft.github.io/monaco-editor/) (quick-edit) |
| Dialogs | `tauri-plugin-dialog` (native file/folder pickers) |
| Mono font | [JetBrains Mono](https://www.jetbrains.com/lp/mono/) bundled via `@fontsource` (offline, consistent everywhere) |

The UI uses the OS system font (`-apple-system` → SF Pro on macOS); terminals and
the editor use the bundled **JetBrains Mono** (with ligatures in the editor).

## Prerequisites

- **Node.js** ≥ 20 (developed on 22) and **pnpm** ≥ 9
- **Rust** stable (developed on 1.95) — install via [rustup](https://rustup.rs/)
- Platform build dependencies for Tauri — see the
  [Tauri prerequisites guide](https://tauri.app/start/prerequisites/).
  On macOS this is just the Xcode Command Line Tools (`xcode-select --install`).

## Getting started

```bash
# install JS dependencies
pnpm install

# run the app in development (starts Vite + the Tauri shell)
pnpm tauri dev
```

The first `pnpm tauri dev` compiles the Rust dependencies (~30 s); subsequent
runs are fast. Vite serves the frontend on port `1420` automatically.

### Build a release bundle

```bash
pnpm tauri build
```

The artifacts land in `src-tauri/target/release/bundle/`.

> **Icons:** `src-tauri/icons/icon.png` is a placeholder generated by
> `node scripts/gen-icon.mjs`. For a real release, drop in a 1024×1024 PNG and run
> `pnpm tauri icon your-logo.png` to generate every required size and format.

## Usage

1. On first launch the app scans `~/Code`. Use the **⋯** button in the left
   sidebar (or the command palette) to pick a different folder.
2. Click a project to open a terminal tab in that directory.
3. Split with `⌘D` / `⇧⌘D`, drag the dividers to resize, close panes with `⌘W`.
4. Browse files in the right sidebar; click one to open it in Monaco (`⌘S` saves).
5. Hit `⌘K` any time to jump between projects, tabs, and actions.
6. Use the per-project **zed / code / cursor** buttons to open the project in
   your full editor.

## Architecture

The key design decision: **terminals live in a flat, absolutely-positioned layer**
keyed by terminal id, while a pure layout module computes their geometry from a
split tree. This means splitting a pane or dragging a divider never remounts a
terminal — the PTY session and scrollback survive. Rendering the tree recursively
would have killed and respawned shells on every relayout.

```
┌─────────────── Webview (Svelte) ─────────────────────────────┐
│  Sidebar (projects)   Tabs + split panes   Editor   Files     │
└───────────────────────────────────────────────────────────────┘
        │  Tauri IPC (commands + pty:// events)
┌─────────────── Rust core ─────────────────────────────────────┐
│  pty.rs       spawn/write/resize/kill + byte streaming         │
│  projects.rs  scan folder, git branch, detect/launch editors   │
│  fs.rs        list_dir / read_file / write_file                │
└───────────────────────────────────────────────────────────────┘
```

**Terminal flow:** Rust spawns a PTY per pane and reads it on a dedicated thread,
emitting bytes as `pty://data/<id>` events. The frontend writes them straight to
`xterm.js`. Input and resize travel back via `invoke('pty_write' | 'pty_resize')`.

## Project structure

```
elyra-conductor/
├── index.html
├── package.json
├── vite.config.js
├── svelte.config.js
├── scripts/
│   └── gen-icon.mjs              # placeholder icon generator
├── src/                          # frontend (Svelte 5)
│   ├── App.svelte                # layout, tabs, splits, shortcuts
│   ├── app.css                   # theme tokens
│   ├── main.js
│   └── lib/
│       ├── Sidebar.svelte        # project list + search + "open in editor"
│       ├── Terminal.svelte       # xterm.js wrapper ↔ pty:// events
│       ├── Editor.svelte         # Monaco quick-edit
│       ├── FileExplorer.svelte   # right sidebar root + header
│       ├── FileTree.svelte       # recursive, lazy-loaded file node
│       ├── CommandPalette.svelte # ⌘K palette
│       ├── layout.js             # pure split-tree ops + geometry
│       └── monaco-setup.js       # Monaco web-worker wiring for Vite
└── src-tauri/                    # Rust core (Tauri 2)
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json # window permissions (core + dialog)
    ├── icons/icon.png
    └── src/
        ├── main.rs               # entrypoint → lib::run()
        ├── lib.rs                # builder, plugins, command registry
        ├── pty.rs                # PTY sessions (binary Channel streaming)
        ├── projects.rs           # project scan, git (+ worktrees), gh PRs, editors
        ├── agent.rs              # elyra --mode rpc JSONL bridge
        ├── db.rs                 # database browser connections
        ├── history.rs            # persistent command history & insights (SQLite)
        └── fs.rs                 # directory + file commands
```

### Tauri commands

| Command | Purpose |
|---------|---------|
| `pty_spawn` / `pty_write` / `pty_resize` / `pty_kill` | Terminal session lifecycle |
| `pty_title` | Foreground process name of a pty (for dynamic tab/pane titles) |
| `agent_spawn` / `agent_send` / `agent_kill` | JSONL transport to `elyra --mode rpc` (host only) |
| `list_projects` | Scan the root folder, return projects + git branch |
| `git_status` | Per-project dirty / ahead / behind state |
| `git_changes` / `git_commit` | List working-tree changes; stage, commit, optionally push |
| `git_worktree_list` / `git_worktree_add` / `git_worktree_remove` | Parallel isolated worktrees per branch |
| `detect_gh` / `gh_pr_list` | GitHub PR + CI status per branch (via the `gh` CLI) |
| `run_step` | Run one runbook step headless (login shell, timeout) for **Verify** |
| `history_add` / `history_query` / `history_stats` / `history_clear` | Persistent command history, search & insights |
| `detect_editors` / `open_in_editor` | Find and launch external editors |
| `detect_terminal` / `run_in_external_terminal` | Run a file in iTerm2 / Terminal.app |
| `home_dir` | Resolve `$HOME` for the default root |
| `list_dir` | Directory listing for the file tree (dirs first) |
| `read_file` / `write_file` | Back the Monaco editor |

## Roadmap

- [x] Git status (dirty / ahead / behind) in the sidebar (refresh button + auto on window focus)
- [x] Persist tabs + split layout across restarts (fresh shells, same structure)
- [x] Notification rings — pulse a tab when a background terminal has new output
- [x] Hide `node_modules` / `.git` in the file tree (toggle, default hides noise)
- [x] Tab/pane titles derived from the running process (e.g. `bun`, `vim`)
- [x] Git worktrees — parallel branches, one agent each
- [x] Agent command center — presence (working / waiting) across all agents
- [x] GitHub PR status per branch (via `gh`)
- [ ] Persisted command timeline (SQLite) — semantic recall, flow metrics, runbook suggestions
- [ ] Persisted projects & favorites (SQLite via `tauri-plugin-sql`)

## Documentation

Full documentation lives in [`docs/`](docs/) (start at [`docs/index.md`](docs/index.md)) —
features, architecture & boundaries, the Tauri command reference, persistence, the Elyra
agent integration, releasing, development, and troubleshooting.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for the full version history. Notable changes are
recorded there following [Keep a Changelog](https://keepachangelog.com/) and
[Semantic Versioning](https://semver.org/).

## Releasing

See [RELEASING.md](RELEASING.md) for how to cut a signed release that the in-app
updater can consume.

## License

[MIT](LICENSE) © 2026 kwhorne
