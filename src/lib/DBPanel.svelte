<script>
  let {
    project = null,
    projectName = "",
    conn = null, // { id, config, tables } when connected
    connecting = false,
    error = null,
    onconnectenv,
    onconnect,
    ondisconnect,
    onopentable,
    onnewquery,
    onrefresh,
  } = $props();

  let showManual = $state(false);
  let filter = $state("");
  let form = $state({ engine: "mysql", host: "127.0.0.1", port: 3306, database: "", username: "root", password: "", path: "" });

  let tables = $derived(conn?.tables ?? []);
  let shown = $derived(tables.filter((t) => t.toLowerCase().includes(filter.toLowerCase())));

  function submitManual() {
    const cfg = { ...form, port: Number(form.port) || (form.engine === "mysql" ? 3306 : 0) };
    onconnect?.(cfg);
  }
</script>

<div class="dbpanel">
  <div class="header">
    <span class="title">Database</span>
    {#if conn}
      <button class="icon" title="Refresh tables" onclick={() => onrefresh?.()}>⟳</button>
      <button class="icon" title="Disconnect" onclick={() => ondisconnect?.()}>⏏</button>
    {/if}
  </div>

  {#if !conn}
    <div class="connect">
      <button class="primary" disabled={!project || connecting} onclick={() => onconnectenv?.()}>
        {connecting ? "Connecting…" : `Connect from .env${projectName ? " · " + projectName : ""}`}
      </button>
      <button class="link" onclick={() => (showManual = !showManual)}>
        {showManual ? "Hide manual connection" : "Manual connection…"}
      </button>

      {#if showManual}
        <div class="form">
          <label>Engine
            <select bind:value={form.engine} onchange={() => { const def = { mysql: 3306, postgres: 5432, clickhouse: 9000 }; const known = [3306, 5432, 9000, 0]; if (def[form.engine] && known.includes(Number(form.port))) form.port = def[form.engine]; }}>
              <option value="mysql">MySQL / MariaDB</option>
              <option value="postgres">PostgreSQL</option>
              <option value="clickhouse">ClickHouse</option>
              <option value="sqlite">SQLite</option>
            </select>
          </label>
          {#if form.engine !== "sqlite"}
            <label>Host <input bind:value={form.host} /></label>
            <label>Port <input type="number" bind:value={form.port} /></label>
            <label>Database <input bind:value={form.database} /></label>
            <label>User <input bind:value={form.username} /></label>
            <label>Password <input type="password" bind:value={form.password} /></label>
          {:else}
            <label>File path <input bind:value={form.path} placeholder="/path/to/database.sqlite" /></label>
          {/if}
          <button class="primary" disabled={connecting} onclick={submitManual}>
            {connecting ? "Connecting…" : "Connect"}
          </button>
        </div>
      {/if}

      {#if error}<div class="err">{error}</div>{/if}
      {#if !project}<div class="hint">Select a project to read its <code>.env</code>.</div>{/if}
    </div>
  {:else}
    <div class="connbar">
      <span class="conn-label" title={conn.config.engine === "sqlite" ? conn.config.path : `${conn.config.username}@${conn.config.host}/${conn.config.database}`}>
        <span class="engine">{conn.config.engine}</span>
        {conn.config.database || conn.config.label || (conn.config.path?.split("/").pop() ?? "db")}
      </span>
    </div>
    <button class="newquery" onclick={() => onnewquery?.()}>＋ New query</button>
    <input class="search" placeholder="Filter tables…" bind:value={filter} />
    {#if error}<div class="err">{error}</div>{/if}
    <div class="tables">
      {#each shown as t (t)}
        <button class="table" onclick={() => onopentable?.(t)} title={t}>
          <span class="ico">▦</span><span class="tn">{t}</span>
        </button>
      {/each}
      {#if shown.length === 0}
        <div class="hint">{tables.length === 0 ? "No tables." : "No match."}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dbpanel { width: 260px; min-width: 260px; height: 100%; display: flex; flex-direction: column; background: var(--bg); border-left: 1px solid var(--border); }
  .header { display: flex; align-items: center; padding: 10px 12px 6px; gap: 6px; }
  .title { font-weight: 600; flex: 1; }
  .icon { background: transparent; border: 1px solid var(--border); color: var(--text-dim); border-radius: 6px; padding: 2px 7px; font-size: 12px; }
  .icon:hover { color: var(--text); border-color: var(--accent); }
  .connect { padding: 8px 12px; display: flex; flex-direction: column; gap: 8px; }
  .primary { background: var(--accent-2); border: 1px solid var(--accent); color: var(--text); border-radius: 6px; padding: 6px 10px; font-size: 12px; font-weight: 600; }
  .primary:disabled { opacity: 0.5; }
  .link { background: none; border: none; color: var(--accent); font-size: 12px; text-align: left; cursor: pointer; padding: 0; }
  .form { display: flex; flex-direction: column; gap: 6px; border: 1px solid var(--border); border-radius: 8px; padding: 8px; }
  .form label { display: flex; flex-direction: column; gap: 2px; font-size: 11px; color: var(--text-dim); }
  .form input, .form select { background: var(--bg-2); color: var(--text); border: 1px solid var(--border); border-radius: 5px; padding: 4px 6px; font-size: 12px; outline: none; }
  .form input:focus, .form select:focus { border-color: var(--accent); }
  .err { color: #f7768e; font-size: 11px; font-family: var(--font-mono); padding: 4px 12px; word-break: break-word; }
  .hint { color: var(--text-dim); font-size: 11px; padding: 6px 12px; }
  .connbar { padding: 4px 12px 8px; }
  .conn-label { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .engine { font-size: 9px; text-transform: uppercase; letter-spacing: .04em; background: var(--accent-2); border: 1px solid var(--accent); color: var(--text); border-radius: 4px; padding: 0 5px; }
  .newquery { margin: 0 12px 8px; background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px; font-size: 12px; }
  .newquery:hover { border-color: var(--accent); }
  .search { margin: 0 12px 6px; background: var(--bg-2); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 5px 8px; font-size: 12px; outline: none; }
  .search:focus { border-color: var(--accent); }
  .tables { flex: 1; overflow-y: auto; padding: 0 6px 8px; }
  .table { display: flex; align-items: center; gap: 7px; width: 100%; text-align: left; background: transparent; border: none; color: var(--text); padding: 4px 6px; border-radius: 5px; font-size: 12px; cursor: pointer; }
  .table:hover { background: var(--accent-2); }
  .table .ico { color: var(--text-dim); font-size: 11px; }
  .table .tn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
