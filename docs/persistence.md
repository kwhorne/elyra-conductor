# State & persistence

Conductor persists your working state in the webview's `localStorage`. There is no
database yet (a future SQLite-backed projects store is on the roadmap). This page
documents every key and what it holds.

## Storage keys

| Key | Holds |
|-----|-------|
| `conductor:state` | The auto-saved current session (debounced, ~250 ms). |
| `conductor:workspaces` | Named [workspaces](workspaces.md) as `{ name → snapshot }`. |
| `conductor:sb:<key>` | Per-pane scrollback buffer, keyed by a stable pane id. |

## The session snapshot

Both the auto-saved session and each workspace use the same `serialize()` shape:

```jsonc
{
  "root": "/Users/you/Code",          // projects folder
  "activeProjectPath": "/…/my-app",   // or null
  "pinned": [{ "path": "…", "name": "…" }],
  "showFiles": true,
  "showHidden": false,
  "theme": "dark",                     // "dark" | "light"
  "showEditor": false,
  "editorPath": null,                  // open file, or null
  "activeTabIndex": 0,
  "tabs": [
    // terminal tab — split tree stripped to structure + cwd/title + stable key
    { "kind": "term", "title": "my-app", "projectPath": "…",
      "root": { "kind": "leaf", "cwd": "…", "title": "shell", "key": "…" } },
    // agent tab
    { "kind": "agent", "title": "elyra", "projectPath": "…", "cwd": "…" }
  ]
}
```

On restore, terminal trees are rebuilt with fresh `termId`s but the same stable `key`s,
so persisted scrollback can be matched back to the right pane.

## What persists across restarts

- **Layout:** tabs, split trees, ratios, active tab, active project.
- **UI:** theme, file-sidebar visibility, hidden-files toggle, open editor file.
- **Pinned** projects.
- **Scrollback:** recent output per pane (read-only replay; see below).

## Scrollback (read-only history)

A live PTY cannot be revived once the app closes — the process is gone. So scrollback
restore is pragmatic, not a live resurrection:

- `Terminal.svelte` uses the xterm **serialize addon** to snapshot a pane's buffer
  periodically (every ~4 s) and on teardown, storing it under `conductor:sb:<key>`.
- Each buffer is **capped** (≈60 KB per pane) to stay well under the storage quota.
- On next launch the previous buffer is **written back as read-only history** with a
  `── previous session (restored) ──` divider, and a **fresh shell** starts beneath it
  in the same working directory.
- Buffers for panes that no longer exist are **pruned** automatically whenever the
  session is saved, so closed panes/tabs don't accumulate orphaned data.

> True live-session resurrection would require a tmux-style daemon that outlives the app
> — a separate project, and one that would push against the host-only boundary.

## Resetting

`⌘K` → **Reset saved layout** removes `conductor:state` and reloads. Workspaces and
scrollback keys are left intact (delete workspaces individually via the palette).

## Related

- [Workspaces](workspaces.md) — named snapshots built on this format.
- [Architecture & boundaries](architecture.md) — the layout model behind `tabs`.
