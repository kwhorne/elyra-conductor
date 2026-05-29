# Architecture & boundaries

Elyra Conductor is an **orchestrator / launcher / terminal multiplexer / host UI**. It is
**not** an AI agent. This page covers both the technical layering and the hard boundary
that keeps Conductor small and secret-free.

## The core boundary: Conductor orchestrates, Elyra reasons

All intelligence lives in external programs — chiefly the
[Elyra](https://elyracode.com) coding-agent CLI — which Conductor runs and displays the
same way it runs Zed, iTerm, or git. This is a hard rule, not a preference: it keeps
Conductor auditable and free of secrets, and keeps the AI surface in one place where it
is versioned and reusable.

### Conductor MUST NOT

- ❌ call an LLM or depend on an AI SDK
- ❌ store, read, or manage API keys / model credentials
- ❌ define prompts, system prompts, tools, or model configuration
- ❌ contain agent loops, reasoning, planning, or summarization logic
- ❌ grow a library of "AI prompt features"

### Conductor MAY

- ✅ start/stop processes (shells, editors, `elyra`)
- ✅ stream and render process I/O (PTY today; JSONL/RPC for the agent)
- ✅ provide UI: tabs, split panes, dialogs, notifications, file tree, editor
- ✅ construct convenience commands (e.g. `elyra @file "…"`) that delegate to Elyra

> **Rule of thumb:** if a feature needs an API key or a model call, it belongs in Elyra,
> not in Conductor.

## Layering

```
┌─────────────── Webview (Svelte 5) ───────────────────────────┐
│  Sidebar (projects)   Tabs + split panes   Editor   Files     │
└───────────────────────────────────────────────────────────────┘
        │  Tauri IPC (commands + pty:// / agent events)
┌─────────────── Rust core (Tauri 2) ───────────────────────────┐
│  pty.rs       spawn/write/resize/kill + byte streaming         │
│  projects.rs  scan folder, git, detect/launch editors          │
│  fs.rs        list_dir / read_file / write_file / list_tasks   │
│  agent.rs     JSONL transport to `elyra --mode rpc` (host only) │
└───────────────────────────────────────────────────────────────┘
        │  child processes
External tools — your shell, Zed/VS Code/Cursor, iTerm/Terminal, elyra
```

The AI sits one layer **below** Conductor, as an external tool. Keep it there.

## Terminal data flow

Rust spawns a PTY per pane and reads it on a dedicated thread, emitting bytes as
`pty://data/<id>` events. The frontend writes them straight to `xterm.js`. Input and
resize travel back via `invoke('pty_write' | 'pty_resize')`, and process exit is reported
via `pty://exit/<id>`.

## The layout model

The key design decision: **terminals live in a flat, absolutely-positioned layer** keyed
by terminal id, while a pure layout module (`layout.js`) computes their geometry from a
split tree.

- A node is either a `leaf` (`{ termId, key, cwd, title }`) or a `split`
  (`{ dir: 'row' | 'col', ratio, a, b }`).
- `geometry(root)` flattens the tree into absolutely-positioned leaves (in %) plus the
  dividers used for drag-to-resize.
- `splitLeaf`, `removeLeaf`, `setRatio`, `firstLeaf`, and `allLeaves` are pure tree ops.

Because splitting or resizing only changes CSS positions of existing DOM nodes,
**terminals are never remounted** — the PTY session and scrollback survive every
relayout. Rendering the tree recursively would have respawned shells on every change.

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
│   ├── App.svelte                # layout, tabs, splits, shortcuts, persistence
│   ├── app.css                   # theme tokens
│   ├── main.js
│   └── lib/
│       ├── Sidebar.svelte        # project list + search + "open in editor"
│       ├── Terminal.svelte       # xterm.js wrapper ↔ pty:// events + scrollback
│       ├── Editor.svelte         # Monaco quick-edit
│       ├── AgentPanel.svelte     # Elyra RPC host panel
│       ├── FileExplorer.svelte   # right sidebar root + header
│       ├── FileTree.svelte       # recursive, lazy-loaded file node
│       ├── CommandPalette.svelte # ⌘K palette
│       ├── CommitDialog.svelte   # git commit UI
│       ├── RunModal.svelte       # run-a-file modal terminal
│       ├── ContextMenu.svelte    # file-tree right-click menu
│       ├── ShortcutsModal.svelte # ⌘/ help
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
        ├── pty.rs                # PTY sessions
        ├── projects.rs           # project scan + git + editor launch
        ├── agent.rs              # elyra --mode rpc JSONL bridge
        └── fs.rs                 # directory + file + task commands
```

## Related

- [Tauri commands](tauri-commands.md) — the full IPC surface.
- [Elyra agent](elyra-agent.md) — how the host integration respects the boundary.
- [State & persistence](persistence.md) — what the frontend stores.
