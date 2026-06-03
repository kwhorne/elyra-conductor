<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, onclose } = $props();

  let ports = $state([]);
  let loading = $state(false);
  let error = $state(null);
  let timer = null;

  async function refresh() {
    loading = ports.length === 0;
    try {
      ports = await invoke("list_ports");
      error = null;
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  async function openPort(p) {
    try {
      await invoke("open_url", { url: `http://localhost:${p.port}` });
    } catch (e) {
      error = String(e);
    }
  }

  async function kill(p) {
    try {
      await invoke("kill_process", { pid: p.pid });
      setTimeout(refresh, 300);
    } catch (e) {
      error = String(e);
    }
  }

  // Poll while open.
  $effect(() => {
    if (open) {
      refresh();
      clearInterval(timer);
      timer = setInterval(refresh, 2500);
    } else {
      clearInterval(timer);
    }
  });
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onclose?.()} />

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="bar">
        <span class="ttl">⚡ Listening ports</span>
        <span class="count">{ports.length}</span>
        <div class="spacer"></div>
        <button class="btn" onclick={refresh} title="Refresh">⟳</button>
        <button class="x" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>

      {#if error}<div class="err">{error}</div>{/if}

      <div class="body">
        {#if loading}
          <div class="empty">Scanning…</div>
        {:else if ports.length === 0}
          <div class="empty">No local TCP ports are listening.</div>
        {:else}
          <table class="grid">
            <thead>
              <tr><th>Port</th><th>Process</th><th>PID</th><th>Address</th><th></th></tr>
            </thead>
            <tbody>
              {#each ports as p (p.port)}
                <tr>
                  <td class="port"><button class="link" onclick={() => openPort(p)} title="Open http://localhost:{p.port}">:{p.port}</button></td>
                  <td>{p.process}</td>
                  <td class="dim">{p.pid}</td>
                  <td class="dim">{p.addr}</td>
                  <td class="actions">
                    <button class="mini" onclick={() => openPort(p)} title="Open in browser">↗</button>
                    <button class="mini kill" onclick={() => kill(p)} title="Stop process (SIGTERM)">⏹</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
      <div class="foot">Click a port to open <code>http://localhost:&lt;port&gt;</code> · ⏹ sends SIGTERM</div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 150; }
  .modal { width: 620px; max-width: 92vw; max-height: 80vh; display: flex; flex-direction: column; background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; overflow: hidden; box-shadow: 0 24px 60px rgba(0,0,0,0.55); }
  .bar { display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: var(--bg-3); border-bottom: 1px solid var(--border); font-size: 13px; }
  .ttl { font-weight: 600; }
  .count { color: var(--text-dim); font-family: var(--font-mono); font-size: 11px; }
  .spacer { flex: 1; }
  .btn { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 3px 9px; font-size: 12px; }
  .btn:hover { border-color: var(--accent); }
  .x { background: transparent; border: 1px solid var(--border); color: var(--text-dim); border-radius: 6px; padding: 3px 8px; }
  .x:hover { color: var(--text); border-color: var(--accent); }
  .err { padding: 6px 14px; color: #f7768e; font-family: var(--font-mono); font-size: 11px; }
  .body { flex: 1; min-height: 0; overflow: auto; }
  .grid { width: 100%; border-collapse: collapse; font-size: 13px; }
  .grid th { position: sticky; top: 0; text-align: left; background: var(--bg-3); color: var(--text-dim); font-weight: 500; font-size: 11px; padding: 6px 14px; }
  .grid td { padding: 5px 14px; border-top: 1px solid var(--border); }
  .grid tbody tr:hover { background: var(--bg-3); }
  .port .link { background: transparent; border: none; color: var(--accent); font-family: var(--font-mono); font-size: 13px; cursor: pointer; padding: 0; }
  .port .link:hover { text-decoration: underline; }
  .dim { color: var(--text-dim); font-family: var(--font-mono); font-size: 12px; }
  .actions { text-align: right; white-space: nowrap; }
  .mini { background: transparent; border: 1px solid var(--border); color: var(--text-dim); border-radius: 5px; padding: 1px 7px; font-size: 12px; margin-left: 4px; cursor: pointer; }
  .mini:hover { color: var(--text); border-color: var(--accent); }
  .mini.kill:hover { color: #f7768e; border-color: #f7768e; }
  .empty { padding: 32px; text-align: center; color: var(--text-dim); }
  .foot { padding: 6px 14px; font-size: 11px; color: var(--text-dim); background: var(--bg-3); border-top: 1px solid var(--border); }
</style>
