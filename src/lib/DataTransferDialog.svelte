<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  // `conns` = the app's dbConns entries: [{ key, config, id, tables, ... }].
  // Source/target are picked from whichever of these are currently connected;
  // an entry is auto-connected on demand if the user picks one that isn't.
  let { open = false, conns = [], onconnect, onclose } = $props();

  const ENGINE_ICON = { mysql: "🐬", postgres: "🐘", clickhouse: "🟡", sqlite: "📦", sqlanywhere: "🪶" };
  function icon(e) { return ENGINE_ICON[e] ?? "🗄"; }
  function labelOf(entry) {
    const c = entry.config;
    if (c.engine === "sqlite") return c.path?.split("/").pop() ?? "sqlite";
    return c.database || c.label || c.host || "db";
  }

  let sourceKey = $state("");
  let targetKey = $state("");
  let source = $derived(conns.find((c) => c.key === sourceKey) ?? null);
  let target = $derived(conns.find((c) => c.key === targetKey) ?? null);

  let tables = $state([]); // table names on the source
  let selected = $state(new Set());
  let loadingTables = $state(false);
  let error = $state(null);

  let options = $state({ structure: true, data: true, drop_existing: false, truncate_existing: false, batch_size: 500 });

  let running = $state(false);
  let progress = $state(null); // { table, index, total, rowsCopied }
  let results = $state([]); // TransferTableResult[]
  let unlisten = null;

  $effect(() => {
    if (open) {
      sourceKey = conns.find((c) => c.id)?.key ?? "";
      targetKey = "";
      tables = [];
      selected = new Set();
      error = null;
      results = [];
      progress = null;
      running = false;
    }
  });

  $effect(() => {
    if (open && sourceKey) loadTables();
  });

  async function ensureConnected(entry) {
    if (!entry || entry.id) return entry;
    await onconnect?.(entry);
    return entry;
  }

  async function loadTables() {
    error = null;
    tables = [];
    selected = new Set();
    if (!source) return;
    loadingTables = true;
    try {
      await ensureConnected(source);
      if (!source.id) throw new Error(source.error || "Could not connect to source");
      tables = await invoke("db_tables", { id: source.id });
      selected = new Set(tables); // select all by default
    } catch (e) {
      error = String(e?.message ?? e);
    }
    loadingTables = false;
  }

  function toggleTable(t) {
    const next = new Set(selected);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    selected = next;
  }

  let allSelected = $derived(tables.length > 0 && selected.size === tables.length);
  function toggleAll() {
    selected = allSelected ? new Set() : new Set(tables);
  }

  async function start() {
    if (running) return;
    error = null;
    results = [];
    progress = null;
    if (!source || !target) {
      error = "Choose a source and a target connection.";
      return;
    }
    if (source.key === target.key) {
      error = "Source and target must be different connections.";
      return;
    }
    const list = tables.filter((t) => selected.has(t));
    if (list.length === 0) {
      error = "Select at least one table.";
      return;
    }
    running = true;
    try {
      await ensureConnected(target);
      if (!target.id) throw new Error(target.error || "Could not connect to target");

      unlisten = await listen("db-transfer-progress", (e) => {
        progress = { table: e.payload.table, index: e.payload.index, total: e.payload.total, rowsCopied: e.payload.rows_copied, error: e.payload.error };
      });

      results = await invoke("db_transfer_tables", {
        sourceId: source.id,
        targetId: target.id,
        sourceEngine: source.config.engine,
        targetEngine: target.config.engine,
        tables: list,
        options,
      });
    } catch (e) {
      error = String(e?.message ?? e);
    } finally {
      running = false;
      progress = null;
      unlisten?.();
      unlisten = null;
    }
  }

  function onKeydown(e) {
    if (e.key === "Escape" && !running) onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && !running && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
      <div class="head">
        <span class="title">Data Transfer</span>
        <button class="x" onclick={() => !running && onclose()} disabled={running}>✕</button>
      </div>

      <div class="endpoints">
        <div class="side">
          <div class="lbl">Source</div>
          <select bind:value={sourceKey} disabled={running}>
            <option value="" disabled>Choose a connection…</option>
            {#each conns as c (c.key)}
              <option value={c.key}>{icon(c.config.engine)} {labelOf(c)}</option>
            {/each}
          </select>
        </div>
        <span class="arrow">→</span>
        <div class="side">
          <div class="lbl">Target</div>
          <select bind:value={targetKey} disabled={running}>
            <option value="" disabled>Choose a connection…</option>
            {#each conns as c (c.key)}
              <option value={c.key} disabled={c.key === sourceKey}>{icon(c.config.engine)} {labelOf(c)}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="opts">
        <label class="chk"><input type="checkbox" bind:checked={options.structure} disabled={running} /> Create/replace table structure</label>
        <label class="chk"><input type="checkbox" bind:checked={options.data} disabled={running} /> Copy data</label>
        {#if options.structure}
          <label class="chk"><input type="checkbox" bind:checked={options.drop_existing} disabled={running} /> Drop existing table on target first</label>
        {:else}
          <label class="chk"><input type="checkbox" bind:checked={options.truncate_existing} disabled={running} /> Empty target table before copying</label>
        {/if}
      </div>

      <div class="tables-box">
        <div class="tables-head">
          <label class="chk">
            <input type="checkbox" checked={allSelected} indeterminate={selected.size > 0 && !allSelected} onchange={toggleAll} disabled={running || tables.length === 0} />
            Select all
          </label>
          <span class="count">{selected.size}/{tables.length} tables</span>
        </div>
        <div class="tables-list">
          {#if loadingTables}
            <div class="hint">Loading tables…</div>
          {:else if tables.length === 0}
            <div class="hint">{source ? "No tables found." : "Pick a source connection."}</div>
          {:else}
            {#each tables as t (t)}
              <label class="trow">
                <input type="checkbox" checked={selected.has(t)} onchange={() => toggleTable(t)} disabled={running} />
                <span class="tn">{t}</span>
                {#if progress?.table === t}
                  <span class="prog" class:err={progress.error}>{progress.error ? "✗" : `${progress.rowsCopied} rows…`}</span>
                {:else if results.find((r) => r.table === t)}
                  {@const r = results.find((r2) => r2.table === t)}
                  <span class="prog" class:ok={r.ok} class:err={!r.ok} title={r.error ?? ""}>{r.ok ? `✓ ${r.rows_copied} rows` : "✗ failed"}</span>
                {/if}
              </label>
            {/each}
          {/if}
        </div>
      </div>

      {#if error}<div class="err">{error}</div>{/if}
      {#if running && progress}<div class="statusline">Transferring {progress.table} ({progress.index + 1}/{progress.total})…</div>{/if}

      <div class="foot">
        <button class="ghost" onclick={onclose} disabled={running}>Close</button>
        <button class="primary" onclick={start} disabled={running || !source || !target || selected.size === 0}>
          {running ? "Transferring…" : `Start Transfer (${selected.size})`}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 14px; padding: 20px 22px; width: 520px; max-width: 92vw; max-height: 86vh; display: flex; flex-direction: column; box-shadow: 0 20px 56px rgba(0,0,0,0.55); }
  .head { display: flex; align-items: center; margin-bottom: 14px; }
  .title { font-size: 15px; font-weight: 700; flex: 1; }
  .x { background: transparent; border: none; color: var(--text-dim); font-size: 13px; cursor: pointer; }
  .x:hover:not(:disabled) { color: var(--text); }
  .endpoints { display: flex; align-items: flex-end; gap: 10px; margin-bottom: 12px; }
  .side { flex: 1; display: flex; flex-direction: column; gap: 4px; }
  .lbl { font-size: 11px; color: var(--accent); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
  .side select { background: var(--bg-3); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; font-size: 12px; }
  .arrow { color: var(--text-dim); padding-bottom: 8px; }
  .opts { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .chk { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text); cursor: pointer; }
  .tables-box { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; display: flex; flex-direction: column; min-height: 160px; max-height: 300px; }
  .tables-head { display: flex; align-items: center; justify-content: space-between; padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg-3); }
  .tables-head .count { font-size: 11px; color: var(--text-dim); font-family: var(--font-mono); }
  .tables-list { overflow-y: auto; padding: 4px 6px; flex: 1; }
  .trow { display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-radius: 5px; font-size: 12px; cursor: pointer; }
  .trow:hover { background: var(--bg-3); }
  .tn { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { padding: 20px; text-align: center; color: var(--text-dim); font-size: 12px; }
  .prog { font-size: 11px; font-family: var(--font-mono); color: var(--text-dim); }
  .prog.ok { color: var(--green); }
  .prog.err { color: #f7768e; }
  .err { color: #f7768e; font-size: 12px; margin-top: 10px; }
  .statusline { font-size: 11px; color: var(--text-dim); margin-top: 8px; font-family: var(--font-mono); }
  .foot { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  .ghost { background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 7px 14px; font-size: 12px; }
  .primary { background: var(--accent-2); border: 1px solid var(--accent); color: var(--text); border-radius: 6px; padding: 7px 14px; font-size: 12px; font-weight: 600; }
  .primary:disabled, .ghost:disabled { opacity: 0.5; }
</style>
