# Settings (`⌘,`)

`⌘,` opens Settings from anywhere — including while the editor has focus, since Monaco has
no claim on that shortcut. `Esc` or clicking outside closes it.

Everything here was previously reachable only as a command-palette toggle. Those palette
entries still work; they now change the same state.

## Appearance

| Setting | Notes |
| --- | --- |
| **Theme** | Dark or light. Applies to the UI and the terminal palette. |
| **Terminal font size** | 8–28px, shared by every pane. Same as `⌘+` / `⌘−` / `⌘0` in a terminal. |

## Terminal

| Setting | Notes |
| --- | --- |
| **Shell integration (zsh)** | Real commands, exit codes, and durations instead of guessed process names. Affects **new** terminals — see [Terminals](terminals.md#shell-integration-zsh). |
| **Save scrollback across restarts** | Persists each pane's recent output. Stored in `localStorage` as plain text on disk; credential-shaped output is masked first, but that is a mitigation, not a guarantee. Turn it off for a pane that will show secrets. See [Persistence](persistence.md). |
| **Notify when a background command finishes** | System notification for long runs in unfocused tabs — see [Terminals](terminals.md#finished-command-notifications). |

## Projects

**Scan folder** — the directory Conductor scans for projects (default `~/Code`). Changing it
rescans immediately. See [Projects & sidebar](projects.md).

## AI assistance

This section deliberately contains **no providers, models, or API keys**, and it says so
rather than leaving an unexplained gap.

Conductor is a host: it starts processes and renders their output. All reasoning happens in
the external [Elyra](https://elyracode.com) CLI, which already owns providers, models,
credentials, and prompts — configured once, in one place, and reusable outside Conductor.
Configure them with `elyra config`.

Keeping credentials out of this app is also what lets Conductor render untrusted text —
filenames, git output, scrollback — without a key being worth stealing. The hardening in
0.9.1 (CSP, DOMPurify, redaction) was written on the assumption that a sanitiser bypass
gets an attacker nothing valuable; storing keys in the webview's `localStorage` would
retroactively invalidate it.

What the section *does* show is the resolved path to the `elyra` binary and its version.
That is read-only on purpose: a configurable path is a setting that can be wrong, and
`find_bin` plus the login-shell `PATH` already handle the case it would exist to fix (a
GUI-launched app whose `PATH` lacks nvm/volta).

Terminal help itself lives at `⌘↵` in any pane — see
[Ask about this terminal](terminals.md#ask-about-this-terminal-).

## Where settings are stored

In `localStorage`, alongside session and workspace state. See
[State & persistence](persistence.md).

## Related

- [Keyboard shortcuts](keybindings.md) — every shortcut in one table.
- [Architecture & boundaries](architecture.md#the-core-boundary-conductor-orchestrates-elyra-reasons)
  — why there is no AI configuration here.
- [Command palette](command-palette.md) — the same toggles, reachable with `⌘K`.
