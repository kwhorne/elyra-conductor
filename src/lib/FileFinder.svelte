<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, root = "", onopen, onclose } = $props();

  let mode = $state("files"); // "files" | "text"
  let query = $state("");
  let selected = $state(0);
  let inputEl;

  let allFiles = $state([]); // relative paths for the current root
  let filesRoot = null; // which root allFiles was loaded for
  let textHits = $state([]); // { path, line, text }
  let searching = $state(false);
  let searchTimer = null;

  async function ensureFiles() {
    if (filesRoot === root) return;
    filesRoot = root;
    allFiles = [];
    try {
      allFiles = await invoke("list_files", { root });
    } catch {
      allFiles = [];
    }
  }

  // Reset and load when opened.
  $effect(() => {
    if (open) {
      query = "";
      selected = 0;
      textHits = [];
      ensureFiles();
      queueMicrotask(() => inputEl?.focus());
    }
  });

  // ----- fuzzy file matching (subsequence + simple scoring) -----
  function score(path, q) {
    const p = path.toLowerCase();
    const base = p.split("/").pop();
    let qi = 0;
    for (let i = 0; i < p.length && qi < q.length; i++) {
      if (p[i] === q[qi]) qi++;
    }
    if (qi < q.length) return -1; // not a subsequence
    let s = 100 - path.length * 0.1; // prefer shorter paths
    if (base.includes(q)) s += 50; // bonus for basename hit
    if (p.includes(q)) s += 25; // bonus for contiguous hit
    return s;
  }

  let fileResults = $derived.by(() => {
    if (mode !== "files") return [];
    const q = query.trim().toLowerCase();
    if (!q) return allFiles.slice(0, 200);
    return allFiles
      .map((p) => ({ p, s: score(p, q) }))
      .filter((x) => x.s >= 0)
      .sort((a, b) => b.s - a.s)
      .slice(0, 200)
      .map((x) => x.p);
  });

  // ----- content search (debounced) -----
  $effect(() => {
    if (mode !== "text") return;
    const q = query.trim();
    clearTimeout(searchTimer);
    if (!q) {
      textHits = [];
      return;
    }
    searching = true;
    const r = root;
    searchTimer = setTimeout(async () => {
      try {
        textHits = await invoke("search_content", { root: r, query: q });
      } catch {
        textHits = [];
      }
      searching = false;
    }, 180);
  });

  let count = $derived(mode === "files" ? fileResults.length : textHits.length);

  // Keep selection in range.
  $effect(() => {
    if (selected >= count) selected = Math.max(0, count - 1);
  });

  function basename(p) {
    return p ? p.split("/").pop() : "";
  }
  function dirname(p) {
    const i = p.lastIndexOf("/");
    return i > 0 ? p.slice(0, i) : "";
  }

  function choose(i) {
    if (mode === "files") {
      const rel = fileResults[i];
      if (rel == null) return;
      onopen?.(root.replace(/\/$/, "") + "/" + rel, null);
    } else {
      const hit = textHits[i];
      if (!hit) return;
      onopen?.(hit.path, hit.line);
    }
    onclose?.();
  }

  function setMode(m) {
    mode = m;
    selected = 0;
  }

  function onKeydown(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, count - 1);
      scrollSelected();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
      scrollSelected();
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(selected);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onclose?.();
    } else if (e.key === "Tab") {
      e.preventDefault();
      setMode(mode === "files" ? "text" : "files");
    }
  }

  let listEl;
  function scrollSelected() {
    queueMicrotask(() => {
      listEl?.querySelector(".row.sel")?.scrollIntoView({ block: "nearest" });
    });
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="finder" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="tabs">
        <button class="tab" class:on={mode === "files"} onclick={() => setMode("files")}>Files</button>
        <button class="tab" class:on={mode === "text"} onclick={() => setMode("text")}>Text</button>
        <span class="hint">⇥ switch · ↵ open · esc close</span>
      </div>
      <input
        bind:this={inputEl}
        class="search"
        bind:value={query}
        onkeydown={onKeydown}
        placeholder={mode === "files" ? "Find file by name…" : "Search file contents…"}
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
      />
      <div class="list" bind:this={listEl}>
        {#if mode === "files"}
          {#each fileResults as p, i (p)}
            <button class="row" class:sel={i === selected} onclick={() => choose(i)} onmouseenter={() => (selected = i)}>
              <span class="name">{basename(p)}</span>
              <span class="dir">{dirname(p)}</span>
            </button>
          {/each}
          {#if fileResults.length === 0}<div class="empty">No files</div>{/if}
        {:else}
          {#each textHits as h, i (h.path + ":" + h.line + ":" + i)}
            <button class="row text" class:sel={i === selected} onclick={() => choose(i)} onmouseenter={() => (selected = i)}>
              <div class="tline"><span class="name">{basename(h.path)}</span><span class="ln">:{h.line}</span></div>
              <div class="snippet">{h.text}</div>
            </button>
          {/each}
          {#if searching && textHits.length === 0}<div class="empty">Searching…</div>
          {:else if !searching && query.trim() && textHits.length === 0}<div class="empty">No matches</div>{/if}
        {/if}
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
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 1000;
  }
  .finder {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 640px;
    max-width: 92vw;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 0;
  }
  .tab {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 7px;
    cursor: pointer;
  }
  .tab.on {
    background: var(--accent-2);
    color: var(--text);
  }
  .hint {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-dim);
  }
  .search {
    margin: 8px 10px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-size: 14px;
    padding: 9px 11px;
    outline: none;
  }
  .search:focus {
    border-color: var(--accent);
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text);
    padding: 6px 9px;
    border-radius: 7px;
    cursor: pointer;
  }
  .row.text {
    flex-direction: column;
    align-items: stretch;
    gap: 2px;
  }
  .row.sel {
    background: var(--accent-2);
  }
  .name {
    font-size: 13px;
  }
  .dir {
    font-size: 11px;
    color: var(--text-dim);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tline {
    display: flex;
    align-items: baseline;
    gap: 2px;
  }
  .ln {
    font-size: 11px;
    color: var(--text-dim);
  }
  .snippet {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 12px;
    text-align: center;
  }
</style>
