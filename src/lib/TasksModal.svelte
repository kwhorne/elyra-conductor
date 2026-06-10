<script>
  let { open = false, tasks = [], projectName = "", onrun, onclose } = $props();

  // Group by source file (package.json, composer.json, Makefile, justfile, …).
  let groups = $derived.by(() => {
    const by = {};
    for (const t of tasks) (by[t.source] ??= []).push(t);
    return Object.entries(by).sort((a, b) => a[0].localeCompare(b[0]));
  });

  const ICON = {
    "package.json": "📦",
    "composer.json": "🎼",
    Makefile: "🛠",
    makefile: "🛠",
    GNUmakefile: "🛠",
    justfile: "🦬",
    Justfile: "🦬",
    ".justfile": "🦬",
  };

  function run(t) {
    onrun?.(t);
    onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">
        <span class="title">Tasks — {projectName}</span>
        <button class="x" onclick={onclose} title="Close">✕</button>
      </div>
      <div class="list">
        {#if tasks.length === 0}
          <div class="empty">No tasks found. Conductor looks for scripts in package.json,
            composer.json, Makefile, and justfile.</div>
        {/if}
        {#each groups as [source, items] (source)}
          <div class="ghead"><span class="gi">{ICON[source] ?? "▷"}</span>{source}</div>
          {#each items as t (t.label + t.command)}
            <button class="task" onclick={() => run(t)} title={`Run in a new terminal tab:\n${t.command}`}>
              <span class="label">{t.label}</span>
              <span class="cmd">{t.command}</span>
              <span class="play">▶</span>
            </button>
          {/each}
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 12vh; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; width: 560px; max-width: 92vw; max-height: 70vh; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
  .head { display: flex; align-items: center; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .title { font-weight: 600; font-size: 13px; }
  .x { margin-left: auto; background: transparent; border: 1px solid var(--border); border-radius: 7px; color: var(--text-dim); font-size: 11px; padding: 4px 8px; cursor: pointer; }
  .list { overflow: auto; padding: 6px; }
  .empty { color: var(--text-dim); font-size: 12px; padding: 16px; line-height: 1.5; }
  .ghead { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; padding: 10px 8px 4px; }
  .gi { font-size: 13px; }
  .task { display: flex; align-items: baseline; gap: 10px; width: 100%; text-align: left; background: transparent; border: none; color: var(--text); padding: 7px 10px; border-radius: 8px; cursor: pointer; }
  .task:hover { background: var(--accent-2); }
  .label { font-size: 13px; font-weight: 500; flex: none; }
  .cmd { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .play { color: var(--accent); flex: none; }
</style>
