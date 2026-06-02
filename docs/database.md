# Database browser

Conductor includes a lightweight database browser so you can connect to a project's
database, look around the schema, and run queries without leaving the cockpit.

> Supports **MySQL/MariaDB**, **PostgreSQL**, and **SQLite**. Conductor stays
> secret-free: it reads the connection details from the project's existing `.env`, or
> you supply them for the session. Nothing new is persisted, and it never calls a model
> — it's a tool, not an agent. See [Architecture & boundaries](architecture.md).

## Opening it

Toggle the **DB** panel from the toolbar (top right) or the command palette (`⌘K` →
**Show database panel**). It appears on the right, like the file sidebar.

## Connecting

- **Connect from `.env`** — with a project selected, Conductor reads the Laravel-style
  keys from its `.env`:

  | Key | Used for |
  |-----|----------|
  | `DB_CONNECTION` | `mysql` / `mariadb` / `pgsql` / `sqlite` |
  | `DB_HOST`, `DB_PORT` | host/port (MySQL 3306, PostgreSQL 5432) |
  | `DB_DATABASE` | database name (MySQL/PostgreSQL) or file path (SQLite) |
  | `DB_USERNAME`, `DB_PASSWORD` | credentials (MySQL/PostgreSQL) |

  PostgreSQL connects without TLS in this phase (local/dev); remote TLS is a later
  addition.

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
  combined with `AND`). The leading `WHERE` is optional; ✕ clears all filters.
- **Page** — results load 100 rows at a time; use `‹` / `›`.
- **Edit a cell** — double-click it, type a new value, and press Enter to save (via an
  `UPDATE … WHERE <primary key>`). Requires the table to have a primary key; Esc
  cancels. Single-click still copies the cell value.
- **Structure** — flip the **Data / Structure** toggle to see the table's columns with
  their type, nullability, and key.
- **Refresh**, a row count and query time, `NULL` shown in muted italics.

Up to 1000 rows are returned per query (a *truncated* note appears if there are more).

## Running queries

Click **＋ New query** for a SQL editor tab. Write SQL and run it with **`⌘↵`**.
`SELECT` (and `SHOW` / `PRAGMA` / `EXPLAIN` / `WITH`) show a result grid; other
statements report the number of rows affected.

## Export to Excel

Any table or query result can be exported with the **⤓ Excel** button — it opens a save
dialog and writes a real `.xlsx` file. Values are exported as text so identifiers and zip
codes with leading zeros (e.g. `0484`) are preserved exactly.

## Saved queries (private, per project)

In a query tab you can **⭐ Save** the current SQL under a name and reload it later from
the **Saved queries…** dropdown. Saved queries are:

- **Per project** — scoped to the connected project.
- **Private** — stored in `<project>/.conductor/queries/queries.json`, with an
  auto-generated `.gitignore` that ignores the whole folder. They are **never committed**
  and stay on your machine only.

(Runbooks in `.conductor/notes/` remain versionable — only queries are kept private.)

## Notes & limits

- Engines: MySQL/MariaDB, PostgreSQL, SQLite. PostgreSQL is non-TLS for now (local/dev).
- Editing requires a primary key on the table; results from arbitrary joins/queries are
  read-only.
- Database tabs are not restored across restarts (the connection is per session).

## Related

- [Files & editor](files-and-editor.md) — the file sidebar this sits beside.
- [Runbooks](runbooks.md) — runnable notes, also under `.conductor/`.
- [Tauri commands](tauri-commands.md) — `db_*` command reference.
- [Architecture & boundaries](architecture.md) — why a DB client is fine, and AI is not.
