// Pure, state-free helpers shared across the app. Kept out of App.svelte so the
// component file stays focused on UI wiring and reactive state.

/** Parent directory of a path ("/a/b/c" -> "/a/b"). */
export function dirOf(path) {
  const i = path.lastIndexOf("/");
  return i > 0 ? path.slice(0, i) : "/";
}

/** Final path segment ("/a/b/c.txt" -> "c.txt"). */
export function baseOf(path) {
  return path.split("/").pop();
}

// Sensible interpreter for running a file by extension, so a script without the
// executable bit still runs. The command is editable in the run modal, so this
// is only a starting point.
export const RUN_BY_EXT = {
  sh: "bash", bash: "bash", zsh: "zsh", fish: "fish",
  py: "python3", rb: "ruby", pl: "perl", lua: "lua", php: "php",
  js: "node", cjs: "node", mjs: "node", ts: "npx tsx", tsx: "npx tsx",
  go: "go run",
};

export function detectRunCommand(name) {
  const ext = name.includes(".") ? name.split(".").pop().toLowerCase() : "";
  const runner = RUN_BY_EXT[ext];
  // Shell-quote the filename so spaces/specials are safe.
  const q = `'${name.replace(/'/g, "'\\''")}'`;
  return runner ? `${runner} ${q}` : `./${q}`;
}

// Process names that mean "just a shell prompt", i.e. nothing is running in the
// foreground.
export const SHELLS = new Set([
  "zsh", "-zsh", "bash", "-bash", "sh", "-sh", "fish", "-fish",
  "nu", "pwsh", "login", "tmux",
]);

export function isIdleProc(name) {
  return !name || SHELLS.has(name.toLowerCase());
}

// Scoring for guessing a project's "dev" task from its discovered scripts.
const DEV_EXACT = { dev: 100, start: 90, serve: 80, develop: 75, "start:dev": 95, "dev:server": 85, watch: 60 };

export function scoreDevTask(t) {
  const l = t.label.toLowerCase();
  if (DEV_EXACT[l]) return DEV_EXACT[l];
  if (l.includes("dev")) return 50;
  if (l.includes("serve")) return 45;
  if (l.includes("start")) return 40;
  if (l.includes("watch")) return 30;
  return 0;
}

export function rankDevTasks(tasks) {
  return tasks
    .map((t) => ({ t, s: scoreDevTask(t) }))
    .filter((x) => x.s > 0)
    .sort((a, b) => b.s - a.s)
    .map((x) => x.t);
}
