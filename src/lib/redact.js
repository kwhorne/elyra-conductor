// Redaction for persisted terminal scrollback.
//
// Why: each pane's recent output is written to localStorage every few seconds so
// history survives a restart. Terminal output routinely contains credentials —
// `cat .env`, an exported token, a connection string echoed by a migration tool
// — and localStorage is plain text on disk. Nothing has to go wrong for those
// secrets to be written; it happens on every save.
//
// This masks the common shapes before anything is stored. It is a mitigation,
// not a guarantee: a secret in an unusual format can still slip through, which
// is why persistence can also be turned off entirely (see the "Persist terminal
// scrollback" toggle in the command palette).

const MASK = "\u2022\u2022\u2022REDACTED\u2022\u2022\u2022";

// Each entry keeps the identifying prefix so restored history stays readable
// ("API_KEY=•••REDACTED•••" is far more useful than a blank line).
/** @type {Array<[RegExp, string | ((...a: any[]) => string)]>} */
const PATTERNS = [
  // KEY=value / KEY: value for anything that smells like a credential.
  [/\b([A-Z0-9_]*(?:SECRET|TOKEN|PASSWORD|PASSWD|API[_-]?KEY|ACCESS[_-]?KEY|PRIVATE[_-]?KEY|CREDENTIALS?|AUTH)[A-Z0-9_]*)(\s*[=:]\s*)(["']?)([^\s"'`]{4,})\3/gi,
   (_m, key, sep, q) => `${key}${sep}${q}${MASK}${q}`],
  // Authorization headers and bearer tokens.
  [/\b(Bearer|Basic)\s+([A-Za-z0-9._~+/=-]{8,})/g, (_m, scheme) => `${scheme} ${MASK}`],
  // Credentials embedded in a URL: proto://user:pass@host
  [/\b([a-z][a-z0-9+.-]*:\/\/[^\s:/@]+):([^\s@/]+)@/gi, (_m, head) => `${head}:${MASK}@`],
  // Well-known provider token formats.
  [/\bgh[pousr]_[A-Za-z0-9]{16,}/g, MASK],                    // GitHub
  [/\bAKIA[0-9A-Z]{16}\b/g, MASK],                            // AWS access key id
  [/\bsk-[A-Za-z0-9]{20,}/g, MASK],                           // OpenAI-style
  [/\bsk_(?:live|test)_[A-Za-z0-9]{10,}/g, MASK],             // Stripe
  [/\bxox[baprs]-[A-Za-z0-9-]{10,}/g, MASK],                  // Slack
  [/\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}/g, MASK], // JWT
  // PEM private key blocks, including the body.
  [/-----BEGIN[^-]*PRIVATE KEY-----[\s\S]*?-----END[^-]*PRIVATE KEY-----/g, MASK],
];

/**
 * Mask credential-shaped text. Safe to run on xterm's serialized output: the
 * patterns only match printable credential text, never ANSI escape bytes.
 * @param {string} text
 * @returns {string}
 */
export function redactSecrets(text) {
  if (!text) return text;
  let out = text;
  for (const [re, replacement] of PATTERNS) out = out.replace(re, /** @type {any} */ (replacement));
  return out;
}
