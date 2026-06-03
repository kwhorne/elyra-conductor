# Keyboard shortcuts

Conductor is keyboard-first. The shortcuts below use `⌘` on macOS; on Linux/Windows,
`Ctrl` substitutes for `⌘`.

## All shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘K` | Open / close the command palette |
| `⌘1`–`⌘9` | Switch to tab 1–9 by its position in the tab bar |
| `⌘R` | Start the active project's dev command |
| `⇧⌘F` | Search all terminals (global scrollback search) |
| `⌘⌥` ←↑↓→ | Move focus between split panes |
| `⌘⌥Z` | Zoom / unzoom the active pane |
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

## Switching & reordering tabs

Jump straight to a tab with `⌘1`–`⌘9` (the number is the tab's position in the bar,
left to right). **Drag** a tab sideways to reorder it — a blue insertion marker shows
where it will land, and the `⌘`-number shortcuts follow whatever order you choose.

## Modified Enter in terminals

Inside a terminal, `⇧↵` inserts a **newline** instead of submitting — Conductor sends
the Kitty `CSI u` sequence so TUIs (such as the Elyra CLI) can tell `⇧↵` apart from a
plain `↵`. `⌥↵` and `⌃↵` are forwarded the same way. (Plain xterm.js collapses all of
these to a bare carriage return, which is why this needs Conductor's help.)

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
