# Changelog

All notable changes to **Elyra Conductor** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/kwhorne/elyra-conductor/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kwhorne/elyra-conductor/releases/tag/v0.1.0
