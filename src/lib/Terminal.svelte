<script>
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let { id, cwd } = $props();

  let el;
  let term;
  let fit;
  let cleanup = [];

  onMount(async () => {
    term = new Terminal({
      fontSize: 13,
      fontFamily: 'var(--font-mono), Menlo, monospace',
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
    fit.fit();

    const unData = await listen(`pty://data/${id}`, (e) => {
      term.write(new Uint8Array(e.payload));
    });
    const unExit = await listen(`pty://exit/${id}`, () => {
      term.write("\r\n\x1b[90m[process exited]\x1b[0m\r\n");
    });
    cleanup.push(unData, unExit);

    term.onData((d) => invoke("pty_write", { id, data: d }));

    await invoke("pty_spawn", {
      id,
      cwd,
      cols: term.cols,
      rows: term.rows,
    });

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
