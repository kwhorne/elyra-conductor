<script>
  // Inline ask bar: ⌘+Enter in a terminal pane, Esc to dismiss.
  //
  // The point is that it works wherever the shell is — a local zsh, an SSH
  // session, `docker exec`, a REPL — because the context it sends is the
  // *scrollback*, which Conductor has locally no matter where the shell runs.
  //
  // It answers; it does not act. Suggested commands get an **Insert** button
  // that puts them on the prompt line for you to read and run yourself. Nothing
  // is executed on an LLM's say-so, and there is no "run all" — see the note on
  // extractCommands below.
  import { marked } from "marked";
  import { sanitizeMarkdownHtml } from "./sanitize.js";

  let {
    open = false,
    busy = false,
    answer = "",
    error = null,
    contextNote = "",
    onsend,
    oncancel,
    onclose,
    oninsert,
    onescalate,
  } = $props();

  let question = $state("");
  let inputEl = $state(null);

  // Focus the input whenever the bar opens, and clear the previous question so
  // ⌘+Enter is always a fresh start rather than an edit of what you last asked.
  $effect(() => {
    if (open) {
      question = "";
      // After paint: the element does not exist yet on the tick that flips `open`.
      requestAnimationFrame(() => inputEl?.focus());
    }
  });

  let html = $derived.by(() => {
    if (!answer) return "";
    try {
      return sanitizeMarkdownHtml(marked.parser(marked.lexer(answer)));
    } catch {
      return "";
    }
  });

  // Pull shell commands out of fenced code blocks so they can be offered for
  // review. Deliberately conservative: only bash/sh/zsh/console fences, and only
  // whole lines that aren't comments. Anything clever here would be a way to get
  // an LLM's text onto a prompt line without the user having read it.
  let commands = $derived.by(() => {
    const out = [];
    const re = /```(?:bash|sh|zsh|shell|console)?\n([\s\S]*?)```/g;
    let m;
    while ((m = re.exec(answer)) !== null) {
      for (const raw of m[1].split("\n")) {
        const line = raw.replace(/^\s*[$#]\s+/, "").trim();
        if (line && !line.startsWith("#") && out.length < 6) out.push(line);
      }
    }
    return out;
  });

  function submit() {
    const q = question.trim();
    if (!q || busy) return;
    onsend?.(q);
  }

  function onKeydown(e) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onclose?.();
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ask" onkeydown={onKeydown}>
    <div class="row">
      <span class="glyph">✦</span>
      <input
        bind:this={inputEl}
        bind:value={question}
        placeholder="Ask about this terminal…"
        spellcheck="false"
        disabled={busy}
      />
      {#if busy}
        <button class="ghost" onclick={oncancel}>Stop</button>
      {:else}
        <button class="ghost" onclick={submit} disabled={!question.trim()}>Ask ↵</button>
      {/if}
      <button class="icon" title="Close (Esc)" onclick={onclose}>✕</button>
    </div>

    {#if contextNote && !answer && !busy}
      <div class="ctx">{contextNote}</div>
    {/if}

    {#if busy && !answer}
      <div class="ctx thinking">Asking Elyra…</div>
    {/if}

    {#if error}
      <div class="err">{error}</div>
    {/if}

    {#if html}
      <div class="answer">{@html html}</div>
      {#if commands.length > 0}
        <div class="cmds">
          <span class="cmds-label">Put on the prompt line:</span>
          {#each commands as c (c)}
            <button class="cmd" title={c} onclick={() => oninsert?.(c)}>{c}</button>
          {/each}
        </div>
      {/if}
      <div class="foot">
        <span>Advice only — nothing was run. Insert puts a command on the prompt for you to review.</span>
        <button class="ghost" onclick={onescalate}>Open in Elyra ▸</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .ask {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 20;
    background: var(--bg-2);
    border-top: 1px solid var(--accent);
    padding: 8px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    max-height: 62%;
    box-shadow: 0 -10px 26px rgba(0, 0, 0, 0.42);
  }
  .row { display: flex; align-items: center; gap: 8px; }
  .glyph { color: var(--accent); font-size: 12px; flex: none; }
  input {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    padding: 5px 9px;
    outline: none;
    min-width: 0;
  }
  input:focus { border-color: var(--accent); }
  input:disabled { opacity: 0.6; }
  .ghost {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 9px;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }
  .ghost:hover:not(:disabled) { border-color: var(--accent); }
  .ghost:disabled { opacity: 0.45; cursor: default; }
  .icon {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 12px;
    cursor: pointer;
    padding: 2px 4px;
  }
  .icon:hover { color: var(--text); }
  .ctx { font-size: 10px; color: var(--text-dim); }
  .ctx.thinking { color: var(--accent); }
  .err {
    font-size: 11px;
    color: var(--red);
    background: var(--bg);
    border-radius: 6px;
    padding: 6px 8px;
    font-family: var(--font-mono);
    white-space: pre-wrap;
  }
  .answer {
    overflow-y: auto;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text);
    padding-bottom: 2px;
  }
  .answer :global(p) { margin: 0 0 7px; }
  .answer :global(pre) {
    background: var(--bg);
    border-radius: 6px;
    padding: 7px 9px;
    overflow-x: auto;
    margin: 0 0 7px;
  }
  .answer :global(code) { font-family: var(--font-mono); font-size: 11px; }
  .answer :global(ul), .answer :global(ol) { margin: 0 0 7px; padding-left: 18px; }
  .answer :global(li) { margin-bottom: 3px; }
  .cmds { display: flex; flex-wrap: wrap; align-items: center; gap: 5px; }
  .cmds-label { font-size: 10px; color: var(--text-dim); }
  .cmd {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 5px;
    padding: 3px 8px;
    font-family: var(--font-mono);
    font-size: 10px;
    cursor: pointer;
    max-width: 340px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cmd:hover { border-color: var(--accent); }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 10px;
    color: var(--text-dim);
  }
  .foot span { flex: 1; }
</style>
