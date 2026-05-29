<script>
  import { onMount, onDestroy } from "svelte";
  import * as monaco from "monaco-editor";
  import "./monaco-setup.js";
  import { invoke } from "@tauri-apps/api/core";

  let { path } = $props();

  let el;
  let editor;
  let currentPath = $state(null);
  let dirty = $state(false);
  let status = $state("");

  async function openFile(p) {
    if (!editor || !p) return;
    try {
      const content = await invoke("read_file", { path: p });
      const uri = monaco.Uri.file(p);
      let model = monaco.editor.getModel(uri);
      if (model) {
        model.setValue(content);
      } else {
        model = monaco.editor.createModel(content, undefined, uri);
      }
      editor.setModel(model);
      currentPath = p;
      dirty = false;
      status = "";
    } catch (e) {
      status = `Error: ${e}`;
    }
  }

  async function save() {
    if (!editor || !currentPath) return;
    try {
      await invoke("write_file", { path: currentPath, content: editor.getValue() });
      dirty = false;
      status = "Saved";
      setTimeout(() => (status = ""), 1500);
    } catch (e) {
      status = `Error: ${e}`;
    }
  }

  onMount(() => {
    editor = monaco.editor.create(el, {
      theme: "vs-dark",
      automaticLayout: true,
      fontSize: 13,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });
    editor.onDidChangeModelContent(() => (dirty = true));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, save);
    if (path) openFile(path);
  });

  // React to path prop changes
  $effect(() => {
    if (editor && path && path !== currentPath) openFile(path);
  });

  onDestroy(() => editor?.dispose());

  function basename(p) {
    return p ? p.split("/").pop() : "No file";
  }
</script>

<div class="editor-wrap">
  <div class="editor-bar">
    <span class="fname">{basename(currentPath)}{dirty ? " •" : ""}</span>
    <span class="status">{status}</span>
    <button onclick={save} disabled={!currentPath}>Save ⌘S</button>
  </div>
  <div class="monaco" bind:this={el}></div>
</div>

<style>
  .editor-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e1e;
  }
  .editor-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px;
    background: var(--bg-3);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .fname {
    font-family: var(--font-mono);
    color: var(--text);
  }
  .status {
    color: var(--green);
    margin-left: auto;
  }
  .editor-bar button {
    background: var(--accent-2);
    color: var(--text);
    border: none;
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 11px;
  }
  .editor-bar button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .monaco {
    flex: 1;
    min-height: 0;
  }
</style>
