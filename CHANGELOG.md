# Changelog

All notable changes to **Elyra Conductor** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1] — 2026-06-18

### Added

- **Persistent command history + cross-session search.** The command timeline is no longer
  session-only — every shell-integrated command (command, exit code, output tail) is saved
  to an app-internal SQLite store. The timeline modal gains a search box: type to find a
  past command across **every session** (“how did I fix this last time?”), matching both the
  command and its output.
- **Insights — where your time went.** A new tab in the timeline aggregates the history over
  Today / 7 days / All time: how many commands ran, how many failed, total time spent
  waiting, and the biggest time sinks per command (runs, average, failures).

### Changed

- **Shell integration is now on by default.** It powers the timeline, the persistent
  history, and “Fix it”, so new terminals capture real commands and exit codes out of the
  box. An explicit opt-out from a previous session is still honoured.

## [0.8.0] — 2026-06-18

### Added

- **🌳 Git worktrees — parallel branches, one agent each.** A new **Worktrees** button
  (and command palette entry) opens a panel to create isolated worktrees — separate
  checkouts that share the repo's `.git` — and open each as a terminal or an Elyra
  agent. Run several agents on different branches at once without collisions.
- **🤖 Agent command center.** Each Elyra agent reports a coarse state, surfaced as a
  per-tab dot (working / waiting on you / exited) and a pill in the tab strip showing,
  at a glance, how many agents are working and — the signal that matters — how many are
  **waiting for your input**. Click to jump straight to one; a notification fires when an
  agent starts waiting while you're looking elsewhere.
- **✅ GitHub PR status per branch.** With an authenticated `gh`, each worktree shows its
  open PR — number, CI check rollup (pass/fail/pending) and review state — linking to
  GitHub. Open PRs without a worktree are listed too, so you can **check a PR out as a
  worktree** (terminal or agent) in one click; PR branches are tracked from `origin`
  with their real contents.

### Fixed

- Context menus near the right/bottom edge of the screen now stay fully on-screen.
- The git panel now closes on **Escape** like the other modals.

## [0.7.8] — 2026-06-15

### Changed

- **Much snappier terminals.** PTY output now streams to the UI over a binary
  channel instead of JSON events. Previously every byte was serialized into a JSON
  array (~3.6× larger) and re-parsed on the UI thread for every frame — which made
  repaint-heavy TUIs like the Elyra agent feel sluggish next to a native terminal.
  Output is now shipped as raw bytes (zero-copy), cutting per-frame decode work on
  the UI thread by ~100×. Typing and rendering no longer compete with JSON parsing.

## [0.7.7] — 2026-06-10

### Added

- **⚡ Fix it — self-healing terminal.** When a command fails in a shell-integrated
  terminal, a quiet toast offers a one-click handoff to an Elyra agent with the full
  context — command, exit code, output tail, and git branch — and a fix-oriented prompt.
  Interrupts (Ctrl+C & friends) are ignored, and the offer expires on its own; it never nags.
- **🌅 Morning brief.** Open Conductor after a real break (4+ hours) and a welcome-back
  card shows where you left off: last project with git state (branch, dirty, ahead/behind),
  container health, and your last commands with failures flagged. One click to **resume
  the project** or have **Elyra plan your day** from the same context.
- **📖 Living runbooks.** Runbooks can now **verify themselves**: the new ✓ Verify button
  runs every runnable step headless (login shell, 60s timeout per step) and flags the ones
  that no longer work — per-step badges, failure output inline, and a freshness banner
  (“Verified 2h ago — all 4 steps green”) that survives restarts. Failing steps get their
  own ⚡ Fix button straight to Elyra. Mark always-on steps (dev servers) with
  ```` ```bash no-verify ````.

## [0.7.6] — 2026-06-10

### Changed

- **Terminal engine upgraded to xterm.js 6** — the WebGL renderer is now built into the
  core and used automatically, making heavy terminal output noticeably smoother.
- **Editor upgraded to Monaco 0.55**; Svelte, Vite and marked refreshed to their latest
  patch releases.

### Fixed

- File explorer: the visibility filter could read the entry list before it was
  initialized.
- **Dialogs & menus are more accessible** — every modal (command palette, file finder,
  commit dialog, context menu, …) is now properly focusable per ARIA, and
  close-on-outside-click no longer relies on event-propagation tricks.

### Internal

- Gradual TypeScript checking: `pnpm check` (svelte-check over plain JS/Svelte via
  `jsconfig.json`) now runs as a **quality gate** at the start of every release build —
  the build aborts on any type or accessibility finding. Codebase is at 0 errors,
  0 warnings.

## [0.7.5] — 2026-06-05

### Added

- **Ask Elyra about a command** — in the command timeline, each command (from a
  shell-integrated terminal) gets a **🤖** button that hands a context bundle — the command,
  exit code, git branch, and a tail of its **output** — to an Elyra agent. Great for
  “why did this fail?”. Conductor formats and forwards plain text; Elyra reasons.
- **Container-aware actions** — the 🐳 badge in the sidebar is now clickable: **shell into**
  a container, **tail its logs**, or **restart** it, each in a new terminal tab.
- **⇧⌘P** jumps to the project search in the sidebar.

### Fixed

- Arrow-key navigation in the command palette (⌘K) and scrollback search (⇧⌘F) now keeps
  the selected row in view — you can arrow through the whole list without the mouse.

## [0.7.4] — 2026-06-05

### Added

- **Runbook recorder** — hit **⏺ Record**, run your commands in a (shell-integrated)
  terminal, then stop to turn the captured sequence into a **runbook draft** — each command
  a runnable step, failed ones flagged with their exit code. “Do it once, then share it.”
  Saved to `.conductor/notes/` and opened for editing.
- **Release notes in the update toast** — when an update is available, a **What's new**
  toggle shows that version's changelog (the manifest now carries the real notes from
  CHANGELOG.md).

## [0.7.3] — 2026-06-05

### Added

- **Resizable query editor** — drag the handle between the SQL editor and the results to
  make the editor taller or shorter (great for long queries). The height is remembered.

## [0.7.2] — 2026-06-05

### Added

- **⇧⌘N** opens a new database query against the active (or first connected) connection.

### Fixed

- **Naming a saved query now works.** “⭐ Save” used `window.prompt`, which the Tauri
  webview ignores, so you couldn't enter a name. It now opens a proper dialog to name the
  query (private, per project).

## [0.7.1] — 2026-06-05

### Added

- **⌘N** opens a new terminal tab (in the active project) — no need to reach for the ＋
  button. Works from both terminal and editor focus.

## [0.7.0] — 2026-06-05

### Added

- **Shell integration (zsh), opt-in** — enable it from the command palette
  (“Enable shell integration”) and new terminals capture real **command lines** and
  **exit codes** via OSC 133/633. It works by pointing `ZDOTDIR` at a shim that sources
  your own `.zshenv` / `.zprofile` / `.zshrc` (your prompt and environment are untouched)
  and adds `precmd`/`preexec` hooks.
  - **Command timeline** now shows the full command and a ✓ / ✗ exit-code badge (falling
    back to the process-name view for non-integrated terminals).
  - **Health strip** shows a per-project **✓ / ✗ test** badge from the last test run
    (pest, phpunit, vitest, jest, pytest, cargo/go test, …).

## [0.6.6] — 2026-06-05

### Added

- **Detached-HEAD indicator** in the Git panel — when HEAD points at a commit rather than a
  branch (common with pinned Docker/OrbStack checkouts), a **⚠ detached** badge appears
  next to the branch picker, with a hover explaining that commits made here aren't on any
  branch and how to attach to one before committing.

## [0.6.5] — 2026-06-05

### Fixed

- **Git panel no longer shows “(detached)” for a branch with no commits yet.** It detected
  the current branch with `git rev-parse --abbrev-ref HEAD`, which reports `HEAD` for an
  unborn branch; it now uses `git symbolic-ref --short HEAD`, so the panel agrees with the
  sidebar and only shows detached on a genuine detached HEAD.

## [0.6.4] — 2026-06-05

### Added

- **About dialog** — a consistent in-app About (icon, version, and links to the website,
  GitHub, and developer). The macOS menu **Elyra Conductor → About Elyra Conductor** now
  opens this dialog instead of the native panel; also reachable from ⌘K.

### Fixed

- **“Run … (modal)” now reliably runs scripts.** A self-referential effect could remount
  the modal's terminal twice and kill the pty before the command ran; the run is now
  isolated so it behaves like the terminal-tab and external-terminal runners.

### Internal

- Release script auto-retries notarization: if Apple's notary queue stalls past ~12 min on
  a submission, it resubmits a fresh job (which is typically accepted in under a minute).

## [0.6.3] — 2026-06-05

### Added

- **SSH tunnels for remote databases** — a connection can tunnel through SSH. Tick **Use
  SSH tunnel** and fill in host, port, user, and an auth method (public key with optional
  passphrase, or password). Conductor runs a local port-forward via the system `ssh` and
  points the driver at it; secrets are provided via a one-shot askpass helper and stored
  in the Keychain with the rest of the connection.
- **Whole-table export** — in a table's Data view, **Excel** and **CSV** now export the
  *entire* table (not just the visible page), with the database field names as the header
  row. Active filters and ordering are respected.
- **Two-tier header** — the global toolbar and the work tabs now live on separate rows, so
  tabs get full width and scroll horizontally instead of being squeezed. Actions are
  grouped (View · Tools).
- **⌘T** toggles the database explorer.

## [0.6.2] — 2026-06-04

### Added

- **DB → Elyra bridge** — the database browser can hand structured context to an Elyra
  agent (text only). A **🤖 Elyra** button on a result/query sends the columns and first
  rows (and the SQL, in query mode); **Structure** sends the schema; and a per-row 🤖
  (on hover) sends a single row as a `column | value` table. The snippet is pre-filled in
  the agent so you add your question — “explain this row”, “write a migration”, “optimise
  this query”. Conductor only formats and sends text; Elyra reasons.

## [0.6.1] — 2026-06-04

### Added

- **Command timeline (🕘 Timeline)** — a flight recorder for your terminals: every finished
  foreground command is logged with its process, time, duration, and pane. Click an entry
  to jump back to that pane. Session-only; clearable.
- **Health strip 2.0** — each project row in the sidebar now also shows a pulsing **running
  dot** when a command is active in one of its tabs, and a **🐳 container badge**
  (`running/total`) for Docker containers mapped to the project via their Compose
  working-dir (hidden when Docker isn't available) — alongside the existing git status and
  port badges.

### Internal

- Modularisation (phase 1): pure, state-free helpers (`dirOf`, `baseOf`, run-command
  detection, idle-process detection, dev-task ranking) moved out of `App.svelte` into
  `src/lib/util.js`.

## [0.6.0] — 2026-06-04

A workflow release — the file tree, editor, finder, and Git all grow up.

### Added

- **Multi-file editor** — open files become tabs with an unsaved “•” marker; ⌘S saves
  (and formats on save for JSON/JS/TS/CSS/HTML/…); ⌘W closes the active tab; scroll and
  cursor are remembered per tab.
- **Fuzzy finder (⌘P)** — a quick file finder with a **Files** tab (fuzzy by name) and a
  **Text** tab (ripgrep content search across the project). ⇥ switches, ↑/↓ navigates,
  ↵ opens — text hits jump to the line.
- **Drag & drop in the file tree** — move files and folders between folders (or to the
  project root) by dragging.
- **Git panel (⌘G)** — stage/unstage/discard files (and “all”), a diff view, branch
  switching + create, stash (push/pop/drop), and commit / commit & push of the staged index.
- **Workspaces** — save the whole layout (tabs, panes, files) under a name and switch
  between them from the **⬡ Layout** menu.
- **Task dashboard (☰ Tasks)** — one-click run of scripts discovered in package.json,
  composer.json, Makefile, and justfile, grouped by source.
- **.env explorer (🔑 Env)** — view and edit `.env` files with masked secret values
  (reveal per row), add/remove keys; values stay in the file.

### Fixed

- Opening a file no longer leaves the terminal pane stuck narrow — terminals re-fit when
  the editor/Files/DB panels toggle, on tab switch, and on zoom.
- Opening a file could throw “ModelService: Cannot add model because it already exists”
  under a concurrent-open race; the editor now re-checks before creating a model.

## [0.5.1] — 2026-06-04

### Added (file explorer)

- **File operations** in the right-click menu: **Rename**, **Duplicate**, **New file**,
  **New folder**, **Reveal in Finder**, **Copy path**, and **Move to Trash**.
- Delete moves items to the **macOS Trash** (recoverable), never a permanent wipe.
- The tree refreshes automatically after a change — including inside expanded subfolders.
  New files open straight in the editor.

## [0.5.0] — 2026-06-04

A milestone release that caps a long run of features — runbooks, a universal
project runner, finished-command notifications, a full multi-engine database
browser, Developer ID signing + notarization, a port dashboard, global scrollback
search, pane navigation/zoom, and per-project health.

### Added

- **Send to Elyra from runbooks** — each shell block in a runbook gets a **🤖 Elyra**
  button (when the Elyra CLI is installed) that opens an agent panel with the snippet
  pre-filled in the composer, so you can add your question and send. Runbooks are now
  runnable, linkable *and* askable. Pure delegation — Conductor only hands Elyra text.

### Internal

- Code-quality pass: `cargo fmt` + a clean `cargo clippy`, and fire-and-forget IPC calls
  are guarded. No behavioural change.

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

[Unreleased]: https://github.com/kwhorne/elyra-conductor/compare/v0.8.1...HEAD
[0.8.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.8...v0.8.0
[0.7.8]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.7...v0.7.8
[0.7.7]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.6...v0.7.7
[0.7.6]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.5...v0.7.6
[0.7.5]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.4...v0.7.5
[0.7.4]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.3...v0.7.4
[0.7.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.6...v0.7.0
[0.6.6]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.5...v0.6.6
[0.6.5]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/kwhorne/elyra-conductor/compare/v0.4.9...v0.5.0
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
