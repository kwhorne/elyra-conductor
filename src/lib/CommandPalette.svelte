<script>
  let { open = false, commands = [], onclose } = $props();

  let query = $state("");
  let selected = $state(0);
  let inputEl;

  let filtered = $derived(
    commands.filter((c) => {
      const hay = `${c.title} ${c.hint ?? ""} ${c.group ?? ""}`.toLowerCase();
      return query
        .toLowerCase()
        .split(/\s+/)
        .filter(Boolean)
        .every((part) => hay.includes(part));
    })
  );

  // Reset + focus whenever the palette opens.
  $effect(() => {
    if (open) {
      query = "";
      selected = 0;
      queueMicrotask(() => inputEl?.focus());
    }
  });

  // Keep selection in range as the list shrinks.
  $effect(() => {
    if (selected >= filtered.length) selected = Math.max(0, filtered.length - 1);
  });

  function run(cmd) {
    onclose?.();
    cmd?.action?.();
  }

  function onKeydown(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, filtered.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      run(filtered[selected]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onclose?.();
    }
  }
</script>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={onclose}
  >
    <div
      class="palette"
      role="dialog"
      aria-modal="true"
      onclick={(e) => e.stopPropagation()}
    >
      <input
        bind:this={inputEl}
        bind:value={query}
        onkeydown={onKeydown}
        placeholder="Search projects, terminals, actions…"
        spellcheck="false"
      />
      <div class="results">
        {#each filtered as cmd, i (cmd.id)}
          <button
            class="result"
            class:active={i === selected}
            onmousemove={() => (selected = i)}
            onclick={() => run(cmd)}
          >
            {#if cmd.icon}<span class="ico">{cmd.icon}</span>{/if}
            <span class="ttl">{cmd.title}</span>
            {#if cmd.hint}<span class="hint">{cmd.hint}</span>{/if}
            {#if cmd.group}<span class="grp">{cmd.group}</span>{/if}
          </button>
        {/each}
        {#if filtered.length === 0}
          <div class="none">No matches</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 100;
  }
  .palette {
    width: 560px;
    max-width: 90vw;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  input {
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 15px;
    padding: 14px 16px;
    outline: none;
    border-bottom: 1px solid var(--border);
  }
  .results {
    max-height: 50vh;
    overflow-y: auto;
    padding: 6px;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text);
    padding: 8px 10px;
    border-radius: 8px;
    font-size: 13px;
  }
  .result.active {
    background: var(--accent-2);
  }
  .ico {
    width: 16px;
    text-align: center;
    color: var(--text-dim);
  }
  .ttl {
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    color: var(--text-dim);
    font-size: 11px;
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grp {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .none {
    color: var(--text-dim);
    padding: 14px;
    font-size: 13px;
  }
</style>
