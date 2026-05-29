// Folders/files that are almost always noise in a project tree. Hidden by
// default; the "show all" toggle reveals them. Other dotfiles (.env, .gitignore)
// stay visible so "all files" still mostly holds.
const NOISE = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  ".next",
  ".nuxt",
  ".svelte-kit",
  ".turbo",
  ".cache",
  "vendor",
  ".DS_Store",
]);

export function filterEntries(entries, showAll) {
  return showAll ? entries : entries.filter((e) => !NOISE.has(e.name));
}
