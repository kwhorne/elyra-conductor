// Print the CHANGELOG section for the current package.json version.
//
// Used by the release workflow for the GitHub release body, and available
// locally for the same purpose — so the notes on the release, the notes in the
// updater manifest and the notes in CHANGELOG.md can never disagree.
//
//   node scripts/release-notes.mjs > notes.md
import { readFileSync } from "node:fs";

const version = JSON.parse(readFileSync("package.json", "utf8")).version;
const lines = readFileSync("CHANGELOG.md", "utf8").split("\n");

const start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
if (start === -1) {
  console.error(`No "## [${version}]" section in CHANGELOG.md — add one before releasing.`);
  process.exit(1);
}

const body = [];
for (let i = start + 1; i < lines.length; i++) {
  if (lines[i].startsWith("## [")) break;
  body.push(lines[i]);
}

const text = body.join("\n").trim();
if (!text) {
  console.error(`The "## [${version}]" section in CHANGELOG.md is empty.`);
  process.exit(1);
}
console.log(text);
