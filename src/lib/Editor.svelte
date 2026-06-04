<script>
  import { onMount, onDestroy, untrack } from "svelte";
  import * as monaco from "monaco-editor";
  import "./monaco-setup.js";
  import { invoke } from "@tauri-apps/api/core";

  let { path, gotoLine = null, openSeq = 0, onclose, theme = "dark" } = $props();

  let el;
  let editor;
  let openTabs = $state([]); // [{ path, dirty }]
  let activePath = $state(null);
  let status = $state("");
  let loadingModel = false; // guard so programmatic setValue doesn't mark dirty
  const viewStates = new Map(); // path -> Monaco view state

  function tabFor(p) {
    return openTabs.find((t) => t.path === p);
  }

  async function openTab(p, line) {
    if (!editor || !p) return;
    // Stash the current tab's scroll/cursor before switching away.
    if (activePath) viewStates.set(activePath, editor.saveViewState());
    try {
      const uri = monaco.Uri.file(p);
      let model = monaco.editor.getModel(uri);
      if (!model) {
        const content = await invoke("read_file", { path: p });
        // Re-check after awaiting: a concurrent openTab (e.g. onMount + the
        // openSeq effect both firing for the initial file) may have created the
        // model while we were reading, which would make createModel throw.
        model = monaco.editor.getModel(uri) ?? monaco.editor.createModel(content, undefined, uri);
      }
      loadingModel = true;
      editor.setModel(model);
      loadingModel = false;

      if (!tabFor(p)) openTabs = [...openTabs, { path: p, dirty: false }];
      activePath = p;
      status = "";

      const vs = viewStates.get(p);
      if (vs && !line) editor.restoreViewState(vs);
      if (line) {
        editor.revealLineInCenter(line);
        editor.setPosition({ lineNumber: line, column: 1 });
      }
      editor.focus();
    } catch (e) {
      status = `Error: ${e}`;
    }
  }

  function switchTo(p) {
    if (p !== activePath) openTab(p, null);
  }

  async function closeTab(p) {
    const t = tabFor(p);
    if (!t) return;
    if (t.dirty && !confirm(`Discard unsaved changes to ${basename(p)}?`)) return;
    const uri = monaco.Uri.file(p);
    monaco.editor.getModel(uri)?.dispose();
    viewStates.delete(p);
    const idx = openTabs.findIndex((x) => x.path === p);
    openTabs = openTabs.filter((x) => x.path !== p);
    if (openTabs.length === 0) {
      activePath = null;
      onclose?.();
      return;
    }
    if (p === activePath) {
      const next = openTabs[Math.min(idx, openTabs.length - 1)];
      openTab(next.path, null);
    }
  }

  // Exposed to the parent so ⌘W can close the active tab.
  export function closeActiveTab() {
    if (activePath) closeTab(activePath);
    else onclose?.();
  }

  async function save() {
    if (!editor || !activePath) return;
    try {
      // Format-on-save when the language has a formatter (JSON/JS/TS/CSS/HTML/…).
      try {
        await editor.getAction("editor.action.formatDocument")?.run();
      } catch {}
      await invoke("write_file", { path: activePath, content: editor.getValue() });
      const t = tabFor(activePath);
      if (t) t.dirty = false;
      openTabs = [...openTabs];
      status = "Saved";
      setTimeout(() => (status = ""), 1500);
    } catch (e) {
      status = `Error: ${e}`;
    }
  }

  onMount(() => {
    editor = monaco.editor.create(el, {
      theme: theme === "light" ? "vs" : "vs-dark",
      automaticLayout: true,
      fontSize: 13,
      fontFamily: '"JetBrains Mono", "SF Mono", Menlo, monospace',
      fontLigatures: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });
    editor.onDidChangeModelContent(() => {
      if (loadingModel) return;
      const t = tabFor(activePath);
      if (t && !t.dirty) {
        t.dirty = true;
        openTabs = [...openTabs];
      }
    });
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, save);
    if (path) openTab(path, gotoLine);
  });

  // Open/switch whenever the parent requests a file (openSeq bumps each time,
  // so reopening the same file at a new line still works).
  $effect(() => {
    openSeq;
    untrack(() => {
      if (editor && path) openTab(path, gotoLine);
    });
  });

  // Monaco theme is global.
  $effect(() => {
    monaco.editor.setTheme(theme === "light" ? "vs" : "vs-dark");
  });

  onDestroy(() => {
    editor?.dispose();
    for (const t of openTabs) monaco.editor.getModel(monaco.Uri.file(t.path))?.dispose();
  });

  function basename(p) {
    return p ? p.split("/").pop() : "No file";
  }
</script>

<div class="editor-wrap">
  <div class="tabbar">
    {#each openTabs as t (t.path)}
      <div class="tab" class:on={t.path === activePath}>
        <button class="tname" title={t.path} onclick={() => switchTo(t.path)}>
          {basename(t.path)}{#if t.dirty}<span class="dot">•</span>{/if}
        </button>
        <button class="x" title="Close (⌘W)" onclick={() => closeTab(t.path)}>✕</button>
      </div>
    {/each}
    <span class="status">{status}</span>
    <button class="save" onclick={save} disabled={!activePath} title="Save (⌘S) — formats on save">Save ⌘S</button>
    <button class="closeall" title="Hide editor" onclick={() => onclose?.()}>⤓</button>
  </div>
  <div class="monaco" bind:this={el}></div>
</div>

<style>
  .editor-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-2);
  }
  .tabbar {
    display: flex;
    align-items: stretch;
    gap: 2px;
    padding: 3px 6px;
    background: var(--bg-3);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    border-radius: 6px;
    background: transparent;
    flex: none;
  }
  .tab.on {
    background: var(--bg-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .tname {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 4px 4px 4px 9px;
    cursor: pointer;
    white-space: nowrap;
  }
  .tab.on .tname {
    color: var(--text);
  }
  .dot {
    color: var(--accent);
    margin-left: 5px;
  }
  .x {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 10px;
    padding: 4px 8px 4px 2px;
    cursor: pointer;
    border-radius: 0 6px 6px 0;
  }
  .x:hover {
    color: var(--text);
  }
  .status {
    color: var(--green);
    margin-left: auto;
    align-self: center;
    font-size: 11px;
  }
  .save,
  .closeall {
    background: var(--accent-2);
    color: var(--text);
    border: none;
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 11px;
    align-self: center;
    margin-left: 6px;
    cursor: pointer;
  }
  .status + .save {
    margin-left: 10px;
  }
  .save:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .closeall {
    background: transparent;
    border: 1px solid var(--border);
    margin-left: 4px;
  }
  .closeall:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .monaco {
    flex: 1;
    min-height: 0;
  }
</style>
