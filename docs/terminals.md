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
- A tab's title reflects the foreground process of its panes (e.g. `vim`, `bun`),
  falling back to the project/shell name.

## Split panes

Split any pane to run multiple processes side by side:

| Action | Shortcut |
|--------|----------|
| Split right (vertical divider) | `⌘D` |
| Split down (horizontal divider) | `⇧⌘D` |
| Close active pane | `⌘W` |

Splits nest freely, and you can **drag the dividers** to resize. Each pane also has hover
controls in its top-right corner (split right / split down / close).

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
