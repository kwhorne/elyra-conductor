<script>
  let { open = false, onsearch, onjump, onclose } = $props();

  let query = $state("");
  let results = $state([]);
  let sel = $state(0);
  let input;

  function runSearch() {
    results = query.trim().length >= 2 ? (onsearch?.(query) ?? []) : [];
    sel = 0;
  }

  function jump(r) {
    onjump?.(r, query);
    onclose?.();
  }

  function onKey(e) {
    if (e.key === "Escape") { onclose?.(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(sel + 1, results.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(sel - 1, 0); }
    else if (e.key === "Enter") { e.preventDefault(); if (results[sel]) jump(results[sel]); }
  }

  $effect(() => {
    if (open) {
      query = "";
      results = [];
      sel = 0;
      queueMicrotask(() => input?.focus());
    }
  });
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="bar">
        <span class="ico">🔎</span>
        <input
          bind:this={input}
          class="q"
          placeholder="Search all terminals' output…"
          bind:value={query}
          oninput={runSearch}
          onkeydown={onKey}
        />
        <span class="hint">{results.length ? results.length + " panes" : ""}</span>
      </div>
      <div class="results">
        {#each results as r, i (r.termId)}
          <button class="row" class:sel={i === sel} onclick={() => jump(r)} onmouseenter={() => (sel = i)}>
            <span class="dot" style:background={r.color}></span>
            <span class="label">{r.label}</span>
            <span class="count">{r.count}×</span>
            <span class="sample">{r.sample}</span>
          </button>
        {/each}
        {#if query.trim().length >= 2 && results.length === 0}
          <div class="empty">No matches in any open terminal.</div>
        {:else if query.trim().length < 2}
          <div class="empty">Type at least 2 characters. Searches the live buffer of every open terminal.</div>
        {/if}
      </div>
      <div class="foot">↑↓ to navigate · ↵ to jump · Esc to close</div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: flex-start; justify-content: center; padding-top: 12vh; z-index: 160; }
  .modal { width: 720px; max-width: 92vw; max-height: 70vh; display: flex; flex-direction: column; background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; box-shadow: 0 24px 60px rgba(0,0,0,0.55); }
  .bar { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-bottom: 1px solid var(--border); }
  .ico { font-size: 14px; }
  .q { flex: 1; background: transparent; border: none; color: var(--text); font-size: 15px; outline: none; }
  .hint { color: var(--text-dim); font-size: 11px; font-family: var(--font-mono); }
  .results { flex: 1; min-height: 0; overflow-y: auto; padding: 4px; }
  .row { display: flex; align-items: center; gap: 8px; width: 100%; text-align: left; background: transparent; border: none; color: var(--text); padding: 6px 10px; border-radius: 7px; cursor: pointer; font-size: 13px; }
  .row.sel { background: var(--accent-2); }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .label { font-weight: 600; white-space: nowrap; }
  .count { color: var(--accent); font-family: var(--font-mono); font-size: 11px; flex: none; }
  .sample { color: var(--text-dim); font-family: var(--font-mono); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .empty { padding: 24px; text-align: center; color: var(--text-dim); font-size: 13px; }
  .foot { padding: 6px 14px; font-size: 11px; color: var(--text-dim); background: var(--bg-3); border-top: 1px solid var(--border); }
</style>
