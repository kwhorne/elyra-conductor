import { JSDOM } from "jsdom";
const dom = new JSDOM("");
globalThis.window = dom.window;
globalThis.document = dom.window.document;
globalThis.Node = dom.window.Node;
globalThis.HTMLElement = dom.window.HTMLElement;
globalThis.DocumentFragment = dom.window.DocumentFragment;

const { sanitizeRunbookHtml, sanitizeMarkdownHtml } = await import("../src/lib/sanitize.js");
const { marked } = await import("marked");

let pass = 0, fail = 0;
const t = (name, cond, extra = "") => {
  if (cond) { pass++; console.log(`  ✓ ${name}`); }
  else { fail++; console.log(`  ✗ ${name}  ${extra}`); }
};

console.log("\n— ATTACKS must be neutralised —");
const attacks = [
  ['<img src=x onerror="alert(1)">',       /onerror/i],
  ['<script>fetch("//evil")</script>',      /<script/i],
  ['<a href="javascript:alert(1)">x</a>',   /javascript:/i],
  ['<iframe src="//evil"></iframe>',        /<iframe/i],
  ['<svg><script>alert(1)</script></svg>',  /<script/i],
  ['<body onload="alert(1)">',              /onload/i],
  ['<form action="x"><input name=y>',       /<form|<input/i],
];
for (const [payload, bad] of attacks) {
  const out = sanitizeRunbookHtml(payload);
  t(payload.slice(0, 42).padEnd(44), !bad.test(out), `-> ${out}`);
}

console.log("\n— RUNBOOK interactivity must SURVIVE —");
const wiki = marked.parser(marked.lexer(
  "[open](cfile:src%2Fmain.rs) and [run](ctask:dev) and [web](https://example.com)"
));
const clean = sanitizeRunbookHtml(wiki);
t("cfile: link preserved", clean.includes("cfile:"), `-> ${clean}`);
t("ctask: link preserved", clean.includes("ctask:"), `-> ${clean}`);
t("https: link preserved", clean.includes("https://example.com"), `-> ${clean}`);

console.log("\n— normal markdown must survive —");
const md = sanitizeRunbookHtml(marked.parser(marked.lexer(
  "# Title\n\n**bold** `code`\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- item"
)));
t("heading", md.includes("<h1"));
t("bold", md.includes("<strong>"));
t("table", md.includes("<table"));
t("list", md.includes("<li>"));

console.log("\n— release-notes sink: custom schemes NOT allowed —");
const rn = sanitizeMarkdownHtml('<a href="cfile:/etc/passwd">x</a><img src=x onerror="alert(1)">');
t("cfile: stripped in release notes", !rn.includes("cfile:"), `-> ${rn}`);
t("onerror stripped", !/onerror/i.test(rn), `-> ${rn}`);

const { redactSecrets } = await import("../src/lib/redact.js");
console.log("\n— SECRETS must be redacted from scrollback —");
const secrets = [
  ["API_KEY=abcd1234efgh5678",            "abcd1234efgh5678"],
  ["export DB_PASSWORD='hunter2xyz'",     "hunter2xyz"],
  ["Authorization: Bearer eyJhbGciOiJI.aaaaaaaaaa.bbbbbbbbbb", "eyJhbGciOiJI"],
  ["ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345", "ghp_ABCDEFGHIJ"],
  ["AKIAIOSFODNN7EXAMPLE",                "AKIAIOSFODNN7EXAMPLE"],
  ["sk-abcdefghijklmnopqrstuvwxyz0123",   "sk-abcdefghij"],
  ["postgres://user:s3cr3tpw@db:5432/x",  "s3cr3tpw"],
  ["-----BEGIN RSA PRIVATE KEY-----\nMIIabc\n-----END RSA PRIVATE KEY-----", "MIIabc"],
];
for (const [input, leak] of secrets) {
  const out = redactSecrets(input);
  t(input.split("\n")[0].slice(0, 42).padEnd(44), !out.includes(leak), `-> ${out.slice(0,60)}`);
}

console.log("\n— ordinary output must be untouched —");
const benign = [
  "npm run dev",
  "\u001b[32m✓\u001b[0m 42 tests passed in 1.2s",
  "  Local:   http://localhost:1420/",
  "commit a1b2c3d4  Fix the thing",
];
for (const b of benign) t(JSON.stringify(b).slice(0, 42).padEnd(44), redactSecrets(b) === b, `-> ${redactSecrets(b)}`);

console.log(`\n${fail === 0 ? "ALL PASS" : "FAILURES"}: ${pass} passed, ${fail} failed\n`);
process.exit(fail === 0 ? 0 : 1);
