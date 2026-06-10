<script>
  let { open = false, entries = [], elyra = false, onask, onjump, onclear, onclose } = $props();

  // Re-tick relative timestamps while open.
  let nowTick = $state(Date.now());
  $effect(() => {
    if (!open) return;
    nowTick = Date.now();
    const t = setInterval(() => (nowTick = Date.now()), 15000);
    return () => clearInterval(t);
  });

  function ago(ts) {
    const s = Math.max(0, Math.round((nowTick - ts) / 1000));
    if (s < 10) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    return `${h}h ago`;
  }
  function dur(ms) {
    const s = Math.round(ms / 1000);
    if (s < 1) return "<1s";
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    return `${m}m ${s % 60}s`;
  }
  function clock(ts) {
    return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">
        <span class="title">🕘 Command timeline</span>
        <span class="sub">{entries.length} recorded · this session</span>
        {#if entries.length}<button class="link" onclick={onclear}>Clear</button>{/if}
        <button class="x" onclick={onclose} title="Close">✕</button>
      </div>
      <div class="list">
        {#if entries.length === 0}
          <div class="empty">Nothing yet. Run a command in a terminal and it'll show up here —
            what ran, where, and how long it took.</div>
        {/if}
        {#each entries as e (e.id)}
          <div class="row" class:fail={e.exitCode != null && e.exitCode !== 0}>
            <button class="jump" onclick={() => onjump?.(e)} title={e.command || "Jump to this pane"}>
              <span class="clock">{clock(e.endedAt)}</span>
              {#if e.exitCode != null}
                <span class="exit" class:ok={e.exitCode === 0} class:efail={e.exitCode !== 0}>{e.exitCode === 0 ? "✓" : "✗ " + e.exitCode}</span>
              {/if}
              <span class="proc">{e.command || e.proc}</span>
              <span class="dur">{dur(e.duration)}</span>
              <span class="where">{e.label}</span>
              <span class="ago">{ago(e.endedAt)}</span>
            </button>
            {#if elyra && (e.command || e.output)}
              <button class="ask" title="Send this command (and its output) to an Elyra agent" onclick={() => onask?.(e)}>🤖</button>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 10vh; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; width: 620px; max-width: 92vw; max-height: 72vh; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
  .head { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .title { font-weight: 600; font-size: 13px; }
  .sub { font-size: 11px; color: var(--text-dim); }
  .link { background: transparent; border: none; color: var(--accent); font-size: 11px; cursor: pointer; margin-left: auto; }
  .x { background: transparent; border: 1px solid var(--border); border-radius: 7px; color: var(--text-dim); font-size: 11px; padding: 4px 8px; cursor: pointer; }
  .list { overflow: auto; padding: 4px; }
  .empty { color: var(--text-dim); font-size: 12px; padding: 16px; line-height: 1.5; }
  .row { display: flex; align-items: center; width: 100%; border-radius: 8px; }
  .row:hover { background: var(--accent-2); }
  .jump { flex: 1; display: flex; align-items: baseline; gap: 10px; min-width: 0; text-align: left; background: transparent; border: none; color: var(--text); padding: 7px 10px; cursor: pointer; }
  .ask { flex: none; background: transparent; border: none; font-size: 13px; padding: 4px 8px; cursor: pointer; opacity: 0; }
  .row:hover .ask, .row.fail .ask { opacity: 0.85; }
  .ask:hover { opacity: 1; }
  .clock { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); flex: none; width: 46px; }
  .proc { font-family: var(--font-mono); font-size: 12px; font-weight: 600; flex: 1; min-width: 70px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .exit { font-family: var(--font-mono); font-size: 11px; font-weight: 700; flex: none; width: 34px; }
  .exit.ok { color: var(--green); }
  .exit.efail { color: #e06c5a; }
  .dur { font-size: 11px; color: var(--accent); flex: none; width: 60px; }
  .where { font-size: 12px; color: var(--text-dim); flex: none; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ago { font-size: 11px; color: var(--text-dim); flex: none; }
</style>
