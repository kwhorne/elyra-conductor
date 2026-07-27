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

## The Git panel (⌘G)

When the active project is a git repo, open the full Git panel from the **⎇ Git**
top-action or `⌘G`. It gives you:

- **Staged** and **Changes** lists — stage, unstage, or discard individual files (or use
  **Stage all** / **Unstage all**). Backed by `git_files`, `git_stage`, `git_unstage`,
  `git_discard`.
- A **diff view** — click a file to see its unified diff (added/removed lines coloured),
  via `git_diff`.
- **Branches** — switch with the dropdown or create a new one (`git_branches`,
  `git_checkout`, `git_create_branch`).
- **Stash** — stash all changes, then pop or drop entries (`git_stash_*`).
- **Commit** — write a message and **Commit** or **Commit & Push** the staged index
  (`git_commit_index`).

## The commit dialog

For a quick all-in-one commit, open the commit dialog from `⌘K` →
**Git: commit `<project>`…**.

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

## 🌱 Garden — what have I left lying around?

The sidebar shows dirty/ahead/behind for one project at a time. **🌱 Garden** in the top
actions (or `⌘K` → *Garden*) asks the same question across every project at once, and
mostly one question in particular: **is there work here that exists nowhere but this
machine?**

A repository is listed when it has:

- **unpushed commits** — `N unpushed on <branch>`
- **a branch that has never been pushed** at all (older than 7 days)
- **a branch whose remote is gone** — usually a merged PR, so the local copy can go
- **a stale branch** — untouched for 60+ days, excluding the one you have checked out and
  `main`/`master`/`develop`/`trunk`

Rows are sorted worst-first and colour-coded by severity. Clicking a name opens the
project; **⎇ Git** opens the [Git panel](#the-git-panel-g) for it.

### Why uncommitted files don't put a repo on the list

They do show up — as `+29 uncommitted` context on rows that qualify for another reason,
because that tells you what is at stake. But they never *cause* a listing, and that was a
deliberate change after measuring against a real machine: qualifying on dirty files listed
**46 of 98** repositories, mostly build output and scratch state, which buried the findings
that mattered. Requiring the repo to also be dormant barely helped (44) — neglected repos
tend to be both. Restricting the list to work that exists nowhere else, plus stale
branches, gives **16 of 98**, and every row is something you would act on.

The view is **read-only**: it never pushes, commits or deletes anything for you. It also
costs exactly two `git` invocations per repository and scans through a small worker pool —
enriching every project at once previously caused a process storm that froze the UI.

## Worktrees

For parallel branches — each an isolated checkout you can open as a terminal or an Elyra
agent, with GitHub PR status per branch — see [Worktrees & parallel agents](worktrees.md).

## Related

- [Projects & sidebar](projects.md) — where status is shown.
- [Worktrees & parallel agents](worktrees.md) — isolated branches and PR status.
- [Tauri commands](tauri-commands.md) — `git_status`, `git_changes`, `git_commit`,
  `git_worktree_*`.
