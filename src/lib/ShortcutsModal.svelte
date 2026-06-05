<script>
  let { open = false, onclose } = $props();

  const groups = [
    {
      title: "General",
      items: [
        ["⌘K", "Command palette"],
        ["⌘/", "Show this help"],
        ["Esc", "Close dialog / palette / search"],
      ],
    },
    {
      title: "Panes & tabs",
      items: [
        ["⌘D", "Split active pane right"],
        ["⇧⌘D", "Split active pane down"],
        ["⌘W", "Close active pane (or editor when focused)"],
        ["Drag tab", "Reorder tabs"],
        ["Drag divider", "Resize panes"],
        ["⌘N · ＋", "New terminal tab"],
      ],
    },
    {
      title: "Terminal",
      items: [
        ["⌘F", "Find in terminal"],
        ["↵ / ⇧↵", "Next / previous match"],
      ],
    },
    {
      title: "Files & editor",
      items: [
        ["⌘B", "Toggle file sidebar"],
        ["⌘P", "Find files / search contents"],
        ["⌘T", "Toggle database explorer"],
        ["Right-click file", "Run in modal / external terminal, open"],
        ["⌘S", "Save file (editor focused)"],
        ["⌘F · ⌘/ · ⌘D", "Editor: find · comment · multi-cursor"],
      ],
    },
    {
      title: "Git",
      items: [
        ["⎇ Commit", "Open commit dialog"],
        ["⌘↵", "Commit (in dialog)"],
      ],
    },
  ];
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onclose?.()} />

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="head">
        <span class="ttl">Keyboard shortcuts</span>
        <button class="x" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>
      <div class="body">
        {#each groups as g (g.title)}
          <div class="group">
            <h3>{g.title}</h3>
            {#each g.items as [keys, desc] (desc)}
              <div class="row">
                <kbd>{keys}</kbd>
                <span class="desc">{desc}</span>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      <div class="foot">On Linux / Windows, <kbd>Ctrl</kbd> substitutes for <kbd>⌘</kbd>.</div>
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
    padding-top: 8vh;
    z-index: 170;
  }
  .modal {
    width: 720px;
    max-width: 92vw;
    max-height: 82vh;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  .head {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .ttl {
    font-weight: 600;
    font-size: 14px;
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
  .body {
    padding: 14px 16px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px 28px;
  }
  .group h3 {
    margin: 0 0 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--accent);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 3px 0;
    font-size: 12px;
  }
  kbd {
    flex: none;
    min-width: 84px;
    text-align: center;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 2px 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text);
  }
  .desc {
    color: var(--text-dim);
  }
  .foot {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-dim);
  }
  .foot kbd {
    min-width: 0;
    display: inline-block;
  }
</style>
