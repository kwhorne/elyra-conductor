<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, path = "", projectName = "", onclose, onchanged } = $props();

  let files = $state([]); // { file, x, y, staged, untracked }
  let branches = $state({ current: null, all: [] });
  let stashes = $state([]);
  let sel = $state(null); // selected file object
  let diff = $state("");
  let message = $state("");
  let busy = $state(false);
  let err = $state("");
  let newBranch = $state(""); // inline create-branch input
  let showNewBranch = $state(false);

  let staged = $derived(files.filter((f) => f.staged));
  let unstaged = $derived(files.filter((f) => !f.staged || f.untracked));

  async function reload() {
    if (!path) return;
    try {
      [files, branches, stashes] = await Promise.all([
        invoke("git_files", { path }),
        invoke("git_branches", { path }),
        invoke("git_stash_list", { path }),
      ]);
      // Refresh the open diff (selection may have changed state).
      if (sel) {
        const still = files.find((f) => f.file === sel.file);
        if (still) { sel = still; loadDiff(); } else { sel = null; diff = ""; }
      }
    } catch (e) {
      err = String(e);
    }
  }

  $effect(() => {
    if (open) {
      err = "";
      message = "";
      sel = null;
      diff = "";
      reload();
    }
  });

  async function loadDiff() {
    if (!sel) { diff = ""; return; }
    diff = await invoke("git_diff", {
      path, file: sel.file, staged: sel.staged && !sel.untracked, untracked: sel.untracked,
    });
  }

  function select(f) {
    sel = f;
    loadDiff();
  }

  async function act(promise) {
    busy = true;
    err = "";
    try { await promise; await reload(); onchanged?.(); }
    catch (e) { err = String(e); }
    busy = false;
  }

  const stage = (f) => act(invoke("git_stage", { path, file: f.file }));
  const unstage = (f) => act(invoke("git_unstage", { path, file: f.file }));
  const discard = (f) => {
    if (!confirm(`Discard changes to ${f.file}? This cannot be undone.`)) return;
    act(invoke("git_discard", { path, file: f.file }));
  };
  const stageAll = () => act(invoke("git_stage_all", { path }));
  const unstageAll = () => act(invoke("git_unstage_all", { path }));

  function checkout(e) {
    const b = e.currentTarget.value;
    if (b && b !== branches.current) act(invoke("git_checkout", { path, branch: b }));
  }
  function createBranch() {
    const n = newBranch.trim();
    if (!n) return;
    newBranch = "";
    showNewBranch = false;
    act(invoke("git_create_branch", { path, name: n }));
  }
  const stashAll = () => act(invoke("git_stash_push", { path, message: "" }));
  const stashPop = (i) => act(invoke("git_stash_pop", { path, index: i }));
  const stashDrop = (i) => act(invoke("git_stash_drop", { path, index: i }));

  async function commit(push) {
    if (!message.trim() || staged.length === 0) return;
    busy = true;
    err = "";
    try {
      await invoke("git_commit_index", { path, message, push });
      message = "";
      await reload();
      onchanged?.();
    } catch (e) {
      err = String(e);
    }
    busy = false;
  }

  function badge(f) {
    if (f.untracked) return "U";
    const s = (f.staged ? f.x : f.y).trim();
    return s || "M";
  }
  function diffClass(line) {
    if (line.startsWith("@@")) return "hunk";
    if (line.startsWith("+++") || line.startsWith("---") || line.startsWith("diff ") || line.startsWith("index ")) return "meta";
    if (line.startsWith("+")) return "add";
    if (line.startsWith("-")) return "del";
    return "";
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="panel" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="top">
        <span class="title">Git — {projectName}</span>
        <div class="branchbox">
          <select value={branches.current ?? ""} onchange={checkout} disabled={busy} title="Switch branch">
            {#if !branches.current}<option value="">(detached)</option>{/if}
            {#each branches.all as b (b)}<option value={b}>{b}</option>{/each}
          </select>
          {#if showNewBranch}
            <input class="nb" bind:value={newBranch} placeholder="new-branch" onkeydown={(e) => { if (e.key === "Enter") createBranch(); if (e.key === "Escape") { showNewBranch = false; newBranch = ""; } }} />
          {:else}
            <button class="mini" title="New branch" onclick={() => (showNewBranch = true)}>＋ branch</button>
          {/if}
          {#if !branches.current}
            <span
              class="detached"
              title="Detached HEAD: you're on a specific commit, not a branch (common with pinned Docker/OrbStack checkouts). Looking around is safe, but commits made here aren't on any branch and can be lost. Pick a branch above, or ＋ branch, before committing."
            >⚠ detached</span>
          {/if}
        </div>
        <button class="mini" onclick={stashAll} disabled={busy || files.length === 0} title="Stash all changes">Stash</button>
        <button class="mini" onclick={reload} disabled={busy} title="Refresh">⟳</button>
        <button class="x" onclick={onclose} title="Close">✕</button>
      </div>

      {#if err}<div class="err">{err}</div>{/if}

      <div class="body">
        <div class="side">
          <div class="grouphead">
            <span>Staged ({staged.length})</span>
            {#if staged.length}<button class="link" onclick={unstageAll} disabled={busy}>Unstage all</button>{/if}
          </div>
          {#each staged as f (f.file)}
            <div class="frow" class:on={sel?.file === f.file && sel?.staged}>
              <span class="st st-{badge(f)}">{badge(f)}</span>
              <button class="fname" onclick={() => select({ ...f, staged: true })} title={f.file}>{f.file}</button>
              <button class="op" title="Unstage" onclick={() => unstage(f)}>−</button>
            </div>
          {/each}

          <div class="grouphead">
            <span>Changes ({unstaged.length})</span>
            {#if unstaged.length}<button class="link" onclick={stageAll} disabled={busy}>Stage all</button>{/if}
          </div>
          {#each unstaged as f (f.file)}
            <div class="frow" class:on={sel?.file === f.file && !sel?.staged}>
              <span class="st st-{badge(f)}">{badge(f)}</span>
              <button class="fname" onclick={() => select({ ...f, staged: false })} title={f.file}>{f.file}</button>
              {#if !f.untracked}<button class="op" title="Discard" onclick={() => discard(f)}>⟲</button>{/if}
              <button class="op" title="Stage" onclick={() => stage(f)}>＋</button>
            </div>
          {/each}

          {#if files.length === 0}<div class="clean">✓ Working tree clean</div>{/if}

          {#if stashes.length}
            <div class="grouphead"><span>Stashes ({stashes.length})</span></div>
            {#each stashes as s (s.index)}
              <div class="frow stash">
                <button class="fname" title={s.text}>{s.text}</button>
                <button class="op" title="Pop" onclick={() => stashPop(s.index)}>▲</button>
                <button class="op" title="Drop" onclick={() => stashDrop(s.index)}>🗑</button>
              </div>
            {/each}
          {/if}
        </div>

        <div class="diff">
          {#if !sel}
            <div class="diff-empty">Select a file to see its diff</div>
          {:else}
            <pre>{#each diff.split("\n") as line}<span class="dl {diffClass(line)}">{line || " "}
</span>{/each}</pre>
          {/if}
        </div>
      </div>

      <div class="commit">
        <textarea bind:value={message} placeholder={staged.length ? "Commit message…" : "Stage files to commit"} rows="2" disabled={busy}></textarea>
        <div class="cbtns">
          <button class="primary" onclick={() => commit(false)} disabled={busy || !message.trim() || staged.length === 0}>Commit ({staged.length})</button>
          <button class="primary push" onclick={() => commit(true)} disabled={busy || !message.trim() || staged.length === 0}>Commit & Push</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .panel { background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; width: 1000px; max-width: 94vw; height: 80vh; max-height: 800px; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
  .top { display: flex; align-items: center; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .title { font-weight: 600; font-size: 13px; }
  .branchbox { display: flex; align-items: center; gap: 6px; margin-left: 8px; }
  .detached { font-size: 11px; font-weight: 600; color: #e0af68; background: color-mix(in srgb, #e0af68 16%, transparent); border: 1px solid color-mix(in srgb, #e0af68 45%, transparent); border-radius: 6px; padding: 2px 7px; cursor: help; white-space: nowrap; }
  select, .nb { background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 12px; padding: 4px 8px; outline: none; }
  .nb:focus { border-color: var(--accent); }
  .mini { background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 11px; padding: 4px 9px; cursor: pointer; }
  .mini:hover:not(:disabled) { border-color: var(--accent); }
  .x { margin-left: auto; background: transparent; border: 1px solid var(--border); border-radius: 7px; color: var(--text-dim); font-size: 11px; padding: 4px 8px; cursor: pointer; }
  .err { background: rgba(192,57,43,0.15); color: #e06c5a; font-size: 12px; padding: 6px 12px; white-space: pre-wrap; }
  .body { flex: 1; display: flex; min-height: 0; }
  .side { width: 340px; min-width: 340px; border-right: 1px solid var(--border); overflow: auto; padding: 4px 0; }
  .grouphead { display: flex; align-items: center; justify-content: space-between; padding: 6px 12px 3px; font-size: 11px; font-weight: 600; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; }
  .link { background: transparent; border: none; color: var(--accent); font-size: 11px; cursor: pointer; }
  .frow { display: flex; align-items: center; gap: 6px; padding: 2px 10px; }
  .frow.on { background: var(--accent-2); }
  .frow:hover { background: var(--bg-3); }
  .st { width: 16px; text-align: center; font-size: 10px; font-family: var(--font-mono); border-radius: 3px; flex: none; }
  .st-M { color: #d6a23a; }
  .st-A { color: #4caf7d; }
  .st-D { color: #e06c5a; }
  .st-U { color: var(--accent); }
  .fname { flex: 1; text-align: left; background: transparent; border: none; color: var(--text); font-family: var(--font-mono); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; padding: 2px 0; }
  .op { background: transparent; border: none; color: var(--text-dim); font-size: 12px; cursor: pointer; padding: 2px 5px; border-radius: 5px; flex: none; }
  .op:hover { color: var(--text); background: var(--bg-4, var(--bg-2)); }
  .stash .fname { color: var(--text-dim); }
  .clean { padding: 16px 12px; color: var(--green); font-size: 12px; }
  .diff { flex: 1; overflow: auto; background: var(--bg); }
  .diff-empty { color: var(--text-dim); font-size: 12px; padding: 16px; }
  pre { margin: 0; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.5; }
  .dl { display: block; padding: 0 12px; white-space: pre-wrap; word-break: break-all; }
  .dl.add { background: rgba(76,175,125,0.14); color: #9fe0bd; }
  .dl.del { background: rgba(224,108,90,0.14); color: #f0b3a8; }
  .dl.hunk { color: var(--accent); }
  .dl.meta { color: var(--text-dim); }
  .commit { border-top: 1px solid var(--border); padding: 10px 12px; display: flex; gap: 10px; align-items: flex-end; }
  textarea { flex: 1; resize: none; background: var(--bg-3); border: 1px solid var(--border); border-radius: 8px; color: var(--text); font-family: inherit; font-size: 13px; padding: 8px 10px; outline: none; }
  textarea:focus { border-color: var(--accent); }
  .cbtns { display: flex; gap: 8px; }
  .primary { background: var(--accent); border: none; border-radius: 8px; color: #fff; font-size: 12px; padding: 8px 14px; cursor: pointer; white-space: nowrap; }
  .primary.push { background: var(--accent-2); color: var(--text); }
  .primary:disabled { opacity: 0.45; cursor: default; }
</style>
