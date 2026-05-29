// Build the `latest.json` update manifest from the signed updater artifact
// produced by `tauri build`. Run after a release build, then upload latest.json
// (and the .app.tar.gz + .sig) to the matching GitHub release.
import { readFileSync, writeFileSync } from "node:fs";

const REPO = "kwhorne/elyra-conductor";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const version = pkg.version;

const sigPath =
  "src-tauri/target/release/bundle/macos/elyra-conductor.app.tar.gz.sig";

let signature;
try {
  signature = readFileSync(sigPath, "utf8").trim();
} catch {
  console.error(
    `Missing ${sigPath}.\nDid you run a signed release build?\n` +
      "  export TAURI_SIGNING_PRIVATE_KEY=... (and _PASSWORD)\n" +
      "  pnpm tauri build"
  );
  process.exit(1);
}

const url = `https://github.com/${REPO}/releases/download/v${version}/elyra-conductor.app.tar.gz`;

const manifest = {
  version,
  notes: `elyra-conductor v${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "darwin-aarch64": { signature, url },
  },
};

writeFileSync("latest.json", JSON.stringify(manifest, null, 2));
console.log(`wrote latest.json for v${version}`);
