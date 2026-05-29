// Build the `latest.json` update manifest from the signed updater artifact
// produced by `tauri build`. Run after a release build, then upload latest.json
// (and the .app.tar.gz + .sig) to the matching GitHub release.
import { readFileSync, writeFileSync, readdirSync } from "node:fs";

const REPO = "kwhorne/elyra-conductor";

// Stable, space-free asset name to upload the updater payload under (the
// productName-derived bundle name may contain spaces; the signature is over the
// file contents, not its name, so a stable name keeps the update URL clean).
const ASSET_NAME = "elyra-conductor.app.tar.gz";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const version = pkg.version;

const macosDir = "src-tauri/target/release/bundle/macos";
const sigFile = readdirSync(macosDir).find((f) => f.endsWith(".app.tar.gz.sig"));

if (!sigFile) {
  console.error(
    `No *.app.tar.gz.sig in ${macosDir}.\nDid you run a signed release build?\n` +
      "  export TAURI_SIGNING_PRIVATE_KEY=... (and _PASSWORD)\n" +
      "  pnpm tauri build"
  );
  process.exit(1);
}

const signature = readFileSync(`${macosDir}/${sigFile}`, "utf8").trim();
const url = `https://github.com/${REPO}/releases/download/v${version}/${ASSET_NAME}`;

console.log(`updater tarball: ${sigFile.replace(/\.sig$/, "")}  ->  upload as ${ASSET_NAME}`);

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
