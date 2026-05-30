<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Sidebar from "./lib/Sidebar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import AgentPanel from "./lib/AgentPanel.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";
  import FileExplorer from "./lib/FileExplorer.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import RunModal from "./lib/RunModal.svelte";
  import CommitDialog from "./lib/CommitDialog.svelte";
  import ShortcutsModal from "./lib/ShortcutsModal.svelte";
  import { check as checkUpdate } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { geometry, splitLeaf, removeLeaf, setRatio, firstLeaf, allLeaves } from "./lib/layout.js";

  let root = $state("");
  let projects = $state([]);
  let pinned = $state([]); // [{ path, name, is_git, branch, dirty, ahead, behind }]
  let editors = $state([]);
  let elyraVersion = $state(null);
  let activeProject = $state(null);

  let tabs = $state([]); // { id, title, projectPath, root }
  let activeTabId = $state(null);
  let activeTermId = $state(null);

  let showEditor = $state(false);
  let editorPath = $state(null);
  let paletteOpen = $state(false);
  let helpOpen = $state(false);

  // ---------- auto-update ----------
  let update = $state(null);
  let updateStatus = $state(""); // '' | 'downloading' | 'ready' | 'error'
  let updateProgress = $state(0);
  let updateError = $state("");
  let updateDismissed = $state(false);

  async function checkForUpdate(manual = false) {
    try {
      const u = await checkUpdate();
      if (u) {
        update = u;
        updateDismissed = false;
      } else if (manual) {
        alert("You're on the latest version.");
      }
    } catch (e) {
      if (manual) alert(`Update check failed: ${e}`);
      else console.warn("update check failed", e);
    }
  }

  async function installUpdate() {
    if (!update) return;
    updateStatus = "downloading";
    let total = 0;
    let got = 0;
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") total = ev.data?.contentLength ?? 0;
        else if (ev.event === "Progress") {
          got += ev.data.chunkLength;
          updateProgress = total ? Math.round((got / total) * 100) : 0;
        } else if (ev.event === "Finished") updateProgress = 100;
      });
      updateStatus = "ready";
      await relaunch();
    } catch (e) {
      updateStatus = "error";
      updateError = String(e);
    }
  }
  let showFiles = $state(true);
  let showHidden = $state(false);
  let theme = $state("dark");
  let loaded = $state(false);

  // Apply theme to the document root (drives all CSS variables).
  $effect(() => {
    document.documentElement.dataset.theme = theme;
  });

  // ---------- per-project tasks (npm/make/just/composer) ----------
  let projectTasks = $state([]); // [{ label, command, source }]
  $effect(() => {
    const p = activeProject?.path;
    if (!p) {
      projectTasks = [];
      return;
    }
    invoke("list_tasks", { path: p })
      .then((t) => (projectTasks = Array.isArray(t) ? t : []))
      .catch(() => (projectTasks = []));
  });
  function runTask(task) {
    const cwd = activeProject?.path ?? root;
    newTab(cwd, task.label, task.command);
  }

  // ---------- broadcast input (synchronize panes, tmux-style) ----------
  let broadcast = $state(false);
  function onPaneInput(srcId, data) {
    if (!broadcast) return;
    const tab = activeTab;
    if (!tab || tab.kind !== "term") return;
    // Mirror the keystrokes to every other pane in the active tab. The source
    // pane already wrote to its own pty; siblings echo back via pty://data.
    for (const l of allLeaves(tab.root)) {
      if (l.termId !== srcId) invoke("pty_write", { id: l.termId, data }).catch(() => {});
    }
  }

  const STORAGE_KEY = "conductor:state";
  const WORKSPACES_KEY = "conductor:workspaces";
  let workspaces = $state({}); // { name -> snapshot }
  let activeWorkspace = $state(null);

  let fileRoot = $derived(activeProject?.path ?? root);

  function openFile(path) {
    editorPath = path;
    showEditor = true;
  }

  // Monaco is heavy (~2.5MB). Load it on demand (and preload when idle) instead
  // of at startup, so the app opens fast.
  let EditorComp = $state(null);
  function loadEditor() {
    if (!EditorComp) import("./lib/Editor.svelte").then((m) => (EditorComp = m.default));
  }
  $effect(() => {
    if (showEditor) loadEditor();
  });

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
      const items = [
        { label: "Open new terminal here", icon: "\u{1F5A5}", action: () => newTab(e.path, baseOf(e.path)) },
      ];
      if (elyraVersion)
        items.push({ label: "New Elyra agent here", icon: "\u{1F916}", action: () => newElyraAgent(e.path, baseOf(e.path)) });
      return items;
    }
    const items = [
      { label: "Open in editor", icon: "\u270E", action: () => openFile(e.path) },
    ];
    if (elyraVersion)
      items.push({ label: "Ask Elyra about this file", icon: "\u{1F916}", action: () => askElyra(e) });
    items.push(
      { separator: true },
      { label: `Run ./${baseOf(e.path)} (modal)`, icon: "\u25B6", action: () => runInModal(e) },
      { label: `Run in ${terminalName}`, icon: "\u{1F680}", action: () => runExternal(e) }
    );
    return items;
  });

  let drag = null; // { ...divider, container } during resize
  let activity = $state({}); // tabId -> true when a background tab produced output
  let titles = $state({}); // termId -> foreground process name (or null)

  // Title shown on a tab: the foreground process of any of its panes, else the
  // project/shell name.
  function tabTitle(t) {
    if (t.kind !== "term") return t.title;
    for (const l of allLeaves(t.root)) {
      if (titles[l.termId]) return titles[l.termId];
    }
    return t.title;
  }

  async function pollTitles() {
    const ids = [];
    for (const t of tabs) {
      if (t.kind !== "term") continue;
      for (const l of allLeaves(t.root)) ids.push(l.termId);
    }
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
  let geo = $derived(activeTab?.kind === "term" ? geometry(activeTab.root) : { leaves: [], dividers: [] });

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

  function togglePin(project) {
    if (pinned.some((p) => p.path === project.path)) {
      pinned = pinned.filter((p) => p.path !== project.path);
    } else {
      pinned = [
        ...pinned,
        {
          path: project.path,
          name: project.name,
          is_git: project.is_git,
          branch: project.branch,
          dirty: project.dirty,
          ahead: project.ahead,
          behind: project.behind,
        },
      ];
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
    // Refresh both the scanned projects and the pinned list (pinned may point
    // outside the current root, so we resolve their status independently).
    const enrich = async (p, assumeGit) => {
      if (!assumeGit && !p.is_git) return;
      try {
        const s = await invoke("git_status", { path: p.path });
        p.is_git = p.is_git || !!s.branch;
        p.branch = s.branch ?? p.branch;
        p.dirty = s.dirty;
        p.ahead = s.ahead;
        p.behind = s.behind;
      } catch {}
    };
    await Promise.all([
      ...projects.map((p) => enrich(p, false)),
      ...pinned.map((p) => enrich(p, true)),
    ]);
  }

  function makeLeaf(cwd, title, runOnce = null, key = null) {
    // `key` is a stable per-pane id that survives serialize/restore, so the
    // saved scrollback can be matched back to the right pane after restart.
    return { kind: "leaf", id: nextId("n"), termId: nextId("term"), key: key ?? crypto.randomUUID(), cwd, title: title ?? "shell", runOnce };
  }

  // ---------- Elyra agent (RPC host) ----------
  function newElyraAgent(cwd, name, initialPrompt = null) {
    const tab = { id: nextId("tab"), kind: "agent", title: name ?? "elyra", projectPath: cwd, cwd, initialPrompt };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = null;
  }
  function askElyra(entry) {
    const dir = dirOf(entry.path);
    const base = baseOf(entry.path);
    newElyraAgent(dir, base, `Explain what \`${base}\` does.`);
  }

  // ---------- tabs ----------
  function newTab(cwd, title, runOnce = null) {
    const leaf = makeLeaf(cwd, title, runOnce);
    const tab = { id: nextId("tab"), kind: "term", title: title ?? "shell", projectPath: cwd, root: leaf };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = leaf.termId;
  }

  function focusTab(t) {
    activeTabId = t.id;
    activeTermId = t.kind === "agent" ? null : firstLeaf(t.root).termId;
  }

  // Pointer-based tab reordering. HTML5 drag-and-drop is unreliable inside the
  // Tauri/WebKit webview (drags often never start), so we track the pointer
  // ourselves. `drag` is null when idle, otherwise the in-flight gesture.
  let tabDrag = $state(null); // { fromIndex, startX, moved, insertAt }
  let dragOver = $derived(tabDrag && tabDrag.moved ? tabDrag.insertAt : null);
  let justDragged = false; // suppress the click WebKit fires right after a drag

  function tabPointerDown(e, i) {
    if (e.button !== 0) return; // left button only
    tabDrag = { fromIndex: i, startX: e.clientX, moved: false, insertAt: i };
    window.addEventListener("pointermove", tabPointerMove);
    window.addEventListener("pointerup", tabPointerUp);
  }

  function tabPointerMove(e) {
    if (!tabDrag) return;
    if (!tabDrag.moved && Math.abs(e.clientX - tabDrag.startX) < 4) return;
    tabDrag = { ...tabDrag, moved: true, insertAt: insertIndexAt(e.clientX) };
  }

  // Map a screen X to an insertion slot (0..tabs.length) by inspecting the
  // rendered tab rects.
  function insertIndexAt(x) {
    const els = document.querySelectorAll(".tabs .tab");
    for (let j = 0; j < els.length; j++) {
      const r = els[j].getBoundingClientRect();
      if (x < r.left + r.width / 2) return j;
    }
    return els.length;
  }

  function tabPointerUp() {
    window.removeEventListener("pointermove", tabPointerMove);
    window.removeEventListener("pointerup", tabPointerUp);
    const d = tabDrag;
    tabDrag = null;
    if (!d || !d.moved) return; // a plain click; focus is handled separately
    justDragged = true;
    setTimeout(() => (justDragged = false), 0); // clear after the click fires
    let to = d.insertAt;
    const arr = [...tabs];
    const [moved] = arr.splice(d.fromIndex, 1);
    if (d.fromIndex < to) to -= 1; // removal shifts later indices left
    to = Math.max(0, Math.min(arr.length, to));
    arr.splice(to, 0, moved);
    tabs = arr;
  }

  // Was the last pointer gesture an actual drag? Used to swallow the click that
  // WebKit fires after a drag so we don't also "focus" the tab.
  function tabClick(t) {
    if (justDragged) return;
    focusTab(t);
  }

  function closeTab(id) {
    tabs = tabs.filter((t) => t.id !== id);
    if (activeTabId === id) {
      const next = tabs.at(-1) ?? null;
      if (next) focusTab(next);
      else {
        activeTabId = null;
        activeTermId = null;
      }
    }
  }

  function selectProject(p) {
    activeProject = p;
    const existing = tabs.find((t) => t.projectPath === p.path);
    if (existing) {
      focusTab(existing);
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
    if (!tab || tab.kind !== "term") return;
    const leaf = makeLeaf(cwdOf(termId));
    tab.root = splitLeaf(tab.root, termId, dir, leaf, nextId("s"));
    activeTermId = leaf.termId;
  }

  function closePane(termId) {
    const tab = activeTab;
    if (!tab || tab.kind !== "term") return;
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
      list.push({ id: `tab:${t.id}`, title: t.title, hint: t.projectPath, group: "tab", icon: "\u{1F5C2}", action: () => focusTab(t) });
    list.push({ id: "act:new-tab", title: "New terminal tab", hint: "", group: "action", icon: "\u002B", action: () => newTab(activeProject?.path ?? root, activeProject?.name) });
    if (elyraVersion)
      list.push({ id: "act:new-elyra", title: "New Elyra agent here", group: "action", icon: "\u{1F916}", action: () => newElyraAgent(activeProject?.path ?? root, activeProject?.name) });
    list.push({ id: "act:split-right", title: "Split right", hint: "\u2318D", group: "action", icon: "\u25A5", action: () => activeTermId && splitPane(activeTermId, "row") });
    list.push({ id: "act:split-down", title: "Split down", hint: "\u21E7\u2318D", group: "action", icon: "\u25A4", action: () => activeTermId && splitPane(activeTermId, "col") });
    list.push({ id: "act:close-pane", title: "Close pane", hint: "\u2318W", group: "action", icon: "\u00D7", action: () => activeTermId && closePane(activeTermId) });
    list.push({ id: "act:toggle-editor", title: showEditor ? "Hide editor" : "Show editor", group: "action", icon: "\u270E", action: () => (showEditor = !showEditor) });
    list.push({ id: "act:toggle-files", title: showFiles ? "Hide file sidebar" : "Show file sidebar", hint: "\u2318B", group: "action", icon: "\u{1F5C2}", action: () => (showFiles = !showFiles) });
    list.push({ id: "act:toggle-hidden", title: showHidden ? "Hide node_modules/.git in tree" : "Show all files in tree", group: "action", icon: "\u{1F441}", action: () => (showHidden = !showHidden) });
    list.push({ id: "act:toggle-theme", title: theme === "dark" ? "Switch to light theme" : "Switch to dark theme", group: "action", icon: theme === "dark" ? "\u2600" : "\u263D", action: () => (theme = theme === "dark" ? "light" : "dark") });
    list.push({ id: "act:toggle-broadcast", title: broadcast ? "Stop broadcasting input" : "Broadcast input to all panes", group: "action", icon: "\u2301", action: () => (broadcast = !broadcast) });
    list.push({ id: "act:quick-edit", title: "Quick edit file\u2026", group: "action", icon: "\u270E", action: quickEdit });
    list.push({ id: "act:change-root", title: "Change projects folder\u2026", group: "action", icon: "\u{1F4C2}", action: changeRoot });
    if (activeProject?.is_git)
      list.push({ id: "act:commit", title: `Git: commit ${activeProject.name}\u2026`, group: "action", icon: "\u2387", action: openCommit });
    list.push({ id: "act:help", title: "Keyboard shortcuts", hint: "\u2318/", group: "action", icon: "?", action: () => (helpOpen = true) });
    list.push({ id: "act:check-update", title: "Check for updates\u2026", group: "action", icon: "\u21BB", action: () => checkForUpdate(true) });
    list.push({ id: "act:reset-layout", title: "Reset saved layout", group: "action", icon: "\u21BA", action: () => { try { localStorage.removeItem(STORAGE_KEY); } catch {} location.reload(); } });
    list.push({ id: "act:save-workspace", title: "Save workspace\u2026", group: "action", icon: "\u{1F4BE}", action: saveWorkspacePrompt });
    for (const name of Object.keys(workspaces)) {
      list.push({ id: `ws:load:${name}`, title: `Load workspace: ${name}`, hint: workspaces[name]?.root ?? "", group: "workspace", icon: "\u{1F5C4}", action: () => loadWorkspace(name) });
      list.push({ id: `ws:del:${name}`, title: `Delete workspace: ${name}`, group: "workspace", icon: "\u{1F5D1}", action: () => deleteWorkspace(name) });
    }
    for (const task of projectTasks)
      list.push({ id: `task:${task.source}:${task.label}`, title: `Run: ${task.label}`, hint: `${task.command}  \u00B7  ${task.source}`, group: "task", icon: "\u25B6", action: () => runTask(task) });
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
    } else {
      // ⌘1–⌘9 (Ctrl on non-mac) jump straight to a tab by its position in the
      // bar. Fall back to e.code (Digit1..9) for layouts where the number row
      // requires Shift to produce a digit.
      let n = null;
      if (k >= "1" && k <= "9") n = Number(k);
      else if (/^Digit[1-9]$/.test(e.code ?? "")) n = Number(e.code.slice(5));
      if (n != null) {
        e.preventDefault();
        const target = tabs[n - 1];
        if (target) focusTab(target);
      }
    }
  }

  // ---------- persistence ----------
  function stripTree(n) {
    if (n.kind === "leaf") return { kind: "leaf", cwd: n.cwd, title: n.title, key: n.key };
    return { kind: "split", dir: n.dir, ratio: n.ratio, a: stripTree(n.a), b: stripTree(n.b) };
  }
  function reviveTree(n) {
    if (n.kind === "leaf") return makeLeaf(n.cwd, n.title, null, n.key);
    return { kind: "split", id: nextId("s"), dir: n.dir, ratio: n.ratio ?? 0.5, a: reviveTree(n.a), b: reviveTree(n.b) };
  }

  function serialize() {
    return {
      root,
      activeProjectPath: activeProject?.path ?? null,
      pinned: pinned.map((p) => ({ path: p.path, name: p.name })),
      showFiles,
      showHidden,
      theme,
      showEditor,
      editorPath,
      activeTabIndex: tabs.findIndex((t) => t.id === activeTabId),
      tabs: tabs.map((t) =>
        t.kind === "agent"
          ? { kind: "agent", title: t.title, projectPath: t.projectPath, cwd: t.cwd }
          : { kind: "term", title: t.title, projectPath: t.projectPath, root: stripTree(t.root) }
      ),
    };
  }

  // Remove saved scrollback for panes that no longer exist (closed panes/tabs),
  // so localStorage doesn't accumulate orphaned buffers.
  const SB_PREFIX = "conductor:sb:";
  function pruneScrollback() {
    try {
      const live = new Set();
      for (const t of tabs) {
        if (t.kind !== "term") continue;
        for (const l of allLeaves(t.root)) live.add(l.key);
      }
      for (let i = localStorage.length - 1; i >= 0; i--) {
        const k = localStorage.key(i);
        if (k && k.startsWith(SB_PREFIX) && !live.has(k.slice(SB_PREFIX.length))) {
          localStorage.removeItem(k);
        }
      }
    } catch {}
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
      pruneScrollback();
    }, 250);
  });

  // Apply a serialized snapshot (from auto-save or a named workspace) to the
  // live state. Does not touch `root` — callers load projects for the right
  // root first when switching workspaces.
  function applySnapshot(saved) {
    if (!saved) return false;

    if (Array.isArray(saved.pinned)) pinned = saved.pinned.map((p) => ({ ...p }));
    activeProject = saved.activeProjectPath
      ? projects.find((p) => p.path === saved.activeProjectPath) ?? null
      : null;
    showFiles = saved.showFiles ?? true;
    showHidden = saved.showHidden ?? false;
    theme = saved.theme ?? "dark";
    showEditor = saved.showEditor ?? false;
    editorPath = saved.editorPath ?? null;

    if (Array.isArray(saved.tabs) && saved.tabs.length) {
      tabs = saved.tabs.map((t) =>
        t.kind === "agent"
          ? { id: nextId("tab"), kind: "agent", title: t.title, projectPath: t.projectPath, cwd: t.cwd, initialPrompt: null }
          : { id: nextId("tab"), kind: "term", title: t.title, projectPath: t.projectPath, root: reviveTree(t.root) }
      );
      const at = tabs[saved.activeTabIndex] ?? tabs[0];
      focusTab(at);
    } else {
      tabs = [];
      activeTabId = null;
      activeTermId = null;
    }
    return true;
  }

  function restore() {
    let saved;
    try {
      saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    } catch {
      saved = null;
    }
    return applySnapshot(saved);
  }

  // ---------- named workspaces ----------
  // A workspace is just a serialized snapshot stored under a name, reusing the
  // exact same shape as the auto-saved session. Global (across root folders).
  function loadWorkspaces() {
    try {
      workspaces = JSON.parse(localStorage.getItem(WORKSPACES_KEY) ?? "{}") ?? {};
    } catch {
      workspaces = {};
    }
  }
  function persistWorkspaces() {
    try {
      localStorage.setItem(WORKSPACES_KEY, JSON.stringify(workspaces));
    } catch {}
  }
  function saveWorkspacePrompt() {
    const name = prompt("Save current layout as workspace:", activeWorkspace ?? "")?.trim();
    if (!name) return;
    workspaces = { ...workspaces, [name]: serialize() };
    activeWorkspace = name;
    persistWorkspaces();
  }
  async function loadWorkspace(name) {
    const snap = workspaces[name];
    if (!snap) return;
    // The workspace may point at a different projects folder; switch to it and
    // re-scan so activeProject resolution works before applying the snapshot.
    if (snap.root && snap.root !== root) {
      root = snap.root;
      await loadProjects();
    }
    applySnapshot(snap);
    activeWorkspace = name;
  }
  function deleteWorkspace(name) {
    if (!workspaces[name]) return;
    if (!confirm(`Delete workspace "${name}"?`)) return;
    const next = { ...workspaces };
    delete next[name];
    workspaces = next;
    if (activeWorkspace === name) activeWorkspace = null;
    persistWorkspaces();
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
    elyraVersion = await invoke("detect_elyra");
    loadWorkspaces();

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
    loadGitStatus(); // enrich pinned items resolved during restore
    loaded = true;

    // Dismiss the startup splash once the UI is ready.
    const splash = document.getElementById("splash");
    if (splash) {
      splash.classList.add("hide");
      setTimeout(() => splash.remove(), 450);
    }

    checkForUpdate(false); // silent check on startup
    // Preload the editor in the background so the first open is instant.
    if ("requestIdleCallback" in window) requestIdleCallback(loadEditor);
    else setTimeout(loadEditor, 1500);
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
    {pinned}
    {editors}
    {root}
    activePath={activeProject?.path}
    onselect={selectProject}
    onopen={openInEditor}
    onroot={changeRoot}
    onrefresh={loadProjects}
    onpin={togglePin}
    elyra={!!elyraVersion}
    onagent={(p) => newElyraAgent(p.path, p.name)}
  />

  <div class="main">
    <div class="topbar">
      <div class="tabs">
        {#each tabs as t, i (t.id)}
          <div
            class="tab"
            class:active={t.id === activeTabId}
            class:ring={activity[t.id]}
            class:drop-left={dragOver === i}
            class:drop-right={dragOver === tabs.length && i === tabs.length - 1}
            class:dragging={tabDrag && tabDrag.moved && tabDrag.fromIndex === i}
            onpointerdown={(e) => tabPointerDown(e, i)}
          >
            {#if activity[t.id]}<span class="ring-dot" title="New activity"></span>{/if}
            <span
              class="tab-label"
              role="button"
              tabindex="0"
              onclick={() => tabClick(t)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && focusTab(t)}
            >{tabTitle(t)}</span>
            <span
              class="tab-x"
              role="button"
              tabindex="0"
              title="Close tab"
              onpointerdown={(e) => e.stopPropagation()}
              onclick={(e) => { e.stopPropagation(); closeTab(t.id); }}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && closeTab(t.id)}
            >×</span>
          </div>
        {/each}
        <button class="new-tab" title="New tab" onclick={() => newTab(activeProject?.path ?? root, activeProject?.name)}>＋</button>
      </div>
      <div class="top-actions">
        <button class="kbd-btn" onclick={() => (paletteOpen = true)}>Search <kbd>⌘K</kbd></button>
        <button onclick={quickEdit}>Quick edit</button>
        <button class:on={showEditor} onclick={() => (showEditor = !showEditor)}>{showEditor ? "Hide editor" : "Show editor"}</button>
        <button class:on={showFiles} title="Toggle file sidebar (⌘B)" onclick={() => (showFiles = !showFiles)}>Files</button>
        <button class:on={broadcast} title="Broadcast input to all panes in this tab" onclick={() => (broadcast = !broadcast)}>⌁ Sync</button>
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
          {#if tab.kind === "agent"}
            <div class="term-area" style:display={tab.id === activeTabId ? "block" : "none"}>
              <AgentPanel id={tab.id} cwd={tab.cwd} initialPrompt={tab.initialPrompt ?? null} onactivity={() => markActivity(tab.id)} ontitle={(t) => (tab.title = t)} />
            </div>
          {:else}
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
                <Terminal id={leaf.termId} cwd={leaf.cwd} {theme} persistKey={leaf.key} runCommand={leaf.runOnce ?? null} onactivity={() => markActivity(tab.id)} onuserinput={onPaneInput} />
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
          {/if}
        {/each}
      </div>

      {#if showEditor}
        <div class="editor-area">
          {#if EditorComp}
            <EditorComp path={editorPath} {theme} onclose={() => (showEditor = false)} />
          {:else}
            <div class="editor-loading">Loading editor…</div>
          {/if}
        </div>
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

  {#if update && !updateDismissed}
    <div class="update-toast">
      {#if updateStatus === "downloading"}
        <span>⬇ Downloading update… {updateProgress}%</span>
      {:else if updateStatus === "ready"}
        <span>✓ Restarting…</span>
      {:else if updateStatus === "error"}
        <span class="err">Update failed: {updateError}</span>
        <button onclick={() => (updateDismissed = true)}>Dismiss</button>
      {:else}
        <span>⬆ Update available: <strong>v{update.version}</strong></span>
        <button class="primary" onclick={installUpdate}>Install &amp; restart</button>
        <button onclick={() => (updateDismissed = true)}>Later</button>
      {/if}
    </div>
  {/if}
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
  .tab.drop-left { box-shadow: inset 2px 0 0 0 var(--accent); }
  .tab.drop-right { box-shadow: inset -2px 0 0 0 var(--accent); }
  .tab { cursor: grab; }
  .tab:active { cursor: grabbing; }
  .tab.dragging { opacity: 0.5; }
  .ring-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--green); margin-left: 6px; flex: none; animation: ring-pulse 1.2s ease-in-out infinite; }
  @keyframes ring-pulse { 0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(158, 206, 106, 0.6); } 50% { opacity: 0.5; box-shadow: 0 0 0 4px rgba(158, 206, 106, 0); } }
  .tab-label { display: inline-block; background: transparent; border: none; color: var(--text); padding: 4px 6px; font-size: 12px; max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; -webkit-user-drag: none; user-select: none; }
  .tab-x { display: inline-block; background: transparent; border: none; color: var(--text-dim); padding: 0 4px; font-size: 14px; cursor: pointer; -webkit-user-drag: none; user-select: none; }
  .tab-x:hover { color: var(--text); }
  .tab { -webkit-user-select: none; user-select: none; }
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
  .editor-loading { color: var(--text-dim); padding: 16px; font-size: 12px; }
  .update-toast {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 180;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 14px;
    font-size: 12px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  }
  .update-toast .err { color: var(--red); }
  .update-toast button {
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 11px;
  }
  .update-toast button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
