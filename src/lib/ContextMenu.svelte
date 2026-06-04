<script>
  let { open = false, x = 0, y = 0, items = [], onclose } = $props();

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
    onclick={onclose}
    oncontextmenu={(e) => {
      e.preventDefault();
      onclose?.();
    }}
  >
    <div
      class="menu"
      style:left="{x}px"
      style:top="{y}px"
      role="menu"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
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
