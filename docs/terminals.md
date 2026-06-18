# Terminals & panes

Every terminal in Conductor is a real PTY: one [`portable-pty`](https://crates.io/crates/portable-pty)
session per pane in Rust, rendered with [`xterm.js`](https://xtermjs.org/) in the
webview. Your login shell runs with `TERM=xterm-256color`, rooted at the pane's working
directory.

## Tabs

Each tab is an open session — either a **terminal** (with its own split layout) or an
**Elyra agent** panel. Open a new terminal tab with the **＋** button, `⌘K` → **New
terminal tab**, or by selecting a project.

- **Drag to reorder** tabs — grab a tab and drag it sideways; a blue insertion marker
  shows where it will land. (Reordering uses pointer events, so it works reliably
  inside the webview.)
- **Jump to a tab** with `⌘1`–`⌘9` (its position in the bar, left to right). The
  shortcuts follow whatever order you've dragged the tabs into.
- **Close** a tab with its **✕**, or close the focused pane with `⌘W` (closing the last
  pane closes the tab).
- A tab is labelled by its **project name**, so you can always tell which project it
  belongs to. When a foreground command is running (e.g. `⌘R` / `npm run dev`), a
  pulsing marker and a small process chip (`vite`, `php`, …) appear next to the name.

## Split panes

Split any pane to run multiple processes side by side:

| Action | Shortcut |
|--------|----------|
| Split right (vertical divider) | `⌘D` |
| Split down (horizontal divider) | `⇧⌘D` |
| Close active pane | `⌘W` |

Splits nest freely, and you can **drag the dividers** to resize. Each pane also has hover
controls in its top-right corner (split right / split down / close).

**Move between panes** with `⌘⌥` + the arrow keys, and **zoom** the active pane to fill the
tab with `⌘⌥Z` (press again to restore) — tmux-style. Switching tabs, splitting, or
navigating unzooms automatically.

## Global scrollback search

Press `⇧⌘F` (or the command palette → **Search all terminals**) to search the buffer of
*every* open terminal at once — not just the focused one. Matches are grouped by pane with
a count and a sample line; pick one to jump straight to that tab/pane with the match
highlighted. Great for “which of my six running services logged that error?”

### Why splitting never loses your session

Terminals live in a **flat, absolutely-positioned layer** keyed by terminal id. A pure
layout module (`layout.js`) computes each pane's geometry from a split tree. Because the
DOM node for a terminal is never remounted when the tree changes, **the PTY session and
scrollback survive** every split and divider drag. (Rendering the tree recursively would
have killed and respawned shells on every relayout.)

See [Architecture & boundaries](architecture.md) for the full data flow.

## Notification rings

Background tabs **pulse green** when their terminal produces new output, and the
indicator clears when you switch to the tab. All tabs stay alive in the background, so
nothing is lost when you switch away from a long-running process.

## Finished-command notifications

When a long-running command in a **background** tab returns to the shell — a build,
test run, or deploy completing while you're in another app — Conductor fires a native
notification (e.g. *“✓ cargo finished · my-project · ran 54s”*). It watches the
foreground-process titles it already polls, so there's nothing to configure per command:

- Only fires for commands that ran at least ~8 seconds (no noise from quick commands).
- Never fires for the tab you're actively watching (focused window + active tab).
- Toggle it from the command palette — **Notify when a background command finishes** /
  **Disable finished-command notifications**. The choice persists across restarts.

First use asks for macOS notification permission; you can also manage it later under
System Settings → Notifications → Elyra Conductor.

## Shell integration (zsh)

**On by default**, shell integration makes **new** terminals capture the real **command
line** and **exit code** of each command you run, via OSC 133/633 sequences. Toggle it from
the command palette (`⌘K` → *Enable/Disable shell integration*); your choice is remembered.
It powers the command timeline, the persisted history, and "Fix it".

It's safe with your existing setup: Conductor points `ZDOTDIR` at a small shim that sources
your own `.zshenv` / `.zprofile` / `.zshrc` (so your prompt — powerlevel10k, instant
prompt, aliases — is untouched) and then adds `precmd`/`preexec` hooks.

With it on:

- The **command timeline** (🕘) shows the full command and a ✓ / ✗ exit-code badge, and
  every command is saved to a searchable history — see
  [Command history & insights](command-history.md).
- The sidebar shows a per-project **✓ / ✗ test** badge from the last test run (pest,
  phpunit, vitest, jest, pytest, cargo/go test, …).

Only zsh is supported for now; other shells keep the lighter, process-name-based timeline.

## In-terminal search

Press `⌘F` while a terminal is focused to open a find bar. Use `↵` / `⇧↵` to step
through matches and `Esc` to close. Powered by the xterm search addon.

## Newline vs. submit (`⇧↵`)

Many TUIs use `⇧↵` to insert a newline while `↵` submits. Plain xterm.js can't send a
distinct sequence for modified Enter — it collapses `⇧↵`, `⌥↵`, and `⌃↵` to a bare
carriage return. Conductor fixes this by emitting the Kitty keyboard-protocol `CSI u`
sequence for modified Enter, so apps like the Elyra CLI receive `⇧↵` as a real newline
instead of submitting. No configuration is needed.

## Broadcast input (synchronize panes)

Toggle **⌁ Sync** in the top actions (or `⌘K` → **Broadcast input to all panes**) to
mirror your keystrokes to every pane in the active tab — tmux `synchronize-panes`-style.
Useful for running the same command across several services in a monorepo. Toggle it off
to return to typing in just the focused pane.

## Scrollback across restarts

Each pane's recent output is persisted and replayed as **read-only history** the next
time Conductor launches, with a fresh shell started beneath it. A live PTY cannot be
revived once the app closes, so this restores context, not a live process. Details and
limits are in [State & persistence](persistence.md).

## Running a command on open

Tabs opened for a [task](tasks.md) (or a file run) start a shell and then run the chosen
command once. This is the same mechanism behind **Run: …** in the command palette.
