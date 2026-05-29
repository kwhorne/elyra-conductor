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

## Running a task

1. Open the command palette (`⌘K`).
2. Type the task name, or scroll to the **task** group (entries shown as **Run: `<name>`**
   with the resolved command and source as a hint).
3. Select it. Conductor opens a **new terminal tab** in the project directory and runs
   the command once.

Because each task runs in an ordinary terminal tab, you get full output, scrollback, and
the ability to re-run or Ctrl-C it like any other shell command.

## Related

- [Terminals & panes](terminals.md) — where tasks run.
- [Tauri commands](tauri-commands.md) — the `list_tasks` command reference.
