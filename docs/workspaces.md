# Workspaces

A **workspace** is a named snapshot of your entire layout. Save the setup you use for a
particular task and reopen it later in one action.

## What a workspace captures

A workspace stores the same shape as the auto-saved session:

- the **projects folder** (root) and the **active project**,
- every **tab** — terminals (with their full split-pane tree, working directories, and
  titles) and Elyra agent tabs,
- which tab was **active**,
- **pinned** projects,
- UI state: **theme**, file-sidebar visibility, hidden-files toggle, and the open editor
  file.

Workspaces are **global** — independent of which root folder is currently selected.

## The Layout menu

The top bar has a **⬡ Layout** button (it shows the active workspace name once one is
loaded). Click it to **Save current layout…**, switch to any saved workspace, or delete
one. The same actions are also in the command palette (`⌘K`).

## Saving a workspace

1. Arrange your tabs, splits, and open project the way you want.
2. **⬡ Layout** → **Save current layout…** (or `⌘K` → **Save workspace…**).
3. Enter a name. Saving again with the same name overwrites it.

## Loading a workspace

Pick a workspace from the **⬡ Layout** menu (or `⌘K` → **Load workspace: `<name>`**). If
the workspace was saved against a different projects folder, Conductor switches the root
and re-scans automatically before applying the layout.

> Loading a workspace replaces the current tabs. Because terminals are real shells that
> cannot be revived, the panes come back with the same structure and working directories
> (and replayed scrollback), but as fresh sessions.

## Deleting a workspace

Open the command palette (`⌘K`) → **Delete workspace: `<name>`** and confirm.

## Where workspaces are stored

Workspaces live in `localStorage` under `conductor:workspaces` as a map of
`name → snapshot`, reusing the exact serialization format of the auto-saved session
(`conductor:state`). See [State & persistence](persistence.md) for the snapshot shape.

## Related

- [Command palette](command-palette.md) — where all workspace actions live.
- [State & persistence](persistence.md) — the serialization details.
