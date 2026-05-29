<script>
  import Terminal from "./Terminal.svelte";

  let { open = false, cwd = "", command = "", title = "", onclose } = $props();

  let finished = $state(false);
  let closeTimer = null;

  // Unique id per run so reopening remounts a fresh PTY.
  let runId = $derived(open ? `run-${cwd}-${command}-${title}` : null);

  // The shell exits as soon as the command completes, so the modal can
  // auto-close once the whole flow has been shown.
  let runCommand = $derived(`${command}; exit`);

  function handleExit() {
    finished = true;
    clearTimeout(closeTimer);
    closeTimer = setTimeout(() => onclose?.(), 1600);
  }

  // Reset state each time the modal (re)opens.
  $effect(() => {
    if (open) {
      finished = false;
    } else {
      clearTimeout(closeTimer);
    }
  });
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onclose?.()} />

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="bar">
        <span class="ttl">▶ {title}</span>
        <span class="cmd">{command}</span>
        <span class="cwd" title={cwd}>{cwd}</span>
        <button class="x" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>
      <div class="term-host">
        {#key runId}
          <Terminal id={runId} {cwd} {runCommand} onexit={handleExit} />
        {/key}
      </div>
      <div class="foot" class:done={finished}>
        {#if finished}
          ✓ Finished — closing…
        {:else}
          ● Running…
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 150;
  }
  .modal {
    width: 820px;
    max-width: 92vw;
    height: 520px;
    max-height: 84vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--bg-3);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .ttl {
    font-weight: 600;
    color: var(--green);
  }
  .cmd {
    font-family: var(--font-mono);
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 7px;
  }
  .cwd {
    color: var(--text-dim);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    margin-left: auto;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 6px;
    padding: 3px 8px;
  }
  .x:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .term-host {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .foot {
    padding: 5px 12px;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--accent);
    background: var(--bg-3);
    border-top: 1px solid var(--border);
  }
  .foot.done {
    color: var(--green);
  }
</style>
