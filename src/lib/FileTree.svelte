<script>
  import { invoke } from "@tauri-apps/api/core";
  import Self from "./FileTree.svelte";
  import { filterEntries } from "./fileFilter.js";

  let { entry, onopen, oncontext, activePath = null, depth = 0, showAll = false } = $props();

  let expanded = $state(false);
  let children = $state(null); // null = not loaded yet
  let loading = $state(false);

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
  style:padding-left="{8 + depth * 12}px"
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
    <Self entry={c} {onopen} {oncontext} {activePath} {showAll} depth={depth + 1} />
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
