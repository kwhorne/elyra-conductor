// Sanitising for the two places Conductor renders untrusted HTML.
//
// Why this exists: markdown reaches the webview from sources we do not control —
// runbooks in `.conductor/notes/*.md` are written by agents and travel with any
// repository you clone, and release notes come down from GitHub. Rendering that
// through `{@html}` unfiltered would let a crafted note run script inside the
// webview, which holds IPC access to `pty_write`, `write_file`, `run_step` and
// friends. That is local code execution, so both sinks go through DOMPurify.
//
// See also the CSP in `src-tauri/tauri.conf.json` — belt and braces: DOMPurify
// removes the payload, the CSP stops anything that slips past from doing damage.
import DOMPurify from "dompurify";

// RunbookPanel rewrites Obsidian-style `[[file]]` / `[[task:name]]` into links
// with custom `cfile:` / `ctask:` schemes, which a delegated click handler turns
// into "open in editor" / "run task". DOMPurify drops unknown schemes by
// default, which would silently break that feature while looking like it works
// — so allow exactly those two alongside the normal web schemes.
//
// Derived from DOMPurify's own default expression, with `cfile|ctask` added.
const RUNBOOK_URI = /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|cfile|ctask):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i;

// Tags that markdown never legitimately produces but which carry real risk if a
// note contains raw HTML. DOMPurify already strips <script> and every `on*`
// handler; this is defence in depth for the embedding/navigation vectors.
const FORBID_TAGS = ["style", "iframe", "object", "embed", "form", "input", "base", "meta", "link"];
const FORBID_ATTR = ["style", "formaction", "form", "srcset", "ping"];

/**
 * Sanitise rendered runbook markdown, preserving the `cfile:`/`ctask:` links
 * the panel's click handler depends on.
 * @param {string} html
 * @returns {string}
 */
export function sanitizeRunbookHtml(html) {
  if (!html) return "";
  return DOMPurify.sanitize(html, {
    ALLOWED_URI_REGEXP: RUNBOOK_URI,
    FORBID_TAGS,
    FORBID_ATTR,
  });
}

/**
 * Sanitise rendered markdown from a remote source (GitHub release notes).
 * No custom schemes here — plain web links only.
 * @param {string} html
 * @returns {string}
 */
export function sanitizeMarkdownHtml(html) {
  if (!html) return "";
  return DOMPurify.sanitize(html, { FORBID_TAGS, FORBID_ATTR });
}
