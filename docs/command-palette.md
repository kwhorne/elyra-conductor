# Command palette

The command palette is the fastest way to do anything in Conductor. Open it with `⌘K`
and start typing — it fuzzy-matches across titles, hints, and groups.

## Opening and using it

- **Open / close:** `⌘K`
- **Filter:** type any part of a command's title, hint, or group.
- **Run:** select an entry to execute it immediately.

## What's in the palette

The palette aggregates several groups:

| Group | Entries |
|-------|---------|
| **project** | Every project in the current root — selecting one opens/focuses its tab. |
| **tab** | Every open tab — jump straight to it. |
| **task** | Discovered tasks for the active project, shown as **Run: `<name>`**. See [Tasks](tasks.md). |
| **workspace** | **Load workspace: …** and **Delete workspace: …**. See [Workspaces](workspaces.md). |
| **action** | All the commands below. |

### Actions

- **New terminal tab** and (when Elyra is detected) **New Elyra agent here**
- **Split right** (`⌘D`), **Split down** (`⇧⌘D`), **Close pane** (`⌘W`)
- **Show / hide editor**, **Toggle file sidebar** (`⌘B`)
- **Show all files / hide node_modules·.git in tree**
- **Switch to light / dark theme**
- **Broadcast input to all panes** / **Stop broadcasting input**
- **Quick edit file…**, **Change projects folder…**
- **Git: commit `<project>`…** (when the active project is a repo)
- **Keyboard shortcuts** (`⌘/`), **Check for updates…**
- **Reset saved layout** — clears the persisted session and reloads
- **Save workspace…**
- **Open `<project>` in `<editor>`** — one per detected editor

## Tips

- Searching `ws` surfaces workspace actions; searching a script name surfaces its task.
- The palette is the canonical home for actions that don't have a toolbar button or
  shortcut, so when in doubt, press `⌘K`.

## Related

- [Keyboard shortcuts](keybindings.md)
- [Tasks](tasks.md) · [Workspaces](workspaces.md)
