<script>
  let {
    projects = [],
    editors = [],
    activePath = null,
    onselect,
    onopen,
    onroot,
    root = "",
  } = $props();

  let filter = $state("");

  let filtered = $derived(
    projects.filter((p) =>
      p.name.toLowerCase().includes(filter.toLowerCase())
    )
  );

  function pickRoot() {
    onroot?.();
  }
</script>

<div class="sidebar">
  <div class="header">
    <span class="title">Projects</span>
    <button class="root-btn" title="Change folder" onclick={pickRoot}>⋯</button>
  </div>
  <div class="root-path" title={root}>{root || "No folder selected"}</div>

  <input
    class="search"
    placeholder="Search projects…"
    bind:value={filter}
  />

  <div class="list">
    {#each filtered as p (p.path)}
      <div
        class="item"
        class:active={p.path === activePath}
        role="button"
        tabindex="0"
        onclick={() => onselect?.(p)}
        onkeydown={(e) => e.key === "Enter" && onselect?.(p)}
      >
        <div class="row1">
          <span class="name">{p.name}</span>
          {#if p.is_git}
            <span class="branch">⎇ {p.branch ?? "detached"}</span>
          {/if}
        </div>
        {#if p.path === activePath}
          <div class="actions">
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
    {/each}
    {#if filtered.length === 0}
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
    margin-left: auto;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 16px;
    line-height: 1;
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
  .branch {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-dim);
    font-family: var(--font-mono);
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
  .empty {
    color: var(--text-dim);
    padding: 12px;
    font-size: 12px;
  }
</style>
