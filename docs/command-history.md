# Command history & insights

Conductor keeps a **flight recorder** of the commands you run: what ran, where, the exit
code, how long it took, and a tail of the output. It powers three things — a live
timeline, a searchable cross-session history, and an insights view — all local, with no
telemetry.

The rich data comes from [shell integration](terminals.md#shell-integration-zsh) (on by
default, zsh). Without it, the timeline falls back to coarse process names.

## The timeline (🕘)

Open it from the toolbar or the command palette. The default view is **this session**:
each finished command with its time, exit code (✓ / ✗), duration, and the pane it ran in.

- **Jump** — click a row to jump back to the pane it ran in.
- **Ask Elyra** — the 🤖 button hands the command and its output to an
  [Elyra agent](elyra-agent.md) (e.g. to ask *why did this fail?*).

## Persistent history & search

Every shell-integrated command is persisted to an app-internal **SQLite store**
(`history.db` in the app data dir) — distinct from the [database browser](database.md),
which connects to *your* databases.

Type in the timeline's search box to query the history **across every session**, not just
the current one. The search matches both the **command** and its **captured output**, so
you can find that fix you half-remember:

> *“how did I fix the notarization hang last time?”* → search `notarization` → the session
> from weeks ago, with the commands that resolved it.

Use **Clear history** (shown while searching) to wipe the persisted store.

## Insights — where your time went

The **Insights** tab aggregates the history over **Today / 7 days / All time**:

- A headline: how many commands ran, how many failed, and total time spent waiting.
- **Where the time went** — the biggest time sinks grouped by command, with run count,
  average duration, and failures.

It answers questions like *“why did today feel slow?”* — often the honest answer is
something like `pnpm build` ran 41× at 34s each.

## Privacy

Everything stays on your machine: a local SQLite file, no network, no telemetry. Clearing
the history deletes it.

## Related

- [Terminals & panes](terminals.md) — shell integration, which captures the data.
- [Elyra agent](elyra-agent.md) — where *Ask Elyra* / *Fix it* send a command's context.
- [Tauri commands](tauri-commands.md) — `history_add`, `history_query`, `history_stats`,
  `history_clear`.
