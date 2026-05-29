<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, path = "", projectName = "", onclose, oncommitted } = $props();

  let changes = $state([]);
  let message = $state("");
  let push = $state(false);
  let busy = $state(false);
  let error = $state("");
  let messageEl;

  async function load() {
    error = "";
    try {
      changes = await invoke("git_changes", { path });
    } catch (e) {
      changes = [];
      error = String(e);
    }
  }

  $effect(() => {
    if (open && path) {
      message = "";
      push = false;
      error = "";
      busy = false;
      load();
      queueMicrotask(() => messageEl?.focus());
    }
  });

  async function commit(doPush) {
    if (busy || !message.trim()) return;
    busy = true;
    error = "";
    try {
      await invoke("git_commit", { path, message, push: doPush });
      oncommitted?.();
      onclose?.();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function onKeydown(e) {
    if (e.key === "Escape") onclose?.();
    else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") commit(push);
  }

  const STATUS_LABEL = { M: "modified", A: "added", D: "deleted", R: "renamed", "??": "new" };
  function badgeClass(s) {
    if (s === "M") return "mod";
    if (s === "A") return "add";
    if (s === "D") return "del";
    if (s === "??") return "new";
    return "other";
  }
</script>

<svelte:window onkeydown={(e) => open && onKeydown(e)} />

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="dialog" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="head">
        <span class="ttl">⎇ Commit — {projectName}</span>
        <button class="x" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>

      <div class="changes">
        {#if changes.length === 0}
          <div class="clean">Working tree clean — nothing to commit.</div>
        {:else}
          {#each changes as c (c.file)}
            <div class="change">
              <span class="badge {badgeClass(c.status)}">{STATUS_LABEL[c.status] ?? c.status}</span>
              <span class="file">{c.file}</span>
            </div>
          {/each}
        {/if}
      </div>

      <textarea
        bind:this={messageEl}
        bind:value={message}
        placeholder="Commit message…  (⌘↵ to commit)"
        rows="3"
      ></textarea>

      {#if error}
        <pre class="error">{error}</pre>
      {/if}

      <div class="foot">
        <label class="push">
          <input type="checkbox" bind:checked={push} /> Push after commit
        </label>
        <div class="actions">
          <button onclick={onclose}>Cancel</button>
          <button
            class="primary"
            disabled={busy || !message.trim() || changes.length === 0}
            onclick={() => commit(push)}
          >
            {busy ? "Working…" : push ? "Commit & Push" : "Commit"}
          </button>
        </div>
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
    align-items: flex-start;
    padding-top: 10vh;
    z-index: 160;
  }
  .dialog {
    width: 560px;
    max-width: 92vw;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  .head {
    display: flex;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .ttl {
    font-weight: 600;
    font-size: 13px;
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
  .changes {
    max-height: 200px;
    overflow-y: auto;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
  }
  .clean {
    color: var(--text-dim);
    font-size: 13px;
    padding: 6px 0;
  }
  .change {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: 12px;
  }
  .badge {
    font-size: 10px;
    border-radius: 4px;
    padding: 1px 6px;
    min-width: 60px;
    text-align: center;
    color: var(--bg);
    background: var(--text-dim);
  }
  .badge.mod { background: #e0af68; }
  .badge.add,
  .badge.new { background: var(--green); }
  .badge.del { background: var(--red); }
  .file {
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  textarea {
    width: 100%;
    border: none;
    background: var(--bg-3);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 12px 14px;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }
  .error {
    margin: 0;
    padding: 10px 14px;
    background: rgba(247, 118, 142, 0.12);
    color: var(--red);
    font-size: 11px;
    font-family: var(--font-mono);
    white-space: pre-wrap;
    max-height: 140px;
    overflow: auto;
  }
  .foot {
    display: flex;
    align-items: center;
    padding: 10px 14px;
    gap: 12px;
    border-top: 1px solid var(--border);
  }
  .push {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .actions button {
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 7px;
    padding: 6px 14px;
    font-size: 12px;
  }
  .actions .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .actions .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
