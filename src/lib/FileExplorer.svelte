<script>
  import { invoke } from "@tauri-apps/api/core";
  import FileTree from "./FileTree.svelte";

  let { root, onopen, oncontext, activePath = null } = $props();

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
    <button class="refresh" title="Refresh" onclick={refresh}>⟳</button>
  </div>
  <div class="list">
    {#if !root}
      <div class="empty">No folder</div>
    {:else if error}
      <div class="empty">{error}</div>
    {:else if entries.length === 0}
      <div class="empty">Empty folder</div>
    {:else}
      {#each entries as e (e.path)}
        <FileTree entry={e} {onopen} {oncontext} {activePath} depth={0} />
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
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px 0;
  }
  .empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 10px;
  }
</style>
