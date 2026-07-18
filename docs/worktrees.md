# Worktrees & parallel agents

Conductor turns the single-agent workflow into something you can **conduct**: several
[Elyra agents](elyra-agent.md) running at once, each isolated on its own branch in a git
worktree, with each branch's pull-request and CI status in view.

Three pieces work together — worktrees (isolation), the command center (presence), and
GitHub PR status (where each branch stands.)

## Git worktrees

A **worktree** is a second checkout of the same repository: its own directory and branch,
sharing the repo's `.git`. That isolation is what lets multiple agents work different
branches at once without colliding over files.

Open the panel from the **🌳 Worktrees** toolbar button (or the command palette →
*Worktrees: parallel branches…*). It's available for any git repository.

### Creating a worktree

Type a branch name, optionally a base (defaults to `HEAD`), and choose how to open it:

- **＋ Terminal** — create the worktree and open a terminal in it.
- **🤖 Agent** — create the worktree and open an Elyra agent in it.

New worktrees are created in a sibling folder next to the repo:

```
<repo>.worktrees/<branch>/
```

Branch handling is automatic:

- An existing **local** branch is checked out into the new worktree.
- A branch that exists only on **`origin`** (e.g. a PR branch) is tracked from
  `origin/<branch>`, so the worktree has the real contents — not an empty new branch.
- Otherwise a new branch is created off the base.

### Opening and removing

Each row offers **🖥 Terminal** and **🤖 Agent** to open that worktree, and **🗑 Remove**
for non-main worktrees. Removal keeps the branch — only the working copy goes. If the
worktree has uncommitted changes, Conductor offers a forced remove.

### Conflict warning — two worktrees touching the same file

Each refresh, the panel does a cheap check across every worktree of the repo: any file
with **uncommitted changes in more than one worktree right now** shows up as a warning
banner — `⚠️ 2 files with uncommitted changes in more than one worktree` — listing the
file and which branches are touching it. It's an early heads-up for exactly the situation
worktrees are meant to avoid: two agents (or you and an agent) about to step on each
other's edits before either side has committed. Read-only and best-effort — it only looks
at working-tree status, not history, and never blocks anything.

## Agent command center

With several agents running, you need to know — at a glance — which one needs you. Each
agent reports a coarse state, in a small shared semantic vocabulary (**blocked**,
**working**, **done**, **idle**) rather than raw colors, surfaced three ways:

- **A dot on its tab:** blue pulse = working, amber pulse = **blocked** (waiting on you),
  green = **done** (exited), grey = idle.
- **The herd strip:** every open agent gets its own glyph chip next to the tab strip —
  ⏸ blocked, ▶ working, ✓ done, ○ idle — with its title, so you see every agent **at a
  glance across every project** without opening anything. Blocked agents sort first.
  Click a chip to jump straight to it; more than six collapse into a **+N** chip that
  opens the full [dashboard](#agent-dashboard-multi-agent-cockpit--auto-merge-queue).

When an agent starts waiting while you're looking at another tab or app, you get a system
notification (*Agent needs you*). See [Elyra agent](elyra-agent.md#agent-command-center)
for how the state is derived.

## Agent dashboard (multi-agent cockpit + auto-merge queue)

With several agents running across different worktrees, the herd strip is the at-a-glance
view; click **🎛 Dashboard** next to it (shown whenever at least one agent tab is open;
also in the command palette — *Agent dashboard…*) for the full cockpit:

- **Agents** — every open agent tab, across every project and worktree, in one list:
  presence glyph (⏸ blocked / ▶ working / ○ idle / ✓ done), its owning project, the
  latest **status line**, and **last activity** ("2m ago"). Click a row to jump straight
  to that tab. Sorted so **blocked** agents float to the top — the ones costing you time.
- **Ready to merge** — an auto-merge queue: for every project behind an open agent,
  Conductor checks its open PRs and lists the ones that are **not a draft** and have
  **all checks passing** (no failing/pending checks). Each row shows the PR number/title,
  branch, check + review summary, and a **Squash & merge** button. Merging:
  1. Runs `gh pr merge --squash --delete-branch` (deletes the remote branch).
  2. Closes any agent tab(s) still open on that branch.
  3. Removes the local worktree, if there was one.

  Confirms before merging; a failed merge (e.g. a new commit landed, or a required
  check reappeared) shows an inline error and leaves the row in the queue.

Both panels are read-only observation plus explicit clicks — no AI, no background
auto-merging without a click. Requires an authenticated [`gh`](https://cli.github.com/)
for the merge queue; the agents list works regardless.

## GitHub PR status

With an authenticated [`gh`](https://cli.github.com/) CLI, the worktree panel shows each
branch's open pull request:

- **Number** and a link that opens the PR on GitHub.
- **CI check rollup** — passing (✓), failing (✗ N), or pending (○ N).
- **Review state** — approved (🟢) or changes requested (🔴).

Open PRs that **don't** have a worktree yet are listed separately, so you can check one
out as a worktree (terminal or agent) in a single click. The full loop:

> See a PR → spin up a worktree tracking it → drop an agent into it → watch its presence
> and CI status from the same window.

PR status is best-effort: if `gh` is missing or unauthenticated, no badges show and the
worktree panel still works.

### PR status on the tab itself

You don't have to open the Worktrees panel or the Agent dashboard to see CI status — a
small colored dot appears directly on any terminal or agent **tab** whose folder is a
worktree with an open PR: 🟢 green (all checks passing), 🔴 red (a check failed), 🟡
yellow (checks still pending). Hover it for the PR number/title. It refreshes in the
background every 45s — same best-effort, read-only `gh` lookup as the panels above.

## Related

- [Elyra agent](elyra-agent.md) — the agent panel and the command center state model.
- [Git](git.md) — status in the sidebar and the in-app commit dialog.
- [Tauri commands](tauri-commands.md) — `git_worktree_list` / `git_worktree_add` /
  `git_worktree_remove` / `git_worktree_conflicts`, `detect_gh` / `gh_pr_list` /
  `gh_pr_merge`.
- [Database browser](database.md#compare-schemas-schema-diff--migration-script) —
  Compare Schemas, the same "diff two connections" idea applied to table structure.
