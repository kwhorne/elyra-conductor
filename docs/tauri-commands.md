# Tauri commands

The webview talks to the Rust core through Tauri's IPC: `invoke('<command>', args)` for
request/response, and `listen('<event>')` for streamed events. All commands are
registered in `src-tauri/src/lib.rs`.

## Command reference

| Command | Module | Purpose |
|---------|--------|---------|
| `pty_spawn` | `pty.rs` | Open a PTY for a pane (id, cwd, cols, rows) and start streaming. |
| `pty_write` | `pty.rs` | Write input bytes to a PTY. |
| `pty_resize` | `pty.rs` | Resize a PTY to new cols/rows. |
| `pty_kill` | `pty.rs` | Terminate a PTY session. |
| `pty_title` | `pty.rs` | Foreground process name of a PTY (for dynamic tab/pane titles); `null` for an idle shell. |
| `agent_spawn` | `agent.rs` | Spawn `elyra --mode rpc` for an agent tab (host only). |
| `agent_send` | `agent.rs` | Send a JSONL message to the agent's stdin. |
| `agent_kill` | `agent.rs` | Terminate an agent process. |
| `list_projects` | `projects.rs` | Scan the root folder; return projects + git branch. |
| `git_status` | `projects.rs` | Per-project dirty / ahead / behind state. |
| `git_changes` | `projects.rs` | List working-tree changes for the commit dialog. |
| `git_commit` | `projects.rs` | Stage, commit, and optionally push. |
| `git_worktree_list` / `git_worktree_add` / `git_worktree_remove` | `projects.rs` | Parallel isolated worktrees per branch. See [Worktrees](worktrees.md). |
| `detect_gh` / `gh_pr_list` | `projects.rs` | GitHub PR + CI status per branch via the `gh` CLI. |
| `run_step` | `projects.rs` | Run one runbook step headless (login shell, timeout) for [Verify](runbooks.md). |
| `history_add` / `history_query` / `history_stats` / `history_clear` | `history.rs` | Persistent command history & insights. See [Command history](command-history.md). |
| `detect_editors` | `projects.rs` | Find installed external editors (Zed / VS Code / Cursor). |
| `open_in_editor` | `projects.rs` | Launch a project in a chosen editor. |
| `detect_elyra` | `projects.rs` | Resolve the `elyra` binary via the login shell. |
| `detect_terminal` | `projects.rs` | Detect the external terminal app (iTerm2 / Terminal). |
| `run_in_external_terminal` | `projects.rs` | Run a file in the external terminal. |
| `home_dir` | `projects.rs` | Resolve `$HOME` for the default root. |
| `list_dir` | `fs.rs` | Directory listing for the file tree (directories first). |
| `read_file` | `fs.rs` | Read a file (backs the Monaco editor). |
| `write_file` | `fs.rs` | Write a file (backs `⌘S`). |
| `list_tasks` | `fs.rs` | Scan a project for runnable tasks. See below. |
| `list_runbooks` | `fs.rs` | List `.conductor/notes/*.md` for a project (creates the folder if missing). Backs [Runbooks](runbooks.md). |
| `write_bytes` | `fs.rs` | Write raw bytes to a path (binary exports like `.xlsx`). |
| `list_queries` / `save_queries` | `fs.rs` | Per-project, private saved queries in `.conductor/queries/` (auto-`.gitignore`d). |
| `db_from_env` | `db.rs` | Build a DB connection config from a project's `.env`. |
| `db_connect` / `db_disconnect` | `db.rs` | Open / close a MySQL or SQLite connection. |
| `db_tables` / `db_columns` | `db.rs` | Schema: list tables / a table's columns. |
| `db_query` | `db.rs` | Run SQL; returns columns + rows (or rows affected). Backs the [database browser](database.md). |
| `db_transfer_tables` | `db.rs` | Copy one or more tables from one open connection to another (structure and/or data); emits `db-transfer-progress` events. Backs [Tools ▸ Data Transfer](database.md#data-transfer-copy-a-database-or-just-some-tables). |

## Events

| Event | Payload | Meaning |
|-------|---------|---------|
| `pty://exit/<id>` | exit code (number) | The PTY's child process exited. |
| `agent://event/<id>` / `agent://exit/<id>` | JSON | Streamed agent output / exit. |
| `db-transfer-progress` | `{ table, index, total, rows_copied, done, error }` | Live progress while `db_transfer_tables` runs. |

PTY **output** is not an event: it streams over a binary `Channel<Response>` passed to
`pty_spawn`, which ships raw bytes (an `ArrayBuffer`) straight to xterm.js — far cheaper
than JSON-serializing every frame.

## `list_tasks`

```ts
invoke('list_tasks', { path: string }): Promise<Task[]>

type Task = {
  label: string;   // e.g. "dev"
  command: string; // e.g. "pnpm run dev"
  source: string;  // "package.json" | "composer.json" | "Makefile" | "justfile"
};
```

Scans `package.json` (with package-manager detection by lockfile), `composer.json`,
`Makefile`/`makefile`/`GNUmakefile`, and `justfile`/`Justfile`/`.justfile`. See
[Tasks](tasks.md) for the parsing rules.

## Adding a command

1. Write a `#[tauri::command]` function in the appropriate module under `src-tauri/src/`.
2. Register it in the `tauri::generate_handler![ … ]` list in `lib.rs`.
3. Call it from the frontend with `invoke('your_command', { …args })`.

Keep new commands within the [boundary](architecture.md): transport, filesystem, process
launching, and UI support — never model calls or secret handling.
