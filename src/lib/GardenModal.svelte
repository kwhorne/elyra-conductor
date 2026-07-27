<script>
  // "Garden tending": what have you left lying around?
  //
  // The sidebar already shows dirty/ahead/behind per project, but only for the
  // one you are looking at, and only as a dot. This aggregates the same kind of
  // data across every project and answers the question you never think to ask —
  // "is there work here that exists nowhere but this machine?"
  //
  // Pure observation: it reads git state and offers to take you there. It never
  // pushes, commits or deletes anything on your behalf.
  let { open = false, rows = [], loading = false, staleDays = 60, onopen, ongit, onrefresh, onclose } = $props();

  const DAY = 86400;
  function ago(unixSeconds) {
    if (!unixSeconds) return "never";
    const s = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);
    if (s < 3600) return `${Math.max(1, Math.round(s / 60))}m ago`;
    if (s < DAY) return `${Math.round(s / 3600)}h ago`;
    const d = Math.round(s / DAY);
    if (d < 31) return `${d}d ago`;
    const mo = Math.round(d / 30);
    return mo < 12 ? `${mo}mo ago` : `${Math.round(d / 365)}y ago`;
  }

  const SEVERITY_ORDER = { high: 0, medium: 1, low: 2 };
  let sorted = $derived.by(() =>
    [...rows].sort((a, b) => {
      const s = (SEVERITY_ORDER[a.severity] ?? 9) - (SEVERITY_ORDER[b.severity] ?? 9);
      return s !== 0 ? s : a.oldest - b.oldest; // oldest neglect first
    }),
  );

  let summary = $derived.by(() => {
    const onlyHere = rows.filter((r) => r.findings.some((f) => f.kind === "unpushed" || f.kind === "never-pushed")).length;
    return { repos: rows.length, onlyHere };
  });

  function onKeydown(e) {
    if (e.key === "Escape") onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
      <div class="head">
        <span class="title">🌱 Garden</span>
        {#if !loading && rows.length > 0}
          <span class="sub">
            {summary.repos} {summary.repos === 1 ? "repo needs" : "repos need"} attention
            {#if summary.onlyHere}· <b>{summary.onlyHere}</b> with work only on this machine{/if}
          </span>
        {/if}
        <div class="spacer"></div>
        <button class="icon" title="Rescan" onclick={onrefresh} disabled={loading}>⟳</button>
        <button class="icon" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>

      {#if loading}
        <div class="hint">Reading git state…</div>
      {:else if rows.length === 0}
        <div class="hint tidy">Nothing overgrown — every repo is pushed and clean. 🌿</div>
      {:else}
        <div class="rows">
          {#each sorted as r (r.path)}
            <div class="row {r.severity}">
              <div class="rmain">
                <button class="rname" onclick={() => onopen?.(r)} title={r.path}>{r.name}</button>
                <span class="rbranch">{r.branch ?? "—"}</span>
                <span class="rago">last commit {ago(r.lastCommit)}</span>
              </div>
              <div class="findings">
                {#each r.findings as f (f.text)}
                  <span class="finding {f.kind}">{f.text}</span>
                {/each}
                {#if r.dirtyFiles > 0}
                  <!-- Context, not a reason to be listed: almost every repo has
                       some uncommitted state. It says what is at stake here. -->
                  <span class="finding ctx">+{r.dirtyFiles} uncommitted</span>
                {/if}
              </div>
              <div class="ractions">
                <button class="ghost" onclick={() => ongit?.(r)}>⎇ Git</button>
                <button class="ghost" onclick={() => onopen?.(r)}>Open</button>
              </div>
            </div>
          {/each}
        </div>
        <div class="foot">
          Listed for work that exists only here, or branches nobody has touched in
          {staleDays} days. Uncommitted files alone don't qualify — read-only, nothing
          is pushed, committed or deleted for you.
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 14px; padding: 18px 20px; width: 720px; max-width: 94vw; max-height: 86vh; display: flex; flex-direction: column; box-shadow: 0 20px 56px rgba(0,0,0,0.55); }
  .head { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .title { font-size: 15px; font-weight: 700; flex: none; }
  .sub { font-size: 11px; color: var(--text-dim); }
  .spacer { flex: 1; }
  .icon { background: transparent; border: 1px solid var(--border); color: var(--text-dim); border-radius: 6px; padding: 2px 8px; font-size: 12px; cursor: pointer; }
  .icon:hover:not(:disabled) { color: var(--text); border-color: var(--accent); }
  .icon:disabled { opacity: 0.5; }
  .hint { padding: 28px; text-align: center; color: var(--text-dim); font-size: 12px; }
  .hint.tidy { color: var(--green); }
  .rows { overflow-y: auto; display: flex; flex-direction: column; gap: 4px; }
  .row { display: grid; grid-template-columns: 1fr auto; gap: 4px 10px; background: var(--bg-3); border-radius: 8px; padding: 8px 10px; border-left: 3px solid transparent; }
  .row.high { border-left-color: var(--red); }
  .row.medium { border-left-color: #e0af68; }
  .row.low { border-left-color: var(--border); }
  .rmain { display: flex; align-items: baseline; gap: 8px; min-width: 0; }
  .rname { background: transparent; border: none; color: var(--text); font-size: 13px; font-weight: 600; cursor: pointer; padding: 0; }
  .rname:hover { color: var(--accent); text-decoration: underline; }
  .rbranch { font-family: var(--font-mono); font-size: 10px; color: var(--text-dim); background: var(--bg); border-radius: 4px; padding: 1px 6px; }
  .rago { font-size: 10px; color: var(--text-dim); }
  .findings { grid-column: 1; display: flex; flex-wrap: wrap; gap: 5px; }
  .finding { font-size: 11px; border-radius: 4px; padding: 1px 7px; background: var(--bg); color: var(--text-dim); }
  .finding.unpushed, .finding.never-pushed { color: #f7768e; }
  .finding.gone { color: #e0af68; }
  .finding.ctx { opacity: 0.65; }
  .ractions { grid-row: 1 / span 2; grid-column: 2; display: flex; align-items: center; gap: 6px; }
  .ghost { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 9px; font-size: 11px; cursor: pointer; white-space: nowrap; }
  .ghost:hover { border-color: var(--accent); }
  .foot { margin-top: 12px; font-size: 11px; color: var(--text-dim); text-align: center; }
</style>
