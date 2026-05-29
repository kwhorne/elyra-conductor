# elyra-conductor

En lokal "conductor" for prosjektene dine — inspirert av CMUX. Oversikt over alle
lokale prosjekter, rask switching, integrerte terminaler og en innebygd quick-edit
(Monaco). For ekte redigering åpnes prosjektet i Zed/VS Code/Cursor.

## Stack

- **Tauri 2** (Rust core + webview)
- **Svelte 5** frontend
- **xterm.js + portable-pty** — ekte terminaler (én PTY per fane i Rust)
- **Monaco** — innebygd quick-edit
- **Ekstern launch** — `zed` / `code` / `cursor` for fullverdig redigering

## Kjør i dev

```bash
pnpm install
pnpm tauri dev
```

Første gang kompileres Rust-avhengighetene (~30 s). `beforeDevCommand` starter Vite
automatisk på port 1420.

## Bygg app

```bash
pnpm tauri build
```

> Ikonet i `src-tauri/icons/icon.png` er en placeholder generert av
> `node scripts/gen-icon.mjs`. For en ekte bundle: legg inn en 1024×1024 PNG og kjør
> `pnpm tauri icon din-logo.png` for å generere alle størrelser/formater.

## Arkitektur

```
src/                     frontend (Svelte)
  App.svelte             layout: sidebar + faner + editor-panel
  lib/Sidebar.svelte     prosjektliste, søk, "open in editor"
  lib/Terminal.svelte    xterm.js  <-> pty:// events
  lib/Editor.svelte      Monaco quick-edit (⌘S = lagre)
src-tauri/src/
  pty.rs                 PTY-sesjoner (spawn/write/resize/kill) + byte-streaming
  projects.rs            scan av prosjektmappe, git-branch, editor-detect/launch
  fs.rs                  read_file / write_file for editoren
```

### Terminal-flyt
Rust spawner en PTY per fane, leser bytes i en egen tråd og emitter dem som
`pty://data/<id>`. Frontend skriver dem til xterm. Input/resize går tilbake via
`invoke('pty_write' | 'pty_resize')`.

## Snarveier

| Tast | Handling |
|------|----------|
| ⌘K | Command palette (prosjekter, faner, handlinger) |
| ⌘D | Split right |
| ⇧⌘D | Split down |
| ⌘W | Lukk aktiv pane |

Hver pane har også hover-verktøy (split/lukk), og dividers kan dras for å endre størrelse.

## Ferdig

- ✅ Ekte split panes (horisontal/vertikal, drag-to-resize, persistente terminaler)
- ✅ Command palette (⌘K)
- ✅ Native folder/file-picker (`tauri-plugin-dialog`)
- ✅ Faner med uavhengige split-layouter per prosjekt

## Neste steg (ideer)

- Git-status (dirty/ahead/behind) og siste commit i sidebaren
- Persistente prosjekter + favoritter (SQLite via `tauri-plugin-sql`)
- Notification rings: flash på fane når en prosess venter på input
- Lagre/gjenopprett fane- og split-layout mellom restart
- Tab/pane-tittel fra kjørende prosess (f.eks. `bun dev`, `claude`)
```
