# Using Conductor

Conductor is a single-window cockpit for your local projects. This page describes the
main UI areas and the everyday workflow.

## The window at a glance

```
┌────────────┬───────────────────────────────────────────┬───────────┐
│ Sidebar    │  Tabs  ───────────────────────  Top actions │  Files    │
│ (projects) ├───────────────────────────────────────────┤  (tree)   │
│            │                                             │           │
│ Pinned     │           Split panes (terminals,           │           │
│ Projects   │            agent panels, editor)            │           │
│            │                                             │           │
└────────────┴───────────────────────────────────────────┴───────────┘
```

- **Left sidebar** — the project switcher with fuzzy search, git status per repo, and
  pinned favourites. See [Projects & sidebar](projects.md).
- **Tabs** — one tab per open terminal session or Elyra agent. Drag to reorder; a tab
  pulses green when a background terminal produces output.
- **Top actions** — search (`⌘K`), quick edit, show/hide editor, toggle files, broadcast
  input (**⌁ Sync**), theme toggle, keyboard help, and (for git repos) commit.
- **Panes** — the active tab's split layout. Each pane is a real terminal, an agent
  panel, or the Monaco editor.
- **Right sidebar (Files)** — a lazy-loaded file tree of the active project. Toggle with
  `⌘B`. See [Files & editor](files-and-editor.md).

## Typical workflow

1. **Pick a project** from the sidebar (or `⌘K` → its name). A terminal tab opens in
   that folder. Selecting a project that is already open just focuses its tab.
2. **Split panes** to run several processes side by side — a dev server, a test watcher,
   a shell. The PTY and scrollback survive splits and divider drags.
3. **Run a task** with `⌘K` → **Run: …** (scripts discovered from the project — see
   [Tasks](tasks.md)).
4. **Browse and edit files** in the right sidebar, or open the project in your real
   editor with the per-project **zed / code / cursor** buttons.
5. **Commit** with the in-app dialog when a repo is dirty. See [Git](git.md).
6. **Save the layout** as a workspace if you want to come back to exactly this setup —
   see [Workspaces](workspaces.md).

## Sessions persist automatically

Conductor remembers your tabs, split layout, active project, theme, and open file across
restarts (via `localStorage`). Terminals come back with the same structure and working
directory, and recent scrollback is replayed as read-only history. See
[State & persistence](persistence.md) for exactly what is stored.

## Keyboard-first

Almost everything is reachable from the keyboard. Start with `⌘K` (command palette) and
`⌘/` (shortcut help). The full list is in [Keyboard shortcuts](keybindings.md).
