# Database browser

Conductor includes a lightweight database browser so you can connect to a project's
database, look around the schema, and run queries without leaving the cockpit.

> Supports **MySQL/MariaDB**, **PostgreSQL**, **ClickHouse**, and **SQLite**. Conductor
> stays secret-free: it reads the connection details from the project's existing `.env`,
> or you supply them for the session. Nothing new is persisted, and it never calls a
> model — it's a tool, not an agent. See [Architecture & boundaries](architecture.md).
>
> Conductor is a native (Rust) app, so it talks to each database with a Rust client
> directly — it does not (and cannot) load the project's PHP/Laravel drivers. It only
> reads the connection *values* from `.env`. ClickHouse uses its **native TCP protocol**
> (port 9000), the same one `clickhouse-client` uses — not the HTTP interface.

## Opening it

Toggle the **DB** panel from the toolbar (top right) or the command palette (`⌘K` →
**Show database panel**). It appears on the right, like the file sidebar.

## Multiple connections per project

The panel is a **tree of connections**, so a project can have several databases open at
once — for example MySQL for app data and **ClickHouse for BI/reports**. Click **＋** to
add a connection (from `.env` or manually); each one expands to its own table list and
has per-connection actions on hover: new query (`⌗`), refresh (`⟳`), disconnect (`⏏`),
and remove (`✕`).

Saved connections — including passwords — are stored in the **OS keychain** (macOS
Keychain), keyed per project. **Nothing is written into the project folder**, so nothing
can be committed; the connections simply reappear (collapsed) when you reopen the
project, ready to connect with a click.

Hover a connection and use **✎** to edit it (host/port/credentials/TLS) — it reconnects
and re-saves. Give a connection a **Group** to organise the list into collapsible folders,
and use **Test connection** in the form to verify the settings before saving.

### TLS (remote PostgreSQL / ClickHouse)

When adding a PostgreSQL or ClickHouse connection, tick **Use TLS** to connect over an
encrypted channel; tick **Skip certificate verification** for self-signed/internal
hosts. TLS uses the system stack (macOS Secure Transport). ClickHouse keeps its native
protocol over the encrypted stream.

### SSH tunnels (remote databases)

For a database that's only reachable through a jump host, tick **Use SSH tunnel** in the
connection form and fill in:

- **SSH host / port / user**
- **Auth method** — **Public key** (with a **Private key** path like `~/.ssh/id_ed25519`
  and an optional **Passphrase**) or **Password**.

The **Host** and **Port** above are the database address *as seen from the SSH server*
(often `127.0.0.1` and the default port). Conductor opens a local port-forward with the
system `ssh` and connects the driver to it; the tunnel lives as long as the connection.
Secrets are passed to `ssh` via a one-shot askpass helper (deleted right after auth) and
stored in the Keychain with the rest of the connection. TLS is skipped for tunneled
connections — the SSH channel is the encryption.

## Connecting

- **Connect from `.env`** — with a project selected, Conductor reads the Laravel-style
  keys from its `.env`:

  | Key | Used for |
  |-----|----------|
  | `DB_CONNECTION` | `mysql` / `mariadb` / `pgsql` / `clickhouse` / `sqlite` |
  | `DB_HOST`, `DB_PORT` | host/port (MySQL 3306, PostgreSQL 5432, ClickHouse 9000) |
  | `DB_DATABASE` | database name (MySQL/PostgreSQL/ClickHouse) or file path (SQLite) |
  | `DB_USERNAME`, `DB_PASSWORD` | credentials (MySQL/PostgreSQL/ClickHouse) |

  PostgreSQL and ClickHouse connect without TLS in this phase (local/dev); remote TLS is
  a later addition. ClickHouse is most often set up via **manual connection**.

  For SQLite, a relative `DB_DATABASE` resolves against the project, and an empty value
  falls back to `database/database.sqlite`.

- **Manual connection** — pick the engine and fill in the details yourself. Manual
  credentials are kept in memory for the session only; they are not written anywhere.

## Browsing tables

The panel lists the database's tables (with a filter box). Click a table to open it in
the **main window** as its own tab — a data grid with a toolbar:

- **Sort** — click a column header (click again to reverse). Primary-key columns are
  marked with 🔑.
- **Filter** — type a `WHERE` condition (e.g. `city = 'Oslo'`) in the box, *and/or* use
  the **per-column filter row** under the headers (each box matches that column,
  combined with `AND`). The leading `WHERE` is optional.
- **Order** — type a full `ORDER BY` clause (e.g. `created_at DESC, id`) in the
  **ORDER BY** box. Clicking a column header also sorts; an explicit `ORDER BY` wins.
  `✕` clears filters and order.
- **Page** — results load 100 rows at a time; use `‹` / `›`.
- **Edit a cell** — double-click it to open a small editor (a multi-line field for long
  values, plus a **Set to NULL** option); `⌘↵` saves via an `UPDATE … WHERE <primary
  key>`. Requires the table to have a primary key. Single-click still copies the value.
- **Structure** — flip the **Data / Structure** toggle to see the table's columns with
  their type, nullability, and key, plus the table's approximate **row count** and
  **on-disk size**.
- **Export** — **⤓ Excel** (`.xlsx`) or **⤓ CSV**.
- **Refresh**, a row count and query time, `NULL` shown in muted italics.

Up to 1000 rows are returned per query (a *truncated* note appears if there are more).

## Running queries

Click **＋ New query** for a SQL editor tab. Write SQL and run it with **`⌘↵`**.
`SELECT` (and `SHOW` / `PRAGMA` / `EXPLAIN` / `WITH`) show a result grid; other
statements report the number of rows affected. A query tab keeps a per-project **history**
of what you've run — pick one from the **History…** dropdown to load it back (last 50).

## Export to Excel / CSV

Any table or query result can be exported with **⤓ Excel** (a real `.xlsx`) or **⤓ CSV**.
Values are exported as text so identifiers and zip codes with leading zeros (e.g. `0484`)
are preserved exactly.

In a table's **Data** view the export covers the **whole table** (not just the visible
page), with the database field names as the header row, and it respects any active WHERE /
column filters and ordering. In a query tab, the current result is exported.

## Ask Elyra about your data

When the [Elyra](elyra-agent.md) CLI is installed, a **🤖 Elyra** button appears on results.
It hands structured context to a fresh agent (pre-filled in its composer, so you add the
question) — Conductor only formats and sends text; the reasoning happens in Elyra:

- **On a result / query** — sends the columns and first rows (and the SQL, in query mode).
  Good for “explain this result” or “optimise this query”.
- **Structure view** — sends the table schema. Good for “write a migration that…”.
- **Per row** — hover a row and click the small 🤖 in the **#** column to send that single
  row as a `column | value` table. Good for “explain this row”.

## Saved queries (private, per project)

In a query tab you can **⭐ Save** the current SQL under a name and reload it later from
the **Saved queries…** dropdown. Saved queries are:

- **Per project** — scoped to the connected project.
- **Private** — stored in `<project>/.conductor/queries/queries.json`, with an
  auto-generated `.gitignore` that ignores the whole folder. They are **never committed**
  and stay on your machine only.

(Runbooks in `.conductor/notes/` remain versionable — only queries are kept private.)

## Notes & limits

- Engines: MySQL/MariaDB, PostgreSQL, ClickHouse (native protocol, port 9000), SQLite.
  PostgreSQL and ClickHouse are non-TLS for now (local/dev).
- Editing requires a primary key on the table; results from arbitrary joins/queries are
  read-only. **ClickHouse tabs are read-only** (it isn't an OLTP row-update store) —
  browse, query, and export instead.
- Database tabs are not restored across restarts (the connection is per session).

## Related

- [Files & editor](files-and-editor.md) — the file sidebar this sits beside.
- [Runbooks](runbooks.md) — runnable notes, also under `.conductor/`.
- [Tauri commands](tauri-commands.md) — `db_*` command reference.
- [Architecture & boundaries](architecture.md) — why a DB client is fine, and AI is not.
