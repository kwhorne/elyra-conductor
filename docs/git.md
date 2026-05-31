# Git

Conductor surfaces git status in the sidebar and lets you commit without leaving the
window. All git work runs through the Rust core shelling out to `git`; there is no
embedded git library and no network credentials handling beyond what `git` itself does.

## Status in the sidebar

Each project that is a git repo shows its **branch**, a **dirty** marker for uncommitted
changes, and **ahead / behind** counts versus upstream. Status is computed by the
`git_status` command and refreshed:

- right after the initial project listing,
- automatically when the window regains focus (throttled),
- on demand via the sidebar refresh button.

Each `git_status` is a single `git status --porcelain=v2 --branch` call, and refreshes
run through a concurrency-capped worker pool (max 6 repos at a time) so a folder with
many repositories doesn't spawn a burst of git processes and freeze the UI on focus.

Pinned projects resolve their status independently, so they stay accurate even outside
the current root folder.

## The commit dialog

When the active project is a git repo, open the commit dialog from the **⎇ Commit**
top-action or `⌘K` → **Git: commit `<project>`…**.

The dialog lets you:

1. **Review changes** — the working-tree changes are listed (via `git_changes`).
2. **Write a message** in the input.
3. **Commit** — press the commit button or `⌘↵`. Optionally **push** in the same step.

Committing (and the optional push) runs through the `git_commit` command, which stages
changes, commits with your message, and pushes when requested.

> Conductor uses your existing git configuration and credentials. Pushing relies on the
> same auth (SSH keys, credential helper, etc.) that `git push` uses in your shell.

## Anything else

For rebases, interactive staging, history browsing, and the rest, use a terminal pane
(`git …`) or open the project in your editor. Conductor intentionally keeps the built-in
git surface small — commit + status are the high-frequency actions.

## Related

- [Projects & sidebar](projects.md) — where status is shown.
- [Tauri commands](tauri-commands.md) — `git_status`, `git_changes`, `git_commit`.
