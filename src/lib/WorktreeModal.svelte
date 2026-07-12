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
  let prs = $state({}); // branch -> PR info (from gh)
  let prsLoading = $state(false);
  let conflicts = $state([]); // { file, worktrees: [branch, ...] } — uncommitted changes overlapping across worktrees

  function baseOf(p) {
    return p ? p.split("/").pop() : "";
  }

  async function refresh() {
    if (!project) return;
    loading = true;
    try {
      trees = await invoke("git_worktree_list", { path: project.path });
      error = null;
    } catch (e) {
      error = String(e);
      trees = [];
    }
    loading = false;
    loadPrs();
    loadConflicts();
  }

  // Cheap, best-effort heads-up: files with uncommitted changes in more than
  // one worktree right now — e.g. two agents about to step on each other.
  async function loadConflicts() {
    if (!project) return;
    try {
      conflicts = await invoke("git_worktree_conflicts", { path: project.path });
    } catch {
      conflicts = [];
    }
  }

  // PRs load lazily and never block the worktree list. Needs an authenticated
  // `gh`; if it's missing we just show no badges.
  async function loadPrs() {
    if (!project) return;
    prsLoading = true;
    try {
      const list = await invoke("gh_pr_list", { repo: project.path });
      const map = {};
      for (const pr of list) map[pr.branch] = pr;
      prs = map;
    } catch {
      prs = {};
    }
    prsLoading = false;
  }

  function openUrl(url) {
    invoke("open_url", { url }).catch(() => {});
  }

  // Open PRs that don't yet have a worktree — offer to check them out.
  let orphanPrs = $derived.by(() => {
    const have = new Set(trees.map((t) => t.branch).filter(Boolean));
    return Object.values(prs).filter((pr) => !have.has(pr.branch));
  });

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

  // Load once per open (keyed on the repo path). Reading reactive state inside
  // refresh() must not feed back into this effect, or it re-runs forever.
  let loadedKey = null;
  $effect(() => {
    const key = open ? project?.path : null;
    if (key && key !== loadedKey) {
      loadedKey = key;
      trees = [];
      prs = {};
      refresh();
      queueMicrotask(() => inputEl?.focus());
    } else if (!open) {
      loadedKey = null;
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

      {#if conflicts.length > 0}
        <div class="conflict-warn">
          <span class="cw-icon">⚠️</span>
          <div class="cw-body">
            <div class="cw-title">{conflicts.length} file{conflicts.length === 1 ? "" : "s"} with uncommitted changes in more than one worktree</div>
            <div class="cw-list">
              {#each conflicts.slice(0, 6) as c (c.file)}
                <div class="cw-row"><code>{c.file}</code> <span class="cw-branches">{c.worktrees.join(", ")}</span></div>
              {/each}
              {#if conflicts.length > 6}<div class="cw-row cw-more">+{conflicts.length - 6} more…</div>{/if}
            </div>
          </div>
        </div>
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
                  {#if t.branch && prs[t.branch]}
                    {@const pr = prs[t.branch]}
                    <button
                      class="pr"
                      class:draft={pr.is_draft}
                      class:fail={pr.checks_failed > 0}
                      class:pass={pr.checks_failed === 0 && pr.checks_pending === 0 && pr.checks_passed > 0}
                      onclick={() => openUrl(pr.url)}
                      title={`${pr.title}\n${pr.is_draft ? "Draft · " : ""}${pr.review_decision || "no review"} · open on GitHub`}
                    >
                      #{pr.number}
                      {#if pr.checks_failed > 0}✗ {pr.checks_failed}{:else if pr.checks_pending > 0}○ {pr.checks_pending}{:else if pr.checks_passed > 0}✓{/if}
                      {#if pr.review_decision === "APPROVED"} 🟢{:else if pr.review_decision === "CHANGES_REQUESTED"} 🔴{/if}
                    </button>
                  {/if}
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
      {#if orphanPrs.length}
        <div class="pr-section">
          <div class="sec-lbl">Open PRs without a worktree</div>
          {#each orphanPrs as pr (pr.number)}
            <div class="pr-row">
              <button
                class="pr"
                class:draft={pr.is_draft}
                class:fail={pr.checks_failed > 0}
                class:pass={pr.checks_failed === 0 && pr.checks_pending === 0 && pr.checks_passed > 0}
                onclick={() => openUrl(pr.url)}
                title="Open on GitHub"
              >
                #{pr.number}
                {#if pr.checks_failed > 0}✗ {pr.checks_failed}{:else if pr.checks_pending > 0}○ {pr.checks_pending}{:else if pr.checks_passed > 0}✓{/if}
                {#if pr.review_decision === "APPROVED"} 🟢{:else if pr.review_decision === "CHANGES_REQUESTED"} 🔴{/if}
              </button>
              <div class="pr-meta">
                <div class="pr-title" title={pr.title}>{pr.title}</div>
                <div class="pr-branch">{pr.branch}</div>
              </div>
              <button class="btn" onclick={() => { newBranch = pr.branch; create("terminal"); }} disabled={busy} title="Check out this PR as a worktree (terminal)">🖥</button>
              {#if elyra}
                <button class="btn" onclick={() => { newBranch = pr.branch; create("agent"); }} disabled={busy} title="Check out this PR as a worktree (Elyra agent)">🤖</button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

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
  .conflict-warn { margin: 10px 14px 0; background: rgba(224, 175, 104, 0.1); border: 1px solid rgba(224, 175, 104, 0.35); border-radius: 7px; padding: 8px 12px; display: flex; gap: 8px; }
  .cw-icon { flex: none; }
  .cw-title { font-size: 12px; font-weight: 600; color: #e0af68; margin-bottom: 4px; }
  .cw-list { display: flex; flex-direction: column; gap: 2px; }
  .cw-row { font-size: 11px; color: var(--text-dim); display: flex; gap: 8px; align-items: baseline; }
  .cw-row code { color: var(--text); font-family: var(--font-mono); }
  .cw-branches { font-family: var(--font-mono); }
  .cw-more { font-style: italic; }
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
  .pr { display: inline-flex; align-items: center; gap: 3px; font-size: 11px; font-family: var(--font-mono); background: var(--bg); border: 1px solid var(--border); color: var(--text-dim); border-radius: 10px; padding: 1px 8px; cursor: pointer; }
  .pr:hover { color: var(--text); border-color: var(--accent); }
  .pr.pass { color: #9ece6a; border-color: color-mix(in srgb, #9ece6a 40%, var(--border)); }
  .pr.fail { color: #f7768e; border-color: color-mix(in srgb, #f7768e 45%, var(--border)); }
  .pr.draft { opacity: 0.7; }
  .path { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: 2px; }
  .actions { display: flex; gap: 4px; flex: none; }
  .btn { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 9px; font-size: 12px; cursor: pointer; }
  .btn:hover:not(:disabled) { border-color: var(--accent); }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .btn.agent { background: var(--accent-2); }
  .btn.danger:hover:not(:disabled) { border-color: #f7768e; color: #f7768e; }
  .pr-section { border-top: 1px solid var(--border); padding: 8px; }
  .sec-lbl { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-dim); padding: 4px 6px 6px; }
  .pr-row { display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: 8px; }
  .pr-row:hover { background: var(--bg-3); }
  .pr-meta { flex: 1; min-width: 0; }
  .pr-title { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pr-branch { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { padding: 8px 14px; border-top: 1px solid var(--border); font-size: 11px; color: var(--text-dim); }
  .hint code { font-family: var(--font-mono); }
</style>
