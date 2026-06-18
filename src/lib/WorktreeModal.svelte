<script>
  import { invoke } from "@tauri-apps/api/core";

  // Manage git worktrees for one repository: isolated checkouts that share the
  // repo's .git, so several Elyra agents can work different branches in
  // parallel. Open a worktree as a terminal or as an agent straight from here.
  let { open = false, project = null, elyra = false, onclose, onopenterminal, onopenagent } = $props();

  let trees = $state([]);
  let loading = $state(false);
  let error = $state(null);
  let busy = $state(false);
  let newBranch = $state("");
  let base = $state("");
  let inputEl = $state();

  function baseOf(p) {
    return p ? p.split("/").pop() : "";
  }

  async function refresh() {
    if (!project) return;
    loading = trees.length === 0;
    try {
      trees = await invoke("git_worktree_list", { path: project.path });
      error = null;
    } catch (e) {
      error = String(e);
      trees = [];
    }
    loading = false;
  }

  async function create(openAs) {
    const branch = newBranch.trim();
    if (!branch || busy) return;
    busy = true;
    error = null;
    try {
      const path = await invoke("git_worktree_add", { repo: project.path, branch, base: base.trim() || null });
      newBranch = "";
      base = "";
      await refresh();
      if (openAs === "agent") onopenagent?.(path, branch);
      else onopenterminal?.(path, branch);
      onclose?.();
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }

  async function remove(t) {
    if (t.is_main) return;
    if (!window.confirm(`Remove worktree for "${t.branch ?? baseOf(t.path)}"?\n\n${t.path}\n\nThe branch itself is kept; only the working copy is removed.`)) return;
    busy = true;
    error = null;
    try {
      await invoke("git_worktree_remove", { repo: project.path, worktreePath: t.path, force: false });
      await refresh();
    } catch (e) {
      // Most failures are "contains modified or untracked files" — offer force.
      if (window.confirm(`Couldn't remove cleanly:\n\n${e}\n\nForce remove (discards uncommitted changes in that worktree)?`)) {
        try {
          await invoke("git_worktree_remove", { repo: project.path, worktreePath: t.path, force: true });
          await refresh();
        } catch (e2) {
          error = String(e2);
        }
      }
    }
    busy = false;
  }

  function onKey(e) {
    if (e.key === "Enter") {
      e.preventDefault();
      create("terminal");
    }
  }

  $effect(() => {
    if (open) {
      trees = [];
      refresh();
      queueMicrotask(() => inputEl?.focus());
    }
  });
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onclose?.()} />

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
      <div class="bar">
        <span class="ttl">🌳 Worktrees</span>
        <span class="repo">{project?.name ?? baseOf(project?.path)}</span>
        <div class="spacer"></div>
        <button class="btn" onclick={refresh} title="Refresh">⟳</button>
        <button class="btn" onclick={onclose} title="Close (Esc)">✕</button>
      </div>

      <div class="create">
        <input
          bind:this={inputEl}
          bind:value={newBranch}
          onkeydown={onKey}
          placeholder="New branch name (e.g. feature/login)…"
          spellcheck="false"
        />
        <input class="base" bind:value={base} placeholder="from (default: HEAD)" spellcheck="false" />
        <button class="btn primary" onclick={() => create("terminal")} disabled={!newBranch.trim() || busy}>＋ Terminal</button>
        {#if elyra}
          <button class="btn agent" onclick={() => create("agent")} disabled={!newBranch.trim() || busy}>🤖 Agent</button>
        {/if}
      </div>

      {#if error}
        <div class="err">{error}</div>
      {/if}

      <div class="list">
        {#if loading}
          <div class="empty">Loading…</div>
        {:else if trees.length === 0}
          <div class="empty">Not a git repository, or no worktrees.</div>
        {:else}
          {#each trees as t (t.path)}
            <div class="row" class:main={t.is_main}>
              <div class="info">
                <div class="branch">
                  <span class="dot"></span>
                  {t.branch ?? "(detached)"}
                  {#if t.is_main}<span class="badge">main</span>{/if}
                  {#if t.locked}<span class="badge lock">locked</span>{/if}
                  <span class="head">{t.head}</span>
                </div>
                <div class="path" title={t.path}>{t.path}</div>
              </div>
              <div class="actions">
                <button class="btn" onclick={() => onopenterminal?.(t.path, t.branch ?? baseOf(t.path))} title="Open a terminal in this worktree">🖥</button>
                {#if elyra}
                  <button class="btn" onclick={() => onopenagent?.(t.path, t.branch ?? baseOf(t.path))} title="Open an Elyra agent in this worktree">🤖</button>
                {/if}
                {#if !t.is_main}
                  <button class="btn danger" onclick={() => remove(t)} disabled={busy} title="Remove this worktree">🗑</button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
      <div class="hint">Worktrees are created in <code>{baseOf(project?.path)}.worktrees/</code> next to the repo.</div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); z-index: 190; display: flex; align-items: flex-start; justify-content: center; padding-top: 8vh; }
  .modal { width: 680px; max-width: 92vw; max-height: 80vh; display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5); overflow: hidden; }
  .bar { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-bottom: 1px solid var(--border); }
  .ttl { font-weight: 600; font-size: 13px; }
  .repo { color: var(--text-dim); font-size: 12px; }
  .spacer { flex: 1; }
  .create { display: flex; gap: 8px; padding: 12px 14px; border-bottom: 1px solid var(--border); }
  .create input { flex: 1; background: var(--bg); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 13px; padding: 7px 10px; outline: none; }
  .create input.base { flex: 0 0 150px; }
  .create input:focus { border-color: var(--accent); }
  .err { margin: 10px 14px 0; background: rgba(247, 118, 142, 0.12); color: #f7768e; border-radius: 7px; padding: 8px 12px; font-size: 12px; font-family: var(--font-mono); white-space: pre-wrap; }
  .list { overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 4px; }
  .empty { color: var(--text-dim); text-align: center; padding: 28px; font-size: 13px; }
  .row { display: flex; align-items: center; gap: 10px; padding: 9px 12px; border-radius: 8px; background: var(--bg-3); }
  .row.main { background: color-mix(in srgb, var(--accent) 8%, var(--bg-3)); }
  .info { flex: 1; min-width: 0; }
  .branch { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); flex: none; }
  .badge { font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); border: 1px solid var(--border); border-radius: 4px; padding: 0 5px; }
  .badge.lock { color: #e0af68; border-color: color-mix(in srgb, #e0af68 40%, var(--border)); }
  .head { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); }
  .path { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: 2px; }
  .actions { display: flex; gap: 4px; flex: none; }
  .btn { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 9px; font-size: 12px; cursor: pointer; }
  .btn:hover:not(:disabled) { border-color: var(--accent); }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .btn.agent { background: var(--accent-2); }
  .btn.danger:hover:not(:disabled) { border-color: #f7768e; color: #f7768e; }
  .hint { padding: 8px 14px; border-top: 1px solid var(--border); font-size: 11px; color: var(--text-dim); }
  .hint code { font-family: var(--font-mono); }
</style>
