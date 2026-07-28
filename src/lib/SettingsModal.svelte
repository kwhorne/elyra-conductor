<script>
  // ⌘, — one place for the preferences that were previously only reachable as
  // scattered command-palette toggles.
  //
  // Note what is deliberately absent: providers, models, API keys. Someone
  // opening this window looking for "where do I put my key?" should find a clear
  // answer rather than an unexplained gap, which is what the AI section is for.
  // See ARCHITECTURE.md — Conductor must not store credentials or choose models.
  let {
    open = false,
    theme = "dark",
    termFontSize = 13,
    persistScrollback = true,
    notifyOnFinish = true,
    shellIntegration = true,
    root = "",
    elyraVersion = null,
    elyraBin = null,
    ontheme,
    onfont,
    onpersist,
    onnotify,
    onshellint,
    onchangeroot,
    onclose,
  } = $props();

  function onKeydown(e) {
    if (e.key === "Escape") onclose?.();
  }
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
      <div class="head">
        <span class="title">⚙ Settings</span>
        <div class="spacer"></div>
        <button class="icon" title="Close (Esc)" onclick={onclose}>✕</button>
      </div>

      <div class="body">
        <section>
          <h3>Appearance</h3>
          <div class="row">
            <div class="label">
              <span>Theme</span>
              <span class="sub">Applies to the UI and terminal palette</span>
            </div>
            <div class="seg">
              <button class:on={theme === "dark"} onclick={() => ontheme?.("dark")}>☾ Dark</button>
              <button class:on={theme === "light"} onclick={() => ontheme?.("light")}>☀ Light</button>
            </div>
          </div>
          <div class="row">
            <div class="label">
              <span>Terminal font size</span>
              <span class="sub">⌘+ / ⌘− / ⌘0 in a terminal</span>
            </div>
            <div class="stepper">
              <button onclick={() => onfont?.(-1)} disabled={termFontSize <= 8}>−</button>
              <span class="val">{termFontSize}px</span>
              <button onclick={() => onfont?.(1)} disabled={termFontSize >= 28}>+</button>
              <button class="reset" onclick={() => onfont?.(0)}>Reset</button>
            </div>
          </div>
        </section>

        <section>
          <h3>Terminal</h3>
          <div class="row">
            <div class="label">
              <span>Shell integration (zsh)</span>
              <span class="sub">Real commands, exit codes, and durations. Affects new terminals.</span>
            </div>
            <button
              class="toggle"
              class:on={shellIntegration}
              role="switch"
              aria-checked={shellIntegration}
              aria-label="Shell integration (zsh)"
              onclick={() => onshellint?.(!shellIntegration)}
            >
              <span class="knob"></span>
            </button>
          </div>
          <div class="row">
            <div class="label">
              <span>Save scrollback across restarts</span>
              <span class="sub">
                Stored in localStorage as plain text on disk. Credential-shaped output is masked
                first, but that is a mitigation, not a guarantee — turn this off if a pane will
                show secrets.
              </span>
            </div>
            <button
              class="toggle"
              class:on={persistScrollback}
              role="switch"
              aria-checked={persistScrollback}
              aria-label="Save scrollback across restarts"
              onclick={() => onpersist?.(!persistScrollback)}
            >
              <span class="knob"></span>
            </button>
          </div>
          <div class="row">
            <div class="label">
              <span>Notify when a background command finishes</span>
              <span class="sub">System notification for long runs in unfocused tabs</span>
            </div>
            <button
              class="toggle"
              class:on={notifyOnFinish}
              role="switch"
              aria-checked={notifyOnFinish}
              aria-label="Notify when a background command finishes"
              onclick={() => onnotify?.(!notifyOnFinish)}
            >
              <span class="knob"></span>
            </button>
          </div>
        </section>

        <section>
          <h3>Projects</h3>
          <div class="row">
            <div class="label">
              <span>Scan folder</span>
              <span class="sub mono">{root || "—"}</span>
            </div>
            <button class="ghost" onclick={onchangeroot}>Change…</button>
          </div>
        </section>

        <section>
          <h3>AI assistance</h3>
          <div class="note">
            <p>
              <b>There are no API keys or model settings here, by design.</b> Conductor is a host:
              it starts processes and shows their output. All reasoning happens in the external
              <b>Elyra</b> CLI, which already owns providers, models, keys, and prompts — configured
              once, in one place, and reusable outside Conductor.
            </p>
            <p>
              Keeping credentials out of this app is also what lets the terminal render untrusted
              text — filenames, git output, scrollback — without a key being worth stealing.
            </p>
            <p class="hint">
              Terminal help: press <kbd>⌘↵</kbd> in any pane, <kbd>Esc</kbd> to dismiss. It sends
              that pane's recent output (secrets masked) and exit codes to <code>elyra</code>, and
              answers without running anything. Works over SSH and in containers, because the
              context is the scrollback.
            </p>
            <p class="hint">Configure providers and models with <code>elyra config</code>.</p>
          </div>
          <div class="row">
            <div class="label">
              <span>Elyra CLI</span>
              <span class="sub mono">{elyraBin || "not found on PATH"}</span>
            </div>
            <span class="badge" class:ok={!!elyraVersion}>{elyraVersion || "missing"}</span>
          </div>
        </section>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .modal { background: var(--bg-2); border: 1px solid var(--border); border-radius: 14px; width: 640px; max-width: 94vw; max-height: 86vh; display: flex; flex-direction: column; box-shadow: 0 20px 56px rgba(0,0,0,0.55); }
  .head { display: flex; align-items: center; gap: 10px; padding: 16px 20px 12px; border-bottom: 1px solid var(--border); }
  .title { font-size: 15px; font-weight: 700; }
  .spacer { flex: 1; }
  .icon { background: transparent; border: none; color: var(--text-dim); font-size: 13px; cursor: pointer; }
  .icon:hover { color: var(--text); }
  .body { overflow-y: auto; padding: 4px 20px 14px; }
  section { padding: 14px 0; border-bottom: 1px solid var(--border); }
  section:last-child { border-bottom: none; }
  h3 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); margin: 0 0 10px; font-weight: 600; }
  .row { display: flex; align-items: flex-start; gap: 14px; padding: 7px 0; }
  .label { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .label > span:first-child { font-size: 12px; color: var(--text); }
  .sub { font-size: 10px; color: var(--text-dim); line-height: 1.45; }
  .mono { font-family: var(--font-mono); word-break: break-all; }
  .seg { display: flex; gap: 0; flex: none; }
  .seg button { background: var(--bg); border: 1px solid var(--border); color: var(--text-dim); font-size: 11px; padding: 4px 10px; cursor: pointer; }
  .seg button:first-child { border-radius: 6px 0 0 6px; }
  .seg button:last-child { border-radius: 0 6px 6px 0; border-left: none; }
  .seg button.on { background: var(--accent); color: #fff; border-color: var(--accent); }
  .stepper { display: flex; align-items: center; gap: 4px; flex: none; }
  .stepper button { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 5px; width: 24px; height: 24px; font-size: 13px; cursor: pointer; line-height: 1; }
  .stepper button:disabled { opacity: 0.4; cursor: default; }
  .stepper button:hover:not(:disabled) { border-color: var(--accent); }
  .stepper .val { font-family: var(--font-mono); font-size: 11px; min-width: 38px; text-align: center; color: var(--text-dim); }
  .stepper .reset { width: auto; padding: 0 8px; font-size: 10px; }
  .toggle { flex: none; width: 38px; height: 21px; border-radius: 11px; background: var(--bg); border: 1px solid var(--border); position: relative; cursor: pointer; padding: 0; transition: background 0.14s, border-color 0.14s; margin-top: 1px; }
  .toggle.on { background: var(--accent); border-color: var(--accent); }
  .knob { position: absolute; top: 2px; left: 2px; width: 15px; height: 15px; border-radius: 50%; background: var(--text-dim); transition: transform 0.14s, background 0.14s; }
  .toggle.on .knob { transform: translateX(17px); background: #fff; }
  .ghost { background: var(--bg); border: 1px solid var(--border); color: var(--text); border-radius: 6px; padding: 4px 10px; font-size: 11px; cursor: pointer; flex: none; }
  .ghost:hover { border-color: var(--accent); }
  .note { background: var(--bg-3); border-radius: 8px; padding: 11px 13px; margin-bottom: 6px; }
  .note p { margin: 0 0 8px; font-size: 11px; line-height: 1.55; color: var(--text-dim); }
  .note p:last-child { margin-bottom: 0; }
  .note b { color: var(--text); }
  .note code { font-family: var(--font-mono); background: var(--bg); border-radius: 3px; padding: 1px 4px; font-size: 10px; }
  .note .hint { color: var(--text-dim); }
  kbd { font-family: var(--font-mono); background: var(--bg); border: 1px solid var(--border); border-radius: 4px; padding: 0 4px; font-size: 10px; }
  .badge { flex: none; font-family: var(--font-mono); font-size: 10px; border-radius: 5px; padding: 3px 8px; background: var(--bg); color: var(--red); border: 1px solid var(--border); }
  .badge.ok { color: var(--green); }
</style>
