<script>
  // "Here's where you left off" — a dismissable welcome-back card shown when
  // the app opens after a real break (not every restart). Pure presentation:
  // App.svelte assembles the brief from data it already has (git status,
  // containers, last session's command log); this component just renders it
  // and offers two actions — resume the project, or hand the brief to Elyra.
  let { brief, elyra = false, onresume = null, onplan = null, onclose } = $props();

  function rel(ts) {
    const m = Math.round((Date.now() - ts) / 60000);
    if (m < 60) return `${m} min ago`;
    const h = Math.round(m / 60);
    if (h < 36) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }
  function short(cmd, n = 46) {
    return cmd && cmd.length > n ? cmd.slice(0, n) + "…" : cmd;
  }
</script>

<div class="brief" role="dialog" aria-modal="false" aria-label="Welcome back" tabindex="-1">
  <div class="head">
    <span class="hi">🌅 Welcome back</span>
    <span class="when">last active {rel(brief.lastAt)}</span>
    <button class="x" title="Dismiss" onclick={onclose}>✕</button>
  </div>

  {#if brief.project}
    <div class="row proj">
      <span class="dot" style:background={brief.project.color || "var(--accent)"}></span>
      <b>{brief.project.name}</b>
      {#if brief.project.branch}
        <span class="git">
          {brief.project.branch}{brief.project.dirty ? " ±" : ""}{brief.project.ahead
            ? ` ↑${brief.project.ahead}`
            : ""}{brief.project.behind ? ` ↓${brief.project.behind}` : ""}
        </span>
      {/if}
    </div>
  {/if}

  {#if brief.containers}
    <div class="row note" class:warn={brief.containers.running < brief.containers.total}>
      🐳 {brief.containers.running}/{brief.containers.total} containers running
      {#if brief.containers.running < brief.containers.total}— some stopped while you were away{/if}
    </div>
  {/if}

  {#if brief.commands?.length}
    <div class="cmds">
      <span class="lbl">You were working on</span>
      {#each brief.commands as c, i (i)}
        <div class="cmd" class:failed={c.exitCode != null && c.exitCode !== 0}>
          <code title={c.command}>{short(c.command || c.proc)}</code>
          {#if c.exitCode != null && c.exitCode !== 0}<span class="ec">exit {c.exitCode}</span>{/if}
        </div>
      {/each}
    </div>
  {/if}

  <div class="actions">
    {#if brief.project && onresume}
      <button class="btn primary" onclick={() => onresume(brief.project.path)}>▶ Resume {brief.project.name}</button>
    {/if}
    {#if elyra && onplan}
      <button class="btn" onclick={onplan}>🤖 Plan my day with Elyra</button>
    {/if}
  </div>
</div>

<style>
  .brief {
    position: fixed;
    top: 52px;
    right: 16px;
    z-index: 170;
    width: 340px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px 14px;
    font-size: 12px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head { display: flex; align-items: baseline; gap: 8px; }
  .hi { font-size: 13px; font-weight: 600; color: var(--text); }
  .when { color: var(--text-dim); flex: 1; }
  .x { background: transparent; border: none; color: var(--text-dim); cursor: pointer; padding: 0 2px; }
  .x:hover { color: var(--text); }
  .row { display: flex; align-items: center; gap: 7px; color: var(--text); }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .git { color: var(--text-dim); font-family: monospace; font-size: 11px; }
  .note { color: var(--text-dim); }
  .note.warn { color: #e0af68; }
  .lbl { display: block; color: var(--text-dim); text-transform: uppercase; font-size: 10px; letter-spacing: 0.05em; margin-bottom: 3px; }
  .cmds { display: flex; flex-direction: column; gap: 2px; }
  .cmd { display: flex; align-items: center; gap: 8px; }
  .cmd code { color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cmd.failed code { color: #f7768e; }
  .ec { color: #f7768e; font-size: 10px; flex: none; }
  .actions { display: flex; gap: 8px; margin-top: 2px; }
  .btn {
    background: var(--accent-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 7px;
    padding: 5px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn:hover { border-color: var(--accent); }
  .btn.primary { border-color: var(--accent); }
</style>
