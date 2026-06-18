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

## Agent command center

With several agents running, you need to know — at a glance — which one needs you. Each
agent reports a coarse state, surfaced two ways:

- **A dot on its tab:** blue pulse = working, amber pulse = **waiting on you**, grey =
  exited.
- **A pill in the tab strip:** counts how many agents are *working* vs *waiting*. Click
  it to jump straight to one. `waiting` is prioritised — a blocked agent is wasted time.

When an agent starts waiting while you're looking at another tab or app, you get a system
notification (*Agent needs you*). See [Elyra agent](elyra-agent.md#agent-command-center)
for how the state is derived.

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

## Related

- [Elyra agent](elyra-agent.md) — the agent panel and the command center state model.
- [Git](git.md) — status in the sidebar and the in-app commit dialog.
- [Tauri commands](tauri-commands.md) — `git_worktree_list` / `git_worktree_add` /
  `git_worktree_remove`, `detect_gh` / `gh_pr_list`.
