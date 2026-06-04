<script>
  import { invoke } from "@tauri-apps/api/core";

  let { open = false, path = "", projectName = "", onclose } = $props();

  let files = $state([]); // [{ name, path }]
  let activeFile = $state(null); // path
  let entries = $state([]); // [{ type:'kv'|'raw', key, value, raw }]
  let revealed = $state({}); // key -> bool
  let dirty = $state(false);
  let status = $state("");
  let err = $state("");

  $effect(() => {
    if (open) {
      err = "";
      status = "";
      dirty = false;
      revealed = {};
      findFiles();
    }
  });

  async function findFiles() {
    try {
      const list = await invoke("list_dir", { path });
      files = list
        .filter((e) => !e.is_dir && (e.name === ".env" || e.name.startsWith(".env.")))
        .map((e) => ({ name: e.name, path: e.path }))
        .sort((a, b) => a.name.localeCompare(b.name));
      if (files.length) openFile(files.find((f) => f.name === ".env")?.path ?? files[0].path);
      else { activeFile = null; entries = []; }
    } catch (e) {
      err = String(e);
    }
  }

  async function openFile(p) {
    if (dirty && !confirm("Discard unsaved changes to this file?")) return;
    activeFile = p;
    dirty = false;
    revealed = {};
    try {
      const text = await invoke("read_file", { path: p });
      entries = parse(text);
    } catch (e) {
      err = String(e);
    }
  }

  function parse(text) {
    return text.split("\n").map((raw) => {
      const m = raw.match(/^(\s*)([A-Za-z_][A-Za-z0-9_.]*)\s*=(.*)$/);
      if (m && !raw.trimStart().startsWith("#")) {
        return { type: "kv", key: m[2], value: stripQuotes(m[3]), quoted: /^\s*"/.test(m[3]) };
      }
      return { type: "raw", raw };
    });
  }
  function stripQuotes(v) {
    const t = v.trim();
    if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) return t.slice(1, -1);
    return t;
  }

  function looksSecret(key) {
    return /(SECRET|PASSWORD|PASSWD|TOKEN|KEY|API|PRIVATE|CREDENTIAL|DSN|AUTH)/i.test(key);
  }

  function serialize() {
    return entries
      .map((e) => {
        if (e.type === "raw") return e.raw;
        const v = e.quoted || /[\s#"']/.test(e.value) ? `"${e.value.replace(/"/g, '\\"')}"` : e.value;
        return `${e.key}=${v}`;
      })
      .join("\n");
  }

  async function save() {
    if (!activeFile) return;
    try {
      await invoke("write_file", { path: activeFile, content: serialize() });
      dirty = false;
      status = "Saved";
      setTimeout(() => (status = ""), 1500);
    } catch (e) {
      err = String(e);
    }
  }

  function setValue(i, v) {
    entries[i].value = v;
    entries = [...entries];
    dirty = true;
  }
  function removeEntry(i) {
    entries = entries.filter((_, idx) => idx !== i);
    dirty = true;
  }
  function addEntry() {
    entries = [...entries, { type: "kv", key: "NEW_KEY", value: "", quoted: false }];
    dirty = true;
  }
  function renameKey(i, k) {
    entries[i].key = k;
    entries = [...entries];
    dirty = true;
  }

  let kvEntries = $derived(entries.map((e, i) => ({ e, i })).filter((x) => x.e.type === "kv"));
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={onclose}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="head">
        <span class="title">.env — {projectName}</span>
        {#if files.length > 1}
          <select value={activeFile} onchange={(e) => openFile(e.currentTarget.value)}>
            {#each files as f (f.path)}<option value={f.path}>{f.name}</option>{/each}
          </select>
        {/if}
        <span class="status">{status}</span>
        <button class="save" onclick={save} disabled={!dirty} title="Save (.env stays local; never committed if gitignored)">Save</button>
        <button class="x" onclick={onclose} title="Close">✕</button>
      </div>

      {#if err}<div class="err">{err}</div>{/if}

      <div class="body">
        {#if !activeFile}
          <div class="empty">No <code>.env</code> file in this project.</div>
        {:else if kvEntries.length === 0}
          <div class="empty">No variables in this file.</div>
        {:else}
          {#each kvEntries as { e, i } (i)}
            <div class="row">
              <input class="k" value={e.key} oninput={(ev) => renameKey(i, ev.currentTarget.value)} spellcheck="false" />
              <span class="eq">=</span>
              <input
                class="v"
                type={revealed[e.key] || !looksSecret(e.key) ? "text" : "password"}
                value={e.value}
                oninput={(ev) => setValue(i, ev.currentTarget.value)}
                spellcheck="false"
                autocomplete="off"
              />
              <button class="eye" title={revealed[e.key] ? "Hide" : "Reveal"} onclick={() => (revealed = { ...revealed, [e.key]: !revealed[e.key] })}>{revealed[e.key] ? "🙈" : "👁"}</button>
              <button class="del" title="Remove" onclick={() => removeEntry(i)}>✕</button>
            </div>
          {/each}
        {/if}
      </div>

      <div class="foot">
        <button class="add" onclick={addEntry} disabled={!activeFile}>＋ Add variable</button>
        <span class="note">Secrets are masked by default. Values stay in the file — keep <code>.env</code> gitignored.</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 10vh; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 12px; width: 640px; max-width: 92vw; max-height: 76vh; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
  .head { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .title { font-weight: 600; font-size: 13px; }
  select { background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 12px; padding: 4px 8px; outline: none; }
  .status { color: var(--green); font-size: 11px; margin-left: auto; }
  .save { background: var(--accent); border: none; border-radius: 7px; color: #fff; font-size: 12px; padding: 5px 12px; cursor: pointer; margin-left: 8px; }
  .save:disabled { opacity: 0.45; cursor: default; }
  .x { background: transparent; border: 1px solid var(--border); border-radius: 7px; color: var(--text-dim); font-size: 11px; padding: 4px 8px; cursor: pointer; }
  .err { background: rgba(192,57,43,0.15); color: #e06c5a; font-size: 12px; padding: 6px 12px; white-space: pre-wrap; }
  .body { overflow: auto; padding: 8px 12px; flex: 1; }
  .empty { color: var(--text-dim); font-size: 12px; padding: 16px; }
  .row { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
  .k { width: 220px; flex: none; background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-family: var(--font-mono); font-size: 12px; padding: 6px 8px; outline: none; }
  .eq { color: var(--text-dim); }
  .v { flex: 1; background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-family: var(--font-mono); font-size: 12px; padding: 6px 8px; outline: none; }
  .k:focus, .v:focus { border-color: var(--accent); }
  .eye, .del { background: transparent; border: none; color: var(--text-dim); font-size: 12px; cursor: pointer; padding: 4px 6px; border-radius: 6px; flex: none; }
  .eye:hover, .del:hover { color: var(--text); background: var(--bg-3); }
  .foot { border-top: 1px solid var(--border); padding: 10px 12px; display: flex; align-items: center; gap: 12px; }
  .add { background: var(--bg-3); border: 1px solid var(--border); border-radius: 7px; color: var(--text); font-size: 12px; padding: 5px 11px; cursor: pointer; }
  .add:hover:not(:disabled) { border-color: var(--accent); }
  .note { font-size: 11px; color: var(--text-dim); }
  code { font-family: var(--font-mono); background: var(--bg-3); padding: 1px 4px; border-radius: 4px; }
</style>
