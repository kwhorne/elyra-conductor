<script>
  import { invoke } from "@tauri-apps/api/core";

  // `conns` = the app's dbConns entries: [{ key, config, id, tables, ... }].
  // Tools ▸ Compare Schemas: diff two connections table-by-table and
  // column-by-column, and generate a best-effort migration script (never runs
  // it — purely a diff + a script for the user to review).
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

  let loading = $state(false);
  let error = $state(null);
  let result = $state(null); // SchemaDiffResult
  let showOnlyDiffs = $state(true);
  let copied = $state(false);

  $effect(() => {
    if (open) {
      sourceKey = conns.find((c) => c.id)?.key ?? "";
      targetKey = "";
      result = null;
      error = null;
      copied = false;
    }
  });

  async function ensureConnected(entry) {
    if (!entry || entry.id) return entry;
    await onconnect?.(entry);
    return entry;
  }

  async function compare() {
    if (!source || !target) {
      error = "Choose a source and a target connection.";
      return;
    }
    if (source.key === target.key) {
      error = "Source and target must be different connections.";
      return;
    }
    error = null;
    result = null;
    loading = true;
    try {
      await ensureConnected(source);
      await ensureConnected(target);
      if (!source.id) throw new Error(source.error || "Could not connect to source");
      if (!target.id) throw new Error(target.error || "Could not connect to target");
      result = await invoke("db_schema_diff", {
        sourceId: source.id,
        targetId: target.id,
        sourceEngine: source.config.engine,
        targetEngine: target.config.engine,
      });
    } catch (e) {
      error = String(e?.message ?? e);
    }
    loading = false;
  }

  let visibleTables = $derived.by(() => {
    if (!result) return [];
    return showOnlyDiffs ? result.tables.filter((t) => t.status !== "same") : result.tables;
  });

  const STATUS_LABEL = {
    same: "same",
    different: "columns differ",
    missing_in_target: "missing on target",
    missing_in_source: "missing on source",
  };

  function copyMigration() {
    if (!result?.migration_sql) return;
    navigator.clipboard?.writeText(result.migration_sql).then(() => {
      copied = true;
      setTimeout(() => (copied = false), 1500);
    });
  }

  function onKeydown(e) {
    if (e.key === "Escape") onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
      <div class="head">
        <span class="title">Compare Schemas</span>
        <button class="x" onclick={onclose}>✕</button>
      </div>

      <div class="endpoints">
        <div class="side">
          <div class="lbl">Source</div>
          <select bind:value={sourceKey} disabled={loading}>
            <option value="" disabled>Choose a connection…</option>
            {#each conns as c (c.key)}
              <option value={c.key}>{icon(c.config.engine)} {labelOf(c)}</option>
            {/each}
          </select>
        </div>
        <span class="arrow">⇄</span>
        <div class="side">
          <div class="lbl">Target</div>
          <select bind:value={targetKey} disabled={loading}>
            <option value="" disabled>Choose a connection…</option>
            {#each conns as c (c.key)}
              <option value={c.key} disabled={c.key === sourceKey}>{icon(c.config.engine)} {labelOf(c)}</option>
            {/each}
          </select>
        </div>
        <button class="primary" onclick={compare} disabled={loading || !source || !target}>{loading ? "Comparing…" : "Compare"}</button>
      </div>

      {#if error}<div class="err">{error}</div>{/if}

      {#if result}
        <div class="toolbar">
          <label class="chk"><input type="checkbox" bind:checked={showOnlyDiffs} /> Only show differences</label>
          <span class="summary">{result.tables.filter((t) => t.status !== "same").length} of {result.tables.length} tables differ</span>
        </div>

        <div class="tables">
          {#if visibleTables.length === 0}
            <div class="hint">No differences — schemas match.</div>
          {/if}
          {#each visibleTables as t (t.table)}
            <div class="table-block">
              <div class="table-head">
                <span class="tstatus {t.status}"></span>
                <span class="tname">{t.table}</span>
                <span class="tlabel">{STATUS_LABEL[t.status]}</span>
              </div>
              {#if t.status !== "missing_in_target" && t.status !== "missing_in_source"}
                {#each t.columns.filter((c) => c.status !== "same") as c (c.name)}
                  <div class="col-row">
                    <span class="cname">{c.name}</span>
                    <span class="ctype src">{c.source_type ?? "—"}</span>
                    <span class="carrow">→</span>
                    <span class="ctype tgt">{c.target_type ?? "—"}</span>
                    <span class="cstatus {c.status}">{STATUS_LABEL[c.status] ?? c.status}</span>
                  </div>
                {/each}
              {/if}
            </div>
          {/each}
        </div>

        {#if result.migration_sql}
          <div class="migration">
            <div class="mig-head">
              <span>Suggested migration (bring target up to date — review before running)</span>
              <button class="ghost" onclick={copyMigration}>{copied ? "Copied ✓" : "Copy SQL"}</button>
            </div>
            <pre class="mig-sql">{result.migration_sql}</pre>
          </div>
        {/if}
      {/if}

      <div class="foot">
        <button class="ghost" onclick={onclose}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 14px; padding: 20px 22px; width: 640px; max-width: 94vw; max-height: 88vh; display: flex; flex-direction: column; box-shadow: 0 20px 56px rgba(0,0,0,0.55); }
  .head { display: flex; align-items: center; margin-bottom: 14px; }
  .title { font-size: 15px; font-weight: 700; flex: 1; }
  .x { background: transparent; border: none; color: var(--text-dim); font-size: 13px; cursor: pointer; }
  .x:hover { color: var(--text); }
  .endpoints { display: flex; align-items: flex-end; gap: 10px; margin-bottom: 12px; }
  .side { flex: 1; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .lbl { font-size: 11px; color: var(--accent); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
  .side select { background: var(--bg-3); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; font-size: 12px; width: 100%; }
  .arrow { color: var(--text-dim); padding-bottom: 8px; }
  .primary { background: var(--accent-2); border: 1px solid var(--accent); color: var(--text); border-radius: 6px; padding: 7px 14px; font-size: 12px; font-weight: 600; white-space: nowrap; }
  .primary:disabled { opacity: 0.5; }
  .err { color: #f7768e; font-size: 12px; margin-bottom: 10px; }
  .toolbar { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .chk { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text); cursor: pointer; }
  .summary { font-size: 11px; color: var(--text-dim); font-family: var(--font-mono); }
  .tables { overflow-y: auto; flex: 1; min-height: 100px; max-height: 320px; border: 1px solid var(--border); border-radius: 8px; padding: 6px; }
  .hint { padding: 16px; text-align: center; color: var(--text-dim); font-size: 12px; }
  .table-block { padding: 6px 4px; border-bottom: 1px solid var(--border); }
  .table-block:last-child { border-bottom: none; }
  .table-head { display: flex; align-items: center; gap: 8px; font-size: 12px; font-weight: 600; margin-bottom: 4px; }
  .tstatus { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .tstatus.same { background: var(--green); }
  .tstatus.different { background: #e0af68; }
  .tstatus.missing_in_target, .tstatus.missing_in_source { background: #f7768e; }
  .tlabel { font-size: 10px; color: var(--text-dim); font-weight: 400; margin-left: auto; }
  .col-row { display: flex; align-items: center; gap: 8px; font-size: 11px; padding: 2px 0 2px 16px; font-family: var(--font-mono); color: var(--text-dim); }
  .cname { color: var(--text); min-width: 120px; }
  .ctype { min-width: 90px; }
  .ctype.src { color: #e0af68; }
  .ctype.tgt { color: var(--accent); }
  .carrow { flex: none; }
  .cstatus { margin-left: auto; font-size: 10px; }
  .migration { margin-top: 12px; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  .mig-head { display: flex; align-items: center; justify-content: space-between; padding: 6px 10px; background: var(--bg-3); font-size: 11px; color: var(--text-dim); }
  .mig-sql { margin: 0; padding: 10px 12px; font-family: var(--font-mono); font-size: 11px; white-space: pre-wrap; max-height: 160px; overflow-y: auto; color: var(--text); }
  .ghost { background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 10px; font-size: 11px; cursor: pointer; }
  .ghost:hover { border-color: var(--accent); }
  .foot { display: flex; justify-content: flex-end; margin-top: 14px; }
</style>
