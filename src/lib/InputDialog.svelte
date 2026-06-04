<script>
  // A small reusable dialog: either a single-line text prompt (rename, new file,
  // new folder) or a confirm (delete). Pure UI — the caller does the work.
  let {
    open = false,
    title = "",
    message = "",
    input = false,
    value = "",
    placeholder = "",
    confirmLabel = "OK",
    danger = false,
    onconfirm,
    onclose,
  } = $props();

  let draft = $state("");
  let box; // input element
  let okBtn; // confirm button (for confirm-only dialogs)

  // Reset the draft each time the dialog opens, and focus the right control so
  // Enter works whether we're prompting or just confirming.
  $effect(() => {
    if (open) {
      draft = value;
      queueMicrotask(() => {
        if (input) {
          box?.focus();
          box?.select?.();
        } else {
          okBtn?.focus();
        }
      });
    }
  });

  let canConfirm = $derived(!input || draft.trim().length > 0);

  function confirm() {
    if (!canConfirm) return;
    onconfirm?.(input ? draft.trim() : true);
    onclose?.();
  }

  function onKeydown(e) {
    if (e.key === "Enter") {
      e.preventDefault();
      confirm();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onclose?.();
    }
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="title">{title}</div>
      {#if message}<div class="msg">{message}</div>{/if}
      {#if input}
        <input
          bind:this={box}
          class="field"
          bind:value={draft}
          {placeholder}
          onkeydown={onKeydown}
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
        />
      {/if}
      <div class="row">
        <button class="btn" onclick={onclose}>Cancel</button>
        <button bind:this={okBtn} class="btn primary" class:danger onclick={confirm} onkeydown={onKeydown} disabled={!canConfirm}>{confirmLabel}</button>
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
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 18px;
    width: 380px;
    max-width: 90vw;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }
  .title {
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 6px;
  }
  .msg {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 12px;
    word-break: break-word;
  }
  .field {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 8px 10px;
    outline: none;
    margin-bottom: 14px;
  }
  .field:focus {
    border-color: var(--accent);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-size: 12px;
    padding: 6px 14px;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--bg-4, var(--bg-3));
    filter: brightness(1.1);
  }
  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .btn.primary.danger {
    background: #c0392b;
    border-color: #c0392b;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
