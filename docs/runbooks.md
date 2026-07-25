# Runbooks

Runbooks are **local, project-scoped markdown notes that are runnable** — a living
checklist where shell commands, file links, and tasks are one click away. Think
"project journal + lightweight runbook" rather than a Notion/Obsidian replacement:
everything is a plain `.md` file on disk, versionable with git, with no server and no
secrets.

> Conductor stays a **host**: runbooks only *run and display*. There is no LLM, no API
> key, and no prompt logic here — see [Architecture & boundaries](architecture.md).

## Where they live

Each runbook is a markdown file under:

```
<project>/.conductor/notes/*.md
```

The folder is created on first use. Commit it to share runbooks with your team, or add
`.conductor/` to `.gitignore` to keep them local.

## Opening a runbook

- **Command palette** (`⌘K`) → **Open project runbook**, or
- **Right-click a folder** in the file tree → **Open runbook here**.

A runbook opens as its own tab (so `⌘1`–`⌘9` and drag-reordering work). Use the picker
in the toolbar to switch between runbooks, or **＋ New** to create one from a starter
template.

## Recording a runbook

Instead of writing one by hand, you can **record** what you do. With
[shell integration](terminals.md) on, click **⏺ Record** in the toolbar, open a new
terminal (`⌘N`), and run your commands. Click **⏺ Rec** again to stop, give it a name,
and Conductor writes a runbook draft — each command a runnable step, failed ones flagged
with their exit code. Edit it freely afterwards. *Do it once, then share it.*

## Editing

Toggle **✎ Edit** to edit the markdown, then **Save** (`⌘S`) writes it back to
`.conductor/notes/`. Toggle back to **Preview** to use the interactive elements below.

## Interactive elements

### ▶ Run — shell code fences

Any fenced code block tagged as a shell language gets a **▶ Run** button (and a **⧉ Copy**
button):

````markdown
```bash
pnpm install && pnpm dev
```
````

Recognised languages: `bash`, `sh`, `shell`, `zsh`, `console`, `terminal`, or no language.
**Run** sends the command to the project's terminal — it reuses an existing terminal pane
for that project if there is one, otherwise it opens a new terminal tab. Other languages
(e.g. `js`, `python`) render normally with a Copy button but no Run.

### `[[file]]` — open a file in the editor

Obsidian-style links open a file in the inline Monaco editor:

```markdown
- [[src/App.svelte]]                  → opens that file
- [[src/App.svelte|App.svelte]]       → custom link label
- [[/Users/me/notes.txt]]             → absolute path
```

Relative paths resolve against the **project root**. Links are shown with a 📄 marker.

### 🤖 Send to Elyra

When the [Elyra](elyra-agent.md) CLI is installed, each shell block also shows a **🤖
Elyra** button. It opens an agent panel with the snippet pre-filled in the composer, so
you can add a question ("what does this do?", "make this safer", …) and send. Conductor
just hands Elyra the text — the reasoning happens in Elyra.

### `[[task:name]]` — run a discovered task

Run a project task (from `package.json`, `composer.json`, `Makefile`, or `justfile`) as a
clickable chip:

```markdown
[[task:dev]]  ·  [[task:build|Build it]]
```

The name is matched against [discovered tasks](tasks.md); the task's command runs in the
project's terminal. If no task matches, the text after `task:` is run literally, so
`[[task:npm test]]` also works as an ad-hoc command.

## Example

````markdown
# Onboarding

## Setup
```bash
pnpm install
```

## Run the app
[[task:dev]]

## Key files
- [[src/App.svelte|Main UI]]
- [[README.md]]
````

## Verify — living runbooks

Documentation rots silently. The **✓ Verify** button runs every runnable step headless
(in your login shell, with a per-step timeout) and flags the ones that no longer work —
per-step badges (✓ / ✗ exit code / timeout), the failing step's output inline, and a
freshness banner (*“Verified 2h ago — all 4 steps green”*) backed by a sidecar
`.verify.json` next to the runbook, so it survives restarts.

Steps that are meant to run forever (dev servers) opt out by tagging the fence:

````markdown
```bash no-verify
pnpm dev
```
````

A failing step gets its own **⚡ Fix** button that hands the command and output to an
Elyra agent. Verification runs each step via the `run_step` Tauri command.

## A note on trust

Runbooks are markdown files that live in the repository, which means a runbook can
arrive from anywhere you cloned from — or be written by an agent. Conductor therefore
treats runbook content as untrusted: the rendered HTML is sanitised before display, and
external links open in your browser rather than inside the app. Shell blocks are never
run on their own — `▶ Run` is always an explicit click, and you see the command first.
See [Architecture](architecture.md#untrusted-content--the-webview-boundary).

## Related

- [Terminals & panes](terminals.md) — where Run sends its commands.
- [Tasks](tasks.md) — what `[[task:name]]` resolves against.
- [Files & editor](files-and-editor.md) — where `[[file]]` links open.
- [Architecture & boundaries](architecture.md) — why runbooks never call a model.
