# Troubleshooting

Common issues and how to resolve them.

## The app won't build or `tauri dev` fails

- Ensure **Rust stable** is installed (`rustup show`) and the
  [Tauri platform deps](https://tauri.app/start/prerequisites/) are present. On macOS,
  run `xcode-select --install`.
- The first `pnpm tauri dev` compiles all Rust dependencies and can take ~30 s — let it
  finish.
- Run `cd src-tauri && cargo check` to see Rust errors in isolation, and `pnpm build` to
  check the frontend in isolation.

## "Elyra agent" options don't appear

Conductor only shows agent entry points when it can find the `elyra` binary. It resolves
binaries through your **login shell**, so:

- Confirm `elyra` runs in a normal terminal (`elyra --version`).
- If you use nvm/Volta/asdf, make sure your shell init file (e.g. `~/.zshrc`) sets up the
  version manager so a login shell can find `elyra` and its Node runtime.
- Restart Conductor after fixing your PATH.

See [Elyra agent](elyra-agent.md).

## A task / file run does nothing or shows no output

- Tasks run in a real shell tab. Make sure the underlying tool exists (e.g. `pnpm`,
  `make`, `just`, `composer` are on your PATH).
- For the **Run ./file (modal)** action, the file must be executable and runnable as
  `./<file>` from its directory.
- Historically, PTY ids containing a `.` broke streaming because Tauri event names reject
  `.`; pane ids are dot-free for this reason. If you add custom ids, keep them dot-free.

## My terminals didn't come back after restart

Conductor restores the **layout** (tabs, splits, working directories) and replays recent
**scrollback as read-only history**, then starts **fresh shells**. Live processes are not
resurrected — a closed PTY's process is gone. Long-running processes need to be started
again. See [State & persistence](persistence.md).

## Scrollback history looks truncated

Per-pane scrollback is capped (≈60 KB) to stay under the browser storage quota, so only
recent output is replayed. This is expected.

## I want a clean slate

`⌘K` → **Reset saved layout** clears the saved session (`conductor:state`) and reloads.
Workspaces and stored scrollback are left intact; delete workspaces individually via the
palette.

## Updates won't install

- Updates must be **signed**; an unsigned or mis-signed payload is rejected. See
  [Releasing & auto-update](releasing.md).
- On macOS, the first launch of a downloaded build is **not notarized**, so you may need
  right-click → **Open** once.
- Trigger a manual check with `⌘K` → **Check for updates…**.

## Git status or commit isn't working

Conductor shells out to your system `git` and uses your existing config and credentials.
Confirm `git status` and `git push` work from a terminal in that repo. See [Git](git.md).

## Still stuck?

Check the [Development](development.md) debugging tips, and the Rust output from
`pnpm tauri dev` for panics or command errors.
