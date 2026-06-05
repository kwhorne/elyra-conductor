<script>
  let {
    projects = [],
    pinned = [],
    editors = [],
    activePath = null,
    onselect,
    onopen,
    onroot,
    onrefresh,
    onpin,
    onagent,
    onstart,
    ports = {},
    containers = {},
    running = {},
    lastTest = {},
    onopenport,
    elyra = false,
    root = "",
  } = $props();

  let filter = $state("");

  let pinnedPaths = $derived(new Set(pinned.map((p) => p.path)));

  function match(p) {
    return p.name.toLowerCase().includes(filter.toLowerCase());
  }

  let filteredPinned = $derived(pinned.filter(match));
  let filteredUnpinned = $derived(
    projects.filter((p) => !pinnedPaths.has(p.path) && match(p))
  );
</script>

<div class="sidebar">
  <div class="header">
    <span class="title">Projects</span>
    <button class="root-btn" title="Refresh projects & git status" onclick={() => onrefresh?.()}>⟳</button>
    <button class="root-btn" title="Change folder" onclick={() => onroot?.()}>⋯</button>
  </div>
  <div class="root-path" title={root}>{root || "No folder selected"}</div>

  <input class="search" placeholder="Search projects…" bind:value={filter} />

  {#snippet item(p)}
    <div
      class="item"
      class:active={p.path === activePath}
      role="button"
      tabindex="0"
      onclick={() => onselect?.(p)}
      onkeydown={(e) => e.key === "Enter" && onselect?.(p)}
    >
      <div class="row1">
        {#if running[p.path]}
          <span class="run-dot" title="A command is running in this project"></span>
        {/if}
        <span class="name">{p.name}</span>
        <span class="spacer"></span>
        {#if containers[p.path]}
          <span
            class="ctr"
            class:up={containers[p.path].running > 0}
            title={`${containers[p.path].running}/${containers[p.path].total} container(s) running`}
          >🐳{containers[p.path].running}/{containers[p.path].total}</span>
        {/if}
        {#if lastTest[p.path]}
          <span class="ltest" class:ok={lastTest[p.path].ok} title={`Last test run ${lastTest[p.path].ok ? "passed" : "failed"}`}>{lastTest[p.path].ok ? "✓ test" : "✗ test"}</span>
        {/if}
        {#if p.dirty}
          <span class="dirty" title="Uncommitted changes">●</span>
        {/if}
        {#if p.ahead}
          <span class="track up" title="{p.ahead} ahead of upstream">↑{p.ahead}</span>
        {/if}
        {#if p.behind}
          <span class="track down" title="{p.behind} behind upstream">↓{p.behind}</span>
        {/if}
        {#if p.is_git}
          <span class="branch">⎇ {p.branch ?? "detached"}</span>
        {/if}
        {#each (ports[p.path] ?? []).slice(0, 3) as pt (pt.port)}
          <button
            class="port"
            title={`${pt.process} listening on :${pt.port} — open in browser`}
            onclick={(e) => { e.stopPropagation(); onopenport?.(pt.port); }}
          >⚡{pt.port}</button>
        {/each}
        <button
          class="play"
          title="Start this project's dev command (⌘R)"
          onclick={(e) => {
            e.stopPropagation();
            onstart?.(p);
          }}>▶</button
        >
        <button
          class="pin"
          class:pinned={pinnedPaths.has(p.path)}
          title={pinnedPaths.has(p.path) ? "Unpin" : "Pin"}
          onclick={(e) => {
            e.stopPropagation();
            onpin?.(p);
          }}>📌</button
        >
      </div>
      {#if p.path === activePath}
        <div class="actions">
          {#if elyra}
            <button
              class="agent"
              title="Open an Elyra agent in this project"
              onclick={(e) => {
                e.stopPropagation();
                onagent?.(p);
              }}>🤖 elyra</button
            >
          {/if}
          {#each editors as ed}
            <button
              onclick={(e) => {
                e.stopPropagation();
                onopen?.(ed, p);
              }}>{ed}</button
            >
          {/each}
        </div>
      {/if}
    </div>
  {/snippet}

  <div class="list">
    {#if filteredPinned.length > 0}
      <div class="section-label">Pinned</div>
      {#each filteredPinned as p (p.path)}
        {@render item(p)}
      {/each}
      <div class="section-label">Projects</div>
    {/if}

    {#each filteredUnpinned as p (p.path)}
      {@render item(p)}
    {/each}

    {#if filteredPinned.length === 0 && filteredUnpinned.length === 0}
      <div class="empty">No projects</div>
    {/if}
  </div>
</div>

<style>
  .sidebar {
    width: 260px;
    min-width: 260px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-right: 1px solid var(--border);
  }
  .header {
    display: flex;
    align-items: center;
    padding: 10px 12px 4px;
  }
  .title {
    font-weight: 600;
    font-size: 13px;
  }
  .root-btn {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 15px;
    line-height: 1;
    padding: 0 4px;
  }
  .root-btn:first-of-type {
    margin-left: auto;
  }
  .root-btn:hover {
    color: var(--text);
  }
  .root-path {
    padding: 0 12px 8px;
    font-size: 11px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .search {
    margin: 0 10px 8px;
    padding: 6px 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    outline: none;
  }
  .search:focus {
    border-color: var(--accent);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 6px 10px;
  }
  .section-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-dim);
    padding: 8px 8px 4px;
  }
  .item {
    padding: 7px 8px;
    border-radius: 7px;
    margin-bottom: 2px;
  }
  .item:hover {
    background: var(--bg-3);
  }
  .item.active {
    background: var(--accent-2);
  }
  .row1 {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .name {
    font-weight: 500;
  }
  .spacer {
    flex: 1 1 auto;
  }
  .branch {
    font-size: 10px;
    color: var(--text-dim);
    font-family: var(--font-mono);
  }
  .dirty {
    color: #e0af68;
    font-size: 9px;
    line-height: 1;
  }
  .run-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--green);
    flex: none;
    margin-right: 5px;
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--green) 70%, transparent);
    animation: run-pulse 1.6s ease-out infinite;
  }
  @keyframes run-pulse {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--green) 60%, transparent); }
    70% { box-shadow: 0 0 0 5px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  .ctr {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
    flex: none;
  }
  .ctr.up {
    color: #4aa3df;
    border-color: color-mix(in srgb, #4aa3df 50%, transparent);
    background: color-mix(in srgb, #4aa3df 15%, transparent);
  }
  .ltest {
    font-size: 10px;
    font-family: var(--font-mono);
    border-radius: 4px;
    padding: 0 4px;
    flex: none;
    color: #e06c5a;
    border: 1px solid color-mix(in srgb, #e06c5a 45%, transparent);
    background: color-mix(in srgb, #e06c5a 14%, transparent);
  }
  .ltest.ok {
    color: var(--green);
    border-color: color-mix(in srgb, var(--green) 45%, transparent);
    background: color-mix(in srgb, var(--green) 14%, transparent);
  }
  .track {
    font-size: 10px;
    font-family: var(--font-mono);
  }
  .track.up {
    color: var(--green);
  }
  .track.down {
    color: #f7768e;
  }
  .pin {
    background: transparent;
    border: none;
    font-size: 11px;
    line-height: 1;
    padding: 0 2px;
    opacity: 0;
    filter: grayscale(1);
    transition: opacity 0.12s;
    align-self: center;
  }
  .item:hover .pin {
    opacity: 0.55;
  }
  .pin.pinned {
    opacity: 1;
    filter: none;
  }
  .pin:hover {
    opacity: 1;
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .actions button {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 5px;
    padding: 2px 8px;
    font-size: 11px;
  }
  .actions button:hover {
    border-color: var(--accent);
  }
  .play {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    padding: 0 4px;
    opacity: 0.55;
  }
  .play:hover { color: var(--accent); opacity: 1; }
  .port { background: color-mix(in srgb, var(--green) 18%, transparent); border: 1px solid color-mix(in srgb, var(--green) 55%, transparent); color: var(--green); border-radius: 4px; padding: 0 5px; font-size: 10px; font-family: var(--font-mono); cursor: pointer; flex: none; }
  .port:hover { background: color-mix(in srgb, var(--green) 35%, transparent); }
  .actions button.agent {
    border-color: #f9731680;
    color: #fb923c;
  }
  .actions button.agent:hover {
    border-color: #fb923c;
  }
  .empty {
    color: var(--text-dim);
    padding: 12px;
    font-size: 12px;
  }
</style>
