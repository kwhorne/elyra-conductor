# Elyra agent

If the [Elyra](https://elyracode.com) coding-agent CLI is installed, Conductor can open a
**native agent panel** in a tab and drive it over JSON-RPC. Crucially, Conductor remains a
**host** for the agent — it never becomes the agent. All reasoning, model calls, keys,
prompts, and tools stay inside the Elyra process.

> See [Architecture & boundaries](architecture.md) for why this boundary is a hard rule.

## Detection

On startup Conductor runs `detect_elyra` to find the `elyra` binary. Binaries are
resolved through your **login shell**, so an `elyra` installed via nvm/Volta/asdf is
found even when Conductor is launched from the GUI (Finder/Dock), not just from a
terminal. When detected, agent entry points appear in the UI; otherwise they are hidden.

## Opening an agent

- **Sidebar:** the agent button on a project.
- **Command palette:** `⌘K` → **New Elyra agent here**.
- **File tree right-click:** **New Elyra agent here** (on a folder), or **Ask Elyra about
  this file** (on a file) — which opens an agent tab pre-seeded with a prompt about that
  file.

Each agent runs in its own tab, rooted at the chosen directory.

## How it works (Level 1 — RPC host)

Conductor spawns `elyra --mode rpc` and communicates over the child process's stdio using
line-delimited JSON-RPC (JSONL). The Rust bridge lives in `agent.rs` (`agent_spawn`,
`agent_send`, `agent_kill`); the panel is `AgentPanel.svelte`.

To stay strictly a host/renderer, Conductor only does **transport** and **UI mapping**:

| Elyra host request | Conductor UI |
|--------------------|--------------|
| `notify` / `setStatus` / `setTitle` | tab title and status |
| `confirm` / `select` / `input` | native dialogs |
| agent-waiting | the existing notification rings |
| `get_session_stats` | token / cost display |

No intelligence enters Conductor: it forwards messages it displays and the buttons you
press. Think of it as a terminal emulator _for_ an agent.

The agent panel also has a **Restart** button. The RPC child is given the login-shell
`PATH` so its Node runtime is found in GUI launches.

## Levels recap

- **Level 0 — launch only:** spawn `elyra` in a terminal tab (same spirit as "Open in
  Zed"). No AI logic in Conductor.
- **Level 1 — RPC host:** the native panel described above. Still host-only.

## Related

- [Architecture & boundaries](architecture.md)
- [Tauri commands](tauri-commands.md) — `agent_spawn`, `agent_send`, `agent_kill`,
  `detect_elyra`.
