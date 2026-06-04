<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  // Pure host UI for an external `elyra --mode rpc` process. No AI logic here —
  // we only render events and forward commands (see ARCHITECTURE.md).
  let { id, cwd, initialPrompt = null, initialDraft = null, onactivity = null, ontitle = null } = $props();

  let entries = $state([]); // { kind: 'user'|'assistant'|'tool'|'note', text, ... }
  let status = $state({}); // statusKey -> text (from setStatus)
  let widgets = $state({}); // widgetKey -> string[]
  let pendingUI = $state(null); // confirm/select/input/editor request
  let busy = $state(false);
  let exited = $state(false);
  let draft = $state("");
  let title = $state("elyra");
  let uiDraft = $state("");

  // Prefill the input/editor field when a request arrives.
  $effect(() => {
    if (pendingUI && (pendingUI.method === "input" || pendingUI.method === "editor")) {
      uiDraft = pendingUI.prefill ?? "";
    }
  });

  let curIdx = -1; // index of the streaming assistant entry
  const toolMap = {}; // toolCallId -> entry index
  let listEl;
  let lastActivity = 0;
  let cleanup = [];

  function bump() {
    queueMicrotask(() => {
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    });
    const now = Date.now();
    if (onactivity && now - lastActivity > 350) {
      lastActivity = now;
      onactivity();
    }
  }

  function extractText(m) {
    if (!m || !Array.isArray(m.content)) return "";
    return m.content.filter((c) => c && c.type === "text").map((c) => c.text).join("");
  }
  function extractThinking(m) {
    if (!m || !Array.isArray(m.content)) return "";
    return m.content.filter((c) => c && c.type === "thinking").map((c) => c.thinking).join("");
  }

  function send(cmd) {
    invoke("agent_send", { id, command: cmd }).catch(() => {});
  }

  function onEvent(m) {
    switch (m.type) {
      case "agent_start":
        busy = true;
        bump();
        break;
      case "agent_end":
        busy = false;
        break;
      case "message_start":
        if (m.message?.role === "assistant") {
          entries.push({ kind: "assistant", text: "", thinking: "" });
          curIdx = entries.length - 1;
        }
        break;
      case "message_update":
        if (m.message?.role === "assistant") {
          if (curIdx < 0) {
            entries.push({ kind: "assistant", text: "", thinking: "" });
            curIdx = entries.length - 1;
          }
          entries[curIdx] = { kind: "assistant", text: extractText(m.message), thinking: extractThinking(m.message) };
          bump();
        }
        break;
      case "message_end":
        if (m.message?.role === "assistant" && curIdx >= 0) {
          entries[curIdx] = { kind: "assistant", text: extractText(m.message), thinking: extractThinking(m.message) };
          curIdx = -1;
        }
        break;
      case "tool_execution_start": {
        entries.push({ kind: "tool", name: m.toolName, status: "running" });
        toolMap[m.toolCallId] = entries.length - 1;
        bump();
        break;
      }
      case "tool_execution_end": {
        const i = toolMap[m.toolCallId];
        if (i != null && entries[i]) entries[i] = { ...entries[i], status: m.isError ? "error" : "done" };
        break;
      }
      case "extension_ui_request":
        handleUI(m);
        break;
      case "response":
        if (m.success === false && m.error) {
          entries.push({ kind: "note", level: "error", text: m.error });
          busy = false;
          bump();
        }
        break;
      case "extension_error":
        entries.push({ kind: "note", level: "error", text: `Extension error: ${m.error ?? ""}` });
        bump();
        break;
    }
  }

  function handleUI(m) {
    switch (m.method) {
      case "notify":
        entries.push({ kind: "note", level: m.notifyType ?? "info", text: m.message });
        bump();
        break;
      case "setStatus": {
        const next = { ...status };
        if (m.statusText == null) delete next[m.statusKey];
        else next[m.statusKey] = m.statusText;
        status = next;
        break;
      }
      case "setTitle":
        title = m.title;
        ontitle?.(m.title);
        break;
      case "setWidget": {
        const next = { ...widgets };
        if (!m.widgetLines) delete next[m.widgetKey];
        else next[m.widgetKey] = m.widgetLines;
        widgets = next;
        break;
      }
      case "set_editor_text":
        draft = m.text ?? draft;
        break;
      case "confirm":
      case "select":
      case "input":
      case "editor":
        pendingUI = m;
        bump();
        break;
    }
  }

  function respondUI(resp) {
    if (!pendingUI) return;
    send({ type: "extension_ui_response", id: pendingUI.id, ...resp });
    pendingUI = null;
  }

  function submit() {
    const text = draft.trim();
    if (!text || exited) return;
    entries.push({ kind: "user", text });
    if (busy) send({ type: "steer", message: text });
    else send({ type: "prompt", message: text });
    draft = "";
    bump();
  }

  async function restart() {
    entries = [];
    curIdx = -1;
    pendingUI = null;
    busy = false;
    exited = false;
    try {
      await invoke("agent_spawn", { id, cwd });
    } catch (e) {
      entries.push({ kind: "note", level: "error", text: `Could not start Elyra: ${e}` });
      exited = true;
    }
  }

  function onKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  onMount(async () => {
    const unEv = await listen(`agent://event/${id}`, (e) => onEvent(e.payload));
    const unExit = await listen(`agent://exit/${id}`, () => {
      exited = true;
      busy = false;
      entries.push({ kind: "note", level: "info", text: "Agent exited." });
      bump();
    });
    cleanup.push(unEv, unExit);

    try {
      await invoke("agent_spawn", { id, cwd });
      if (initialPrompt) {
        entries.push({ kind: "user", text: initialPrompt });
        send({ type: "prompt", message: initialPrompt });
        bump();
      } else if (initialDraft) {
        // Pre-fill the composer (don't send) so the user can add their question.
        draft = initialDraft;
      }
    } catch (e) {
      entries.push({ kind: "note", level: "error", text: `Could not start Elyra: ${e}` });
    }
  });

  onDestroy(() => {
    cleanup.forEach((fn) => fn?.());
    invoke("agent_kill", { id }).catch(() => {});
  });

  let statusText = $derived(Object.values(status).filter(Boolean).join("  ·  "));
</script>

<div class="agent">
  <div class="bar">
    <span class="title">🤖 {title}</span>
    {#if busy}<span class="busy">● working</span>{/if}
    {#if statusText}<span class="status">{statusText}</span>{/if}
    {#if busy}
      <button class="abort" onclick={() => send({ type: "abort" })}>Stop</button>
    {/if}
    {#if exited}
      <button class="restart" onclick={restart}>↻ Restart</button>
    {/if}
  </div>

  <div class="messages" bind:this={listEl}>
    {#each entries as e, i (i)}
      {#if e.kind === "user"}
        <div class="msg user"><div class="who">you</div><div class="text">{e.text}</div></div>
      {:else if e.kind === "assistant"}
        <div class="msg assistant">
          {#if e.thinking}<div class="thinking">{e.thinking}</div>{/if}
          <div class="text">{e.text}</div>
        </div>
      {:else if e.kind === "tool"}
        <div class="tool" class:err={e.status === "error"}>
          {e.status === "running" ? "⚙" : e.status === "error" ? "✗" : "✓"} {e.name}
        </div>
      {:else if e.kind === "note"}
        <div class="note {e.level}">{e.text}</div>
      {/if}
    {/each}

    {#each Object.entries(widgets) as [k, lines] (k)}
      <div class="widget">{lines.join("\n")}</div>
    {/each}

    {#if pendingUI}
      <div class="ui-req">
        <div class="ui-title">{pendingUI.title}</div>
        {#if pendingUI.method === "confirm"}
          {#if pendingUI.message}<div class="ui-msg">{pendingUI.message}</div>{/if}
          <div class="ui-actions">
            <button class="primary" onclick={() => respondUI({ confirmed: true })}>Yes</button>
            <button onclick={() => respondUI({ confirmed: false })}>No</button>
          </div>
        {:else if pendingUI.method === "select"}
          <div class="ui-actions wrap">
            {#each pendingUI.options as opt}
              <button onclick={() => respondUI({ value: opt })}>{opt}</button>
            {/each}
          </div>
        {:else}
          <textarea class="ui-input" bind:value={uiDraft} placeholder={pendingUI.placeholder ?? ""} rows={pendingUI.method === "editor" ? 4 : 1}></textarea>
          <div class="ui-actions">
            <button class="primary" onclick={() => respondUI({ value: uiDraft })}>Submit</button>
            <button onclick={() => respondUI({ cancelled: true })}>Cancel</button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="composer">
    <textarea
      bind:value={draft}
      onkeydown={onKeydown}
      placeholder={exited ? "Agent exited" : busy ? "Steer the agent…  (↵ to send)" : "Message the agent…  (↵ to send, ⇧↵ newline)"}
      disabled={exited}
      rows="2"
    ></textarea>
    <button class="send" onclick={submit} disabled={exited || !draft.trim()}>{busy ? "Steer" : "Send"}</button>
  </div>
</div>

<style>
  .agent { display: flex; flex-direction: column; height: 100%; background: var(--bg-2); }
  .bar {
    display: flex; align-items: center; gap: 10px;
    padding: 6px 12px; background: var(--bg-3);
    border-bottom: 1px solid var(--border); font-size: 12px;
  }
  .title { font-weight: 600; }
  .busy { color: var(--accent); font-size: 11px; }
  .status { color: var(--text-dim); font-size: 11px; font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .abort { margin-left: auto; background: transparent; border: 1px solid var(--border); color: var(--red); border-radius: 6px; padding: 3px 10px; font-size: 11px; }
  .restart { margin-left: auto; background: var(--accent); border: none; color: #fff; border-radius: 6px; padding: 3px 10px; font-size: 11px; }
  .messages { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
  .msg { display: flex; flex-direction: column; gap: 3px; }
  .who { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-dim); }
  .msg .text { white-space: pre-wrap; word-break: break-word; font-size: 13px; line-height: 1.5; }
  .msg.user .text { background: var(--accent-2); border-radius: 8px; padding: 8px 10px; align-self: flex-start; }
  .thinking { white-space: pre-wrap; font-size: 11px; color: var(--text-dim); font-style: italic; border-left: 2px solid var(--border); padding-left: 8px; }
  .tool { font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); }
  .tool.err { color: var(--red); }
  .note { font-size: 12px; border-radius: 6px; padding: 6px 10px; }
  .note.info { background: var(--bg-3); color: var(--text-dim); }
  .note.warning { background: rgba(224,175,104,0.15); color: #e0af68; }
  .note.error { background: rgba(247,118,142,0.12); color: var(--red); white-space: pre-wrap; font-family: var(--font-mono); }
  .widget { font-family: var(--font-mono); font-size: 11px; white-space: pre-wrap; color: var(--text-dim); background: var(--bg-3); border-radius: 6px; padding: 6px 10px; }
  .ui-req { background: var(--bg-3); border: 1px solid var(--accent); border-radius: 8px; padding: 10px; }
  .ui-title { font-weight: 600; font-size: 12px; margin-bottom: 6px; }
  .ui-msg { font-size: 12px; color: var(--text-dim); margin-bottom: 8px; white-space: pre-wrap; }
  .ui-actions { display: flex; gap: 6px; }
  .ui-actions.wrap { flex-wrap: wrap; }
  .ui-input { width: 100%; box-sizing: border-box; resize: vertical; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-family: inherit; font-size: 12px; padding: 6px 8px; outline: none; margin-bottom: 8px; }
  .ui-input:focus { border-color: var(--accent); }
  .ui-actions button, .composer .send { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 5px 12px; font-size: 12px; }
  .ui-actions button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .composer { display: flex; gap: 8px; padding: 10px; border-top: 1px solid var(--border); align-items: flex-end; }
  .composer textarea { flex: 1; resize: none; background: var(--bg-3); border: 1px solid var(--border); border-radius: 8px; color: var(--text); font-family: inherit; font-size: 13px; padding: 8px 10px; outline: none; }
  .composer textarea:focus { border-color: var(--accent); }
  .composer .send { background: var(--accent); border-color: var(--accent); color: #fff; height: 36px; }
  .composer .send:disabled { opacity: 0.45; cursor: default; }
</style>
