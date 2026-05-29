<script>
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let { id, cwd, runCommand = null, onexit = null } = $props();

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
      theme: {
        background: "#1a1b26",
        foreground: "#c0caf5",
        cursor: "#c0caf5",
        selectionBackground: "#2f3650",
      },
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
    });
    const unExit = await listen(`pty://exit/${id}`, () => {
      term.write("\r\n\x1b[90m[process exited]\x1b[0m\r\n");
      onexit?.();
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
    background: #1a1b26;
  }
</style>
