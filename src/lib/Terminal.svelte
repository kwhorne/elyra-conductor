<script>
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { SerializeAddon } from "@xterm/addon-serialize";
  import "@xterm/xterm/css/xterm.css";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let { id, cwd, runCommand = null, onexit = null, onactivity = null, onuserinput = null, persistKey = null, theme = "dark", active = false, register = null, unregister = null, shellIntegration = false, oncommand = null } = $props();

  // Focus the xterm when this pane becomes active (e.g. via keyboard pane-nav).
  $effect(() => {
    if (active && term) term.focus();
  });

  // Scrollback persistence. A pty can't be revived across restarts, so we only
  // restore the *visual* history (read-only) and start a fresh shell beneath it.
  // svelte-ignore state_referenced_locally -- persistKey is fixed for a pane's lifetime
  const SB_KEY = persistKey ? `conductor:sb:${persistKey}` : null;
  const SB_MAX = 60000; // cap stored bytes per pane to stay well under quota
  let serializeAddon;
  let sbTimer;
  let sbDirty = false; // true when new output arrived since the last save
  function saveScrollback() {
    if (!SB_KEY || !serializeAddon || !sbDirty) return; // skip when unchanged
    try {
      let data = serializeAddon.serialize({ scrollback: 1000 });
      if (data.length > SB_MAX) data = data.slice(data.length - SB_MAX);
      if (data.trim()) localStorage.setItem(SB_KEY, data);
      sbDirty = false;
    } catch {}
  }

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
  let searchInput = $state();

  const SEARCH_OPTS = { decorations: { matchOverviewRuler: "#7aa2f7", activeMatchColorOverviewRuler: "#e0af68" } };

  // Plain-text snapshot of the whole buffer (scrollback + viewport), for global search.
  function getLines() {
    if (!term) return [];
    const buf = term.buffer.active;
    const out = [];
    for (let i = 0; i < buf.length; i++) {
      out.push(buf.getLine(i)?.translateToString(true) ?? "");
    }
    return out;
  }
  function findInTerm(query) {
    if (search && query) search.findNext(query, SEARCH_OPTS);
    term?.focus();
  }

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
    serializeAddon = new SerializeAddon();
    term.loadAddon(serializeAddon);
    term.open(el);

    // Shell-integration OSC sequences (emitted by the zsh shim): 633;E;<cmdline>,
    // 133;C (output start), 133;D[;exit] (done). We turn these into command
    // records with the real command line and exit code.
    let scCmd = null;
    let scStart = 0;
    let scActive = false;
    let scStartRow = 0;
    // Read the rendered output of the command (between the C and D markers) from
    // the terminal buffer — clean text, no ANSI. Capped so bundles stay small.
    function captureOutput(endRow) {
      try {
        const buf = term.buffer.active;
        const from = Math.max(0, scStartRow);
        const to = Math.min(endRow, buf.length - 1);
        const lines = [];
        for (let r = Math.max(from, to - 200); r <= to; r++) {
          const line = buf.getLine(r);
          if (line) lines.push(line.translateToString(true));
        }
        return lines.join("\n").replace(/\n{3,}/g, "\n\n").replace(/^\n+|\n+$/g, "").slice(-4000);
      } catch {
        return "";
      }
    }
    term.parser.registerOscHandler(633, (data) => {
      if (data.startsWith("E;")) scCmd = data.slice(2);
      return true;
    });
    term.parser.registerOscHandler(133, (data) => {
      if (data[0] === "C") {
        scStart = Date.now();
        scActive = true;
        const buf = term.buffer.active;
        scStartRow = buf.baseY + buf.cursorY;
      } else if (data[0] === "D") {
        if (scActive) {
          const m = data.match(/^D;?(\d+)?/);
          const exitCode = m && m[1] != null ? Number(m[1]) : null;
          const buf = term.buffer.active;
          const output = captureOutput(buf.baseY + buf.cursorY);
          oncommand?.({ command: scCmd, exitCode, startedAt: scStart, duration: Date.now() - scStart, output });
        }
        scActive = false;
        scCmd = null;
      }
      return true;
    });

    register?.(id, {
      getLines,
      find: findInTerm,
      focus: () => term?.focus(),
      // Re-measure and resize the pty to the current container size. Used when
      // the surrounding layout changes (editor/files/db panels open or close),
      // where ResizeObserver doesn't always fire reliably in WebKit.
      fit: () => {
        try {
          fit?.fit();
          invoke("pty_resize", { id, cols: term.cols, rows: term.rows }).catch(() => {});
        } catch {}
      },
    });

    // Intercept Cmd/Ctrl+F to open the in-terminal search instead of the shell.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type === "keydown" && (e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === "f") {
        openSearch();
        return false;
      }
      // xterm.js doesn't speak the Kitty keyboard protocol, so modified Enter
      // (shift/alt/ctrl) collapses to a bare CR and TUIs can't tell it apart
      // from a plain Enter. Emit the Kitty CSI-u sequence ourselves so apps
      // like the Elyra CLI receive shift+enter as a real "new line".
      //
      // We must handle BOTH keydown and keypress: returning false from the
      // keydown handler makes xterm bail without calling preventDefault(), so
      // the browser still fires a keypress for Enter (charCode 13) which xterm
      // would otherwise turn into a plain CR. Send the sequence once on
      // keydown, and suppress every related event so no stray \r leaks through.
      if (e.key === "Enter") {
        // Kitty modifier value is a 1-based bitmask: shift=1, alt=2, ctrl=4.
        let mod = 1;
        if (e.shiftKey) mod += 1;
        if (e.altKey) mod += 2;
        if (e.ctrlKey) mod += 4;
        if (mod > 1) {
          if (e.type === "keydown") {
            e.preventDefault();
            invoke("pty_write", { id, data: `\x1b[13;${mod}u` }).catch(() => {});
          }
          return false;
        }
      }
      return true;
    });
    // Wait for the bundled font so xterm measures glyphs with correct metrics.
    try {
      await document.fonts.load('13px "JetBrains Mono"');
      await document.fonts.ready;
    } catch {}
    fit.fit();

    // PTY output streams over a binary Channel (raw ArrayBuffer) instead of a
    // Tauri event. Events JSON-serialize the payload — a 3.6x size hit plus a
    // JSON.parse per frame on the UI thread, which made repaint-heavy TUIs
    // (e.g. the Elyra agent) feel sluggish. The Channel ships bytes directly,
    // and `new Uint8Array(buffer)` is a zero-copy view.
    /** @type {Channel<ArrayBuffer>} */
    const onData = new Channel();
    onData.onmessage = (msg) => {
      term.write(new Uint8Array(msg));
      sbDirty = true; // mark scrollback for the next periodic save
      const now = Date.now();
      if (onactivity && now - lastActivity > 350) {
        lastActivity = now;
        onactivity();
      }
    };
    const unExit = await listen(`pty://exit/${id}`, (e) => {
      const code = typeof e.payload === "number" ? e.payload : null;
      const tail = code != null && code >= 0 ? ` (code ${code})` : "";
      term.write(`\r\n\x1b[90m[process exited${tail}]\x1b[0m\r\n`);
      onexit?.(code);
    });
    cleanup.push(unExit);

    term.onData((d) => {
      invoke("pty_write", { id, data: d }).catch(() => {});
      onuserinput?.(id, d);
    });

    // Replay last session's scrollback as read-only history before the new
    // shell starts, so context isn't lost across restarts.
    if (SB_KEY) {
      try {
        const prev = localStorage.getItem(SB_KEY);
        if (prev) {
          term.write(prev);
          term.write("\r\n\x1b[90m\u2500\u2500 previous session (restored) \u2500\u2500\x1b[0m\r\n");
        }
      } catch {}
    }

    // The command (if any) is run by the shell itself at startup (see pty.rs),
    // which avoids the race of typing into a not-yet-ready interactive shell.
    await invoke("pty_spawn", {
      id,
      cwd,
      cols: term.cols,
      rows: term.rows,
      runCommand: runCommand ?? null,
      shellIntegration: shellIntegration ?? false,
      onData,
    });

    // Periodically snapshot the buffer so a hard window close still persists it.
    if (SB_KEY) {
      sbTimer = setInterval(saveScrollback, 4000);
      // onDestroy may not run on app quit; flush on pagehide/beforeunload too.
      window.addEventListener("pagehide", saveScrollback);
      window.addEventListener("beforeunload", saveScrollback);
    }


    const ro = new ResizeObserver(() => {
      fit.fit();
      invoke("pty_resize", { id, cols: term.cols, rows: term.rows }).catch(() => {});
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
    unregister?.(id);
    clearInterval(sbTimer);
    window.removeEventListener("pagehide", saveScrollback);
    window.removeEventListener("beforeunload", saveScrollback);
    saveScrollback();
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
