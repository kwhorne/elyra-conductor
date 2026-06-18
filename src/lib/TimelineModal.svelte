<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, entries = [], elyra = false, onask, onjump, onclear, onclose } = $props();

  // Search the persisted history (across sessions). Empty query = the live
  // session log passed in via `entries`; a query hits the SQLite flight recorder.
  let q = $state("");
  let results = $state([]);
  let searching = $state(false);
  let searchEl = $state();
  let debounce;

  const searchMode = $derived(q.trim().length > 0);

  // Insights: aggregate flow metrics from the persisted history.
  let view = $state("timeline"); // "timeline" | "insights"
  let statsWindow = $state("today"); // "today" | "7d" | "all"
  let stats = $state(null);
  function windowSince() {
    if (statsWindow === "all") return 0;
    if (statsWindow === "7d") return Date.now() - 7 * 86400e3;
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  }
  function loadStats() {
    invoke("history_stats", { since: windowSince() })
      .then((s) => (stats = s))
      .catch(() => (stats = null));
  }
  $effect(() => {
    void statsWindow;
    if (open && view === "insights") loadStats();
  });

  function runSearch() {
    const query = q.trim();
    if (!query) {
      results = [];
      return;
    }
    searching = true;
    invoke("history_query", { query, limit: 300 })
      .then((rows) => {
        results = rows;
      })
      .catch(() => {
        results = [];
      })
      .finally(() => {
        searching = false;
      });
  }
  $effect(() => {
    void q;
    clearTimeout(debounce);
    debounce = setTimeout(runSearch, 180);
    return () => clearTimeout(debounce);
  });
  $effect(() => {
    if (open) queueMicrotask(() => searchEl?.focus());
    else {
      q = "";
      results = [];
    }
  });

  // Normalize a session entry (camelCase) or a history row (snake_case) into one
  // display shape, so the same row markup renders both.
  function norm(r) {
    const session = r.endedAt != null;
    return {
      key: session ? r.id : `h${r.id}`,
      command: r.command,
      proc: r.proc,
      exitCode: session ? r.exitCode ?? null : r.exit_code ?? null,
      duration: r.duration ?? 0,
      ts: session ? r.endedAt : r.ts,
      label: r.label,
      output: r.output,
      projectPath: session ? r.projectPath : r.project_path,
      session,
      raw: r,
    };
  }
  const items = $derived((searchMode ? results : entries).map(norm));

  async function clearHistory() {
    if (!window.confirm("Delete all persisted command history?")) return;
    try {
      await invoke("history_clear", {});
    } catch {}
    runSearch();
  }

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
  function durLong(ms) {
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m} min`;
    const h = Math.floor(m / 60);
    return `${h}h ${m % 60}m`;
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
        <div class="tabs">
          <button class:on={view === "timeline"} onclick={() => (view = "timeline")}>Timeline</button>
          <button class:on={view === "insights"} onclick={() => (view = "insights")}>Insights</button>
        </div>
        <span class="sub">
          {#if searchMode}{searching ? "searching…" : `${items.length} match${items.length === 1 ? "" : "es"} · all history`}{:else}{entries.length} recorded · this session{/if}
        </span>
        {#if searchMode}<button class="link" onclick={clearHistory}>Clear history</button>{:else if entries.length}<button class="link" onclick={onclear}>Clear</button>{/if}
        <button class="x" onclick={onclose} title="Close">✕</button>
      </div>

      {#if view === "insights"}
        <div class="insights">
          <div class="win">
            <button class:on={statsWindow === "today"} onclick={() => (statsWindow = "today")}>Today</button>
            <button class:on={statsWindow === "7d"} onclick={() => (statsWindow = "7d")}>7 days</button>
            <button class:on={statsWindow === "all"} onclick={() => (statsWindow = "all")}>All time</button>
          </div>
          {#if !stats || stats.total_runs === 0}
            <div class="empty">No commands recorded in this window yet. Run some (with shell integration on) and check back.</div>
          {:else}
            <div class="headline">
              You ran <b>{stats.total_runs}</b> command{stats.total_runs === 1 ? "" : "s"}
              {#if stats.failed > 0}· <span class="bad">{stats.failed} failed</span>{/if}
              · spent <b>{durLong(stats.total_ms)}</b> waiting.
            </div>
            <div class="sec-lbl">Where the time went</div>
            {#each stats.top as s (s.name)}
              <div class="stat">
                <span class="sname" title={s.name}>{s.name}</span>
                <span class="sruns">{s.runs}×</span>
                <span class="savg">avg {dur(s.runs ? s.total_ms / s.runs : 0)}</span>
                {#if s.fails > 0}<span class="sfail">{s.fails} failed</span>{/if}
                <span class="stotal">{durLong(s.total_ms)}</span>
              </div>
            {/each}
          {/if}
        </div>
      {:else}
      <div class="search">
        <input
          bind:this={searchEl}
          bind:value={q}
          placeholder="Search all history — command or output (e.g. “notarization hung”)…"
          spellcheck="false"
        />
        {#if searchMode}<button class="clearq" title="Clear search" onclick={() => (q = "")}>✕</button>{/if}
      </div>
      <div class="list">
        {#if items.length === 0}
          <div class="empty">
            {#if searchMode}No matches in your command history.{:else}Nothing yet. Run a command in a terminal and it'll show up here —
            what ran, where, and how long it took. Type above to search across every session.{/if}
          </div>
        {/if}
        {#each items as e (e.key)}
          <div class="row" class:fail={e.exitCode != null && e.exitCode !== 0}>
            <button class="jump" onclick={() => e.session && onjump?.(e.raw)} title={e.session ? e.command || "Jump to this pane" : e.command || ""} class:nojump={!e.session}>
              <span class="clock">{clock(e.ts)}</span>
              {#if e.exitCode != null}
                <span class="exit" class:ok={e.exitCode === 0} class:efail={e.exitCode !== 0}>{e.exitCode === 0 ? "✓" : "✗ " + e.exitCode}</span>
              {/if}
              <span class="proc">{e.command || e.proc}</span>
              <span class="dur">{dur(e.duration)}</span>
              <span class="where">{e.label}</span>
              <span class="ago">{ago(e.ts)}</span>
            </button>
            {#if elyra && (e.command || e.output)}
              <button class="ask" title="Send this command (and its output) to an Elyra agent" onclick={() => onask?.({ command: e.command, proc: e.proc, exitCode: e.exitCode, duration: e.duration, projectPath: e.projectPath, output: e.output, label: e.label })}>🤖</button>
            {/if}
          </div>
        {/each}
      </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 10vh; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; width: 620px; max-width: 92vw; max-height: 72vh; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
  .search { padding: 8px 12px; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 6px; }
  .search input { flex: 1; background: var(--bg); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 12px; padding: 7px 10px; outline: none; }
  .search input:focus { border-color: var(--accent); }
  .clearq { background: transparent; border: none; color: var(--text-dim); cursor: pointer; font-size: 12px; padding: 2px 6px; }
  .clearq:hover { color: var(--text); }
  .jump.nojump { cursor: default; }
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
  .tabs { display: flex; gap: 2px; background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 2px; }
  .tabs button { background: transparent; border: none; color: var(--text-dim); font-size: 11px; padding: 3px 10px; border-radius: 6px; cursor: pointer; }
  .tabs button.on { background: var(--accent-2); color: var(--text); }
  .insights { padding: 12px; overflow: auto; }
  .win { display: flex; gap: 4px; margin-bottom: 12px; }
  .win button { background: var(--bg); border: 1px solid var(--border); color: var(--text-dim); font-size: 11px; padding: 4px 10px; border-radius: 7px; cursor: pointer; }
  .win button.on { border-color: var(--accent); color: var(--text); }
  .headline { font-size: 14px; line-height: 1.5; margin-bottom: 14px; }
  .headline b { color: var(--accent); }
  .headline .bad { color: #e06c5a; }
  .sec-lbl { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-dim); margin-bottom: 6px; }
  .stat { display: flex; align-items: baseline; gap: 10px; padding: 6px 8px; border-radius: 7px; }
  .stat:hover { background: var(--accent-2); }
  .sname { flex: 1; min-width: 0; font-family: var(--font-mono); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sruns { font-size: 11px; color: var(--text-dim); flex: none; width: 36px; text-align: right; }
  .savg { font-size: 11px; color: var(--text-dim); flex: none; width: 80px; }
  .sfail { font-size: 11px; color: #e06c5a; flex: none; }
  .stotal { font-size: 12px; color: var(--accent); flex: none; width: 70px; text-align: right; font-variant-numeric: tabular-nums; }
</style>
