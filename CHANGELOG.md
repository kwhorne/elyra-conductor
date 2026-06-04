# Changelog

All notable changes to **Elyra Conductor** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.9] — 2026-06-04

### Added (database browser)

- **Connection groups** — give a connection a *Group* and the panel organises connections
  into collapsible folders (ungrouped first).
- **Test connection** — a button in the connection form verifies host/port/credentials/TLS
  without saving, showing ✓ OK or the error.
- **Query history** — a query tab keeps a per-project history of the queries you've run;
  pick one from the **History…** dropdown to load it back (last 50, newest first).

## [0.4.8] — 2026-06-03

### Added

- **Project health: running servers in the sidebar.** Each project now shows a green
  **⚡port** badge for any local server whose working directory is inside it (detected
  via `lsof`, so it works with any stack — no config). Click a badge to open
  `http://localhost:<port>`. Alongside the existing git status, the sidebar is now a
  glanceable health view: what has changes, and what's running (and where).

## [0.4.7] — 2026-06-03

### Added

- **Global scrollback search** (`⇧⌘F`) — search every open terminal's buffer at once.
  Results are grouped by pane (with match counts and a sample line); jump straight to a
  match, which switches to that tab/pane and highlights it.
- **Pane navigation** (`⌘⌥`←↑↓→) — move keyboard focus between split panes by direction.
- **Pane zoom** (`⌘⌥Z`) — temporarily maximise the active pane (tmux-style); switching
  tabs, splitting, or navigating unzooms.

## [0.4.6] — 2026-06-03

### Added

- **Port dashboard** — a **⚡ Ports** button (and command palette entry) lists the local
  TCP ports currently listening, with their process and PID. Click a port to open
  `http://localhost:<port>`, or stop the process (SIGTERM). Refreshes live while open.
- **Per-project accent colour** — each tab gets a stable colour (derived from the
  project path) on its left edge, so you can tell at a glance which project a tab
  belongs to.
- **Sound on finished-command notifications** — the native notification when a
  background command completes now plays the default sound.

## [0.4.5] — 2026-06-03

### Changed

- **The macOS app is now Developer ID signed and Apple-notarized.** Downloads open
  normally — no more “damaged” or “unidentified developer” warnings, and no quarantine
  workaround needed.
- **Bundle identifier is now `com.gets.elyra-conductor`** (GETS AS, Team `7G383N3VY7`).
  macOS treats this as a fresh app, so locally saved window state starts clean once.

## [0.4.4] — 2026-06-03

### Added

- **Editable connections** — edit a saved connection (host/port/credentials/TLS) with the
  ✎ action; it reconnects and re-saves.
- **Edit cells with NULL & long values** — double-clicking a cell now opens a small
  editor dialog with a multi-line field and a **Set to NULL** option (`⌘↵` saves).
- **CSV export** — alongside Excel, on any table or query result.
- **Table metadata** — the Structure view shows the (approximate) **row count** and
  **on-disk size** for the table.
- **TLS for remote PostgreSQL & ClickHouse** — a **Use TLS** option (with an optional
  **Skip certificate verification** for self-signed/internal hosts) in the connection
  form. Both use the system TLS stack (macOS Secure Transport); ClickHouse keeps its
  native protocol over the encrypted stream.

### Fixed

- **“Elyra Conductor is damaged” on download.** The bundled app only carried the Rust
  linker's ad-hoc signature, which fails `codesign` validation — so a downloaded
  (quarantined) copy was rejected by Gatekeeper as *damaged*. Builds now ad-hoc sign the
  app properly (`bundle.macOS.signingIdentity = "-"`), producing a valid signature.

  > The app still isn't Apple-notarized, so first launch needs one of: right-click →
  > **Open**, or remove the quarantine flag:
  > `xattr -dr com.apple.quarantine "/Applications/Elyra Conductor.app"`.

## [0.4.3] — 2026-06-03

### Added

- **Multiple database connections per project.** The DB panel is now a tree of
  connections — keep e.g. MySQL (app data) and ClickHouse (BI/reports) open at the
  same time. Add connections with **＋** (from `.env` or manually); each expands to its
  own table list and has per-connection actions (new query, refresh, disconnect,
  remove).
- **Secure, persistent connections.** Saved connections (including passwords) are
  stored in the **OS keychain** (macOS Keychain), keyed per project — nothing is
  written into the project folder, so nothing can be committed. They reappear when you
  reopen the project.
- **`ORDER BY` field** in the table view, next to `WHERE` — type a full clause
  (e.g. `created_at DESC, id`). Column-header sorting still works and an explicit
  `ORDER BY` takes precedence.

## [0.4.2] — 2026-06-02

### Added

- **ClickHouse** support in the database browser, over the **native TCP protocol**
  (port 9000) — the same protocol `clickhouse-client` uses, not HTTP. Connect manually
  (or via `DB_CONNECTION=clickhouse` in `.env`), browse tables, sort, filter
  (incl. per-column), page, run queries, view structure, and export to Excel.
  - Uses a dynamic row reader so arbitrary queries work without compile-time schemas;
    all ClickHouse types (Decimal, Date/DateTime64, UUID, Enum, Array, Tuple, …) render
    correctly.
  - ClickHouse tabs are **read-only** (no inline cell editing) — ClickHouse isn't an
    OLTP row-update store; browsing, querying, and export are fully supported.

## [0.4.1] — 2026-06-02

### Added

- **PostgreSQL** support in the database browser (alongside MySQL and SQLite).
  Connect from `.env` (`DB_CONNECTION=pgsql`, port 5432) or manually. Uses Postgres'
  simple query protocol so values come back as text — arbitrary queries just work.
  (Phase 1 is non-TLS / local; remote TLS can be added later.)
- **Per-column filters** — a filter row under the table headers; type in any column to
  narrow results (`CAST(col AS TEXT) LIKE '%…%'`, combined with the `WHERE` box).
- **Editable cells** — double-click a cell to edit it in place; Enter saves via an
  `UPDATE … WHERE <primary key>`. Requires the table to have a primary key (detected
  automatically); Esc cancels.
- **Structure view** — a **Data / Structure** toggle on a table tab; Structure lists the
  columns with type, nullability, and key. Primary-key columns are marked with 🔑.

## [0.4.0] — 2026-06-02

### Added

- **Database browser** (phase 1: **MySQL** + **SQLite**) — connect to a project's
  database and explore it without leaving Conductor. Toggle the **DB** panel from the
  toolbar or command palette.
  - **Connect from `.env`** — reads `DB_CONNECTION` / `DB_HOST` / `DB_PORT` /
    `DB_DATABASE` / `DB_USERNAME` / `DB_PASSWORD` (Laravel conventions), or set up a
    manual connection. Nothing new is stored — Conductor just reads the `.env` that's
    already there; manual credentials stay in memory for the session.
  - **Table list** in the side panel; opening a table fills the main window as its own
    tab with a **data grid** — click a column to **sort**, type a `WHERE` condition to
    **filter**, page through results, refresh, copy cells, and see row count + timing.
  - **Query tab** — a SQL editor (run with `⌘↵`); non-`SELECT` statements report rows
    affected.
  - **Export to Excel** (`.xlsx`) of any table or query result (values kept as text so
    IDs and zip codes with leading zeros survive).
  - **Saved queries** — save a query for reuse. Stored **per project and private**
    (`<project>/.conductor/queries/`, auto-`.gitignore`d so they're never committed).

  Like everything in Conductor, this is a tool — it connects, lists, and queries; it
  never calls a model.

## [0.3.3] — 2026-06-01

### Changed

- **Tabs show the project name instead of the running process.** A tab's label is
  now its project/identifier name, so you can always tell which project it belongs to
  (previously the label was replaced by the foreground process, e.g. `php` or `node`,
  making several tabs indistinguishable).
- **Running marker** — when a tab is running a foreground command (e.g. `⌘R` /
  `npm run dev`), it shows a pulsing dot and a small process chip (`vite`, `php`, …)
  next to the project name, so you can see at a glance which projects are live.

## [0.3.2] — 2026-06-01

### Fixed

- **Commands launched into a terminal tab now actually run.** Running a file, a
  task (**Run: …** / `[[task:…]]`), or **Start project** (when it opens a new tab)
  silently did nothing: the pane-layout helper dropped the leaf's `runOnce` command
  (and its `key`), so the terminal spawned but never received the command.
  - The command is now executed by the shell itself at startup
    (`$SHELL -lic '<cmd>; exec $SHELL -li'`) rather than typed into the PTY, which
    also fixes shells whose startup (oh-my-zsh, instant prompt) swallowed the
    keystrokes. Full login + interactive environment, so `ssh`, `PATH`, agents, etc.
    are all present — a deploy script behaves exactly as in your own terminal.
  - The same layout fix restores **per-pane scrollback persistence** (the pane `key`
    was being dropped too).

### Added

- File right-click: **Run `<file>` in a terminal tab** — runs a script in a real,
  persistent, interactive terminal tab (ideal for deploy scripts you want to watch and
  interact with), alongside the existing modal and external-terminal options.

## [0.3.1] — 2026-05-31

### Added

- **Finished-command notifications** — get a native notification when a long-running
  command in a **background** tab returns to the shell (e.g. a build or test run
  completes while you're in another app). Uses the foreground-process titles
  Conductor already polls; only fires for commands that ran ≥ 8s and never for the
  tab you're actively watching. Toggle it from the command palette
  (**Notify when a background command finishes**); the setting persists.

## [0.3.0] — 2026-05-31

### Added

- **Start project (`⌘R`)** — one action to run any project's dev command, whatever
  the stack. Conductor already resolves the right command per task source; now it
  picks the best one automatically:
  - Ranks detected `dev` / `start` / `serve` / `watch` tasks across `package.json`
    (with package-manager detection), `composer.json`, `Makefile`, and `justfile`.
  - When several are equally likely (e.g. a Laravel app with both `composer run dev`
    and `npm run dev`), it shows a quick picker and **remembers your choice**.
  - **▶ button on every project** in the sidebar, a command-palette **Start project**
    action, and a per-project override via **Set start command…** (also on a folder's
    right-click menu) for cases the heuristic can't know (e.g. `php artisan serve`,
    `pnpm tauri dev`).

  Runs in the project's terminal (reusing a pane when possible). Pure detection and
  launch — no AI.

## [0.2.0] — 2026-05-31

### Added

- **Runbooks** — local, project-scoped markdown notes that are *runnable*. Open one
  from the command palette (**Open project runbook**) or a folder's right-click
  menu (**Open runbook here**). Runbooks live in `<project>/.conductor/notes/*.md`,
  so they're versionable with git.
  - **▶ Run** on shell code fences (` ```bash `, `sh`, `zsh`, …) sends the command
    to the project's terminal (reusing an existing pane, or opening a new tab), with
    a **⧉ Copy** button on every block.
  - **`[[file]]` links** open a file in the editor (`[[path|label]]` for a custom
    label; relative paths resolve against the project root).
  - **`[[task:name]]` chips** run a discovered project task (npm/composer/make/just),
    falling back to running the literal text.
  - **Edit/Preview** with save (`⌘S`) to `.conductor/notes/`, a runbook picker, and
    **＋ New** from a starter template. Runbook tabs persist across restarts.

  Conductor stays a host: runbooks only *run and display* — no LLM, keys, or prompts.

## [0.1.9] — 2026-05-30

### Fixed

- **No more multi-second freeze (beachball) when refocusing the app.** Returning
  to Conductor refreshed git status for every scanned repo at once, and each
  refresh spawned three `git` subprocesses — dozens of repos meant a burst of
  hundreds of processes on every focus. Now:
  - `git_status` uses a **single** `git status --porcelain=v2 --branch` call
    (branch + ahead/behind + dirty) instead of three separate git invocations.
  - The frontend refresh runs through a **concurrency-capped worker pool**
    (max 6 at a time) instead of firing every repo in parallel.

## [0.1.8] — 2026-05-30

### Changed

- **Run a file (right-click → Run …)** is now a proper runner instead of a fixed
  `./file` that failed without the execute bit:
  - **Smart command** picked from the file extension (`.py` → `python3`, `.js` →
    `node`, `.sh` → `bash`, `.ts` → `npx tsx`, …), falling back to `./file`.
  - **Editable command** field — tweak it and **Run** / **Re-run** (`⌘↵`).
  - **Stops on error:** a non-zero exit keeps the modal open with the output
    visible so you can read it, instead of auto-closing. Successful runs still
    close on their own.

### Performance

- **Scrollback is only re-serialized when a pane actually produced output**,
  instead of every 4 s regardless — removes continuous CPU/IO for idle panes.
- **Tab-title polling pauses while the window is in the background** (`document.hidden`).
- **State and scrollback are flushed on window close** (`pagehide`/`beforeunload`),
  since Svelte's `onDestroy` is not guaranteed to run when the Tauri window quits.

## [0.1.7] — 2026-05-30

### Added

- **Switch tabs with `⌘1`–`⌘9`** — jump straight to a tab by its position in the
  bar (`Ctrl`+number on Linux/Windows).

### Fixed

- **Reorder tabs by dragging** — tab drag now uses pointer events instead of HTML5
  drag-and-drop, which frequently never started inside the Tauri/WebKit webview.
  A blue insertion marker shows exactly where the tab will land, and `⌘1`–`⌘9`
  follow the order you choose.
- **`⇧↵` inserts a newline in terminal tabs** — Conductor now emits the Kitty
  `CSI u` sequence for modified Enter (`⇧↵`, `⌥↵`, `⌃↵`) so TUIs like the Elyra CLI
  receive it as a real "new line" instead of submitting. xterm.js otherwise
  collapses modified Enter to a bare carriage return, indistinguishable from `↵`.

## [0.1.6] — 2026-05-29

### Added

- **Workspaces** — save the entire layout (projects, tabs, split panes, the open
  file, and theme) as a named workspace and reopen it later from the command
  palette (**Save workspace…** / **Load workspace: …** / **Delete workspace: …**).
  Workspaces are global across project folders; loading one switches the root and
  re-scans automatically when needed.
- **Per-project tasks** — Conductor scans the active project for runnable tasks in
  `package.json` (with package-manager detection via lockfile), `composer.json`,
  `Makefile`, and `justfile`, and lists them in the command palette as **Run: …**.
  Selecting one launches it in a fresh terminal tab. New Rust command `list_tasks`.
- **Broadcast input** — a **⌁ Sync** toggle (toolbar and palette) mirrors your
  keystrokes to every pane in the active tab, tmux `synchronize-panes`-style.
- **Scrollback restore** — each pane's recent output is persisted (capped per pane)
  and replayed as read-only history on the next launch, with a fresh shell started
  beneath it. Orphaned buffers from closed panes are pruned automatically.

> All AI continues to live in external tools (e.g. the Elyra CLI). Conductor
> remains a host/launcher and never calls a model — see
> [ARCHITECTURE.md](ARCHITECTURE.md).

## [0.1.5] — 2026-05-29

### Fixed

- **Elyra agent (RPC):** give the `elyra --mode rpc` child process the login-shell
  `PATH` so a Node runtime installed via nvm is found when Conductor is launched
  from the GUI. Added a **Restart** button to the agent panel.

## [0.1.4] — 2026-05-29

### Fixed

- **Binary detection:** resolve external binaries through the login shell so an
  nvm-installed `elyra` is discovered in GUI (Finder/Dock) launches, not just when
  started from a terminal.

## [0.1.3] — 2026-05-29

### Added

- **Elyra agent integration (Level 0 + Level 1):** detect the `elyra` CLI, open
  agent tabs, and "Ask Elyra about this file" from the file tree. Level 1 adds a
  native agent panel driven by `elyra --mode rpc` over JSON-RPC — streamed replies,
  tool activity, confirm/select/input prompts, and notification rings — with
  Conductor acting strictly as the host/renderer.

### Changed

- **Documentation:** codified the core boundary in `ARCHITECTURE.md` and the README
  — *Conductor orchestrates; it never reasons.* All model calls, keys, prompts, and
  tools stay inside Elyra.
- Renamed the README title to **Elyra Conductor** and pointed Elyra links to
  <https://elyracode.com>.

## [0.1.2] — 2026-05-29

### Changed

- **Display name:** set the product name and window title to *Elyra Conductor*
  while keeping a stable binary/updater name.
- **Release tooling:** more robust release script — clean DMG mount handling and a
  `CI=true` path that skips the flaky AppleScript step.

### Performance

- **Lazy-load Monaco:** load the editor on demand, cutting the startup bundle from
  ~3.7 MB to ~412 KB, and added a startup splash screen.

## [0.1.1] — 2026-05-29

### Added

- **Auto-update:** in-app updates from GitHub Releases via the Tauri updater
  (signed), with a one-click install. See [RELEASING.md](RELEASING.md).
- **Pinned projects:** pin favourites to a sticky section at the top of the sidebar,
  persisted across project-folder changes.

## [0.1.0] — 2026-05-29

Initial public release — a cross-platform (Tauri + Rust + Svelte) project conductor:
project switcher, real PTY terminals, split panes, file tree, and quick-edit.

### Added

- **Terminals & panes:** one PTY per pane (`portable-pty` + `xterm.js`), split
  horizontally/vertically with nesting and drag-to-resize, and a command palette
  (⌘K) for jumping between projects, tabs, and actions.
- **Project switcher:** scans a folder (default `~/Code`), shows per-repo git status
  (branch, dirty state, ahead/behind), refresh button, and throttled auto-refresh on
  window focus.
- **File sidebar (⌘B):** lazy-loaded recursive file tree with a toggle to hide
  `node_modules`/`.git`/build noise (persisted). Right-click to run a file in a modal
  terminal or an external terminal (iTerm2 / Terminal.app).
- **Run modal:** shows full process output, surfaces the exit code (green/red), and
  auto-closes when the command finishes.
- **Editor:** inline Monaco quick-edit with a close button and focus-aware shortcuts
  so the editor owns its own keys.
- **Git commit dialog:** review changes, write a message, and commit (with optional
  push) without leaving the app.
- **Terminal search (⌘F):** in-terminal find via the xterm search addon.
- **Dynamic titles:** tab/pane titles reflect the PTY foreground process.
- **Notification rings:** background tabs pulse on new activity; all tabs' terminals
  stay alive across switches.
- **Light/dark theme:** toggle across CSS variables, xterm, and Monaco (persisted).
- **Drag-to-reorder tabs** and a **keyboard-shortcuts help modal** (⌘/).
- **Session persistence:** tabs, splits, and UI state restored across restarts
  (localStorage), plus a reset action.
- **Bundled JetBrains Mono** for the terminal and editor.
- **Open in your editor:** auto-detect installed editors and launch in Zed, VS Code,
  or Cursor.

### Changed

- Upgraded to Vite 8 + `@sveltejs/vite-plugin-svelte` 7 for faster builds.

### Fixed

- **Run modal:** use a dot-free PTY id so Tauri event names accept it and output
  streams correctly.

[Unreleased]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.9...HEAD
[0.4.9]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.8...v0.4.9
[0.4.8]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.9...v0.2.0
[0.1.9]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kwhorne/elyra-conductor/releases/tag/v0.1.0
