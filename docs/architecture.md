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

### A worked example: terminal help (`⌘↵`)

This boundary gets tested by real requests. The most natural one is *"put AI in the
terminal — add a Settings pane for providers and an API key."* The feature is worth having;
the mechanism would break the rule. Here is how it was resolved, as a template for the next
time.

What was kept:

- ✅ a UI affordance — `⌘↵` opens a bar, `Esc` closes it
- ✅ **context assembly** from data Conductor already holds: cwd, git branch, recent
  commands and exit codes, and the redacted scrollback tail
- ✅ spawning `elyra --print` and streaming its stdout (`ask.rs`)

What was refused:

- ❌ a provider/API-key section in Settings
- ❌ a model picker, thinking level, or system prompt

Conductor supplies **facts**; the user supplies the **question**. There is no persona and no
system prompt — that is what keeps context assembly on the "convenience command" side of
the line rather than "defining prompts".

Two arguments settled it:

1. **Preferences live in `localStorage`, i.e. inside the webview.** The v0.9.1 hardening
   (CSP, DOMPurify, redaction) was written on the assumption that a sanitiser bypass gets
   an attacker nothing worth having. Storing credentials there would retroactively
   invalidate that assumption.
2. **For the SSH case, keys in Conductor buy nothing.** Conductor runs locally either way,
   so a local API call and a local `elyra --print` have identical access to the remote
   context — and that context is the scrollback, which Conductor already has.

The cost is real and accepted: terminal help does nothing without `elyra` on `PATH`. That
dependency already existed for the agent panel.

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
│  ask.rs       one-shot transport to `elyra --print` (⌘↵ bar)     │
└───────────────────────────────────────────────────────────────┘
        │  child processes
External tools — your shell, Zed/VS Code/Cursor, iTerm/Terminal, elyra
```

The AI sits one layer **below** Conductor, as an external tool. Keep it there.

## Untrusted content & the webview boundary

The webview holds IPC access to `pty_write`, `pty_spawn`, `write_file` and `run_step`.
Anything that can run script in it can therefore run code on your machine — so any
markdown Conductor renders is treated as **untrusted input**, because it is:

- **Runbooks** (`.conductor/notes/*.md`) are written by agents and travel with every
  repository you clone.
- **Release notes** come from GitHub; the updater signature covers the binary, not the
  note text.

Two independent layers guard this:

1. **Sanitising.** Both `{@html}` sinks go through DOMPurify (`src/lib/sanitize.js`).
   The runbook variant deliberately keeps the custom `cfile:` / `ctask:` schemes that
   `[[file]]` and `[[task:name]]` links depend on — DOMPurify strips unknown schemes by
   default, which would break the feature silently. `scripts/check-sanitize.mjs` asserts
   both halves (attacks stripped, links preserved) and runs as part of `pnpm check`.
2. **Content-Security-Policy** (`app.security.csp`). `script-src 'self'` with no inline
   scripts, `object-src`/`frame-src` `'none'`. Tauri injects its own nonces into
   `script-src`/`style-src` automatically, so its bootstrap still runs. `connect-src`
   must include `ipc:` — Tauri's IPC is a `fetch()` to an `ipc:` URL, so omitting it
   breaks every command. `style-src` needs `'unsafe-inline'` because Monaco and xterm
   inject stylesheets at runtime. Monaco's workers load from same-origin `/assets/`,
   so `worker-src 'self'` is sufficient — no `blob:` needed.

A separate `devCsp` relaxes just enough for the Vite HMR websocket, so the production
policy stays strict.

External links inside runbooks are handed to the OS browser via `open_url` rather than
followed in-place — a plain `<a href>` would navigate the whole webview away from the
app, leaving a window you can only escape by restarting.

### What is deliberately *not* locked down

The filesystem commands (`read_file`, `write_file`, `rename_path`, …) accept absolute
paths and are not scoped to a project root. That is intentional: Conductor is a file
manager and editor, and scoping would break opening a file outside the tree, the `~/Code`
scan, and “Reveal in Finder”. Their safety rests on the webview being trustworthy, which
is what the two layers above exist to guarantee — the fix for “script could touch any
file” is to stop script from running at all, not to fence off a file manager.

This has been reviewed and **settled**, not left undone: a security audit will flag
“unscoped filesystem access” every time, and the answer is the paragraph above. What
*would* reopen it is the premise changing — if the webview ever loaded remote content, ran
a third-party plugin, or rendered anything that escaped the sanitiser, then “the webview is
trustworthy” stops being true and scoping becomes worth its cost in broken features.

### One ordering invariant worth keeping

`Terminal.svelte` writes any restored scrollback **before** it registers `term.onData`.
That is not cosmetic. `onData` fires for terminal *replies* as well as keystrokes, so a
query sequence in the replayed bytes (`ESC[6n`, say) would have its answer forwarded
straight into the shell's stdin as though you had typed it. Two things happen to make that
unreachable — the serialize addon stores buffer *state*, so query sequences never survive
persistence, and the pty does not exist yet at that point — but both are incidental. The
ordering is the one guard that still holds if either changes.

## Threading model for commands

Commands that spawn a process, hit the network, walk the filesystem or query a database
are marked `#[tauri::command(async)]`. For a *synchronous* function that attribute does
not require an async rewrite — Tauri simply runs the body off the main thread instead of
on it, which is what keeps a ten-minute `run_step` from freezing the window. Per-keystroke
commands (`pty_write`, `pty_resize`) stay synchronous on purpose; a thread hop there would
only add latency.

One consequence is easy to miss: the macro wraps a sync body in an `async move` block, so
it executes *inside* the tokio runtime. `tauri::async_runtime::block_on` panics there
(“cannot block the current thread from within a runtime”), and the ClickHouse and SQL
Anywhere paths are sync functions that block on futures. They go through
`util::block_on_future`, which drives the future on a dedicated thread and is therefore
correct from either context. Code compiled fine without it and would have crashed only at
runtime, only on those two engines — so keep using the helper.

The session registries (`PtyManager`, `AgentManager`, `DbManager`) are locked through
`util::lock_recover`, which recovers a poisoned mutex rather than propagating the panic.
These are plain `HashMap`s of live sessions: a panic elsewhere in the command should not
take every terminal and database connection down with it until the app restarts.

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
