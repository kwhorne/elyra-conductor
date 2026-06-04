<script>
  import { invoke } from "@tauri-apps/api/core";
  import FileTree from "./FileTree.svelte";
  import { filterEntries } from "./fileFilter.js";

  let { root, onopen, oncontext, activePath = null, showAll = false, ontoggleall, refreshKey = 0, onmove } = $props();

  let rootDropping = $state(false);
  function onRootDragOver(e) {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    rootDropping = true;
  }
  function onRootDrop(e) {
    e.preventDefault();
    rootDropping = false;
    const from = e.dataTransfer.getData("text/plain");
    if (from && root) onmove?.(from, root);
  }

  let visible = $derived(filterEntries(entries, showAll));

  let entries = $state([]);
  let loadedRoot = $state(null);
  let error = $state("");

  async function load(r) {
    loadedRoot = r;
    error = "";
    try {
      entries = await invoke("list_dir", { path: r });
    } catch (e) {
      entries = [];
      error = String(e);
    }
  }

  // (Re)load when the root folder changes.
  $effect(() => {
    if (root && root !== loadedRoot) load(root);
  });

  // Reload the top level whenever the parent bumps refreshKey (after a file op).
  let lastRefresh = 0;
  $effect(() => {
    if (refreshKey !== lastRefresh) {
      lastRefresh = refreshKey;
      if (root) load(root);
    }
  });

  function refresh() {
    if (root) load(root);
  }

  function basename(p) {
    return p ? p.split("/").pop() : "";
  }
</script>

<div class="explorer">
  <div class="head">
    <span class="title">Files</span>
    <span class="root" title={root}>{basename(root)}</span>
    <button
      class="refresh"
      class:on={showAll}
      title={showAll ? "Hiding nothing — click to hide node_modules/.git/…" : "Show all (incl. node_modules/.git)"}
      onclick={() => ontoggleall?.()}>{showAll ? "👁" : "⊘"}</button>
    <button class="refresh" title="Refresh" onclick={refresh}>⟳</button>
  </div>
  <div class="list" class:dropping={rootDropping} ondragover={onRootDragOver} ondragleave={() => (rootDropping = false)} ondrop={onRootDrop} role="tree">
    {#if !root}
      <div class="empty">No folder</div>
    {:else if error}
      <div class="empty">{error}</div>
    {:else if entries.length === 0}
      <div class="empty">Empty folder</div>
    {:else}
      {#each visible as e (e.path)}
        <FileTree entry={e} {onopen} {oncontext} {activePath} {showAll} {refreshKey} {onmove} depth={0} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .explorer {
    width: 240px;
    min-width: 240px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-left: 1px solid var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .title {
    font-weight: 600;
    font-size: 12px;
  }
  .root {
    font-size: 11px;
    color: var(--text-dim);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .refresh {
    margin-left: auto;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 13px;
  }
  .refresh:hover {
    color: var(--text);
  }
  .refresh.on {
    color: var(--accent);
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px 0;
  }
  .list.dropping {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 10px;
  }
</style>
