# Tasks

Conductor discovers runnable **tasks** in the active project and lets you launch them
from the command palette. It is purely a launcher — the command runs in a real shell;
Conductor never interprets or reasons about it.

## Discovered task sources

When you select a project, Conductor scans its folder (via the `list_tasks` Tauri
command) for tasks from these files:

| Source | Detected tasks | Command run |
|--------|----------------|-------------|
| `package.json` | `scripts` keys | `<pm> run <name>` |
| `composer.json` | `scripts` keys | `composer run-script <name>` |
| `Makefile` (or `makefile`, `GNUmakefile`) | top-level targets | `make <name>` |
| `justfile` (or `Justfile`, `.justfile`) | recipe names | `just <name>` |

### JavaScript package manager detection

For `package.json` scripts, the package manager is chosen by lockfile:

| Lockfile present | Command prefix |
|------------------|----------------|
| `pnpm-lock.yaml` | `pnpm` |
| `bun.lockb` / `bun.lock` | `bun` |
| `yarn.lock` | `yarn` |
| _(none of the above)_ | `npm` |

### Parsing notes

- **Makefile:** indented lines (recipes), comments, special targets (anything starting
  with `.`, such as `.PHONY`), and variable assignments (`FOO := bar`) are skipped.
- **justfile:** comment lines, `@`-prefixed lines, and variable assignments (`:=`) are
  skipped; the recipe name is the first token before any parameters.
- The first matching `Makefile`/`justfile` variant found is used.

## The task dashboard (☰ Tasks)

When the active project has any tasks, a **☰ Tasks** button appears in the top bar. It
opens a dashboard listing every discovered task grouped by source (package.json,
composer.json, Makefile, justfile) with a one-click **▶** to run each in a new terminal
tab.

## Running a task

1. Open the command palette (`⌘K`).
2. Type the task name, or scroll to the **task** group (entries shown as **Run: `<name>`**
   with the resolved command and source as a hint).
3. Select it. Conductor opens a **new terminal tab** in the project directory and runs
   the command once.

Because each task runs in an ordinary terminal tab, you get full output, scrollback, and
the ability to re-run or Ctrl-C it like any other shell command.

## Start project (`⌘R`)

You rarely need to remember whether a project uses `npm run dev`, `pnpm dev`,
`composer run dev`, or `make dev`. **Start project** picks the right one for you:

- Press `⌘R`, click the **▶** button on a project in the sidebar, or run **Start project**
  from the command palette.
- Conductor ranks the detected tasks (`dev` > `start` > `serve` > `watch`) across all
  sources and runs the best match in the project's terminal (reusing a pane if one is
  open).
- When two are equally likely — e.g. a Laravel app exposing both `composer run dev` and
  `npm run dev` — it shows a quick picker and **remembers your choice** for next time.
- For commands that aren't a standard task (e.g. `php artisan serve` or `pnpm tauri dev`),
  use **Set start command…** (command palette or a folder's right-click menu) to pin a
  per-project override that always wins.

## Related

- [Terminals & panes](terminals.md) — where tasks run.
- [Tauri commands](tauri-commands.md) — the `list_tasks` command reference.
