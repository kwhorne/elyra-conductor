<script>
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
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
  let cleanup = [];

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
    term.open(el);
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

<div class="term" bind:this={el}></div>

<style>
  .term {
    width: 100%;
    height: 100%;
    padding: 6px 8px;
    background: var(--bg-2);
  }
</style>
