<script>
  import { invoke } from "@tauri-apps/api/core";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { untrack } from "svelte";

  let {
    connId,
    engine = "mysql",
    mode = "table", // "table" | "query"
    table = null,
    projectPath = null,
    ontitle = null,
    theme = "dark",
  } = $props();

  const PAGE = 100;

  let columns = $state([]);
  let rows = $state([]);
  let error = $state(null);
  let loading = $state(false);
  let elapsed = $state(0);
  let truncated = $state(false);
  let rowsAffected = $state(null);
  let isSelect = $state(true);

  // table-mode controls
  let sortCol = $state(null);
  let sortDir = $state("asc");
  let where = $state("");
  let orderBy = $state(""); // explicit ORDER BY clause (overrides header sort)
  let page = $state(0);
  let subview = $state("data"); // "data" | "structure"
  let colFilters = $state({}); // column name -> filter text
  let meta = $state([]); // column info { name, data_type, nullable, key }
  let pkCols = $derived(meta.filter((c) => c.key === "PRI").map((c) => c.name));
  let editing = $state(null); // { ri, ci }
  let editValue = $state("");

  // query-mode
  let sql = $state("");
  let exporting = $state(false);

  // ----- saved queries: per-project, private (gitignored) -----
  // Stored in <project>/.conductor/queries/queries.json (never committed).
  let saved = $state([]);
  let selectedSaved = $state("");
  async function loadSaved() {
    if (!projectPath) {
      saved = [];
      return;
    }
    try {
      saved = await invoke("list_queries", { project: projectPath });
    } catch {
      saved = [];
    }
  }
  async function persistSaved() {
    if (!projectPath) return;
    try {
      await invoke("save_queries", { project: projectPath, queries: saved });
    } catch (e) {
      error = String(e);
    }
  }
  function saveCurrentQuery() {
    if (!sql.trim() || !projectPath) return;
    const name = (window.prompt("Save query as:", selectedSaved || "") || "").trim();
    if (!name) return;
    const i = saved.findIndex((q) => q.name === name);
    const item = { name, sql };
    saved = i >= 0 ? saved.map((q, n) => (n === i ? item : q)) : [...saved, item];
    saved.sort((a, b) => a.name.localeCompare(b.name));
    selectedSaved = name;
    persistSaved();
  }
  function loadSavedQuery(name) {
    selectedSaved = name;
    const q = saved.find((x) => x.name === name);
    if (q) sql = q.sql;
  }
  function deleteSavedQuery() {
    if (!selectedSaved) return;
    saved = saved.filter((q) => q.name !== selectedSaved);
    selectedSaved = "";
    persistSaved();
  }

  async function exportExcel() {
    if (!columns.length || exporting) return;
    exporting = true;
    try {
      const base = (table || "query").replace(/[^\w.-]+/g, "_");
      const path = await saveDialog({
        defaultPath: `${base}.xlsx`,
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
      });
      if (!path) return;
      const XLSX = await import("xlsx");
      // Keep values as text to preserve IDs/zips with leading zeros.
      const aoa = [columns, ...rows.map((r) => r.map((c) => (c === null ? "" : c)))];
      const ws = XLSX.utils.aoa_to_sheet(aoa);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, base.slice(0, 31) || "Sheet1");
      const out = XLSX.write(wb, { type: "array", bookType: "xlsx" });
      await invoke("write_bytes", { path, bytes: Array.from(new Uint8Array(out)) });
    } catch (e) {
      error = `Export failed: ${e}`;
    } finally {
      exporting = false;
    }
  }

  function q(name) {
    return engine === "mysql" ? `\`${name.replace(/`/g, "``")}\`` : `"${name.replace(/"/g, '""')}"`;
  }
  function esc(v) {
    return String(v).replace(/'/g, "''");
  }
  function castText(col) {
    if (engine === "mysql") return `CAST(${q(col)} AS CHAR)`;
    if (engine === "clickhouse") return `toString(${q(col)})`; // ClickHouse has no TEXT type
    return `CAST(${q(col)} AS TEXT)`;
  }

  function buildTableSql() {
    const conds = [];
    const global = where.trim().replace(/^where\s+/i, "");
    if (global) conds.push(`(${global})`);
    for (const [col, val] of Object.entries(colFilters)) {
      if (val && val.trim()) conds.push(`${castText(col)} LIKE '%${esc(val.trim())}%'`);
    }
    let s = `SELECT * FROM ${q(table)}`;
    if (conds.length) s += ` WHERE ${conds.join(" AND ")}`;
    const ob = orderBy.trim().replace(/^order\s+by\s+/i, "");
    if (ob) s += ` ORDER BY ${ob}`;
    else if (sortCol) s += ` ORDER BY ${q(sortCol)} ${sortDir === "desc" ? "DESC" : "ASC"}`;
    s += ` LIMIT ${PAGE} OFFSET ${page * PAGE}`;
    return s;
  }

  async function loadMeta() {
    if (!connId || !table) return;
    try {
      meta = await invoke("db_columns", { id: connId, table });
    } catch {
      meta = [];
    }
  }

  // ----- editable cells (table mode, requires a primary key) -----
  function canEdit() {
    return mode === "table" && subview === "data" && pkCols.length > 0;
  }
  function startEdit(ri, ci) {
    if (!canEdit()) return;
    editing = { ri, ci };
    editValue = rows[ri][ci] ?? "";
  }
  function cancelEdit() {
    editing = null;
  }
  async function commitEdit() {
    if (!editing) return;
    const { ri, ci } = editing;
    const col = columns[ci];
    // Build WHERE from this row's primary-key values.
    const cond = pkCols
      .map((pk) => {
        const idx = columns.indexOf(pk);
        return `${q(pk)} = '${esc(rows[ri][idx])}'`;
      })
      .join(" AND ");
    const sqlUpd = `UPDATE ${q(table)} SET ${q(col)} = '${esc(editValue)}' WHERE ${cond}`;
    try {
      await invoke("db_query", { id: connId, sql: sqlUpd });
      rows[ri][ci] = editValue;
      rows = [...rows];
      editing = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function run(rawSql) {
    if (!connId) return;
    loading = true;
    error = null;
    try {
      const res = await invoke("db_query", { id: connId, sql: rawSql });
      columns = res.columns;
      rows = res.rows;
      elapsed = res.elapsed_ms;
      truncated = res.truncated;
      rowsAffected = res.rows_affected;
      isSelect = res.is_select;
    } catch (e) {
      error = String(e);
      columns = [];
      rows = [];
    }
    loading = false;
  }

  function loadTable() {
    run(buildTableSql());
  }

  function sortBy(col) {
    orderBy = ""; // a header click takes over from an explicit ORDER BY
    if (sortCol === col) sortDir = sortDir === "asc" ? "desc" : "asc";
    else {
      sortCol = col;
      sortDir = "asc";
    }
    page = 0;
    loadTable();
  }

  function applyFilter() {
    page = 0;
    loadTable();
  }
  function setColFilter(col, val) {
    colFilters = { ...colFilters, [col]: val };
  }
  function clearColFilters() {
    colFilters = {};
    where = "";
    orderBy = "";
    page = 0;
    loadTable();
  }
  let hasColFilters = $derived(Object.values(colFilters).some((v) => v && v.trim()));

  function nextPage() {
    if (rows.length < PAGE) return;
    page += 1;
    loadTable();
  }
  function prevPage() {
    if (page === 0) return;
    page -= 1;
    loadTable();
  }

  function runQuery() {
    if (sql.trim()) run(sql);
  }

  function onSqlKey(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      runQuery();
    }
  }

  function copyCell(v) {
    if (v != null) navigator.clipboard?.writeText(v).catch(() => {});
  }

  function focusSelect(node) {
    node.focus();
    node.select?.();
  }

  // Load (and reset controls) only when the table/connection/mode changes — not
  // when the filter, sort, or page change. Those are read inside loadTable(), so
  // we run the body untracked to avoid re-triggering on every keystroke (which
  // previously wiped the WHERE field as you typed).
  $effect(() => {
    const t = table;
    const c = connId;
    const m = mode;
    untrack(() => {
      if (m === "table" && t && c) {
        ontitle?.(t);
        sortCol = null;
        where = "";
        orderBy = "";
        page = 0;
        subview = "data";
        colFilters = {};
        editing = null;
        loadMeta();
        loadTable();
      } else if (m === "query") {
        ontitle?.("query");
        loadSaved();
      }
    });
  });
</script>

<div class="dbview" class:light={theme === "light"}>
  <div class="bar">
    {#if mode === "table"}
      <span class="tname">▦ {table}</span>
      <div class="seg">
        <button class="btn" class:on={subview === "data"} onclick={() => (subview = "data")}>Data</button>
        <button class="btn" class:on={subview === "structure"} onclick={() => (subview = "structure")}>Structure</button>
      </div>
      {#if subview === "data"}
        <span class="wlabel">WHERE</span>
        <input
          class="filter"
          placeholder="condition…  e.g.  city = 'Oslo'"
          bind:value={where}
          onkeydown={(e) => e.key === "Enter" && applyFilter()}
        />
        <span class="wlabel">ORDER BY</span>
        <input
          class="filter order"
          placeholder="e.g.  created_at DESC, id"
          bind:value={orderBy}
          onkeydown={(e) => e.key === "Enter" && applyFilter()}
        />
        <button class="btn" onclick={applyFilter} title="Apply filter & order">Apply</button>
        {#if where.trim() || orderBy.trim() || hasColFilters}<button class="btn" onclick={clearColFilters} title="Clear filters & order">✕</button>{/if}
        <button class="btn" onclick={loadTable} title="Refresh">⟳</button>
        <button class="btn" onclick={exportExcel} disabled={!columns.length || exporting} title="Export to Excel (.xlsx)">⤓ Excel</button>
        <div class="spacer"></div>
        <div class="pager">
          <button class="btn" onclick={prevPage} disabled={page === 0}>‹</button>
          <span class="pageinfo">rows {page * PAGE + 1}–{page * PAGE + rows.length}</span>
          <button class="btn" onclick={nextPage} disabled={rows.length < PAGE}>›</button>
        </div>
      {:else}
        <div class="spacer"></div>
        <span class="pageinfo">{meta.length} column{meta.length === 1 ? "" : "s"}</span>
      {/if}
    {:else}
      <span class="tname">⌗ Query</span>
      {#if projectPath}
        <select class="saved" value={selectedSaved} onchange={(e) => e.currentTarget.value && loadSavedQuery(e.currentTarget.value)} title="Load a saved query (private to this project)">
          <option value="">Saved queries…</option>
          {#each saved as q (q.name)}<option value={q.name}>{q.name}</option>{/each}
        </select>
        <button class="btn" onclick={saveCurrentQuery} disabled={!sql.trim()} title="Save this query for reuse (private, gitignored)">⭐ Save</button>
        {#if selectedSaved}<button class="btn" onclick={deleteSavedQuery} title="Delete saved query">✕</button>{/if}
      {/if}
      <div class="spacer"></div>
      <button class="btn" onclick={exportExcel} disabled={!columns.length || exporting} title="Export to Excel (.xlsx)">⤓ Excel</button>
      <button class="btn primary" onclick={runQuery} disabled={loading || !sql.trim()}>▶ Run <kbd>⌘↵</kbd></button>
    {/if}
  </div>

  {#if mode === "query"}
    <textarea
      class="sql"
      bind:value={sql}
      spellcheck="false"
      placeholder="SELECT * FROM …    (⌘↵ to run)"
      onkeydown={onSqlKey}
    ></textarea>
  {/if}

  <div class="status">
    {#if loading}
      <span>Running…</span>
    {:else if error}
      <span class="err">✗ {error}</span>
    {:else if !isSelect}
      <span class="ok">✓ {rowsAffected ?? 0} row(s) affected · {elapsed} ms</span>
    {:else}
      <span>{rows.length} row{rows.length === 1 ? "" : "s"}{truncated ? "+ (truncated)" : ""} · {elapsed} ms</span>
    {/if}
  </div>

  <div class="grid-wrap">
    {#if mode === "table" && subview === "structure"}
      <table class="grid">
        <thead><tr><th>Column</th><th>Type</th><th>Nullable</th><th>Key</th></tr></thead>
        <tbody>
          {#each meta as c (c.name)}
            <tr>
              <td>{c.name}</td>
              <td>{c.data_type}</td>
              <td class:null={c.nullable}>{c.nullable ? "YES" : "NO"}</td>
              <td>{c.key === "PRI" ? "🔑 PRI" : c.key}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if meta.length === 0}<div class="empty">No column info.</div>{/if}
    {:else if isSelect && columns.length > 0}
      <table class="grid">
        <thead>
          <tr>
            <th class="rownum">#</th>
            {#each columns as c (c)}
              <th
                class:sortable={mode === "table"}
                onclick={() => mode === "table" && sortBy(c)}
                title={mode === "table" ? "Click to sort" : ""}
              >
                {c}
                {#if mode === "table" && pkCols.includes(c)}<span class="pk" title="Primary key">🔑</span>{/if}
                {#if mode === "table" && sortCol === c}<span class="arrow">{sortDir === "asc" ? "▲" : "▼"}</span>{/if}
              </th>
            {/each}
          </tr>
          {#if mode === "table" && subview === "data"}
            <tr class="filterrow">
              <th class="rownum"></th>
              {#each columns as c (c)}
                <th>
                  <input
                    class="colfilter"
                    placeholder="filter"
                    value={colFilters[c] ?? ""}
                    oninput={(e) => setColFilter(c, e.currentTarget.value)}
                    onkeydown={(e) => e.key === "Enter" && applyFilter()}
                  />
                </th>
              {/each}
            </tr>
          {/if}
        </thead>
        <tbody>
          {#each rows as row, ri (ri)}
            <tr>
              <td class="rownum">{page * PAGE + ri + 1}</td>
              {#each row as cell, ci (ci)}
                {#if editing && editing.ri === ri && editing.ci === ci}
                  <td class="editcell">
                    <input
                      class="celledit"
                      bind:value={editValue}
                      use:focusSelect
                      onkeydown={(e) => { if (e.key === "Enter") commitEdit(); else if (e.key === "Escape") cancelEdit(); }}
                      onblur={commitEdit}
                    />
                  </td>
                {:else}
                  <td
                    class:null={cell === null}
                    class:editable={canEdit()}
                    onclick={() => copyCell(cell)}
                    ondblclick={() => startEdit(ri, ci)}
                    title={canEdit() ? "Double-click to edit" : (cell ?? "NULL")}
                  >
                    {cell === null ? "NULL" : cell}
                  </td>
                {/if}
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
      {#if rows.length === 0}
        <div class="empty">No rows.</div>
      {/if}
    {:else if !error && isSelect && !loading}
      <div class="empty">Run a query to see results.</div>
    {/if}
  </div>
</div>

<style>
  .dbview { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--bg-2); }
  .bar {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px; background: var(--bg-3); border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .tname { font-weight: 600; color: var(--accent); white-space: nowrap; }
  .wlabel { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); white-space: nowrap; }
  .spacer { flex: 1; }
  .filter {
    flex: 1; min-width: 120px; max-width: 480px;
    background: var(--bg); color: var(--text); border: 1px solid var(--border);
    border-radius: 6px; padding: 4px 8px; font-family: var(--font-mono); font-size: 12px; outline: none;
  }
  .filter:focus { border-color: var(--accent); }
  .filter.order { max-width: 280px; }
  .btn { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 10px; font-size: 12px; }
  .btn:hover:not(:disabled) { border-color: var(--accent); }
  .btn:disabled { opacity: 0.45; }
  .btn.primary { background: var(--accent-2); border-color: var(--accent); font-weight: 600; }
  .btn kbd { font-size: 10px; opacity: 0.7; }
  .pager { display: flex; align-items: center; gap: 6px; }
  .seg { display: flex; gap: 0; }
  .seg .btn { border-radius: 0; border-right-width: 0; }
  .seg .btn:first-child { border-radius: 6px 0 0 6px; }
  .seg .btn:last-child { border-radius: 0 6px 6px 0; border-right-width: 1px; }
  .seg .btn.on { background: var(--accent-2); border-color: var(--accent); color: var(--text); }
  .saved { background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 4px 6px; font-size: 12px; max-width: 160px; outline: none; }
  .saved:focus { border-color: var(--accent); }
  .pageinfo { color: var(--text-dim); font-family: var(--font-mono); font-size: 11px; white-space: nowrap; }
  .sql {
    height: 130px; resize: vertical; box-sizing: border-box;
    background: var(--bg); color: var(--text); border: none; border-bottom: 1px solid var(--border);
    padding: 10px 12px; font-family: var(--font-mono); font-size: 13px; outline: none; line-height: 1.5;
  }
  .status { padding: 4px 12px; font-size: 11px; font-family: var(--font-mono); color: var(--text-dim); background: var(--bg-3); border-bottom: 1px solid var(--border); }
  .status .err { color: #f7768e; }
  .status .ok { color: var(--green); }
  .grid-wrap { flex: 1; min-height: 0; overflow: auto; }
  .grid { border-collapse: collapse; font-size: 12px; font-family: var(--font-mono); width: max-content; min-width: 100%; }
  .grid th, .grid td {
    border: 1px solid var(--border); padding: 3px 8px; text-align: left;
    max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .grid thead th { position: sticky; top: 0; background: var(--bg-3); color: var(--text); z-index: 1; }
  .grid th.sortable { cursor: pointer; user-select: none; }
  .grid th.sortable:hover { background: var(--accent-2); }
  .grid .arrow { color: var(--accent); margin-left: 4px; }
  .grid .pk { margin-left: 4px; font-size: 10px; }
  .grid .filterrow th { position: sticky; top: 22px; background: var(--bg-3); padding: 2px 4px; z-index: 1; }
  .grid .colfilter { width: 100%; min-width: 60px; box-sizing: border-box; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 4px; padding: 1px 5px; font-family: var(--font-mono); font-size: 11px; outline: none; }
  .grid .colfilter:focus { border-color: var(--accent); }
  .grid td.editable { cursor: text; }
  .grid td.editable:hover { box-shadow: inset 0 0 0 1px var(--accent); }
  .grid td.editcell { padding: 0; }
  .grid .celledit { width: 100%; box-sizing: border-box; background: var(--bg-2); color: var(--text); border: 1px solid var(--accent); border-radius: 0; padding: 2px 7px; font-family: var(--font-mono); font-size: 12px; outline: none; }
  .grid tbody tr:nth-child(even) { background: color-mix(in srgb, var(--bg) 40%, transparent); }
  .grid tbody td { color: var(--text); cursor: default; }
  .grid td.null { color: var(--text-dim); font-style: italic; }
  .grid .rownum { color: var(--text-dim); text-align: right; background: var(--bg-3); position: sticky; left: 0; }
  .empty { padding: 24px; text-align: center; color: var(--text-dim); }
</style>
