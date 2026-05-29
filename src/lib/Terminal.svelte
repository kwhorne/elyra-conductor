<script>
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import "@xterm/xterm/css/xterm.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let { id, cwd, runCommand = null, onexit = null, onactivity = null, theme = "dark" } = $props();

  let lastActivity = 0;

  const THEMES = {
    dark: { background: "#1a1b26", foreground: "#c0caf5", cursor: "#c0caf5", selectionBackground: "#2f3650" },
    light: { background: "#ffffff", foreground: "#2a2e3a", cursor: "#2a2e3a", selectionBackground: "#dbe4f7" },
  };

  let el;
  let term;
  let fit;
  let search;
  let cleanup = [];

  let searchOpen = $state(false);
  let searchTerm = $state("");
  let searchInput;

  const SEARCH_OPTS = { decorations: { matchOverviewRuler: "#7aa2f7", activeMatchColorOverviewRuler: "#e0af68" } };

  function openSearch() {
    searchOpen = true;
    queueMicrotask(() => searchInput?.select());
  }
  function closeSearch() {
    searchOpen = false;
    search?.clearDecorations?.();
    term?.focus();
  }
  function findNext() {
    if (searchTerm) search?.findNext(searchTerm, SEARCH_OPTS);
  }
  function findPrev() {
    if (searchTerm) search?.findPrevious(searchTerm, SEARCH_OPTS);
  }
  function onSearchKey(e) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.shiftKey ? findPrev() : findNext();
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeSearch();
    }
  }

  onMount(async () => {
    term = new Terminal({
      fontSize: 13,
      fontFamily: '"JetBrains Mono", "SF Mono", Menlo, monospace',
      cursorBlink: true,
      allowProposedApi: true,
      theme: THEMES[theme] ?? THEMES.dark,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    search = new SearchAddon();
    term.loadAddon(search);
    term.open(el);

    // Intercept Cmd/Ctrl+F to open the in-terminal search instead of the shell.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type === "keydown" && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        openSearch();
        return false;
      }
      return true;
    });
    // Wait for the bundled font so xterm measures glyphs with correct metrics.
    try {
      await document.fonts.load('13px "JetBrains Mono"');
      await document.fonts.ready;
    } catch {}
    fit.fit();

    const unData = await listen(`pty://data/${id}`, (e) => {
      term.write(new Uint8Array(e.payload));
      const now = Date.now();
      if (onactivity && now - lastActivity > 350) {
        lastActivity = now;
        onactivity();
      }
    });
    const unExit = await listen(`pty://exit/${id}`, (e) => {
      const code = typeof e.payload === "number" ? e.payload : null;
      const tail = code != null && code >= 0 ? ` (code ${code})` : "";
      term.write(`\r\n\x1b[90m[process exited${tail}]\x1b[0m\r\n`);
      onexit?.(code);
    });
    cleanup.push(unData, unExit);

    term.onData((d) => invoke("pty_write", { id, data: d }));

    await invoke("pty_spawn", {
      id,
      cwd,
      cols: term.cols,
      rows: term.rows,
    });

    // Optionally auto-run a command (e.g. ./deploy.sh) once the shell is up.
    if (runCommand) {
      try {
        await invoke("pty_write", { id, data: runCommand + "\r" });
      } catch {}
    }

    const ro = new ResizeObserver(() => {
      fit.fit();
      invoke("pty_resize", { id, cols: term.cols, rows: term.rows });
    });
    ro.observe(el);
    cleanup.push(() => ro.disconnect());

    term.focus();
  });

  // React to theme changes after mount.
  $effect(() => {
    if (term) term.options.theme = THEMES[theme] ?? THEMES.dark;
  });

  onDestroy(() => {
    cleanup.forEach((fn) => fn?.());
    invoke("pty_kill", { id }).catch(() => {});
    term?.dispose();
  });
</script>

<div class="term-wrap">
  {#if searchOpen}
    <div class="search">
      <input
        bind:this={searchInput}
        bind:value={searchTerm}
        oninput={findNext}
        onkeydown={onSearchKey}
        placeholder="Find…"
        spellcheck="false"
      />
      <button title="Previous (⇧⏎)" onclick={findPrev}>↑</button>
      <button title="Next (⏎)" onclick={findNext}>↓</button>
      <button title="Close (Esc)" onclick={closeSearch}>✕</button>
    </div>
  {/if}
  <div class="term" bind:this={el}></div>
</div>

<style>
  .term-wrap {
    position: relative;
    width: 100%;
    height: 100%;
  }
  .term {
    width: 100%;
    height: 100%;
    padding: 6px 8px;
    background: var(--bg-2);
  }
  .search {
    position: absolute;
    top: 6px;
    right: 10px;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 3px 4px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
  }
  .search input {
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 12px;
    width: 160px;
    padding: 2px 6px;
  }
  .search button {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 12px;
    padding: 2px 5px;
    border-radius: 4px;
  }
  .search button:hover {
    color: var(--text);
    background: var(--accent-2);
  }
</style>
