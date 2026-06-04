# Files & editor

Conductor includes a file tree for the active project and an inline Monaco editor for
fast edits — without leaving the window.

## File sidebar

Toggle the right-hand file tree with `⌘B` (it is on by default). It shows a lazy-loaded,
recursive tree of the active project, backed by the `list_dir` Tauri command (directories
first, then files).

- **Click a file** to open it in the Monaco editor.
- **Hide noise:** by default `node_modules`, `.git`, and build output are hidden. Toggle
  with `⌘K` → **Show all files in tree** / **Hide node_modules/.git in tree** (persisted).

### Right-click actions

Right-clicking an entry opens a context menu:

**On a folder**

- **Open new terminal here** — a terminal tab rooted at that folder.
- **New Elyra agent here** — _(when the Elyra CLI is detected)_ an agent panel in that
  folder.
- **New file… / New folder…** — create an entry inside that folder (new files open in the
  editor straight away).
- **Rename… / Duplicate…** — rename or copy the folder (refuses to overwrite).
- **Reveal in Finder / Copy path** — jump to it in Finder, or copy its absolute path.
- **Move to Trash** — move it to the macOS Trash (recoverable, never a permanent wipe).

**On a file**

- **Open in editor** — open it in Monaco.
- **Ask Elyra about this file** — _(when detected)_ open an agent tab pre-seeded with a
  prompt about that file. See [Elyra agent](elyra-agent.md).
- **Run `<file>` in a terminal tab** — run the file in a real, persistent, interactive
  terminal tab (full output and scrollback; ideal for deploy scripts you want to watch
  or answer prompts in). The command is executed by the shell at startup with your full
  login environment, then drops to an interactive prompt in the same folder.
- **Run `<file>`… (modal)** — run the file in an in-app modal terminal. Conductor picks a
  sensible command from the extension (`.py` → `python3`, `.js` → `node`, `.sh` →
  `bash`, `.ts` → `npx tsx`, …, otherwise `./file`); the command is **editable** and
  you can **Run / Re-run** with `⌘↵`. A successful run auto-closes, but a **non-zero
  exit keeps the modal open** with the output visible so you can read the error.
- **Run in `<terminal>`** — run it in your external terminal (iTerm2 / Terminal.app),
  via `detect_terminal` / `run_in_external_terminal`.
- **Rename… / Duplicate…** — rename or copy the file (refuses to overwrite).
- **Reveal in Finder / Copy path** — jump to it in Finder, or copy its absolute path.
- **Move to Trash** — move it to the macOS Trash (recoverable).

The tree refreshes automatically after any change, including inside expanded subfolders.

## Inline quick-edit (Monaco)

The editor is the full [Monaco](https://microsoft.github.io/monaco-editor/) editor (the
engine behind VS Code), loaded on demand so startup stays fast.

- Open via a file in the tree, the **Quick edit** top-action, or `⌘K` → **Quick edit
  file…**.
- **Save** with `⌘S` (writes through the `write_file` command).
- **Close** the editor with `⌘W` or its **✕** button.

### Focus-aware shortcuts

While the editor is focused it owns all the usual editor keys — `⌘F` find, `⌘/` toggle
comment, `⌘D` multi-cursor, `⌘K` chords, and so on — and Conductor's pane shortcuts step
aside. Only `⌘W` (close editor) is intercepted. Move focus back to a terminal to get the
pane shortcuts again.

## Open in your real editor

For anything beyond a quick edit, use the per-project **zed / code / cursor** buttons in
the sidebar to launch the project in your full editor. See [Projects & sidebar](projects.md).

## Related

- [Keyboard shortcuts](keybindings.md) — the full shortcut list and editor-focus behavior.
- [Tauri commands](tauri-commands.md) — `list_dir`, `read_file`, `write_file`.
