# Architecture & boundaries

## The core boundary: Conductor dirigerer, Elyra resonnerer

Elyra Conductor is an **orchestrator / launcher / terminal multiplexer / host UI**.
It is **not** an AI agent. All intelligence lives in external programs — chiefly the
[Elyra](https://elyracode.com) coding-agent CLI — which Conductor runs and
displays the same way it runs Zed, iTerm, or git.

This boundary is a hard rule, not a preference. It keeps Conductor small, auditable,
and free of secrets, and it keeps the AI surface in one place (Elyra) where it is
versioned, extensible, and reusable.

### Conductor MUST NOT
- ❌ call an LLM or depend on an AI SDK
- ❌ store, read, or manage API keys / model credentials
- ❌ define prompts, system prompts, tools, or model configuration
- ❌ contain agent loops, reasoning, planning, or summarization logic
- ❌ grow a library of "AI prompt features"

### Conductor MAY
- ✅ start/stop processes (shells, editors, `elyra`)
- ✅ stream and render process I/O (PTY today; JSONL/RPC later)
- ✅ provide UI: tabs, split panes, dialogs, notifications, file tree, editor
- ✅ construct convenience commands (e.g. `elyra @file "…"`) — these are *shortcuts*
  that delegate to Elyra, not AI features. Prefer pushing such templates into Elyra
  (as commands / skills / extensions) when they grow beyond a one-liner.

> **Rule of thumb:** if a feature needs an API key or a model call, it belongs in
> Elyra, not in Conductor.

## How the Elyra integration respects the boundary

### Level 0 (implemented) — launch only
Conductor spawns the `elyra` binary in a terminal tab/pane (in the project's cwd).
Identical in spirit to "Open in Zed". No AI logic enters Conductor.

### Level 1 (implemented) — RPC host, not agent
Conductor spawns `elyra --mode rpc` (Rust JSONL bridge in `agent.rs`) and renders a
native panel (`AgentPanel.svelte`). To stay on the right side of the boundary,
Conductor acts strictly as a **host/renderer**:

- **Transport only:** read/write JSON-RPC (JSONL) over the child process's stdio.
- **UI mapping only:** Elyra's host requests map to UI —
  `notify`/`setStatus`/`setTitle` → tab title & status, `confirm`/`select`/`input`
  → native dialogs, agent-waiting → the existing notification rings,
  `get_session_stats` → token/cost display.
- **No intelligence:** all reasoning, model calls, keys, tools, and prompts remain
  inside the Elyra process. Conductor only forwards messages it displays and buttons
  the user presses.

Think of it as being a terminal emulator *for* an agent — not the agent.

### Level 2 (implemented) — one-shot ask, still transport
The inline terminal bar (`⌘↵`, `ask.rs`) pipes a prompt to `elyra --print` over **stdin**
and streams the answer back. Conductor assembles *context* — cwd, branch, recent commands
with exit codes, redacted scrollback — and the user supplies the question. No system prompt,
no model choice, no key. Tools are disabled, so it advises and the user acts: suggested
commands are *inserted* on the prompt line, never executed.

This is the boundary's stress test, because the obvious way to build it — a Settings pane
with providers and an API key — is exactly what the rule forbids. Two reasons it stayed out:
preferences live in `localStorage` (inside the webview we hardened in 0.9.1 precisely
*because* there was nothing there worth stealing), and for SSH sessions keys in Conductor
buy nothing, since the useful context is the local scrollback either way. Full reasoning in
[docs/architecture.md](docs/architecture.md#a-worked-example-terminal-help-).

> If someone asks for "AI providers in Conductor settings" again: the answer is a passthrough
> to `elyra`, not a second copy of its model registry.

## Layering (today)

```
Webview (Svelte)  — UI: sidebar, tabs, split panes, editor, file tree, dialogs
        │ Tauri IPC (commands + events)
Rust core         — PTYs, filesystem, git, process launching, updater
        │ child processes
External tools    — your shell, Zed/VS Code/Cursor, iTerm/Terminal, **elyra**
```

The AI sits one layer below Conductor, as an external tool. Keep it there.
