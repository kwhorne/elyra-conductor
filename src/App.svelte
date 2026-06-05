<script>
  import { onMount, onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Sidebar from "./lib/Sidebar.svelte";
  import Terminal from "./lib/Terminal.svelte";
  import AgentPanel from "./lib/AgentPanel.svelte";
  import RunbookPanel from "./lib/RunbookPanel.svelte";
  import DBPanel from "./lib/DBPanel.svelte";
  import DBView from "./lib/DBView.svelte";
  import PortsModal from "./lib/PortsModal.svelte";
  import ScrollbackSearch from "./lib/ScrollbackSearch.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";
  import FileFinder from "./lib/FileFinder.svelte";
  import FileExplorer from "./lib/FileExplorer.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import InputDialog from "./lib/InputDialog.svelte";
  import AboutModal from "./lib/AboutModal.svelte";
  import { listen } from "@tauri-apps/api/event";
  import RunModal from "./lib/RunModal.svelte";
  import CommitDialog from "./lib/CommitDialog.svelte";
  import GitPanel from "./lib/GitPanel.svelte";
  import TasksModal from "./lib/TasksModal.svelte";
  import EnvModal from "./lib/EnvModal.svelte";
  import TimelineModal from "./lib/TimelineModal.svelte";
  import ShortcutsModal from "./lib/ShortcutsModal.svelte";
  import { check as checkUpdate } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
  import { geometry, splitLeaf, removeLeaf, setRatio, firstLeaf, allLeaves } from "./lib/layout.js";
  import { dirOf, baseOf, detectRunCommand, isIdleProc, rankDevTasks, scoreDevTask } from "./lib/util.js";

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
  let finderOpen = $state(false);
  let helpOpen = $state(false);
  let aboutOpen = $state(false);

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
  let tasksOpen = $state(false);
  let envOpen = $state(false);
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

  function openFile(path, line = null) {
    editorPath = path;
    pendingLine = line;
    openSeq++;
    showEditor = true;
  }
  let pendingLine = $state(null);
  let openSeq = $state(0);
  let editorRef = $state(null);

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

  // ---------- file operations (rename/new/duplicate/delete) ----------
  // A counter the file tree watches; bump it to force a reload after any change.
  let fileRefresh = $state(0);
  let fileDlg = $state({
    open: false, title: "", message: "", input: false, value: "",
    placeholder: "", confirmLabel: "OK", danger: false, onconfirm: null,
  });
  function closeFileDlg() { fileDlg = { ...fileDlg, open: false }; }
  function alertMsg(title, message) {
    fileDlg = { open: true, title, message, input: false, value: "", placeholder: "",
      confirmLabel: "OK", danger: false, onconfirm: null };
  }
  async function fileOp(promise) {
    try { await promise; fileRefresh++; }
    catch (e) { alertMsg("Couldn't complete that", String(e)); }
  }
  function renameEntry(e) {
    fileDlg = {
      open: true, title: "Rename", message: e.path, input: true, value: baseOf(e.path),
      placeholder: "New name", confirmLabel: "Rename", danger: false,
      onconfirm: (name) => fileOp(invoke("rename_path", { from: e.path, to: dirOf(e.path) + "/" + name })),
    };
  }
  function newFileIn(dir) {
    fileDlg = {
      open: true, title: "New file", message: dir, input: true, value: "",
      placeholder: "filename.ext", confirmLabel: "Create", danger: false,
      onconfirm: (name) => fileOp(invoke("create_file", { path: dir + "/" + name }).then(() => openFile(dir + "/" + name))),
    };
  }
  function newFolderIn(dir) {
    fileDlg = {
      open: true, title: "New folder", message: dir, input: true, value: "",
      placeholder: "folder name", confirmLabel: "Create", danger: false,
      onconfirm: (name) => fileOp(invoke("create_folder", { path: dir + "/" + name })),
    };
  }
  function duplicateEntry(e) {
    const name = baseOf(e.path);
    const dot = name.lastIndexOf(".");
    const dup = dot > 0 ? `${name.slice(0, dot)} copy${name.slice(dot)}` : `${name} copy`;
    fileDlg = {
      open: true, title: "Duplicate", message: e.path, input: true, value: dup,
      placeholder: "New name", confirmLabel: "Duplicate", danger: false,
      onconfirm: (n) => fileOp(invoke("copy_path", { from: e.path, to: dirOf(e.path) + "/" + n })),
    };
  }
  function deleteEntry(e) {
    fileDlg = {
      open: true, title: "Move to Trash", message: `“${baseOf(e.path)}” will be moved to the Trash.`,
      input: false, value: "", placeholder: "", confirmLabel: "Move to Trash", danger: true,
      onconfirm: () => fileOp(invoke("trash_path", { path: e.path })),
    };
  }
  // Move a path into a destination folder (drag & drop). Guards against no-ops
  // and dropping a folder into itself or a descendant.
  function moveEntry(from, toDir) {
    const dest = toDir + "/" + baseOf(from);
    if (from === dest || dirOf(from) === toDir) return;
    if (toDir === from || toDir.startsWith(from + "/")) {
      alertMsg("Can't move there", "A folder can't be moved into itself.");
      return;
    }
    fileOp(invoke("rename_path", { from, to: dest }));
  }
  function revealEntry(e) { invoke("reveal_path", { path: e.path }).catch(() => {}); }
  function copyEntryPath(e) { navigator.clipboard?.writeText(e.path).catch(() => {}); }

  function openCommit() {
    const p = activeProject ?? projects.find((x) => x.path === activeTab?.projectPath);
    if (!p) return;
    commit = { open: true, path: p.path, name: p.name };
  }

  let gitPanel = $state({ open: false, path: "", name: "" });
  function openGitPanel() {
    const p = activeProject ?? projects.find((x) => x.path === activeTab?.projectPath);
    if (!p) return;
    gitPanel = { open: true, path: p.path, name: p.name };
  }

  function onFileContext(entry, x, y) {
    ctx = { open: true, x, y, entry };
  }

  function runInModal(entry) {
    runModal = {
      open: true,
      cwd: dirOf(entry.path),
      command: detectRunCommand(baseOf(entry.path)),
      title: baseOf(entry.path),
    };
  }

  // Run a file in a real, persistent terminal tab (rooted at its folder). Unlike
  // the modal, this stays open, is fully interactive, and keeps all output — the
  // right way to run a deploy script or anything you want to watch/interact with.
  function runFileInTab(entry) {
    newTab(dirOf(entry.path), baseOf(entry.path), detectRunCommand(baseOf(entry.path)));
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
        { label: "Start (dev)", icon: "\u25B6", action: () => startProject(e.path) },
        { label: "Set start command\u2026", icon: "\u2699", action: () => setStartCommand(e.path) },
        { separator: true },
        { label: "Open new terminal here", icon: "\u{1F5A5}", action: () => newTab(e.path, baseOf(e.path)) },
        { label: "Open runbook here", icon: "\u{1F4D3}", action: () => openRunbook(e.path, baseOf(e.path)) },
      ];
      if (elyraVersion)
        items.push({ label: "New Elyra agent here", icon: "\u{1F916}", action: () => newElyraAgent(e.path, baseOf(e.path)) });
      items.push(
        { separator: true },
        { label: "New file\u2026", icon: "\u{1F4C4}", action: () => newFileIn(e.path) },
        { label: "New folder\u2026", icon: "\u{1F4C1}", action: () => newFolderIn(e.path) },
        { label: "Rename\u2026", icon: "\u270E", action: () => renameEntry(e) },
        { label: "Duplicate\u2026", icon: "\u29C9", action: () => duplicateEntry(e) },
        { separator: true },
        { label: "Reveal in Finder", icon: "\u{1F50D}", action: () => revealEntry(e) },
        { label: "Copy path", icon: "\u{1F4CB}", action: () => copyEntryPath(e) },
        { label: "Move to Trash", icon: "\u{1F5D1}", danger: true, action: () => deleteEntry(e) },
      );
      return items;
    }
    const items = [
      { label: "Open in editor", icon: "\u270E", action: () => openFile(e.path) },
    ];
    if (elyraVersion)
      items.push({ label: "Ask Elyra about this file", icon: "\u{1F916}", action: () => askElyra(e) });
    items.push(
      { separator: true },
      { label: `Run ${baseOf(e.path)} in a terminal tab`, icon: "\u25B6", action: () => runFileInTab(e) },
      { label: `Run ${baseOf(e.path)}\u2026 (modal)`, icon: "\u25B7", action: () => runInModal(e) },
      { label: `Run in ${terminalName}`, icon: "\u{1F680}", action: () => runExternal(e) },
      { separator: true },
      { label: "Rename\u2026", icon: "\u270E", action: () => renameEntry(e) },
      { label: "Duplicate\u2026", icon: "\u29C9", action: () => duplicateEntry(e) },
      { label: "Reveal in Finder", icon: "\u{1F50D}", action: () => revealEntry(e) },
      { label: "Copy path", icon: "\u{1F4CB}", action: () => copyEntryPath(e) },
      { label: "Move to Trash", icon: "\u{1F5D1}", danger: true, action: () => deleteEntry(e) }
    );
    return items;
  });

  let drag = null; // { ...divider, container } during resize
  let activity = $state({}); // tabId -> true when a background tab produced output
  let titles = $state({}); // termId -> foreground process name (or null)

  // ---------- finished-command notifications ----------
  // Detect when a long-running foreground command in a *background* tab returns
  // to the shell, and fire a native notification. Uses the titles we already
  // poll — pure observation, no AI.
  let notifyOnFinish = $state(true);
  // Opt-in zsh shell integration (real command lines + exit codes via OSC 133/633).
  let shellIntegration = $state(false);
  let notifyPermission = false;
  let appFocused = true;
  let procRuns = {}; // termId -> { proc, since } for the current foreground command
  const NOTIFY_MIN_MS = 8000; // ignore commands shorter than this

  // ---------- command timeline (flight recorder) ----------
  // Built from the same foreground-process transitions we use for notifications:
  // each finished command is logged with its pane, duration, and timestamps.
  // Session-only (not persisted); newest first, capped.
  let commandLog = $state([]);
  let timelineOpen = $state(false);
  // Terminals that have proven they have shell integration (emitted an OSC
  // command record). For those we use the rich OSC data and suppress the
  // title-based recorder to avoid duplicate entries.
  const integratedTerms = new Set();
  // Per-project last test result (from shell integration): { ok, at }.
  let lastTest = $state({});
  function pushCommand(entry) {
    commandLog = [entry, ...commandLog].slice(0, 200);
  }
  function recordCommand(termId, proc, since, now) {
    const tab = tabForTerm(termId);
    pushCommand({
      id: `cmd-${++idSeq}`,
      termId,
      tabId: tab?.id ?? null,
      label: tab ? tab.title || baseOf(tab.projectPath || "") : "",
      projectPath: tab?.projectPath ?? null,
      proc,
      command: null,
      exitCode: null,
      startedAt: since,
      endedAt: now,
      duration: now - since,
    });
  }

  const TEST_RE = /\b(pest|phpunit|vitest|jest|pytest|cargo test|go test|npm test|rspec|mocha|playwright|cypress)\b/i;
  // A shell-integration command record from a terminal (real command + exit code).
  function onShellCommand(termId, rec) {
    integratedTerms.add(termId);
    const tab = tabForTerm(termId);
    const cmd = (rec.command || "").trim();
    const proc = cmd ? cmd.split(/\s+/)[0].split("/").pop() : "command";
    const now = rec.startedAt + rec.duration;
    pushCommand({
      id: `cmd-${++idSeq}`,
      termId,
      tabId: tab?.id ?? null,
      label: tab ? tab.title || baseOf(tab.projectPath || "") : "",
      projectPath: tab?.projectPath ?? null,
      proc,
      command: cmd || null,
      exitCode: rec.exitCode,
      startedAt: rec.startedAt,
      endedAt: now,
      duration: rec.duration,
    });
    // Track the last test run per project for the health strip.
    if (cmd && TEST_RE.test(cmd) && tab?.projectPath && rec.exitCode != null) {
      lastTest = { ...lastTest, [tab.projectPath]: { ok: rec.exitCode === 0, at: now } };
    }
  }
  function jumpToCommand(entry) {
    const tab = tabs.find((t) => t.id === entry.tabId);
    if (tab) {
      focusTab(tab);
      if (entry.termId) activeTermId = entry.termId;
    }
    timelineOpen = false;
  }
  function tabForTerm(termId) {
    return tabs.find((t) => t.kind === "term" && allLeaves(t.root).some((l) => l.termId === termId));
  }
  async function ensureNotifyPermission() {
    try {
      notifyPermission = await isPermissionGranted();
      if (!notifyPermission) notifyPermission = (await requestPermission()) === "granted";
    } catch {
      notifyPermission = false;
    }
  }
  function notifyFinished(termId, proc, ms) {
    if (!notifyOnFinish || !notifyPermission) return;
    const tab = tabForTerm(termId);
    if (!tab) return;
    if (appFocused && tab.id === activeTabId) return; // you're already watching it
    const secs = Math.round(ms / 1000);
    const dur = secs < 60 ? `${secs}s` : `${Math.round(secs / 60)}m`;
    const where = tab.title || baseOf(tab.projectPath || "");
    try {
      sendNotification({ title: `\u2713 ${proc} finished`, body: `${where} \u00b7 ran ${dur}`, sound: "default" });
    } catch {}
  }

  // A tab's main label is the project/identifier name — stable regardless of what
  // is currently running, so you can always tell which project a tab belongs to.
  function tabTitle(t) {
    if (t.kind === "runbook" || t.kind === "db") return t.title;
    return t.title || (t.projectPath ? baseOf(t.projectPath) : "shell");
  }

  // The foreground process running in a term tab (e.g. "vite", "node", "php"),
  // or null when it's just sitting at a shell prompt. Used for the running
  // marker / process chip on the tab.
  function tabProc(t) {
    if (t.kind !== "term") return null;
    for (const l of allLeaves(t.root)) {
      const name = titles[l.termId];
      if (name && !isIdleProc(name)) return name;
    }
    return null;
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
    const prev = titles;
    const next = {};
    const now = Date.now();
    for (const [id, name] of results) {
      next[id] = name;
      const wasIdle = isIdleProc(prev[id]);
      const nowIdle = isIdleProc(name);
      if (!nowIdle && wasIdle) {
        procRuns[id] = { proc: name, since: now }; // a foreground command started
      } else if (nowIdle && !wasIdle) {
        const run = procRuns[id];
        delete procRuns[id];
        if (run) {
          if (!integratedTerms.has(id)) recordCommand(id, run.proc, run.since, now);
          if (now - run.since >= NOTIFY_MIN_MS) notifyFinished(id, run.proc, now - run.since);
        }
      }
    }
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
    // Cap concurrency: firing git_status for every repo at once (dozens of them)
    // spawned a process storm on each window focus and froze the UI for seconds.
    // A small worker pool keeps the machine responsive.
    const jobs = [
      ...projects.map((p) => () => enrich(p, false)),
      ...pinned.map((p) => () => enrich(p, true)),
    ];
    const CONCURRENCY = 6;
    let i = 0;
    const worker = async () => {
      while (i < jobs.length) {
        const job = jobs[i++];
        await job();
      }
    };
    await Promise.all(Array.from({ length: Math.min(CONCURRENCY, jobs.length) }, worker));
  }

  function makeLeaf(cwd, title, runOnce = null, key = null) {
    // `key` is a stable per-pane id that survives serialize/restore, so the
    // saved scrollback can be matched back to the right pane after restart.
    return { kind: "leaf", id: nextId("n"), termId: nextId("term"), key: key ?? crypto.randomUUID(), cwd, title: title ?? "shell", runOnce };
  }

  // ---------- Elyra agent (RPC host) ----------
  function newElyraAgent(cwd, name, initialPrompt = null, initialDraft = null) {
    const tab = { id: nextId("tab"), kind: "agent", title: name ?? "elyra", projectPath: cwd, cwd, initialPrompt, initialDraft };
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
    activeTermId = t.kind === "term" ? firstLeaf(t.root).termId : null;
    zoomed = false;
  }

  let portsOpen = $state(false);

  // A stable accent colour per project (derived from its path), so you can tell
  // at a glance which project a tab belongs to.
  function projectColor(path) {
    if (!path) return "transparent";
    let h = 0;
    for (let i = 0; i < path.length; i++) h = (h * 31 + path.charCodeAt(i)) >>> 0;
    return `hsl(${h % 360} 60% 60%)`;
  }

  // ---------- per-project health: listening ports ----------
  let projectPorts = $state({}); // projectPath -> [{ port, process }]
  let portsTimer = null;
  function refreshProjectPorts() {
    invoke("list_ports")
      .then((ports) => {
        const all = [...projects, ...pinned];
        const map = {};
        for (const p of ports) {
          if (!p.cwd) continue;
          for (const proj of all) {
            if (p.cwd === proj.path || p.cwd.startsWith(proj.path + "/")) {
              (map[proj.path] ??= []).push({ port: p.port, process: p.process });
            }
          }
        }
        for (const k in map) {
          map[k] = [...new Map(map[k].map((x) => [x.port, x])).values()].sort((a, b) => a.port - b.port);
        }
        projectPorts = map;
      })
      .catch(() => {});
  }
  function openLocalPort(port) {
    invoke("open_url", { url: `http://localhost:${port}` }).catch(() => {});
  }

  // ---------- per-project health: docker containers ----------
  let projectContainers = $state({}); // projectPath -> { running, total }
  let dockerAvailable = $state(true);
  function refreshContainers() {
    if (!dockerAvailable) return;
    invoke("list_containers")
      .then((list) => {
        const all = [...projects, ...pinned];
        const map = {};
        for (const c of list) {
          if (!c.working_dir) continue;
          for (const proj of all) {
            if (c.working_dir === proj.path || c.working_dir.startsWith(proj.path + "/")) {
              const m = (map[proj.path] ??= { running: 0, total: 0 });
              m.total++;
              if (c.state === "running") m.running++;
            }
          }
        }
        projectContainers = map;
      })
      .catch(() => { dockerAvailable = false; });
  }

  // Which projects currently have a foreground command running in one of their
  // term tabs (uses the titles we already poll).
  let projectRunning = $derived.by(() => {
    const map = {};
    for (const t of tabs) {
      if (t.kind !== "term" || !t.projectPath) continue;
      for (const l of allLeaves(t.root)) {
        const name = titles[l.termId];
        if (name && !isIdleProc(name)) { map[t.projectPath] = true; break; }
      }
    }
    return map;
  });

  // ---------- pane navigation / zoom / global scrollback search ----------
  let zoomed = $state(false);
  let scrollbackOpen = $state(false);
  let termApis = {}; // termId -> { getLines, find, focus, fit }
  function registerTerm(id, api) { termApis[id] = api; }
  function unregisterTerm(id) { delete termApis[id]; }
  function refitTerminals() {
    for (const a of Object.values(termApis)) a.fit?.();
  }
  // When the layout around the terminals changes (editor/files/db panels toggle,
  // tab switch, zoom), re-fit them. WebKit's ResizeObserver doesn't always fire
  // when a flex sibling is added/removed, which left terminals stuck narrow.
  $effect(() => {
    showEditor; showFiles; showDb; zoomed; activeTabId;
    setTimeout(refitTerminals, 80);
  });

  // Move pane focus in a direction, by comparing pane-rect centres.
  function navigatePane(dir) {
    if (!activeTab || activeTab.kind !== "term") return;
    const leaves = geo.leaves;
    const cur = leaves.find((l) => l.termId === activeTermId) ?? leaves[0];
    if (!cur) return;
    const cx = cur.rect.x + cur.rect.w / 2;
    const cy = cur.rect.y + cur.rect.h / 2;
    let best = null;
    let bestScore = Infinity;
    for (const l of leaves) {
      if (l.termId === activeTermId) continue;
      const dx = l.rect.x + l.rect.w / 2 - cx;
      const dy = l.rect.y + l.rect.h / 2 - cy;
      let ok = false;
      let score = 0;
      if (dir === "left") { ok = dx < -1; score = Math.abs(dx) + Math.abs(dy) * 3; }
      else if (dir === "right") { ok = dx > 1; score = Math.abs(dx) + Math.abs(dy) * 3; }
      else if (dir === "up") { ok = dy < -1; score = Math.abs(dy) + Math.abs(dx) * 3; }
      else if (dir === "down") { ok = dy > 1; score = Math.abs(dy) + Math.abs(dx) * 3; }
      if (ok && score < bestScore) { bestScore = score; best = l; }
    }
    if (best) { zoomed = false; activeTermId = best.termId; }
  }

  // Search every open terminal's buffer (scrollback + viewport).
  function searchScrollback(query) {
    const q = (query || "").toLowerCase();
    if (q.length < 2) return [];
    const results = [];
    for (const t of tabs) {
      if (t.kind !== "term") continue;
      for (const l of allLeaves(t.root)) {
        const api = termApis[l.termId];
        if (!api) continue;
        let count = 0;
        let sample = "";
        for (const line of api.getLines()) {
          if (line.toLowerCase().includes(q)) {
            count++;
            if (!sample) sample = line.trim().slice(0, 140);
          }
        }
        if (count > 0) {
          results.push({ termId: l.termId, tabId: t.id, label: tabTitle(t), color: projectColor(t.projectPath), count, sample });
        }
      }
    }
    results.sort((a, b) => b.count - a.count);
    return results;
  }

  function jumpToMatch(r, query) {
    const tab = tabs.find((t) => t.id === r.tabId);
    if (tab) { focusTab(tab); activeTermId = r.termId; zoomed = false; }
    setTimeout(() => termApis[r.termId]?.find(query), 60);
  }

  // ---------- database browser (multiple connections per project) ----------
  let showDb = $state(false);
  let dbConns = $state([]); // [{ key, config, id, tables, expanded, connecting, error }]
  let dbProject = $state(null); // project path the list belongs to
  let dbError = $state(null);
  let dbSeq = 0;

  function dbProjectPath() {
    return activeProject?.path ?? activeTab?.projectPath ?? null;
  }

  // Stored connections live in the OS keychain, keyed by project (incl. passwords) —
  // never written into the project, never committed.
  async function loadDbConnections(projectPath) {
    for (const c of dbConns) if (c.id) invoke("db_disconnect", { id: c.id }).catch(() => {});
    dbProject = projectPath;
    dbError = null;
    let configs = [];
    if (projectPath) {
      try {
        configs = await invoke("list_connections", { project: projectPath });
      } catch {}
    }
    dbConns = configs.map((cfg) => ({ key: `dbc-${++dbSeq}`, config: cfg, id: null, tables: [], expanded: false, connecting: false, error: null }));
  }

  async function persistDbConnections() {
    if (!dbProject) return;
    try {
      await invoke("save_connections", { project: dbProject, connections: dbConns.map((c) => c.config) });
    } catch (e) {
      dbError = String(e);
    }
  }

  async function dbConnectEntry(entry) {
    entry.connecting = true;
    entry.error = null;
    dbConns = [...dbConns];
    try {
      const id = await invoke("db_connect", { config: entry.config });
      entry.id = id;
      entry.tables = await invoke("db_tables", { id }).catch(() => []);
      entry.expanded = true;
    } catch (e) {
      entry.error = String(e);
    }
    entry.connecting = false;
    dbConns = [...dbConns];
  }

  function dbToggleEntry(entry) {
    if (!entry.id) {
      dbConnectEntry(entry);
      return;
    }
    entry.expanded = !entry.expanded;
    dbConns = [...dbConns];
  }

  async function dbAddFromEnv() {
    const path = dbProjectPath();
    if (!path) {
      dbError = "Select a project first.";
      return;
    }
    dbError = null;
    try {
      const cfg = await invoke("db_from_env", { project: path });
      if (!cfg) {
        dbError = "No supported DB_CONNECTION (mysql/pgsql/clickhouse/sqlite) in .env";
        return;
      }
      await dbAddConnection(cfg);
    } catch (e) {
      dbError = String(e);
    }
  }

  async function dbTestConnection(cfg) {
    // Throws on failure so the panel can show the error.
    await invoke("db_test", { config: cfg });
  }

  async function dbAddConnection(cfg) {
    if (!dbProject) dbProject = dbProjectPath();
    const entry = { key: `dbc-${++dbSeq}`, config: cfg, id: null, tables: [], expanded: false, connecting: false, error: null };
    dbConns = [...dbConns, entry];
    await dbConnectEntry(entry);
    if (!entry.error) persistDbConnections();
  }

  function closeDbTabsFor(connId) {
    tabs = tabs.filter((t) => !(t.kind === "db" && t.connId === connId));
    if (!tabs.find((t) => t.id === activeTabId)) activeTabId = tabs.at(-1)?.id ?? null;
  }

  async function dbDisconnectEntry(entry) {
    if (entry.id) {
      await invoke("db_disconnect", { id: entry.id }).catch(() => {});
      closeDbTabsFor(entry.id);
    }
    entry.id = null;
    entry.tables = [];
    entry.expanded = false;
    dbConns = [...dbConns];
  }

  async function dbRemoveEntry(entry) {
    await dbDisconnectEntry(entry);
    dbConns = dbConns.filter((c) => c !== entry);
    persistDbConnections();
  }

  async function dbRefreshEntry(entry) {
    if (!entry.id) return;
    try {
      entry.tables = await invoke("db_tables", { id: entry.id });
    } catch (e) {
      entry.error = String(e);
    }
    dbConns = [...dbConns];
  }

  async function dbEditConnection(entry, cfg) {
    entry.config = cfg;
    if (entry.id) {
      await invoke("db_disconnect", { id: entry.id }).catch(() => {});
      closeDbTabsFor(entry.id);
      entry.id = null;
      entry.tables = [];
      await dbConnectEntry(entry);
    }
    dbConns = [...dbConns];
    if (!entry.error) persistDbConnections();
  }

  function openDbTable(entry, table) {
    if (!entry.id) return;
    const tab = { id: nextId("tab"), kind: "db", title: table, view: "table", table, connId: entry.id, engine: entry.config.engine, projectPath: dbProject };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = null;
  }

  function openDbQuery(entry) {
    if (!entry.id) return;
    const tab = { id: nextId("tab"), kind: "db", title: "query", view: "query", table: null, connId: entry.id, engine: entry.config.engine, projectPath: dbProject };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = null;
  }

  // Reload the connection list when the active project changes.
  $effect(() => {
    const p = activeProject?.path ?? null;
    if (p !== dbProject) untrack(() => loadDbConnections(p));
  });

  // ---------- runbooks ----------
  function openRunbook(cwd, name, file = null) {
    // Reuse an existing runbook tab for this project if present.
    const existing = tabs.find((t) => t.kind === "runbook" && t.projectPath === cwd);
    if (existing) {
      focusTab(existing);
      return;
    }
    const tab = { id: nextId("tab"), kind: "runbook", title: name ?? "runbook", projectPath: cwd, file };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    activeTermId = null;
  }

  // Run a shell command (from a runbook) in the project's terminal: reuse a
  // matching terminal tab/pane if one exists, otherwise open a fresh one.
  // Send a runbook step/snippet to a fresh Elyra agent, pre-filled in its
  // composer so the user can add their question before sending. Pure delegation
  // — Conductor just hands Elyra some text.
  function sendToElyra(cwd, text) {
    const draft = "```\n" + (text || "").trim() + "\n```\n\n";
    newElyraAgent(cwd, "elyra", null, draft);
  }

  function runInProjectTerminal(cwd, cmd) {
    let tab =
      activeTab && activeTab.kind === "term" && activeTab.projectPath === cwd
        ? activeTab
        : tabs.find((t) => t.kind === "term" && t.projectPath === cwd);
    if (tab) {
      const leaf = firstLeaf(tab.root);
      focusTab(tab);
      const data = cmd.endsWith("\n") ? cmd : cmd + "\n";
      invoke("pty_write", { id: leaf.termId, data }).catch(() => {});
    } else {
      newTab(cwd, baseOf(cwd), cmd);
    }
  }

  // Run a [[task:<label>]] from a runbook: resolve the label against the
  // project's discovered tasks and run its command; fall back to the literal
  // text so ad-hoc `task:npm test` still works.
  async function runRunbookTask(cwd, label) {
    let command = label;
    try {
      const list = await invoke("list_tasks", { path: cwd });
      const t = list.find((x) => x.label.toLowerCase() === label.toLowerCase());
      if (t) command = t.command;
    } catch {}
    runInProjectTerminal(cwd, command);
  }

  // ---------- universal "Start" (dev) runner ----------
  // Every project starts differently (npm run dev, pnpm dev, composer run dev,
  // make dev, php artisan serve…). Conductor already resolves the *command* per
  // task source; this picks the right one automatically so "start this project"
  // is one action regardless of stack. Pure detection + launch — no AI.
  const DEV_CMDS_KEY = "conductor:devcmds";
  let devCmds = $state({}); // { [projectPath]: command } — user overrides win
  function loadDevCmds() {
    try {
      devCmds = JSON.parse(localStorage.getItem(DEV_CMDS_KEY) ?? "{}") ?? {};
    } catch {
      devCmds = {};
    }
  }
  function setDevCmd(path, cmd) {
    devCmds = { ...devCmds, [path]: cmd };
    try {
      localStorage.setItem(DEV_CMDS_KEY, JSON.stringify(devCmds));
    } catch {}
  }

  // Score a task by how likely it is the project's "start/dev" command.

  // Quick-pick shown when several equally-likely dev commands exist (e.g. a
  // Laravel app with both `composer run dev` and `npm run dev`). The choice is
  // remembered as the project's start command so the next ⌘R is instant.
  let startPick = $state({ open: false, x: 0, y: 0, items: [] });
  function openStartPicker(p, tasks) {
    const items = tasks.map((t) => ({
      label: `${t.command}`,
      icon: "\u25B6",
      action: () => {
        setDevCmd(p.path, t.command);
        runInProjectTerminal(p.path, t.command);
      },
    }));
    startPick = { open: true, x: Math.round(window.innerWidth / 2 - 150), y: 120, items };
  }

  function resolveProject(p) {
    if (p && p.path) return p;
    const path = typeof p === "string" ? p : activeTab?.projectPath;
    if (!path) return null;
    return projects.find((x) => x.path === path) ?? { path, name: baseOf(path) };
  }

  // Start a project's dev command: pinned override > best detected task > ask once.
  async function startProject(arg) {
    const p = resolveProject(arg ?? activeProject);
    if (!p) return;
    if (devCmds[p.path]) {
      runInProjectTerminal(p.path, devCmds[p.path]);
      return;
    }
    let tasks = [];
    try {
      tasks = await invoke("list_tasks", { path: p.path });
    } catch {}
    const scored = tasks
      .map((t) => ({ t, s: scoreDevTask(t) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s);
    if (scored.length > 0) {
      const top = scored[0].s;
      const ties = scored.filter((x) => x.s === top).map((x) => x.t);
      // Distinct top-scoring candidates (e.g. composer `dev` vs npm `dev`) -> ask.
      if (ties.length >= 2) openStartPicker(p, ties);
      else runInProjectTerminal(p.path, scored[0].t.command);
      return;
    }
    const cmd = (window.prompt(`No dev task found for ${p.name}. Command to start it:`, "npm run dev") || "").trim();
    if (cmd) {
      setDevCmd(p.path, cmd);
      runInProjectTerminal(p.path, cmd);
    }
  }

  // Pin/override the start command for a project.
  async function setStartCommand(arg) {
    const p = resolveProject(arg ?? activeProject);
    if (!p) return;
    let suggestion = devCmds[p.path] ?? "";
    if (!suggestion) {
      try {
        suggestion = rankDevTasks(await invoke("list_tasks", { path: p.path }))[0]?.command ?? "npm run dev";
      } catch {
        suggestion = "npm run dev";
      }
    }
    const cmd = (window.prompt(`Start command for ${p.name}:`, suggestion) || "").trim();
    if (cmd) setDevCmd(p.path, cmd);
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
    zoomed = false;
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
    list.push({ id: "act:new-tab", title: "New terminal tab", hint: "\u2318N", group: "action", icon: "\u002B", action: () => newTab(activeProject?.path ?? root, activeProject?.name) });
    list.push({ id: "act:open-runbook", title: "Open project runbook", hint: "", group: "action", icon: "\u{1F4D3}", action: () => openRunbook(activeProject?.path ?? root, activeProject?.name) });
    if (activeProject || activeTab?.projectPath) {
      list.push({ id: "act:start-project", title: `Start project (dev)${activeProject ? ": " + activeProject.name : ""}`, hint: "\u2318R", group: "action", icon: "\u25B6", action: () => startProject(activeProject) });
      list.push({ id: "act:set-start", title: "Set start command\u2026", group: "action", icon: "\u2699", action: () => setStartCommand(activeProject) });
    }
    if (elyraVersion)
      list.push({ id: "act:new-elyra", title: "New Elyra agent here", group: "action", icon: "\u{1F916}", action: () => newElyraAgent(activeProject?.path ?? root, activeProject?.name) });
    list.push({ id: "act:split-right", title: "Split right", hint: "\u2318D", group: "action", icon: "\u25A5", action: () => activeTermId && splitPane(activeTermId, "row") });
    list.push({ id: "act:split-down", title: "Split down", hint: "\u21E7\u2318D", group: "action", icon: "\u25A4", action: () => activeTermId && splitPane(activeTermId, "col") });
    list.push({ id: "act:close-pane", title: "Close pane", hint: "\u2318W", group: "action", icon: "\u00D7", action: () => activeTermId && closePane(activeTermId) });
    list.push({ id: "act:toggle-editor", title: showEditor ? "Hide editor" : "Show editor", group: "action", icon: "\u270E", action: () => (showEditor = !showEditor) });
    list.push({ id: "act:toggle-files", title: showFiles ? "Hide file sidebar" : "Show file sidebar", hint: "\u2318B", group: "action", icon: "\u{1F5C2}", action: () => (showFiles = !showFiles) });
    list.push({ id: "act:toggle-db", title: showDb ? "Hide database panel" : "Show database panel", hint: "\u2318T", group: "action", icon: "\u{1F5C4}", action: () => (showDb = !showDb) });
    list.push({ id: "act:ports", title: "Show listening ports", group: "action", icon: "\u26a1", action: () => (portsOpen = true) });
    list.push({ id: "act:timeline", title: "Command timeline", group: "action", icon: "\ud83d\udd58", action: () => (timelineOpen = true) });
    list.push({ id: "act:scrollback", title: "Search all terminals (scrollback)", hint: "\u21e7\u2318F", group: "action", icon: "\u{1F50E}", action: () => (scrollbackOpen = true) });
    list.push({ id: "act:zoom", title: zoomed ? "Unzoom pane" : "Zoom active pane", hint: "\u2318\u2325Z", group: "action", icon: "\u26f6", action: () => { if (activeTab?.kind === "term") zoomed = !zoomed; } });
    list.push({ id: "act:toggle-hidden", title: showHidden ? "Hide node_modules/.git in tree" : "Show all files in tree", group: "action", icon: "\u{1F441}", action: () => (showHidden = !showHidden) });
    list.push({ id: "act:toggle-theme", title: theme === "dark" ? "Switch to light theme" : "Switch to dark theme", group: "action", icon: theme === "dark" ? "\u2600" : "\u263D", action: () => (theme = theme === "dark" ? "light" : "dark") });
    list.push({ id: "act:toggle-notify", title: notifyOnFinish ? "Disable finished-command notifications" : "Notify when a background command finishes", group: "action", icon: "\u{1F514}", action: () => { notifyOnFinish = !notifyOnFinish; if (notifyOnFinish) ensureNotifyPermission(); } });
    list.push({ id: "act:toggle-shellint", title: shellIntegration ? "Disable shell integration (zsh)" : "Enable shell integration (zsh) \u2014 real commands & exit codes", hint: "new terminals", group: "action", icon: "\u{1F517}", action: () => (shellIntegration = !shellIntegration) });
    list.push({ id: "act:toggle-broadcast", title: broadcast ? "Stop broadcasting input" : "Broadcast input to all panes", group: "action", icon: "\u2301", action: () => (broadcast = !broadcast) });
    list.push({ id: "act:quick-edit", title: "Quick edit file\u2026", group: "action", icon: "\u270E", action: quickEdit });
    list.push({ id: "act:change-root", title: "Change projects folder\u2026", group: "action", icon: "\u{1F4C2}", action: changeRoot });
    if (activeProject?.is_git) {
      list.push({ id: "act:commit", title: `Git: commit ${activeProject.name}\u2026`, group: "action", icon: "\u2387", action: openCommit });
      list.push({ id: "act:git", title: `Git: open panel for ${activeProject.name}`, group: "action", icon: "\u2387", action: openGitPanel });
    }
    list.push({ id: "act:help", title: "Keyboard shortcuts", hint: "\u2318/", group: "action", icon: "?", action: () => (helpOpen = true) });
    list.push({ id: "act:about", title: "About Elyra Conductor", group: "action", icon: "\u2139", action: () => (aboutOpen = true) });
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
        if (editorRef?.closeActiveTab) editorRef.closeActiveTab();
        else showEditor = false;
      } else if (k === "p") {
        e.preventDefault();
        finderOpen = !finderOpen;
      } else if (k === "n") {
        e.preventDefault();
        newTab(activeProject?.path ?? root, activeProject?.name);
      }
      return;
    }

    // Pane navigation (⌘⌥Arrow) and zoom (⌘⌥Z).
    if (e.altKey && k.startsWith("arrow")) {
      e.preventDefault();
      navigatePane(k.slice(5));
      return;
    }
    if (e.altKey && k === "z") {
      e.preventDefault();
      if (activeTab?.kind === "term") zoomed = !zoomed;
      return;
    }
    if (k === "k") {
      e.preventDefault();
      paletteOpen = !paletteOpen;
    } else if (k === "p") {
      e.preventDefault();
      finderOpen = !finderOpen;
    } else if (k === "f" && e.shiftKey) {
      e.preventDefault();
      scrollbackOpen = true;
    } else if (k === "d") {
      e.preventDefault();
      if (activeTermId) splitPane(activeTermId, e.shiftKey ? "col" : "row");
    } else if (k === "w") {
      e.preventDefault();
      if (activeTermId) closePane(activeTermId);
    } else if (k === "r") {
      e.preventDefault();
      startProject(activeProject);
    } else if (k === "g") {
      e.preventDefault();
      openGitPanel();
    } else if (k === "t") {
      e.preventDefault();
      showDb = !showDb;
    } else if (k === "n") {
      e.preventDefault();
      newTab(activeProject?.path ?? root, activeProject?.name);
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
      notifyOnFinish,
      shellIntegration,
      activeTabIndex: tabs.findIndex((t) => t.id === activeTabId),
      tabs: tabs.filter((t) => t.kind !== "db").map((t) =>
        t.kind === "agent"
          ? { kind: "agent", title: t.title, projectPath: t.projectPath, cwd: t.cwd }
          : t.kind === "runbook"
            ? { kind: "runbook", title: t.title, projectPath: t.projectPath, file: t.file ?? null }
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

  // Synchronously persist the latest state. Svelte's onDestroy is not guaranteed
  // to run when the Tauri window closes, so we also flush on pagehide/beforeunload.
  function flushState() {
    if (!loaded) return;
    clearTimeout(saveTimer);
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(serialize()));
    } catch {}
  }

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
    notifyOnFinish = saved.notifyOnFinish ?? true;
    shellIntegration = saved.shellIntegration ?? false;

    if (Array.isArray(saved.tabs) && saved.tabs.length) {
      tabs = saved.tabs.map((t) =>
        t.kind === "agent"
          ? { id: nextId("tab"), kind: "agent", title: t.title, projectPath: t.projectPath, cwd: t.cwd, initialPrompt: null }
          : t.kind === "runbook"
            ? { id: nextId("tab"), kind: "runbook", title: t.title, projectPath: t.projectPath, file: t.file ?? null }
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
    fileDlg = {
      open: true, title: "Save workspace", message: "Save the current layout (tabs, panes, files) under a name.",
      input: true, value: activeWorkspace ?? "", placeholder: "workspace name", confirmLabel: "Save", danger: false,
      onconfirm: (name) => {
        workspaces = { ...workspaces, [name]: serialize() };
        activeWorkspace = name;
        persistWorkspaces();
      },
    };
  }

  // Workspaces menu (topbar).
  let wsMenu = $state({ open: false, x: 0, y: 0 });
  function openWsMenu(e) {
    const r = e.currentTarget.getBoundingClientRect();
    wsMenu = { open: true, x: r.left, y: r.bottom + 4 };
  }
  let wsItems = $derived.by(() => {
    const items = [{ label: "Save current layout\u2026", icon: "\u{1F4BE}", action: saveWorkspacePrompt }];
    const names = Object.keys(workspaces);
    if (names.length) {
      items.push({ separator: true });
      for (const name of names)
        items.push({ label: name === activeWorkspace ? `\u2713 ${name}` : name, icon: "\u{1F5C4}", action: () => loadWorkspace(name) });
      items.push({ separator: true });
      for (const name of names)
        items.push({ label: `Delete ${name}`, icon: "\u{1F5D1}", danger: true, action: () => deleteWorkspace(name) });
    }
    return items;
  });
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
    appFocused = true;
    refreshGitStatusThrottled();
  }
  function onWindowBlur() {
    appFocused = false;
  }

  let titleTimer = null;

  onMount(async () => {
    window.addEventListener("keydown", onGlobalKey);
    listen("menu://about", () => (aboutOpen = true));
    window.addEventListener("focus", onWindowFocus);
    window.addEventListener("blur", onWindowBlur);
    window.addEventListener("pagehide", flushState);
    window.addEventListener("beforeunload", flushState);
    titleTimer = setInterval(pollTitles, 1800);
    portsTimer = setInterval(() => { if (!document.hidden) { refreshProjectPorts(); refreshContainers(); } }, 5000);
    ensureNotifyPermission();
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
    loadDevCmds();
    restore();
    loadGitStatus(); // enrich pinned items resolved during restore
    refreshProjectPorts();
    refreshContainers();
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
    window.removeEventListener("blur", onWindowBlur);
    window.removeEventListener("pagehide", flushState);
    window.removeEventListener("beforeunload", flushState);
    flushState();
    clearInterval(titleTimer);
    clearInterval(portsTimer);
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
    onstart={(p) => startProject(p)}
    ports={projectPorts}
    containers={projectContainers}
    running={projectRunning}
    lastTest={lastTest}
    onopenport={openLocalPort}
    elyra={!!elyraVersion}
    onagent={(p) => newElyraAgent(p.path, p.name)}
  />

  <div class="main">
    <div class="toolbar">
      <button class="kbd-btn" onclick={() => (paletteOpen = true)}>Search <kbd>⌘K</kbd></button>
      <button onclick={quickEdit}>Quick edit</button>
      <span class="tsep"></span>
      <span class="tgroup">View</span>
      <button class:on={showFiles} title="Toggle file sidebar (⌘B)" onclick={() => (showFiles = !showFiles)}>Files</button>
      <button class:on={showDb} title="Toggle database explorer (⌘T)" onclick={() => (showDb = !showDb)}>DB</button>
      <button class:on={showEditor} title="Toggle editor" onclick={() => (showEditor = !showEditor)}>Editor</button>
      <span class="tsep"></span>
      <span class="tgroup">Tools</span>
      <button title="Listening ports" onclick={() => (portsOpen = true)}>⚡ Ports</button>
      <button title="Command timeline — what ran, where, how long" onclick={() => (timelineOpen = true)}>🕘 Timeline</button>
      {#if projectTasks.length}<button title="Run a project task (npm/composer/make/just)" onclick={() => (tasksOpen = true)}>☰ Tasks</button>{/if}
      {#if activeProject}<button title="View & edit .env (masked)" onclick={() => (envOpen = true)}>🔑 Env</button>{/if}
      {#if activeProject?.is_git}<button title="Git: stage, diff, branch, stash, commit (⌘G)" onclick={openGitPanel}>⎇ Git</button>{/if}
      <div class="tspacer"></div>
      <button class:on={broadcast} title="Broadcast input to all panes in this tab" onclick={() => (broadcast = !broadcast)}>⁁ Sync</button>
      <button title="Toggle theme" onclick={() => (theme = theme === "dark" ? "light" : "dark")}>{theme === "dark" ? "☀" : "☽"}</button>
      <button class:on={activeWorkspace} title="Workspaces — save & switch layouts" onclick={openWsMenu}>⬡ {activeWorkspace ?? "Layout"}</button>
      <button title="Keyboard shortcuts (⌘/)" onclick={() => (helpOpen = true)}>?</button>
    </div>

    <div class="tabbar">
      <div class="tabs">
        {#each tabs as t, i (t.id)}
          {@const proc = tabProc(t)}
          <div
            class="tab"
            class:active={t.id === activeTabId}
            class:running={!!proc}
            class:ring={activity[t.id]}
            class:drop-left={dragOver === i}
            class:drop-right={dragOver === tabs.length && i === tabs.length - 1}
            class:dragging={tabDrag && tabDrag.moved && tabDrag.fromIndex === i}
            style:border-left-color={projectColor(t.projectPath)}
            onpointerdown={(e) => tabPointerDown(e, i)}
          >
            {#if proc}<span class="run-dot" title={`Running ${proc}`}></span>{:else if activity[t.id]}<span class="ring-dot" title="New activity"></span>{/if}
            <span
              class="tab-label"
              role="button"
              tabindex="0"
              onclick={() => tabClick(t)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && focusTab(t)}
            >{tabTitle(t)}</span>
            {#if proc}<span class="tab-proc" title={`Running ${proc}`}>{proc}</span>{/if}
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
        <button class="new-tab" title="New tab (⌘N)" onclick={() => newTab(activeProject?.path ?? root, activeProject?.name)}>＋</button>
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
              <AgentPanel id={tab.id} cwd={tab.cwd} initialPrompt={tab.initialPrompt ?? null} initialDraft={tab.initialDraft ?? null} onactivity={() => markActivity(tab.id)} ontitle={(t) => (tab.title = t)} />
            </div>
          {:else if tab.kind === "db"}
            <div class="term-area" style:display={tab.id === activeTabId ? "block" : "none"}>
              <DBView
                connId={tab.connId}
                engine={tab.engine}
                mode={tab.view}
                table={tab.table}
                projectPath={tab.projectPath}
                {theme}
                ontitle={(t) => (tab.title = t)}
                elyra={!!elyraVersion}
                onelyra={(text) => sendToElyra(tab.projectPath ?? activeProject?.path ?? root, text)}
              />
            </div>
          {:else if tab.kind === "runbook"}
            <div class="term-area" style:display={tab.id === activeTabId ? "block" : "none"}>
              <RunbookPanel
                projectPath={tab.projectPath}
                initialFile={tab.file ?? null}
                {theme}
                ontitle={(t) => (tab.title = t)}
                onfile={(f) => (tab.file = f)}
                onrun={(cmd) => runInProjectTerminal(tab.projectPath, cmd)}
                onopenfile={(p) => openFile(p)}
                ontask={(label) => runRunbookTask(tab.projectPath, label)}
                elyra={!!elyraVersion}
                onelyra={(text) => sendToElyra(tab.projectPath, text)}
              />
            </div>
          {:else}
          {@const g = geometry(tab.root)}
          <div class="term-area" style:display={tab.id === activeTabId ? "block" : "none"}>
            {#each g.leaves as leaf (leaf.termId)}
              <div
                class="pane"
                class:active={leaf.termId === activeTermId && tab.id === activeTabId}
                style:left="{zoomed && tab.id === activeTabId ? 0 : leaf.rect.x}%"
                style:top="{zoomed && tab.id === activeTabId ? 0 : leaf.rect.y}%"
                style:width="{zoomed && tab.id === activeTabId ? 100 : leaf.rect.w}%"
                style:height="{zoomed && tab.id === activeTabId ? 100 : leaf.rect.h}%"
                style:display={zoomed && tab.id === activeTabId && leaf.termId !== activeTermId ? "none" : null}
                style:z-index={zoomed && leaf.termId === activeTermId ? 5 : null}
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
                <Terminal id={leaf.termId} cwd={leaf.cwd} {theme} persistKey={leaf.key} runCommand={leaf.runOnce ?? null} active={leaf.termId === activeTermId && tab.id === activeTabId} register={registerTerm} unregister={unregisterTerm} onactivity={() => markActivity(tab.id)} onuserinput={onPaneInput} shellIntegration={shellIntegration} oncommand={(rec) => onShellCommand(leaf.termId, rec)} />
              </div>
            {/each}

            {#if !(zoomed && tab.id === activeTabId)}
            {#each g.dividers as d (d.id)}
              {#if d.dir === "row"}
                <div class="divider row" style:left="{d.pos}%" style:top="{d.rect.y}%" style:height="{d.rect.h}%" role="separator" aria-orientation="vertical" onpointerdown={(e) => startDrag(e, d)}></div>
              {:else}
                <div class="divider col" style:top="{d.pos}%" style:left="{d.rect.x}%" style:width="{d.rect.w}%" role="separator" aria-orientation="horizontal" onpointerdown={(e) => startDrag(e, d)}></div>
              {/if}
            {/each}
            {/if}
          </div>
          {/if}
        {/each}
      </div>

      {#if showEditor}
        <div class="editor-area">
          {#if EditorComp}
            <EditorComp bind:this={editorRef} path={editorPath} gotoLine={pendingLine} {openSeq} {theme} onclose={() => (showEditor = false)} />
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
          refreshKey={fileRefresh}
          onmove={moveEntry}
        />
      {/if}

      {#if showDb}
        <DBPanel
          projectName={activeProject?.name ?? ""}
          conns={dbConns}
          error={dbError}
          ontoggle={dbToggleEntry}
          onconnect={dbConnectEntry}
          ondisconnect={dbDisconnectEntry}
          onremove={dbRemoveEntry}
          onrefresh={dbRefreshEntry}
          onopentable={openDbTable}
          onquery={openDbQuery}
          onaddenv={dbAddFromEnv}
          onaddmanual={dbAddConnection}
          onedit={dbEditConnection}
          ontest={dbTestConnection}
        />
      {/if}
    </div>
  </div>

  <CommandPalette open={paletteOpen} {commands} onclose={() => (paletteOpen = false)} />

  <FileFinder
    open={finderOpen}
    root={fileRoot}
    onopen={(p, line) => openFile(p, line)}
    onclose={() => (finderOpen = false)}
  />
  <PortsModal open={portsOpen} onclose={() => (portsOpen = false)} />
  <ScrollbackSearch open={scrollbackOpen} onsearch={searchScrollback} onjump={jumpToMatch} onclose={() => (scrollbackOpen = false)} />

  <ContextMenu
    open={ctx.open}
    x={ctx.x}
    y={ctx.y}
    items={ctxItems}
    onclose={() => (ctx = { ...ctx, open: false })}
  />

  <InputDialog
    open={fileDlg.open}
    title={fileDlg.title}
    message={fileDlg.message}
    input={fileDlg.input}
    value={fileDlg.value}
    placeholder={fileDlg.placeholder}
    confirmLabel={fileDlg.confirmLabel}
    danger={fileDlg.danger}
    onconfirm={fileDlg.onconfirm}
    onclose={closeFileDlg}
  />

  <ContextMenu
    open={startPick.open}
    x={startPick.x}
    y={startPick.y}
    items={startPick.items}
    onclose={() => (startPick = { ...startPick, open: false })}
  />

  <RunModal
    open={runModal.open}
    cwd={runModal.cwd}
    command={runModal.command}
    title={runModal.title}
    onclose={() => (runModal = { ...runModal, open: false })}
  />

  <GitPanel
    open={gitPanel.open}
    path={gitPanel.path}
    projectName={gitPanel.name}
    onclose={() => (gitPanel = { ...gitPanel, open: false })}
    onchanged={() => loadGitStatus()}
  />

  <ContextMenu open={wsMenu.open} x={wsMenu.x} y={wsMenu.y} items={wsItems} onclose={() => (wsMenu = { ...wsMenu, open: false })} />

  <TasksModal open={tasksOpen} tasks={projectTasks} projectName={activeProject?.name ?? ""} onrun={runTask} onclose={() => (tasksOpen = false)} />

  <EnvModal open={envOpen} path={activeProject?.path ?? ""} projectName={activeProject?.name ?? ""} onclose={() => (envOpen = false)} />

  <TimelineModal open={timelineOpen} entries={commandLog} onjump={jumpToCommand} onclear={() => (commandLog = [])} onclose={() => (timelineOpen = false)} />

  <CommitDialog
    open={commit.open}
    path={commit.path}
    projectName={commit.name}
    onclose={() => (commit = { ...commit, open: false })}
    oncommitted={() => loadProjects()}
  />

  <ShortcutsModal open={helpOpen} onclose={() => (helpOpen = false)} />

  <AboutModal open={aboutOpen} onclose={() => (aboutOpen = false)} />

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
  .toolbar {
    display: flex; align-items: center; height: 38px;
    background: var(--bg-2); border-bottom: 1px solid var(--border);
    padding: 0 8px; gap: 6px; overflow-x: auto;
  }
  .toolbar button { background: var(--bg-3); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 10px; font-size: 12px; flex: none; white-space: nowrap; }
  .toolbar button.on { border-color: var(--accent); color: var(--accent); }
  .tsep { width: 1px; height: 20px; background: var(--border); flex: none; margin: 0 2px; }
  .tgroup { font-size: 10px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; flex: none; }
  .tspacer { margin-left: auto; }
  .tabbar {
    display: flex; align-items: center; height: 34px;
    background: var(--bg); border-bottom: 1px solid var(--border);
    padding: 0 8px;
  }
  .tabs { display: flex; align-items: center; gap: 4px; overflow-x: auto; flex: 1; }
  .tab { display: flex; align-items: center; background: var(--bg-3); border: 1px solid transparent; border-left-width: 3px; border-radius: 6px; padding: 0 2px 0 4px; flex: 0 0 auto; }
  .tab.active { border-color: var(--accent); }
  .tab.ring { border-color: var(--green); }
  .tab.drop-left { box-shadow: inset 2px 0 0 0 var(--accent); }
  .tab.drop-right { box-shadow: inset -2px 0 0 0 var(--accent); }
  .tab { cursor: grab; }
  .tab:active { cursor: grabbing; }
  .tab.dragging { opacity: 0.5; }
  .ring-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--green); margin-left: 6px; flex: none; animation: ring-pulse 1.2s ease-in-out infinite; }
  @keyframes ring-pulse { 0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(158, 206, 106, 0.6); } 50% { opacity: 0.5; box-shadow: 0 0 0 4px rgba(158, 206, 106, 0); } }
  /* Running marker: a tab actively running a foreground command (npm run dev, etc). */
  .run-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); margin-left: 6px; flex: none; animation: run-spin 1s ease-in-out infinite; box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 30%, transparent); }
  @keyframes run-spin { 0% { transform: scale(0.7); opacity: 0.6; } 50% { transform: scale(1); opacity: 1; } 100% { transform: scale(0.7); opacity: 0.6; } }
  .tab-proc { font-family: var(--font-mono); font-size: 10px; color: var(--text-dim); background: var(--bg); border: 1px solid var(--border); border-radius: 4px; padding: 0 5px; margin-left: 4px; max-width: 90px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: none; }
  .tab.running { border-color: color-mix(in srgb, var(--accent) 60%, transparent); }
  .tab-label { display: inline-block; background: transparent; border: none; color: var(--text); padding: 4px 6px; font-size: 12px; max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; -webkit-user-drag: none; user-select: none; }
  .tab-x { display: inline-block; background: transparent; border: none; color: var(--text-dim); padding: 0 4px; font-size: 14px; cursor: pointer; -webkit-user-drag: none; user-select: none; }
  .tab-x:hover { color: var(--text); }
  .tab { -webkit-user-select: none; user-select: none; }
  .tab-x:hover { color: var(--text); }
  .new-tab { background: transparent; border: none; color: var(--text-dim); font-size: 16px; padding: 0 6px; }
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
