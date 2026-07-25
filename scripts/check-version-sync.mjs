// Verify the version is identical in every place that declares it, and
// optionally that it matches a release tag.
//
// These four drift easily — a bump that misses one produces a release whose
// updater manifest advertises a version the binary does not report, so the app
// either re-offers an update forever or never sees it at all.
//
//   node scripts/check-version-sync.mjs            # just check they agree
//   node scripts/check-version-sync.mjs v0.9.2     # also check against a tag
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("package.json", "utf8")).version;
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;

const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargo = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const lockToml = readFileSync("src-tauri/Cargo.lock", "utf8");
const lock = lockToml.match(/name = "elyra-conductor"\nversion = "([^"]+)"/)?.[1];

const found = {
  "package.json": pkg,
  "src-tauri/tauri.conf.json": conf,
  "src-tauri/Cargo.toml": cargo,
  "src-tauri/Cargo.lock": lock,
};

let ok = true;
for (const [file, v] of Object.entries(found)) {
  const good = v === pkg;
  if (!good) ok = false;
  console.log(`  ${good ? "\u2713" : "\u2717"} ${file.padEnd(28)} ${v ?? "(not found)"}`);
}

const tagArg = process.argv[2];
if (tagArg) {
  const tag = tagArg.replace(/^v/, "");
  const good = tag === pkg;
  if (!good) ok = false;
  console.log(`  ${good ? "\u2713" : "\u2717"} ${"git tag".padEnd(28)} ${tagArg}`);
}

console.log(ok ? `\nversion ${pkg} is consistent\n` : `\nVERSION MISMATCH — fix before releasing\n`);
process.exit(ok ? 0 : 1);
