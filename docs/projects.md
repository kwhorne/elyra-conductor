# Projects & sidebar

The left sidebar is the project switcher. Conductor scans a single root folder and lists
every directory inside it, enriched with git status.

## The projects folder (root)

On first launch Conductor scans `~/Code`. To change it:

- Click the **⋯** button at the top of the sidebar, or
- Open the command palette (`⌘K`) → **Change projects folder…**

The chosen root is persisted and re-scanned on the next launch. Switching root is also
what happens automatically when you load a [workspace](workspaces.md) that was saved
against a different folder.

> Backed by the `list_projects` Tauri command, which returns each immediate
> subdirectory and its git branch.

## Git status per project

Each project that is a git repository shows:

- the current **branch**,
- a **dirty** indicator when the working tree has uncommitted changes,
- **ahead / behind** counts relative to its upstream.

The initial listing appears immediately; git status is enriched in parallel just after.
It refreshes automatically when the window regains focus (throttled to once every few
seconds) and via the sidebar refresh button.

## Fuzzy search

Start typing in the sidebar to filter the project list instantly. The same projects are
also searchable from the command palette (`⌘K`).

## Pinned projects

Pin favourites to a sticky **Pinned** section at the top of the sidebar. Pins persist
even when you change the root folder (they store their own path), and stay until you
unpin them. Pinned projects resolve their own git status independently, so they stay
accurate even when they live outside the current root.

## Opening a project

- **Click** a project to open a terminal tab rooted at its folder. If a tab for that
  project already exists, Conductor focuses it instead of opening a duplicate.
- Use the per-project **zed / code / cursor** buttons to open the whole project in your
  real editor. Conductor auto-detects which editors are installed (`detect_editors`)
  and launches the selected one (`open_in_editor`).
- If the [Elyra](elyra-agent.md) CLI is installed, use the agent button to open a native
  agent panel in that project.

## Project health at a glance

Each project row shows its git status (branch, a dirty marker, ahead/behind) **and** a
green **⚡port** badge for any local server currently listening from inside that project
— detected from each process's working directory (`lsof`), so it works with any stack and
needs no configuration. Click a badge to open `http://localhost:<port>` in your browser.

## Related

- [Git](git.md) — the in-app commit dialog.
- [Terminals & panes](terminals.md) — what happens after you open a project.
- [Workspaces](workspaces.md) — save a multi-project layout.
