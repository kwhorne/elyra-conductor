<script>
  import { invoke } from "@tauri-apps/api/core";

  // The multi-agent cockpit: every open Elyra agent tab across every
  // project/worktree, in one place — presence, status, last activity — plus
  // an auto-merge queue for branches whose PR is green and ready to ship.
  // Pure host UI: reads git/gh state and forwards clicks; no AI here.
  let {
    open = false,
    tabs = [],
    projects = [],
    pinned = [],
    agentPresence = {},
    agentStatusText = {},
    agentLastActive = {},
    onjump,
    onclosetab,
    onclose,
  } = $props();

  const PRESENCE_LABEL = { working: "Working", waiting: "Waiting for you", idle: "Idle", exited: "Exited" };
  const PRESENCE_ORDER = { waiting: 0, working: 1, idle: 2, exited: 3 };

  function ownerProject(cwd) {
    if (!cwd) return null;
    const all = [...projects, ...pinned];
    return (
      all.find((p) => cwd === p.path) ??
      all.find((p) => cwd.startsWith(p.path + ".worktrees/") || cwd.startsWith(p.path + "/")) ??
      null
    );
  }
  function baseOf(p) {
    return p ? p.split("/").pop() : "";
  }

  let agentRows = $derived.by(() => {
    const rows = tabs
      .filter((t) => t.kind === "agent")
      .map((t) => {
        const proj = ownerProject(t.cwd);
        return {
          tab: t,
          project: proj,
          projectName: proj?.name ?? baseOf(t.cwd),
          presence: agentPresence[t.id] ?? "idle",
          status: agentStatusText[t.id] ?? "",
          lastActive: agentLastActive[t.id] ?? null,
        };
      });
    rows.sort((a, b) => (PRESENCE_ORDER[a.presence] ?? 9) - (PRESENCE_ORDER[b.presence] ?? 9));
    return rows;
  });

  function ago(ms) {
    if (!ms) return "";
    const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
    if (s < 5) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    return `${h}h ago`;
  }

  // ── auto-merge queue ─────────────────────────────────────────────────────
  // For every distinct project behind an open agent, load its worktrees + open
  // PRs, then surface branches whose PR is green (not draft, no failing/pending
  // checks) as ready to merge.
  let queue = $state([]); // { project, branch, pr, worktreePath, tabIds, merging, error }
  let queueLoading = $state(false);

  async function loadQueue() {
    queueLoading = true;
    const byProject = new Map();
    for (const r of agentRows) {
      if (r.project) byProject.set(r.project.path, r.project);
    }
    const items = [];
    for (const proj of byProject.values()) {
      let prs = [];
      let trees = [];
      try {
        [prs, trees] = await Promise.all([
          invoke("gh_pr_list", { repo: proj.path }),
          invoke("git_worktree_list", { path: proj.path }),
        ]);
      } catch {
        continue;
      }
      for (const pr of prs) {
        if (pr.is_draft || pr.checks_failed > 0 || pr.checks_pending > 0 || pr.checks_passed === 0) continue;
        const wt = trees.find((t) => t.branch === pr.branch);
        const tabIds = tabs.filter((t) => t.kind === "agent" && wt && t.cwd === wt.path).map((t) => t.id);
        items.push({
          project: proj,
          branch: pr.branch,
          pr,
          worktreePath: wt?.path ?? null,
          tabIds,
          merging: false,
          error: null,
        });
      }
    }
    queue = items;
    queueLoading = false;
  }

  let loadedForKey = null;
  $effect(() => {
    const key = open ? tabs.filter((t) => t.kind === "agent").map((t) => t.cwd).join("|") : null;
    if (key !== null && key !== loadedForKey) {
      loadedForKey = key;
      loadQueue();
    } else if (!open) {
      loadedForKey = null;
    }
  });

  function openUrl(url) {
    invoke("open_url", { url }).catch(() => {});
  }

  async function mergeItem(item, method = "squash") {
    if (item.merging) return;
    const label = `#${item.pr.number} ${item.pr.title}`;
    if (!window.confirm(`Merge ${label} into the base branch (squash) and delete the remote branch "${item.branch}"?`)) return;
    item.merging = true;
    item.error = null;
    queue = [...queue];
    try {
      await invoke("gh_pr_merge", { repo: item.project.path, number: item.pr.number, method });
      // Clean up: close any agent tabs on that branch, then remove the worktree.
      for (const id of item.tabIds) onclosetab?.(id);
      if (item.worktreePath) {
        try {
          await invoke("git_worktree_remove", { repo: item.project.path, worktreePath: item.worktreePath, force: false });
        } catch {
          // Left for the user to clean up from the Worktrees panel — the merge itself succeeded.
        }
      }
      queue = queue.filter((q) => q !== item);
    } catch (e) {
      item.error = String(e);
      item.merging = false;
      queue = [...queue];
    }
  }

  function jump(tab) {
    onjump?.(tab);
  }

  function onKeydown(e) {
    if (e.key === "Escape") onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
      <div class="head">
        <span class="title">🎛 Agent dashboard</span>
        <button class="x" onclick={onclose}>✕</button>
      </div>

      <div class="section">
        <div class="section-head">
          <span>Agents</span>
          <span class="count">{agentRows.length}</span>
        </div>
        {#if agentRows.length === 0}
          <div class="hint">No agent tabs open. Start one from a project or a worktree.</div>
        {:else}
          <div class="rows">
            {#each agentRows as r (r.tab.id)}
              <button class="row" onclick={() => jump(r.tab)}>
                <span class="dot {r.presence}" title={PRESENCE_LABEL[r.presence]}></span>
                <span class="rtitle">{r.tab.title || "elyra"}</span>
                <span class="rproj" title={r.tab.cwd}>{r.projectName}</span>
                <span class="rstatus">{r.status}</span>
                <span class="rago">{ago(r.lastActive)}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="section">
        <div class="section-head">
          <span>Ready to merge</span>
          <span class="count">{queueLoading ? "…" : queue.length}</span>
        </div>
        {#if !queueLoading && queue.length === 0}
          <div class="hint">Nothing green yet — branches show up here once their PR's checks all pass.</div>
        {:else}
          <div class="rows">
            {#each queue as item (item.project.path + item.branch)}
              <div class="mrow">
                <div class="mmain">
                  <button class="mlink" title={item.pr.url} onclick={() => openUrl(item.pr.url)}>#{item.pr.number}</button>
                  <span class="mtitle">{item.pr.title}</span>
                  <span class="mbranch">{item.branch}</span>
                  <span class="mchecks ok">✓ {item.pr.checks_passed}</span>
                  {#if item.pr.review_decision === "APPROVED"}<span class="mreview ok">🟢 approved</span>{/if}
                </div>
                <div class="mactions">
                  {#if item.error}<span class="merr" title={item.error}>✗ failed</span>{/if}
                  <button class="ghost" disabled={item.merging} onclick={() => mergeItem(item)}>{item.merging ? "Merging…" : "Squash & merge"}</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 14px; padding: 20px 22px; width: 640px; max-width: 94vw; max-height: 86vh; overflow-y: auto; box-shadow: 0 20px 56px rgba(0,0,0,0.55); }
  .head { display: flex; align-items: center; margin-bottom: 14px; }
  .title { font-size: 15px; font-weight: 700; flex: 1; }
  .x { background: transparent; border: none; color: var(--text-dim); font-size: 13px; cursor: pointer; }
  .x:hover { color: var(--text); }
  .section { margin-bottom: 18px; }
  .section-head { display: flex; align-items: center; gap: 8px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: var(--accent); margin-bottom: 8px; }
  .section-head .count { color: var(--text-dim); font-family: var(--font-mono); font-weight: 400; text-transform: none; letter-spacing: normal; }
  .hint { color: var(--text-dim); font-size: 12px; padding: 10px 4px; }
  .rows { display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; align-items: center; gap: 10px; width: 100%; text-align: left; background: var(--bg-3); border: 1px solid transparent; border-radius: 8px; padding: 8px 10px; cursor: pointer; }
  .row:hover { border-color: var(--accent); }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; background: var(--text-dim); }
  .dot.working { background: var(--accent); animation: pulse 1s ease-in-out infinite; }
  .dot.waiting { background: #e0af68; animation: pulse 1.1s ease-in-out infinite; }
  .dot.exited { background: var(--text-dim); opacity: 0.5; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
  .rtitle { font-weight: 600; font-size: 12px; flex: none; }
  .rproj { font-size: 11px; color: var(--text-dim); flex: none; padding: 1px 6px; background: var(--bg); border-radius: 4px; }
  .rstatus { font-size: 11px; color: var(--text-dim); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rago { font-size: 10px; color: var(--text-dim); font-family: var(--font-mono); flex: none; }
  .mrow { display: flex; align-items: center; justify-content: space-between; gap: 10px; background: var(--bg-3); border-radius: 8px; padding: 8px 10px; }
  .mmain { display: flex; align-items: center; gap: 8px; min-width: 0; flex-wrap: wrap; }
  .mlink { background: transparent; border: none; color: var(--accent); font-family: var(--font-mono); font-size: 11px; cursor: pointer; padding: 0; }
  .mlink:hover { text-decoration: underline; }
  .mtitle { font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
  .mbranch { font-size: 10px; font-family: var(--font-mono); color: var(--text-dim); background: var(--bg); border-radius: 4px; padding: 1px 6px; }
  .mchecks.ok { font-size: 11px; color: var(--green); }
  .mreview.ok { font-size: 11px; color: var(--green); }
  .mactions { display: flex; align-items: center; gap: 8px; flex: none; }
  .merr { color: #f7768e; font-size: 11px; cursor: help; }
  .ghost { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 10px; font-size: 11px; cursor: pointer; }
  .ghost:hover:not(:disabled) { border-color: var(--accent); }
  .ghost:disabled { opacity: 0.6; cursor: default; }
</style>
