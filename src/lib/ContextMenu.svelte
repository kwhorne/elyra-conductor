<script>
  let { open = false, x = 0, y = 0, items = [], onclose } = $props();

  // Keep the menu fully on-screen. A raw click near the right/bottom edge
  // would otherwise push it past the viewport (see file context menu on the
  // last row). We measure the rendered menu and clamp/flip it inward, and hide
  // it for the single measuring frame to avoid a flash at the wrong spot.
  let menuEl = $state();
  let left = $state(0);
  let top = $state(0);
  let placed = $state(false);

  $effect(() => {
    if (!open) {
      placed = false;
      return;
    }
    // Re-measure whenever a new menu opens (coords or items change).
    void x;
    void y;
    void items;
    if (!menuEl) return;
    const pad = 8;
    const r = menuEl.getBoundingClientRect();
    let l = x;
    let t = y;
    if (l + r.width > window.innerWidth - pad) l = Math.max(pad, window.innerWidth - r.width - pad);
    if (t + r.height > window.innerHeight - pad) t = Math.max(pad, window.innerHeight - r.height - pad);
    left = l;
    top = t;
    placed = true;
  });

  function choose(item) {
    if (item.disabled) return;
    onclose?.();
    item.action?.();
  }
</script>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onclose?.()}
    oncontextmenu={(e) => {
      e.preventDefault();
      onclose?.();
    }}
  >
    <div
      class="menu"
      bind:this={menuEl}
      style:left="{placed ? left : x}px"
      style:top="{placed ? top : y}px"
      style:opacity={placed ? 1 : 0}
      role="menu"
      tabindex="-1"
    >
      {#each items as it, i (i)}
        {#if it.separator}
          <div class="sep"></div>
        {:else}
          <button class="item" class:danger={it.danger} disabled={it.disabled} onclick={() => choose(it)}>
            {#if it.icon}<span class="ico">{it.icon}</span>{/if}
            <span class="label">{it.label}</span>
          </button>
        {/if}
      {/each}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
  }
  .menu {
    position: fixed;
    min-width: 220px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
    padding: 4px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text);
    padding: 6px 9px;
    border-radius: 6px;
    font-size: 13px;
  }
  .item:hover:not(:disabled) {
    background: var(--accent-2);
  }
  .item.danger {
    color: #e06c5a;
  }
  .item.danger:hover:not(:disabled) {
    background: rgba(192, 57, 43, 0.18);
  }
  .item:disabled {
    color: var(--text-dim);
    opacity: 0.5;
    cursor: default;
  }
  .ico {
    width: 16px;
    text-align: center;
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 4px 6px;
  }
</style>
