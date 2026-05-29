# Keyboard shortcuts

Conductor is keyboard-first. The shortcuts below use `⌘` on macOS; on Linux/Windows,
`Ctrl` substitutes for `⌘`.

## All shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘K` | Open / close the command palette |
| `⌘D` | Split the active pane right |
| `⇧⌘D` | Split the active pane down |
| `⌘W` | Close the active pane (or close the editor when it is focused) |
| `⌘B` | Toggle the file sidebar |
| `⌘S` | Save the file (when the editor is focused) |
| `⌘F` | Find in terminal (when a terminal is focused) |
| `⌘↵` | Commit (in the commit dialog) |
| `⌘/` | Show the keyboard-shortcuts help |

Open the in-app reference any time with `⌘/`.

## Editor focus changes the rules

While keyboard focus is **inside the Monaco editor**, the editor owns all of its usual
keys — `⌘F` find, `⌘/` toggle comment, `⌘D` multi-cursor, `⌘K` chords, and so on — and
Conductor's pane shortcuts step aside. The only app shortcut kept while editing is `⌘W`,
which closes the editor.

Move focus back to a terminal pane (click it) to restore the pane shortcuts.

## Terminal find bar

When a terminal is focused, `⌘F` opens an in-terminal find bar instead of passing the
keystroke to the shell:

- `↵` — next match
- `⇧↵` — previous match
- `Esc` — close the find bar

## Pane hover controls

Beyond the shortcuts, each pane has hover controls in its top-right corner (split right,
split down, close), and the dividers between panes are drag-to-resize.

## Related

- [Command palette](command-palette.md) — actions without a dedicated shortcut.
- [Terminals & panes](terminals.md) — splitting and search in context.
