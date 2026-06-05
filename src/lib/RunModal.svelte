<script>
  import Terminal from "./Terminal.svelte";
  import { untrack } from "svelte";

  let { open = false, cwd = "", command = "", title = "", onclose } = $props();

  let cmd = $state(""); // editable command
  let running = $state(false);
  let finished = $state(false);
  let exitCode = $state(null);
  let runId = $state(null);
  let autoCloseTimer = null;
  let seq = 0;

  // The exact command the embedded terminal is spawned with. Kept as plain state
  // (not a $derived of `cmd`) so the value is fixed at the moment we remount the
  // Terminal — avoiding a race where the pty could spawn before `cmd` propagated
  // and run nothing. `; exit` ends the shell once the command completes.
  let activeRunCommand = $state("exit");

  function start() {
    if (!cmd.trim()) return;
    clearTimeout(autoCloseTimer);
    finished = false;
    exitCode = null;
    running = true;
    activeRunCommand = `${cmd}; exit`;
    runId = `run-${++seq}`; // remount Terminal -> fresh shell runs the command
  }

  function handleExit(code) {
    running = false;
    finished = true;
    exitCode = code;
    clearTimeout(autoCloseTimer);
    // On success, close shortly. On failure, STOP: keep the modal open so the
    // error output stays readable until the user dismisses or re-runs it.
    if (code === 0) autoCloseTimer = setTimeout(() => onclose?.(), 1400);
  }

  // (Re)initialise each time the modal opens, and auto-run the detected command
  // once. Editing the command + Run re-runs it.
  // Track only `open` and `command`. The body reads/writes `cmd` and `runId`,
  // so we untrack it — otherwise the effect re-runs on its own writes and
  // remounts the Terminal twice, which could kill the pty before the command
  // ran (the "modal doesn't run scripts" bug).
  $effect(() => {
    const isOpen = open;
    const initial = command;
    untrack(() => {
      if (isOpen) {
        cmd = initial;
        finished = false;
        exitCode = null;
        running = false;
        start();
      } else {
        clearTimeout(autoCloseTimer);
        running = false;
        runId = null;
      }
    });
  });

  function onKeydown(e) {
    if (!open) return;
    if (e.key === "Escape") onclose?.();
    // ⌘↵ / Ctrl+↵ runs (re-runs) the current command.
    else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") start();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="bar">
        <span class="ttl">▶ {title}</span>
        <input
          class="cmd"
          bind:value={cmd}
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          placeholder="command to run…"
          onkeydown={(e) => e.key === "Enter" && (e.preventDefault(), start())}
        />
        <button class="run" onclick={start} disabled={!cmd.trim()} title="Run (⌘↵)">
          {running ? "Running…" : finished ? "Re-run" : "Run"}
        </button>
        <span class="cwd" title={cwd}>{cwd}</span>
        <button class="x" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>
      <div class="term-host">
        {#key runId}
          {#if runId}
            <Terminal id={runId} {cwd} runCommand={activeRunCommand} onexit={handleExit} />
          {/if}
        {/key}
      </div>
      <div class="foot" class:done={finished && exitCode === 0} class:fail={finished && exitCode !== 0}>
        {#if running}
          ● Running…
        {:else if finished}
          {#if exitCode === 0}
            ✓ Finished (exit 0) — closing…
          {:else}
            ✗ Failed (exit {exitCode}) — stopped. Fix the command and press Re-run, or Esc to close.
          {/if}
        {:else}
          ○ Idle
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
    white-space: nowrap;
  }
  .cmd {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 4px 8px;
    font-size: 12px;
    outline: none;
  }
  .cmd:focus {
    border-color: var(--accent);
  }
  .run {
    white-space: nowrap;
    background: var(--accent-2);
    border: 1px solid var(--accent);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 12px;
    font-weight: 600;
  }
  .run:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .run:hover:not(:disabled) {
    background: var(--accent);
  }
  .cwd {
    max-width: 220px;
    color: var(--text-dim);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
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
  .foot.fail {
    color: #f7768e;
  }
</style>
