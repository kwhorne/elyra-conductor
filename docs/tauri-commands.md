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

## Events

| Event | Payload | Meaning |
|-------|---------|---------|
| `pty://data/<id>` | bytes | Output from a PTY; written straight to xterm.js. |
| `pty://exit/<id>` | exit code (number) | The PTY's child process exited. |

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
