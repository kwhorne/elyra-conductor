<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Sidebar from "./lib/Sidebar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import Editor from "./lib/Editor.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";
  import FileExplorer from "./lib/FileExplorer.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import RunModal from "./lib/RunModal.svelte";
  import CommitDialog from "./lib/CommitDialog.svelte";
  import ShortcutsModal from "./lib/ShortcutsModal.svelte";
  import { geometry, splitLeaf, removeLeaf, setRatio, firstLeaf, allLeaves } from "./lib/layout.js";

  let root = $state("");
  let projects = $state([]);
  let editors = $state([]);
  let activeProject = $state(null);

  let tabs = $state([]); // { id, title, projectPath, root }
  let activeTabId = $state(null);
  let activeTermId = $state(null);

  let showEditor = $state(false);
  let editorPath = $state(null);
  let paletteOpen = $state(false);
  let helpOpen = $state(false);
  let showFiles = $state(true);
  let showHidden = $state(false);
  let theme = $state("dark");
  let loaded = $state(false);

  // Apply theme to the document root (drives all CSS variables).
  $effect(() => {
    document.documentElement.dataset.theme = theme;
  });

  const STORAGE_KEY = "conductor:state";

  let fileRoot = $derived(activeProject?.path ?? root);

  function openFile(path) {
    editorPath = path;
    showEditor = true;
  }

  // ---------- notification rings ----------
  function markActivity(tabId) {
    if (tabId !== activeTabId && !activity[tabId]) {
      activity = { ...activity, [tabId]: true };
    }
  }

  // Clear the indicator when its tab becomes active.
  $effect(() => {
    const id = activeTabId;
    if (id && activity[id]) {
      const next = { ...activity };
      delete next[id];
      activity = next;
    }
  });

  // ---------- file context menu + run modal ----------
  let terminalName = $state("Terminal");
  let ctx = $state({ open: false, x: 0, y: 0, entry: null });
  let runModal = $state({ open: false, cwd: "", command: "", title: "" });
  let commit = $state({ open: false, path: "", name: "" });

  function openCommit() {
    const p = activeProject ?? projects.find((x) => x.path === activeTab?.projectPath);
    if (!p) return;
    commit = { open: true, path: p.path, name: p.name };
  }

  function dirOf(path) {
    const i = path.lastIndexOf("/");
    return i > 0 ? path.slice(0, i) : "/";
  }
  function baseOf(path) {
    return path.split("/").pop();
  }

  function onFileContext(entry, x, y) {
    ctx = { open: true, x, y, entry };
  }

  function runInModal(entry) {
    runModal = {
      open: true,
      cwd: dirOf(entry.path),
      command: `./${baseOf(entry.path)}`,
      title: baseOf(entry.path),
    };
  }

  async function runExternal(entry) {
    try {
      await invoke("run_in_external_terminal", { path: entry.path });
    } catch (e) {
      alert(`Could not run in ${terminalName}: ${e}`);
    }
  }

  let ctxItems = $derived.by(() => {
    const e = ctx.entry;
    if (!e) return [];
    if (e.is_dir) {
      return [
        { label: "Open new terminal here", icon: "\u{1F5A5}", action: () => newTab(e.path, baseOf(e.path)) },
      ];
    }
    return [
      { label: "Open in editor", icon: "\u270E", action: () => openFile(e.path) },
      { separator: true },
      { label: `Run ./${baseOf(e.path)} (modal)`, icon: "\u25B6", action: () => runInModal(e) },
      { label: `Run in ${terminalName}`, icon: "\u{1F680}", action: () => runExternal(e) },
    ];
  });

  let drag = null; // { ...divider, container } during resize
  let activity = $state({}); // tabId -> true when a background tab produced output
  let titles = $state({}); // termId -> foreground process name (or null)

  // Title shown on a tab: the foreground process of any of its panes, else the
  // project/shell name.
  function tabTitle(t) {
    for (const l of allLeaves(t.root)) {
      if (titles[l.termId]) return titles[l.termId];
    }
    return t.title;
  }

  async function pollTitles() {
    const ids = [];
    for (const t of tabs) for (const l of allLeaves(t.root)) ids.push(l.termId);
    if (ids.length === 0) return;
    const results = await Promise.all(
      ids.map(async (id) => [id, await invoke("pty_title", { id }).catch(() => null)])
    );
    const next = {};
    for (const [id, name] of results) next[id] = name;
    titles = next;
  }

  let idSeq = 0;
  const nextId = (p) => `${p}-${++idSeq}`;

  let activeTab = $derived(tabs.find((t) => t.id === activeTabId) ?? null);
  let geo = $derived(activeTab ? geometry(activeTab.root) : { leaves: [], dividers: [] });

  // ---------- data ----------
  async function loadProjects() {
    if (!root) return;
    try {
      projects = await invoke("list_projects", { root });
      loadGitStatus();
    } catch (e) {
      console.error(e);
    }
  }

  let lastGitRefresh = 0;
  function refreshGitStatusThrottled() {
    const now = Date.now();
    if (now - lastGitRefresh < 3000) return;
    lastGitRefresh = now;
    loadGitStatus();
  }

  // Enrich each git project with dirty/ahead/behind, in parallel, after the
  // initial (fast) listing is shown.
  async function loadGitStatus() {
    const current = projects;
    await Promise.all(
      current.map(async (p) => {
        if (!p.is_git) return;
        try {
          const s = await invoke("git_status", { path: p.path });
          p.branch = s.branch ?? p.branch;
          p.dirty = s.dirty;
          p.ahead = s.ahead;
          p.behind = s.behind;
        } catch {}
      })
    );
  }

  function makeLeaf(cwd, title) {
    return { kind: "leaf", id: nextId("n"), termId: nextId("term"), cwd, title: title ?? "shell" };
  }

  // ---------- tabs ----------
  function newTab(cwd, title) {
    const leaf = makeLeaf(cwd, title);
    const tab = { id: nextId("tab"), title: title ?? "shell", projectPath: cwd, root: leaf };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = leaf.termId;
  }

  let dragFrom = null;
  let dragOver = $state(null);
  function onTabDragStart(e, i) {
    dragFrom = i;
    e.dataTransfer.effectAllowed = "move";
  }
  function onTabDragOver(e, i) {
    e.preventDefault();
    dragOver = i;
  }
  function onTabDrop(e, i) {
    e.preventDefault();
    if (dragFrom != null && dragFrom !== i) {
      const arr = [...tabs];
      const [moved] = arr.splice(dragFrom, 1);
      arr.splice(i, 0, moved);
      tabs = arr;
    }
    dragFrom = null;
    dragOver = null;
  }

  function closeTab(id) {
    tabs = tabs.filter((t) => t.id !== id);
    if (activeTabId === id) {
      const next = tabs.at(-1) ?? null;
      activeTabId = next?.id ?? null;
      activeTermId = next ? firstLeaf(next.root).termId : null;
    }
  }

  function selectProject(p) {
    activeProject = p;
    const existing = tabs.find((t) => t.projectPath === p.path);
    if (existing) {
      activeTabId = existing.id;
      activeTermId = firstLeaf(existing.root).termId;
    } else {
      newTab(p.path, p.name);
    }
  }

  // ---------- panes ----------
  function cwdOf(termId) {
    return geo.leaves.find((l) => l.termId === termId)?.cwd ?? activeProject?.path ?? root;
  }

  function splitPane(termId, dir) {
    const tab = activeTab;
    if (!tab) return;
    const leaf = makeLeaf(cwdOf(termId));
    tab.root = splitLeaf(tab.root, termId, dir, leaf, nextId("s"));
    activeTermId = leaf.termId;
  }

  function closePane(termId) {
    const tab = activeTab;
    if (!tab) return;
    const newRoot = removeLeaf(tab.root, termId);
    if (!newRoot) {
      closeTab(tab.id);
      return;
    }
    tab.root = newRoot;
    if (activeTermId === termId) activeTermId = firstLeaf(newRoot).termId;
  }

  // ---------- divider drag ----------
  function startDrag(e, d) {
    e.preventDefault();
    drag = { ...d, container: e.currentTarget.parentElement };
    window.addEventListener("pointermove", onDrag);
    window.addEventListener("pointerup", endDrag);
  }
  function onDrag(e) {
    if (!drag || !drag.container || !activeTab) return;
    const box = drag.container.getBoundingClientRect();
    let ratio;
    if (drag.dir === "row") {
      const regionLeft = box.left + (drag.rect.x / 100) * box.width;
      const regionW = (drag.rect.w / 100) * box.width;
      ratio = (e.clientX - regionLeft) / regionW;
    } else {
      const regionTop = box.top + (drag.rect.y / 100) * box.height;
      const regionH = (drag.rect.h / 100) * box.height;
      ratio = (e.clientY - regionTop) / regionH;
    }
    ratio = Math.min(0.9, Math.max(0.1, ratio));
    activeTab.root = setRatio(activeTab.root, drag.id, ratio);
  }
  function endDrag() {
    drag = null;
    window.removeEventListener("pointermove", onDrag);
    window.removeEventListener("pointerup", endDrag);
  }

  // ---------- editor / dialogs ----------
  async function openInEditor(ed, p) {
    try {
      await invoke("open_in_editor", { editor: ed, path: p.path });
    } catch (e) {
      alert(`Could not open ${ed}: ${e}`);
    }
  }

  async function quickEdit() {
    const base = activeProject?.path ?? root;
    const sel = await openDialog({ multiple: false, directory: false, defaultPath: base || undefined, title: "Open file to edit" });
    if (typeof sel === "string") {
      editorPath = sel;
      showEditor = true;
    }
  }

  async function changeRoot() {
    const sel = await openDialog({ multiple: false, directory: true, defaultPath: root || undefined, title: "Choose projects folder" });
    if (typeof sel === "string") {
      root = sel;
      await loadProjects();
    }
  }

  // ---------- command palette ----------
  let commands = $derived.by(() => {
    const list = [];
    for (const p of projects)
      list.push({ id: `proj:${p.path}`, title: p.name, hint: p.path, group: "project", icon: "\u{1F4C1}", action: () => selectProject(p) });
    for (const t of tabs)
      list.push({ id: `tab:${t.id}`, title: t.title, hint: t.projectPath, group: "tab", icon: "\u{1F5C2}", action: () => { activeTabId = t.id; activeTermId = firstLeaf(t.root).termId; } });
    list.push({ id: "act:new-tab", title: "New terminal tab", hint: "", group: "action", icon: "\u002B", action: () => newTab(activeProject?.path ?? root, activeProject?.name) });
    list.push({ id: "act:split-right", title: "Split right", hint: "\u2318D", group: "action", icon: "\u25A5", action: () => activeTermId && splitPane(activeTermId, "row") });
    list.push({ id: "act:split-down", title: "Split down", hint: "\u21E7\u2318D", group: "action", icon: "\u25A4", action: () => activeTermId && splitPane(activeTermId, "col") });
    list.push({ id: "act:close-pane", title: "Close pane", hint: "\u2318W", group: "action", icon: "\u00D7", action: () => activeTermId && closePane(activeTermId) });
    list.push({ id: "act:toggle-editor", title: showEditor ? "Hide editor" : "Show editor", group: "action", icon: "\u270E", action: () => (showEditor = !showEditor) });
    list.push({ id: "act:toggle-files", title: showFiles ? "Hide file sidebar" : "Show file sidebar", hint: "\u2318B", group: "action", icon: "\u{1F5C2}", action: () => (showFiles = !showFiles) });
    list.push({ id: "act:toggle-hidden", title: showHidden ? "Hide node_modules/.git in tree" : "Show all files in tree", group: "action", icon: "\u{1F441}", action: () => (showHidden = !showHidden) });
    list.push({ id: "act:toggle-theme", title: theme === "dark" ? "Switch to light theme" : "Switch to dark theme", group: "action", icon: theme === "dark" ? "\u2600" : "\u263D", action: () => (theme = theme === "dark" ? "light" : "dark") });
    list.push({ id: "act:quick-edit", title: "Quick edit file\u2026", group: "action", icon: "\u270E", action: quickEdit });
    list.push({ id: "act:change-root", title: "Change projects folder\u2026", group: "action", icon: "\u{1F4C2}", action: changeRoot });
    if (activeProject?.is_git)
      list.push({ id: "act:commit", title: `Git: commit ${activeProject.name}\u2026`, group: "action", icon: "\u2387", action: openCommit });
    list.push({ id: "act:help", title: "Keyboard shortcuts", hint: "\u2318/", group: "action", icon: "?", action: () => (helpOpen = true) });
    list.push({ id: "act:reset-layout", title: "Reset saved layout", group: "action", icon: "\u21BA", action: () => { try { localStorage.removeItem(STORAGE_KEY); } catch {} location.reload(); } });
    if (activeProject)
      for (const ed of editors)
        list.push({ id: `act:open-${ed}`, title: `Open ${activeProject.name} in ${ed}`, group: "action", icon: "\u{1F680}", action: () => openInEditor(ed, activeProject) });
    return list;
  });

  // Is keyboard focus currently inside the Monaco editor?
  function inEditorContext() {
    const el = document.activeElement;
    return !!(el && el.closest && el.closest(".editor-area"));
  }

  function onGlobalKey(e) {
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;
    const k = e.key.toLowerCase();

    // While editing, let Monaco own its shortcuts (multi-cursor ⌘D, find ⌘F,
    // ⌘K chords, etc). We only add ⌘W to close the editor.
    if (inEditorContext()) {
      if (k === "w") {
        e.preventDefault();
        showEditor = false;
      }
      return;
    }

    if (k === "k") {
      e.preventDefault();
      paletteOpen = !paletteOpen;
    } else if (k === "d") {
      e.preventDefault();
      if (activeTermId) splitPane(activeTermId, e.shiftKey ? "col" : "row");
    } else if (k === "w") {
      e.preventDefault();
      if (activeTermId) closePane(activeTermId);
    } else if (k === "b") {
      e.preventDefault();
      showFiles = !showFiles;
    } else if (k === "/") {
      e.preventDefault();
      helpOpen = !helpOpen;
    }
  }

  // ---------- persistence ----------
  function stripTree(n) {
    if (n.kind === "leaf") return { kind: "leaf", cwd: n.cwd, title: n.title };
    return { kind: "split", dir: n.dir, ratio: n.ratio, a: stripTree(n.a), b: stripTree(n.b) };
  }
  function reviveTree(n) {
    if (n.kind === "leaf") return makeLeaf(n.cwd, n.title);
    return { kind: "split", id: nextId("s"), dir: n.dir, ratio: n.ratio ?? 0.5, a: reviveTree(n.a), b: reviveTree(n.b) };
  }

  function serialize() {
    return {
      root,
      activeProjectPath: activeProject?.path ?? null,
      showFiles,
      showHidden,
      theme,
      showEditor,
      editorPath,
      activeTabIndex: tabs.findIndex((t) => t.id === activeTabId),
      tabs: tabs.map((t) => ({ title: t.title, projectPath: t.projectPath, root: stripTree(t.root) })),
    };
  }

  let saveTimer = null;
  $effect(() => {
    const isLoaded = loaded;
    const snapshot = JSON.stringify(serialize()); // touch reactive deps
    if (!isLoaded) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, snapshot);
      } catch {}
    }, 250);
  });

  function restore() {
    let saved;
    try {
      saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    } catch {
      saved = null;
    }
    if (!saved) return false;

    if (saved.activeProjectPath)
      activeProject = projects.find((p) => p.path === saved.activeProjectPath) ?? null;
    showFiles = saved.showFiles ?? true;
    showHidden = saved.showHidden ?? false;
    theme = saved.theme ?? "dark";
    showEditor = saved.showEditor ?? false;
    editorPath = saved.editorPath ?? null;

    if (Array.isArray(saved.tabs) && saved.tabs.length) {
      tabs = saved.tabs.map((t) => ({
        id: nextId("tab"),
        title: t.title,
        projectPath: t.projectPath,
        root: reviveTree(t.root),
      }));
      const at = tabs[saved.activeTabIndex] ?? tabs[0];
      activeTabId = at.id;
      activeTermId = firstLeaf(at.root).termId;
    }
    return true;
  }

  function onWindowFocus() {
    refreshGitStatusThrottled();
  }

  let titleTimer = null;

  onMount(async () => {
    window.addEventListener("keydown", onGlobalKey);
    window.addEventListener("focus", onWindowFocus);
    titleTimer = setInterval(pollTitles, 1800);
    editors = await invoke("detect_editors");
    terminalName = await invoke("detect_terminal");

    let saved;
    try {
      saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    } catch {
      saved = null;
    }
    if (saved?.root) {
      root = saved.root;
    } else {
      try {
        const home = await invoke("home_dir");
        root = `${home}/Code`;
      } catch {
        root = "";
      }
    }

    await loadProjects();
    restore();
    loaded = true;
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onGlobalKey);
    window.removeEventListener("focus", onWindowFocus);
    clearInterval(titleTimer);
  });
</script>

<div class="app">
  <Sidebar
    {projects}
    {editors}
    {root}
    activePath={activeProject?.path}
    onselect={selectProject}
    onopen={openInEditor}
    onroot={changeRoot}
    onrefresh={loadProjects}
  />

  <div class="main">
    <div class="topbar">
      <div class="tabs">
        {#each tabs as t, i (t.id)}
          <div
            class="tab"
            class:active={t.id === activeTabId}
            class:ring={activity[t.id]}
            class:dragover={dragOver === i}
            draggable="true"
            ondragstart={(e) => onTabDragStart(e, i)}
            ondragover={(e) => onTabDragOver(e, i)}
            ondrop={(e) => onTabDrop(e, i)}
            ondragend={() => (dragOver = null)}
          >
            {#if activity[t.id]}<span class="ring-dot" title="New activity"></span>{/if}
            <button class="tab-label" onclick={() => { activeTabId = t.id; activeTermId = firstLeaf(t.root).termId; }}>{tabTitle(t)}</button>
            <button class="tab-x" onclick={() => closeTab(t.id)}>×</button>
          </div>
        {/each}
        <button class="new-tab" title="New tab" onclick={() => newTab(activeProject?.path ?? root, activeProject?.name)}>＋</button>
      </div>
      <div class="top-actions">
        <button class="kbd-btn" onclick={() => (paletteOpen = true)}>Search <kbd>⌘K</kbd></button>
        <button onclick={quickEdit}>Quick edit</button>
        <button class:on={showEditor} onclick={() => (showEditor = !showEditor)}>{showEditor ? "Hide editor" : "Show editor"}</button>
        <button class:on={showFiles} title="Toggle file sidebar (⌘B)" onclick={() => (showFiles = !showFiles)}>Files</button>
        <button title="Toggle theme" onclick={() => (theme = theme === "dark" ? "light" : "dark")}>{theme === "dark" ? "☀" : "☽"}</button>
        <button title="Keyboard shortcuts (⌘/)" onclick={() => (helpOpen = true)}>?</button>
        {#if activeProject?.is_git}
          <button title="Commit changes" onclick={openCommit}>⎇ Commit</button>
        {/if}
      </div>
    </div>

    <div class="panes">
      <div class="term-stack">
        {#if tabs.length === 0}
          <div class="placeholder">Select a project or press ＋ to open a terminal. <kbd>⌘D</kbd> to split.</div>
        {/if}

        {#each tabs as tab (tab.id)}
          {@const g = geometry(tab.root)}
          <div class="term-area" style:display={tab.id === activeTabId ? "block" : "none"}>
            {#each g.leaves as leaf (leaf.termId)}
              <div
                class="pane"
                class:active={leaf.termId === activeTermId && tab.id === activeTabId}
                style:left="{leaf.rect.x}%"
                style:top="{leaf.rect.y}%"
                style:width="{leaf.rect.w}%"
                style:height="{leaf.rect.h}%"
                role="presentation"
                onpointerdown={() => (activeTermId = leaf.termId)}
              >
                {#if titles[leaf.termId]}
                  <span class="pane-proc">{titles[leaf.termId]}</span>
                {/if}
                <div class="pane-tools">
                  <button title="Split right (⌘D)" onclick={() => splitPane(leaf.termId, "row")}>▥</button>
                  <button title="Split down (⇧⌘D)" onclick={() => splitPane(leaf.termId, "col")}>▤</button>
                  <button title="Close pane (⌘W)" onclick={() => closePane(leaf.termId)}>×</button>
                </div>
                <Terminal id={leaf.termId} cwd={leaf.cwd} {theme} onactivity={() => markActivity(tab.id)} />
              </div>
            {/each}

            {#each g.dividers as d (d.id)}
              {#if d.dir === "row"}
                <div class="divider row" style:left="{d.pos}%" style:top="{d.rect.y}%" style:height="{d.rect.h}%" role="separator" aria-orientation="vertical" onpointerdown={(e) => startDrag(e, d)}></div>
              {:else}
                <div class="divider col" style:top="{d.pos}%" style:left="{d.rect.x}%" style:width="{d.rect.w}%" role="separator" aria-orientation="horizontal" onpointerdown={(e) => startDrag(e, d)}></div>
              {/if}
            {/each}
          </div>
        {/each}
      </div>

      {#if showEditor}
        <div class="editor-area"><Editor path={editorPath} {theme} onclose={() => (showEditor = false)} /></div>
      {/if}

      {#if showFiles}
        <FileExplorer
          root={fileRoot}
          onopen={openFile}
          oncontext={onFileContext}
          activePath={editorPath}
          showAll={showHidden}
          ontoggleall={() => (showHidden = !showHidden)}
        />
      {/if}
    </div>
  </div>

  <CommandPalette open={paletteOpen} {commands} onclose={() => (paletteOpen = false)} />

  <ContextMenu
    open={ctx.open}
    x={ctx.x}
    y={ctx.y}
    items={ctxItems}
    onclose={() => (ctx = { ...ctx, open: false })}
  />

  <RunModal
    open={runModal.open}
    cwd={runModal.cwd}
    command={runModal.command}
    title={runModal.title}
    onclose={() => (runModal = { ...runModal, open: false })}
  />

  <CommitDialog
    open={commit.open}
    path={commit.path}
    projectName={commit.name}
    onclose={() => (commit = { ...commit, open: false })}
    oncommitted={() => loadProjects()}
  />

  <ShortcutsModal open={helpOpen} onclose={() => (helpOpen = false)} />
</div>

<style>
  .app { display: flex; height: 100%; }
  .main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .topbar {
    display: flex; align-items: center; height: 38px;
    background: var(--bg-2); border-bottom: 1px solid var(--border);
    padding: 0 8px; gap: 8px;
  }
  .tabs { display: flex; align-items: center; gap: 4px; overflow-x: auto; flex: 1; }
  .tab { display: flex; align-items: center; background: var(--bg-3); border: 1px solid transparent; border-radius: 6px; padding: 0 2px 0 4px; }
  .tab.active { border-color: var(--accent); }
  .tab.ring { border-color: var(--green); }
  .tab.dragover { border-color: var(--accent); background: var(--accent-2); }
  .tab[draggable="true"] { cursor: grab; }
  .ring-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--green); margin-left: 6px; flex: none; animation: ring-pulse 1.2s ease-in-out infinite; }
  @keyframes ring-pulse { 0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(158, 206, 106, 0.6); } 50% { opacity: 0.5; box-shadow: 0 0 0 4px rgba(158, 206, 106, 0); } }
  .tab-label { background: transparent; border: none; color: var(--text); padding: 4px 6px; font-size: 12px; max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tab-x { background: transparent; border: none; color: var(--text-dim); padding: 0 4px; font-size: 14px; }
  .tab-x:hover { color: var(--text); }
  .new-tab { background: transparent; border: none; color: var(--text-dim); font-size: 16px; padding: 0 6px; }
  .top-actions { display: flex; gap: 6px; }
  .top-actions button { background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 10px; font-size: 12px; }
  .top-actions button.on { border-color: var(--accent); color: var(--accent); }
  .kbd-btn { display: flex; align-items: center; gap: 6px; }
  kbd { background: var(--bg); border: 1px solid var(--border); border-radius: 4px; padding: 1px 5px; font-size: 10px; font-family: var(--font-mono); color: var(--text-dim); }
  .panes { flex: 1; display: flex; min-height: 0; }
  .term-stack { flex: 1; position: relative; min-width: 0; background: #1a1b26; overflow: hidden; }
  .term-area { position: absolute; inset: 0; }
  .pane { position: absolute; overflow: hidden; box-shadow: inset 0 0 0 1px var(--border); }
  .pane.active { box-shadow: inset 0 0 0 1px var(--accent); }
  .pane-proc { position: absolute; top: 3px; left: 6px; z-index: 5; font-family: var(--font-mono); font-size: 10px; color: var(--accent); background: rgba(30, 31, 43, 0.85); border: 1px solid var(--border); border-radius: 4px; padding: 1px 6px; pointer-events: none; }
  .pane-tools { position: absolute; top: 3px; right: 6px; z-index: 5; display: flex; gap: 2px; opacity: 0; transition: opacity 0.12s; }
  .pane:hover .pane-tools { opacity: 1; }
  .pane-tools button { background: rgba(30, 31, 43, 0.9); border: 1px solid var(--border); color: var(--text-dim); border-radius: 4px; font-size: 11px; line-height: 1; padding: 2px 5px; }
  .pane-tools button:hover { color: var(--text); border-color: var(--accent); }
  .divider { position: absolute; z-index: 4; }
  .divider.row { width: 6px; transform: translateX(-3px); cursor: col-resize; }
  .divider.col { height: 6px; transform: translateY(-3px); cursor: row-resize; }
  .divider:hover { background: var(--accent); opacity: 0.4; }
  .editor-area { width: 50%; min-width: 320px; border-left: 1px solid var(--border); }
  .placeholder { color: var(--text-dim); padding: 24px; }
</style>
