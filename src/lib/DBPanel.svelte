<script>
  let {
    projectName = "",
    conns = [],
    error = null,
    ontoggle,
    onconnect,
    ondisconnect,
    onremove,
    onrefresh,
    onopentable,
    onquery,
    onaddenv,
    onaddmanual,
    onedit,
    ontest,
  } = $props();

  let adding = $state(false);
  let editingEntry = $state(null); // entry being edited (null = adding new)
  let filters = $state({}); // entry.key -> table filter text
  let collapsed = $state({}); // group name -> collapsed?
  let testState = $state(null); // null | "testing" | "ok" | error string
  const blankForm = () => ({ engine: "mysql", host: "127.0.0.1", port: 3306, database: "", username: "root", password: "", path: "", tls: false, tls_insecure: false, group: "", use_ssh: false, ssh_host: "", ssh_port: 22, ssh_user: "", ssh_auth: "key", ssh_password: "", ssh_key_path: "", ssh_passphrase: "" });
  let form = $state(blankForm());

  // Group connections for the tree: ungrouped first, then named groups.
  let groups = $derived.by(() => {
    const byGroup = new Map();
    for (const e of conns) {
      const g = (e.config.group || "").trim();
      if (!byGroup.has(g)) byGroup.set(g, []);
      byGroup.get(g).push(e);
    }
    const names = [...byGroup.keys()].sort((a, b) => (a === "" ? -1 : b === "" ? 1 : a.localeCompare(b)));
    return names.map((name) => ({ name, entries: byGroup.get(name) }));
  });

  async function testConn() {
    testState = "testing";
    try {
      await ontest?.({ ...form, port: Number(form.port) || 0, ssh_port: Number(form.ssh_port) || 22 });
      testState = "ok";
    } catch (e) {
      testState = String(e);
    }
  }

  const ENGINE_ICON = { mysql: "🐬", postgres: "🐘", clickhouse: "🟡", sqlite: "📦" };
  function icon(e) { return ENGINE_ICON[e] ?? "🗄"; }
  function label(c) {
    if (c.engine === "sqlite") return c.path?.split("/").pop() ?? "sqlite";
    return c.database || c.label || c.host || "db";
  }
  function sub(c) {
    if (c.engine === "sqlite") return c.path ?? "";
    return `${c.username || ""}@${c.host || ""}${c.port ? ":" + c.port : ""}`;
  }
  function shown(entry) {
    const f = (filters[entry.key] ?? "").toLowerCase();
    return f ? entry.tables.filter((t) => t.toLowerCase().includes(f)) : entry.tables;
  }

  function startEditConn(entry) {
    editingEntry = entry;
    form = { ...blankForm(), ...entry.config };
    adding = true;
  }
  function submitManual() {
    const cfg = { ...form, port: Number(form.port) || 0, ssh_port: Number(form.ssh_port) || 22 };
    if (editingEntry) onedit?.(editingEntry, cfg);
    else onaddmanual?.(cfg);
    adding = false;
    editingEntry = null;
    testState = null;
    form = blankForm();
  }
  function onEngineChange() {
    const def = { mysql: 3306, postgres: 5432, clickhouse: 9000 };
    const known = [3306, 5432, 9000, 0];
    if (def[form.engine] && known.includes(Number(form.port))) form.port = def[form.engine];
  }
</script>

<div class="dbpanel">
  <div class="header">
    <span class="title">Database</span>
    <button class="icon" title="Add a connection" onclick={() => (adding = !adding)}>＋</button>
  </div>

  {#if adding}
    <div class="add">
      <button class="primary" onclick={() => { onaddenv?.(); adding = false; }} title="Read DB_* from the project's .env">
        From .env{projectName ? " · " + projectName : ""}
      </button>
      <div class="formlabel">or connect manually:</div>
      <div class="form">
        <label>Engine
          <select bind:value={form.engine} onchange={onEngineChange}>
            <option value="mysql">MySQL / MariaDB</option>
            <option value="postgres">PostgreSQL</option>
            <option value="clickhouse">ClickHouse</option>
            <option value="sqlite">SQLite</option>
          </select>
        </label>
        {#if form.engine === "sqlite"}
          <label>File path <input bind:value={form.path} placeholder="/path/to/db.sqlite" /></label>
        {:else}
          <label>Host <input bind:value={form.host} /></label>
          <label>Port <input type="number" bind:value={form.port} /></label>
          <label>Database <input bind:value={form.database} /></label>
          <label>User <input bind:value={form.username} /></label>
          <label>Password <input type="password" bind:value={form.password} /></label>
          {#if form.engine === "postgres" || form.engine === "clickhouse"}
            <label class="chk"><input type="checkbox" bind:checked={form.tls} /> Use TLS</label>
            {#if form.tls}
              <label class="chk"><input type="checkbox" bind:checked={form.tls_insecure} /> Skip certificate verification (self-signed)</label>
            {/if}
          {/if}
          <label class="chk"><input type="checkbox" bind:checked={form.use_ssh} /> Use SSH tunnel (remote)</label>
          {#if form.use_ssh}
            <div class="ssh">
              <label>SSH host <input bind:value={form.ssh_host} placeholder="ssh.example.com" /></label>
              <label>SSH port <input type="number" bind:value={form.ssh_port} /></label>
              <label>SSH user <input bind:value={form.ssh_user} placeholder="deploy" /></label>
              <label>Auth method
                <select bind:value={form.ssh_auth}>
                  <option value="key">Public key</option>
                  <option value="password">Password</option>
                </select>
              </label>
              {#if form.ssh_auth === "password"}
                <label>SSH password <input type="password" bind:value={form.ssh_password} /></label>
              {:else}
                <label>Private key <input bind:value={form.ssh_key_path} placeholder="~/.ssh/id_ed25519" /></label>
                <label>Passphrase <input type="password" bind:value={form.ssh_passphrase} placeholder="(if the key has one)" /></label>
              {/if}
            </div>
          {/if}
        {/if}
        <label>Group <input bind:value={form.group} placeholder="(optional, e.g. Staging)" /></label>
        <div class="btnrow">
          <button class="ghost" onclick={testConn} disabled={testState === "testing"}>Test connection</button>
          {#if testState === "testing"}<span class="tdim">testing…</span>
          {:else if testState === "ok"}<span class="tok">✓ OK</span>
          {:else if testState}<span class="terr" title={testState}>✗ failed</span>{/if}
        </div>
        <button class="primary" onclick={submitManual}>{editingEntry ? "Save & reconnect" : "Connect & save"}</button>
        <div class="hint">Saved securely in your OS keychain — never written to the project or git.</div>
      </div>
    </div>
  {/if}

  {#if error}<div class="err">{error}</div>{/if}

  <div class="tree">
    {#each groups as grp (grp.name)}
      {#if grp.name}
        <button class="group-head" onclick={() => (collapsed = { ...collapsed, [grp.name]: !collapsed[grp.name] })}>
          <span class="caret">{collapsed[grp.name] ? "▸" : "▾"}</span>
          <span class="gname">📁 {grp.name}</span>
          <span class="gcount">{grp.entries.length}</span>
        </button>
      {/if}
      {#if !grp.name || !collapsed[grp.name]}
        {#each grp.entries as entry (entry.key)}
      <div class="conn" class:connected={!!entry.id}>
        <div class="conn-row">
          <button class="conn-head" onclick={() => ontoggle?.(entry)} title={sub(entry.config)}>
            <span class="caret">{entry.connecting ? "◌" : entry.expanded ? "▾" : "▸"}</span>
            <span class="eng">{icon(entry.config.engine)}</span>
            <span class="name">{label(entry.config)}</span>
            <span class="count">{entry.id ? entry.tables.length : ""}</span>
          </button>
          <div class="conn-actions">
            {#if entry.id}
              <button title="New query (⇧⌘N)" onclick={() => onquery?.(entry)}>⌗</button>
              <button title="Refresh tables" onclick={() => onrefresh?.(entry)}>⟳</button>
              <button title="Disconnect" onclick={() => ondisconnect?.(entry)}>⏏</button>
            {/if}
            <button title="Edit connection" onclick={() => startEditConn(entry)}>✎</button>
            <button title="Remove connection" onclick={() => onremove?.(entry)}>✕</button>
          </div>
        </div>

        {#if entry.expanded}
          {#if entry.error}<div class="err small">{entry.error}</div>{/if}
          {#if entry.id}
            <input class="tfilter" placeholder="Filter tables…" value={filters[entry.key] ?? ""} oninput={(e) => (filters = { ...filters, [entry.key]: e.currentTarget.value })} />
            <div class="tables">
              {#each shown(entry) as t (t)}
                <button class="table" onclick={() => onopentable?.(entry, t)} title={t}>
                  <span class="tico">▦</span><span class="tn">{t}</span>
                </button>
              {/each}
              {#if shown(entry).length === 0}<div class="hint">{entry.tables.length === 0 ? "No tables." : "No match."}</div>{/if}
            </div>
          {/if}
        {/if}
      </div>
        {/each}
      {/if}
    {/each}

    {#if conns.length === 0 && !adding}
      <div class="empty">
        <p>No connections yet.</p>
        <button class="primary" onclick={() => (adding = true)}>＋ Add a database</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .dbpanel { width: 270px; min-width: 270px; height: 100%; display: flex; flex-direction: column; background: var(--bg); border-left: 1px solid var(--border); }
  .header { display: flex; align-items: center; padding: 10px 12px 6px; gap: 6px; }
  .title { font-weight: 600; flex: 1; }
  .icon { background: transparent; border: 1px solid var(--border); color: var(--text-dim); border-radius: 6px; padding: 1px 8px; font-size: 14px; }
  .icon:hover { color: var(--text); border-color: var(--accent); }
  .add { padding: 8px 12px; display: flex; flex-direction: column; gap: 8px; border-bottom: 1px solid var(--border); }
  .primary { background: var(--accent-2); border: 1px solid var(--accent); color: var(--text); border-radius: 6px; padding: 6px 10px; font-size: 12px; font-weight: 600; }
  .formlabel { font-size: 11px; color: var(--text-dim); }
  .form { display: flex; flex-direction: column; gap: 6px; }
  .form label { display: flex; flex-direction: column; gap: 2px; font-size: 11px; color: var(--text-dim); }
  .form input, .form select { background: var(--bg-2); color: var(--text); border: 1px solid var(--border); border-radius: 5px; padding: 4px 6px; font-size: 12px; outline: none; }
  .form input:focus, .form select:focus { border-color: var(--accent); }
  .form label.chk { flex-direction: row; align-items: center; gap: 6px; color: var(--text); }
  .form label.chk input { width: auto; }
  .hint { color: var(--text-dim); font-size: 11px; }
  .btnrow { display: flex; align-items: center; gap: 8px; }
  .ssh { display: flex; flex-direction: column; gap: 6px; padding: 8px; margin-top: 2px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-2); }
  .ssh select { background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 6px; font-size: 12px; }
  .ghost { background: var(--bg-2); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 10px; font-size: 12px; }
  .ghost:hover:not(:disabled) { border-color: var(--accent); }
  .ghost:disabled { opacity: 0.5; }
  .tok { color: var(--green); font-size: 11px; }
  .terr { color: #f7768e; font-size: 11px; cursor: help; }
  .tdim { color: var(--text-dim); font-size: 11px; }
  .group-head { display: flex; align-items: center; gap: 6px; width: 100%; background: transparent; border: none; color: var(--text-dim); padding: 5px 4px 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; cursor: pointer; }
  .group-head:hover { color: var(--text); }
  .group-head .gname { font-weight: 600; }
  .group-head .gcount { margin-left: auto; font-family: var(--font-mono); }
  .err { color: #f7768e; font-size: 11px; font-family: var(--font-mono); padding: 6px 12px; word-break: break-word; }
  .err.small { padding: 4px 8px 4px 24px; }
  .tree { flex: 1; overflow-y: auto; padding: 4px 6px 10px; }
  .conn { margin-bottom: 2px; }
  .conn-row { display: flex; align-items: center; border-radius: 5px; }
  .conn-row:hover { background: var(--bg-2); }
  .conn-head { flex: 1; display: flex; align-items: center; gap: 6px; min-width: 0; background: transparent; border: none; color: var(--text); padding: 5px 4px; font-size: 12px; cursor: pointer; }
  .caret { width: 12px; color: var(--text-dim); flex: none; }
  .eng { flex: none; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .count { color: var(--text-dim); font-size: 10px; font-family: var(--font-mono); margin-left: auto; }
  .conn-actions { display: none; gap: 2px; padding-right: 4px; }
  .conn-row:hover .conn-actions { display: flex; }
  .conn-actions button { background: transparent; border: none; color: var(--text-dim); border-radius: 4px; padding: 2px 5px; font-size: 12px; cursor: pointer; }
  .conn-actions button:hover { color: var(--text); background: var(--bg-3); }
  .tfilter { width: calc(100% - 24px); margin: 2px 0 4px 22px; box-sizing: border-box; background: var(--bg-2); color: var(--text); border: 1px solid var(--border); border-radius: 5px; padding: 3px 7px; font-size: 11px; outline: none; }
  .tfilter:focus { border-color: var(--accent); }
  .tables { padding-left: 14px; }
  .table { display: flex; align-items: center; gap: 7px; width: 100%; text-align: left; background: transparent; border: none; color: var(--text); padding: 3px 6px; border-radius: 5px; font-size: 12px; cursor: pointer; }
  .table:hover { background: var(--accent-2); }
  .table .tico { color: var(--text-dim); font-size: 11px; }
  .table .tn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 28px 12px; color: var(--text-dim); }
</style>
