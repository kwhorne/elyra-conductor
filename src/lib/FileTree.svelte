<script>
  import { invoke } from "@tauri-apps/api/core";
  import Self from "./FileTree.svelte";
  import { filterEntries } from "./fileFilter.js";
  import { untrack } from "svelte";

  let { entry, onopen, oncontext, activePath = null, depth = 0, showAll = false, refreshKey = 0, onmove } = $props();

  let expanded = $state(false);
  let children = $state(null); // null = not loaded yet
  let loading = $state(false);
  let dropping = $state(false); // folder is a drag-over target

  function onDragStart(e) {
    e.stopPropagation();
    e.dataTransfer.setData("text/plain", entry.path);
    e.dataTransfer.effectAllowed = "move";
  }
  function onDragOver(e) {
    if (!entry.is_dir) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    dropping = true;
  }
  function onDrop(e) {
    if (!entry.is_dir) return;
    e.preventDefault();
    e.stopPropagation();
    dropping = false;
    const from = e.dataTransfer.getData("text/plain");
    if (from) onmove?.(from, entry.path);
  }

  // When the parent bumps refreshKey, re-fetch this folder's children if we've
  // already loaded them (so renames/deletes inside it show up).
  let lastRefresh = 0;
  $effect(() => {
    if (refreshKey !== lastRefresh) {
      lastRefresh = refreshKey;
      untrack(() => {
        if (expanded && children !== null) {
          invoke("list_dir", { path: entry.path }).then((c) => (children = c)).catch(() => {});
        }
      });
    }
  });

  let visibleChildren = $derived(children ? filterEntries(children, showAll) : []);

  async function onClick() {
    if (entry.is_dir) {
      expanded = !expanded;
      if (expanded && children === null) {
        loading = true;
        try {
          children = await invoke("list_dir", { path: entry.path });
        } catch {
          children = [];
        }
        loading = false;
      }
    } else {
      onopen?.(entry.path);
    }
  }
</script>

<button
  class="row"
  class:active={entry.path === activePath}
  class:dropping
  style:padding-left="{8 + depth * 12}px"
  draggable="true"
  ondragstart={onDragStart}
  ondragover={onDragOver}
  ondragleave={() => (dropping = false)}
  ondrop={onDrop}
  onclick={onClick}
  oncontextmenu={(e) => {
    e.preventDefault();
    oncontext?.(entry, e.clientX, e.clientY);
  }}
>
  {#if entry.is_dir}
    <span class="chev">{expanded ? "▾" : "▸"}</span>
    <span class="ico">{expanded ? "📂" : "📁"}</span>
  {:else}
    <span class="chev"></span>
    <span class="ico">📄</span>
  {/if}
  <span class="name">{entry.name}</span>
  {#if loading}<span class="spin">…</span>{/if}
</button>

{#if expanded && children}
  {#each visibleChildren as c (c.path)}
    <Self entry={c} {onopen} {oncontext} {activePath} {showAll} {refreshKey} {onmove} depth={depth + 1} />
  {/each}
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text);
    padding: 3px 8px;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
  }
  .row:hover {
    background: var(--bg-3);
  }
  .row.active {
    background: var(--accent-2);
  }
  .row.dropping {
    background: var(--accent-2);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .chev {
    width: 12px;
    color: var(--text-dim);
    font-size: 10px;
    flex: none;
  }
  .ico {
    font-size: 11px;
    flex: none;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spin {
    margin-left: auto;
    color: var(--text-dim);
  }
</style>
