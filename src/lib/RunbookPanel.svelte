<script>
  import { invoke } from "@tauri-apps/api/core";
  import { marked } from "marked";

  let {
    projectPath,
    initialFile = null,
    ontitle = null,
    onfile = null,
    onrun = null,
    onopenfile = null,
    ontask = null,
    onelyra = null,
    elyra = false,
    theme = "dark",
  } = $props();

  let files = $state([]); // { name, path }
  // svelte-ignore state_referenced_locally -- initialFile is only the starting value by design
  let current = $state(initialFile); // path of the open runbook
  let content = $state("");
  let draft = $state("");
  let mode = $state("preview"); // "preview" | "edit"
  let loading = $state(true);
  let saved = $state(true);
  let error = $state(null);

  const SHELL_LANGS = new Set(["", "sh", "bash", "shell", "zsh", "console", "shell-session", "term", "terminal"]);

  const DEFAULT_TEMPLATE = `# Runbook

A living, runnable checklist for this project. Shell code blocks get a **▶ Run**
button that sends the command to this project's terminal.

## Setup

\`\`\`bash
pnpm install
\`\`\`

## Dev

\`\`\`bash
pnpm dev
\`\`\`

## Notes

- Edit this file with **Edit**; it's saved to \`.conductor/notes/\`.
`;

  // Split markdown into top-level tokens so shell code fences can render as
  // native Svelte blocks (with Run/Copy), while everything else is rendered
  // through marked. Keeps interactivity without dangerouslySetInnerHTML hacks.
  // Turn Obsidian-style [[path]] or [[path|label]] into clickable links that
  // open the file in the editor. Skips fenced code so commands stay literal.
  // Relative paths resolve against the project root.
  function transformWikiLinks(md) {
    const re = /\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]/g;
    let inFence = false;
    return md
      .split("\n")
      .map((line) => {
        const t = line.trimStart();
        if (t.startsWith("```") || t.startsWith("~~~")) {
          inFence = !inFence;
          return line;
        }
        if (inFence) return line;
        return line.replace(re, (_, target, label) => {
          const tgt = target.trim();
          // [[task:dev]] runs a discovered project task; everything else opens a file.
          if (/^task:/i.test(tgt)) {
            const name = tgt.slice(tgt.indexOf(":") + 1).trim();
            const text = (label ?? name).trim();
            return `[${text}](ctask:${encodeURIComponent(name)})`;
          }
          const text = (label ?? tgt).trim();
          return `[${text}](cfile:${encodeURIComponent(tgt)})`;
        });
      })
      .join("\n");
  }

  function onPreviewClick(e) {
    const a = e.target.closest?.("a");
    if (!a) return;
    const href = a.getAttribute("href") || "";
    if (href.startsWith("cfile:")) {
      e.preventDefault();
      const target = decodeURIComponent(href.slice("cfile:".length));
      const abs = target.startsWith("/") ? target : `${projectPath}/${target}`;
      onopenfile?.(abs);
    } else if (href.startsWith("ctask:")) {
      e.preventDefault();
      ontask?.(decodeURIComponent(href.slice("ctask:".length)));
    }
  }

  let blocks = $derived.by(() => {
    if (!content) return [];
    let tokens;
    try {
      tokens = marked.lexer(transformWikiLinks(content));
    } catch {
      return [{ type: "html", html: "<p>Could not parse markdown.</p>" }];
    }
    const out = [];
    for (const tok of tokens) {
      if (tok.type === "code") {
        const lang = (tok.lang || "").trim().toLowerCase().split(/\s+/)[0] ?? "";
        out.push({ type: "code", lang: tok.lang || "", code: tok.text, runnable: SHELL_LANGS.has(lang) });
      } else {
        let html = "";
        try {
          html = marked.parser([tok]);
        } catch {
          html = "";
        }
        out.push({ type: "html", html });
      }
    }
    return out;
  });

  async function loadList() {
    try {
      files = await invoke("list_runbooks", { project: projectPath });
    } catch (e) {
      files = [];
    }
  }

  async function open(path) {
    loading = true;
    error = null;
    current = path;
    onfile?.(path);
    try {
      content = await invoke("read_file", { path });
    } catch (e) {
      content = "";
      error = String(e);
    }
    draft = content;
    saved = true;
    mode = "preview";
    loading = false;
    ontitle?.(titleOf(path));
    loadVerify(path);
  }

  function titleOf(path) {
    return (path?.split("/").pop() ?? "runbook").replace(/\.md$/i, "");
  }

  async function createNew() {
    const name = (window.prompt("New runbook name:", "runbook") || "").trim();
    if (!name) return;
    const file = name.toLowerCase().endsWith(".md") ? name : `${name}.md`;
    const path = `${projectPath}/.conductor/notes/${file}`;
    try {
      await invoke("write_file", { path, content: DEFAULT_TEMPLATE });
    } catch (e) {
      error = String(e);
      return;
    }
    await loadList();
    await open(path);
    mode = "edit";
  }

  async function save() {
    if (!current) return;
    try {
      await invoke("write_file", { path: current, content: draft });
      content = draft;
      saved = true;
      mode = "preview";
      await loadList();
    } catch (e) {
      error = String(e);
    }
  }

  function run(code) {
    const cmd = code.trimEnd();
    if (cmd) onrun?.(cmd);
  }

  // ---------- living runbooks: verify ----------
  // Documentation rots silently. "Verify" runs every runnable step headless
  // (login shell, 60s timeout each) and flags the ones that no longer work —
  // a runbook with an expiry alarm. Results persist in a sidecar JSON next to
  // the markdown so the freshness banner survives restarts. Steps that are
  // meant to run forever (dev servers) opt out with ```bash no-verify.
  let verifying = $state(false);
  let verifyResults = $state({}); // block index -> { status, code, output }
  let lastVerify = $state(null); // { at, total, failed } | null
  const VERIFY_TIMEOUT_S = 60;

  function sidecarPath(p) {
    return p ? p.replace(/\.md$/i, "") + ".verify.json" : null;
  }
  function verifiableSteps() {
    return blocks
      .map((b, i) => ({ b, i }))
      .filter(({ b }) => b.type === "code" && b.runnable && !/\bno-verify\b/i.test(b.lang));
  }
  async function loadVerify(p) {
    lastVerify = null;
    verifyResults = {};
    try {
      const raw = await invoke("read_file", { path: sidecarPath(p) });
      const v = JSON.parse(raw);
      if (v?.at) {
        lastVerify = v;
        for (const s of v.steps || []) {
          if (s.index != null) verifyResults[s.index] = { status: s.status, code: s.code };
        }
        verifyResults = { ...verifyResults };
      }
    } catch {}
  }
  async function verifyAll() {
    if (verifying || !current) return;
    const steps = verifiableSteps();
    if (!steps.length) return;
    if (!window.confirm(`Verify this runbook?\n\nRuns ${steps.length} step(s) in ${projectPath}.\nEach step times out after ${VERIFY_TIMEOUT_S}s — mark always-on steps (dev servers) with \u0060\u0060\u0060bash no-verify.`))
      return;
    verifying = true;
    verifyResults = {};
    let failed = 0;
    const saved = [];
    for (const { b, i } of steps) {
      if (failed) {
        verifyResults = { ...verifyResults, [i]: { status: "skipped" } };
        saved.push({ index: i, command: b.code.split("\n")[0], status: "skipped" });
        continue;
      }
      verifyResults = { ...verifyResults, [i]: { status: "running" } };
      let r;
      try {
        r = await invoke("run_step", { cwd: projectPath, command: b.code, timeoutSecs: VERIFY_TIMEOUT_S });
      } catch (e) {
        r = { code: -1, output: String(e), timed_out: false };
      }
      const status = r.timed_out ? "timeout" : r.code === 0 ? "ok" : "fail";
      if (status !== "ok") failed++;
      verifyResults = { ...verifyResults, [i]: { status, code: r.code, output: r.output } };
      saved.push({ index: i, command: b.code.split("\n")[0], status, code: r.code });
    }
    verifying = false;
    lastVerify = { at: Date.now(), total: steps.length, failed, steps: saved };
    try {
      await invoke("write_file", { path: sidecarPath(current), content: JSON.stringify(lastVerify, null, 2) });
    } catch {}
  }
  function fixStep(b, r) {
    let text = `This runbook step fails when verified. Diagnose and propose a fix — either to the command or to the runbook:\n\n\`\`\`bash\n${b.code}\n\`\`\`\n\nExit code: ${r.code}`;
    if (r.output) text += `\n\nOutput (tail):\n\`\`\`\n${r.output.slice(-2000)}\n\`\`\``;
    onelyra?.(text);
  }
  function relTime(ts) {
    const m = Math.round((Date.now() - ts) / 60000);
    if (m < 1) return "just now";
    if (m < 60) return `${m} min ago`;
    const h = Math.round(m / 60);
    if (h < 36) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }

  async function copy(code) {
    try {
      await navigator.clipboard.writeText(code);
    } catch {}
  }

  function onEditKey(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      save();
    }
  }

  // Initial load: pick the given file, or the first existing runbook, else
  // offer to create one.
  $effect(() => {
    if (!projectPath) return;
    (async () => {
      await loadList();
      const target = initialFile && files.some((f) => f.path === initialFile) ? initialFile : files[0]?.path;
      if (target) await open(target);
      else loading = false;
    })();
  });
</script>

<div class="runbook" class:light={theme === "light"}>
  <div class="bar">
    <span class="ico">📓</span>
    <select
      class="picker"
      value={current ?? ""}
      onchange={(e) => e.currentTarget.value && open(e.currentTarget.value)}
    >
      {#if files.length === 0}
        <option value="" disabled>No runbooks yet</option>
      {/if}
      {#each files as f (f.path)}
        <option value={f.path}>{f.name}</option>
      {/each}
    </select>
    <button class="btn" onclick={createNew} title="Create a new runbook">＋ New</button>
    <div class="spacer"></div>
    {#if current}
      {#if mode === "preview"}
        {#if verifiableSteps().length}
          <button class="btn" onclick={verifyAll} disabled={verifying} title="Run every step headless and flag the ones that no longer work">
            {verifying ? "⏳ Verifying…" : "✓ Verify"}
          </button>
        {/if}
        <button class="btn" onclick={() => { draft = content; mode = "edit"; }}>✎ Edit</button>
      {:else}
        <button class="btn" onclick={() => { draft = content; mode = "preview"; }}>Cancel</button>
        <button class="btn primary" onclick={save} disabled={saved && draft === content}>Save <kbd>⌘S</kbd></button>
      {/if}
    {/if}
  </div>

  {#if error}
    <div class="err">{error}</div>
  {/if}

  {#if lastVerify && mode === "preview" && !verifying}
    <div class="verify-banner" class:ok={lastVerify.failed === 0} class:bad={lastVerify.failed > 0}>
      {#if lastVerify.failed === 0}
        ✓ Verified {relTime(lastVerify.at)} — all {lastVerify.total} step{lastVerify.total === 1 ? "" : "s"} green
      {:else}
        ⚠ Verified {relTime(lastVerify.at)} — {lastVerify.failed} of {lastVerify.total} step{lastVerify.total === 1 ? "" : "s"} failing
      {/if}
    </div>
  {/if}

  <div class="body">
    {#if loading}
      <div class="empty">Loading…</div>
    {:else if !current}
      <div class="empty">
        <p>No runbook for this project yet.</p>
        <button class="btn primary" onclick={createNew}>＋ Create runbook</button>
        <p class="hint">Stored in <code>.conductor/notes/</code> — versionable with git.</p>
      </div>
    {:else if mode === "edit"}
      <textarea
        class="editor"
        bind:value={draft}
        spellcheck="false"
        oninput={() => (saved = false)}
        onkeydown={onEditKey}
        placeholder="# Markdown…  use ```bash fences for runnable commands"
      ></textarea>
    {:else}
      <div class="preview" role="presentation" onclick={onPreviewClick}>
        {#each blocks as b, i (i)}
          {#if b.type === "code"}
            {@const vr = verifyResults[i]}
            <div class="codeblock" class:v-fail={vr?.status === "fail" || vr?.status === "timeout"}>
              <div class="codebar">
                {#if b.lang}<span class="lang">{b.lang}</span>{/if}
                {#if vr}
                  <span class="vbadge {vr.status}" title={vr.status === "timeout" ? `Timed out after ${VERIFY_TIMEOUT_S}s` : vr.code != null ? `Exit ${vr.code}` : ""}>
                    {vr.status === "running" ? "⏳" : vr.status === "ok" ? "✓" : vr.status === "skipped" ? "⊘ skipped" : vr.status === "timeout" ? "⧖ timeout" : `✗ exit ${vr.code}`}
                  </span>
                {/if}
                <div class="spacer"></div>
                {#if elyra && (vr?.status === "fail" || vr?.status === "timeout") && vr.output != null}
                  <button class="run" onclick={() => fixStep(b, vr)} title="Ask Elyra to fix this step">⚡ Fix</button>
                {/if}
                {#if b.runnable}
                  <button class="run" onclick={() => run(b.code)} title="Run in this project's terminal">▶ Run</button>
                {/if}
                {#if elyra}
                  <button class="copy" onclick={() => onelyra?.(b.code)} title="Send to an Elyra agent">🤖 Elyra</button>
                {/if}
                <button class="copy" onclick={() => copy(b.code)} title="Copy">⧉</button>
              </div>
              <pre><code>{b.code}</code></pre>
              {#if (vr?.status === "fail" || vr?.status === "timeout") && vr.output}
                <details class="vout">
                  <summary>output</summary>
                  <pre>{vr.output.slice(-2000)}</pre>
                </details>
              {/if}
            </div>
          {:else}
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html b.html}
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .verify-banner {
    padding: 6px 14px;
    font-size: 12px;
    border-bottom: 1px solid var(--border);
  }
  .verify-banner.ok { color: #9ece6a; }
  .verify-banner.bad { color: #e0af68; }
  .vbadge { font-size: 11px; margin-left: 8px; }
  .vbadge.ok { color: #9ece6a; }
  .vbadge.fail, .vbadge.timeout { color: #f7768e; }
  .vbadge.skipped { color: var(--text-dim); }
  .vbadge.running { color: var(--text-dim); }
  :global(.codeblock.v-fail) { border-color: #f7768e55 !important; }
  .vout { padding: 4px 10px 8px; }
  .vout summary { font-size: 11px; color: var(--text-dim); cursor: pointer; }
  .vout pre {
    margin: 6px 0 0;
    max-height: 160px;
    overflow: auto;
    font-size: 11px;
    color: #f7768e;
    background: var(--bg);
    border-radius: 6px;
    padding: 8px;
  }

  .runbook {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-2);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-3);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .ico { font-size: 14px; }
  .spacer { flex: 1; }
  .picker {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    max-width: 240px;
  }
  .btn {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
  }
  .btn:hover:not(:disabled) { border-color: var(--accent); }
  .btn.primary { background: var(--accent-2); border-color: var(--accent); font-weight: 600; }
  .btn:disabled { opacity: 0.5; }
  .btn kbd {
    font-family: var(--font-mono);
    font-size: 10px;
    opacity: 0.7;
  }
  .err {
    padding: 6px 12px;
    background: rgba(247, 118, 142, 0.12);
    color: #f7768e;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .body { flex: 1; min-height: 0; overflow: auto; }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: var(--text-dim);
  }
  .empty .hint { font-size: 11px; }
  .editor {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    resize: none;
    background: var(--bg);
    color: var(--text);
    border: none;
    outline: none;
    padding: 16px;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.55;
  }
  .preview {
    padding: 8px 28px 40px;
    max-width: 820px;
    margin: 0 auto;
    color: var(--text);
    font-size: 14px;
    line-height: 1.6;
  }
  /* Markdown element styling (rendered via {@html}). */
  .preview :global(h1) { font-size: 1.6em; margin: 0.8em 0 0.4em; border-bottom: 1px solid var(--border); padding-bottom: 0.2em; }
  .preview :global(h2) { font-size: 1.3em; margin: 1em 0 0.4em; }
  .preview :global(h3) { font-size: 1.1em; margin: 1em 0 0.3em; }
  .preview :global(p) { margin: 0.5em 0; }
  .preview :global(ul), .preview :global(ol) { margin: 0.5em 0; padding-left: 1.4em; }
  .preview :global(li) { margin: 0.2em 0; }
  .preview :global(a) { color: var(--accent); }
  .preview :global(a[href^="cfile:"]) { text-decoration-style: dotted; cursor: pointer; }
  .preview :global(a[href^="cfile:"])::before { content: "\1F4C4 "; font-size: 0.9em; }
  .preview :global(a[href^="ctask:"]) {
    text-decoration: none;
    cursor: pointer;
    background: var(--accent-2);
    border: 1px solid var(--accent);
    border-radius: 5px;
    padding: 0 7px;
    font-size: 0.9em;
    white-space: nowrap;
  }
  .preview :global(a[href^="ctask:"])::before { content: "\25B6 "; font-size: 0.85em; }
  .preview :global(code) { font-family: var(--font-mono); font-size: 0.88em; background: var(--bg-3); padding: 1px 5px; border-radius: 4px; }
  .preview :global(blockquote) { margin: 0.6em 0; padding: 2px 12px; border-left: 3px solid var(--border); color: var(--text-dim); }
  .preview :global(table) { border-collapse: collapse; margin: 0.6em 0; }
  .preview :global(th), .preview :global(td) { border: 1px solid var(--border); padding: 4px 10px; }
  .preview :global(input[type="checkbox"]) { margin-right: 6px; }

  .codeblock {
    margin: 0.8em 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg);
  }
  .codebar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-3);
    border-bottom: 1px solid var(--border);
  }
  .codebar .lang { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-dim); }
  .codeblock pre { margin: 0; padding: 12px 14px; overflow-x: auto; }
  .codeblock pre code { background: transparent; padding: 0; font-family: var(--font-mono); font-size: 12.5px; line-height: 1.5; }
  .run {
    background: var(--accent-2);
    border: 1px solid var(--accent);
    color: var(--text);
    border-radius: 5px;
    padding: 2px 10px;
    font-size: 11px;
    font-weight: 600;
  }
  .run:hover { background: var(--accent); }
  .copy {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 5px;
    padding: 2px 7px;
    font-size: 11px;
  }
  .copy:hover { color: var(--text); border-color: var(--accent); }
</style>
