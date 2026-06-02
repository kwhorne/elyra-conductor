// Pure helpers for the split-pane layout tree.
//
// A node is either:
//   leaf:  { kind: 'leaf', id, termId, cwd, title, key, runOnce }
//   split: { kind: 'split', id, dir: 'row' | 'col', ratio, a, b }
//
// 'row' splits side-by-side (vertical divider), 'col' stacks (horizontal divider).

/**
 * Flatten the tree into absolutely-positioned leaves (in %), plus the list of
 * dividers with the region they control (used for drag-to-resize).
 */
export function geometry(root) {
  const leaves = [];
  const dividers = [];

  function walk(node, x, y, w, h) {
    if (node.kind === "leaf") {
      leaves.push({
        termId: node.termId,
        cwd: node.cwd,
        title: node.title,
        key: node.key,
        runOnce: node.runOnce,
        rect: { x, y, w, h },
      });
      return;
    }
    const r = node.ratio;
    if (node.dir === "row") {
      const wa = w * r;
      walk(node.a, x, y, wa, h);
      walk(node.b, x + wa, y, w - wa, h);
      dividers.push({ id: node.id, dir: "row", pos: x + wa, rect: { x, y, w, h } });
    } else {
      const ha = h * r;
      walk(node.a, x, y, w, ha);
      walk(node.b, x, y + ha, w, h - ha);
      dividers.push({ id: node.id, dir: "col", pos: y + ha, rect: { x, y, w, h } });
    }
  }

  if (root) walk(root, 0, 0, 100, 100);
  return { leaves, dividers };
}

/** Replace the leaf matching termId with a split of [leaf, newLeaf]. */
export function splitLeaf(node, termId, dir, newLeaf, splitId) {
  if (node.kind === "leaf") {
    if (node.termId === termId) {
      return { kind: "split", id: splitId, dir, ratio: 0.5, a: node, b: newLeaf };
    }
    return node;
  }
  return {
    ...node,
    a: splitLeaf(node.a, termId, dir, newLeaf, splitId),
    b: splitLeaf(node.b, termId, dir, newLeaf, splitId),
  };
}

/** Remove the leaf with termId; collapse the parent split to its sibling. */
export function removeLeaf(node, termId) {
  if (node.kind === "leaf") return node.termId === termId ? null : node;
  const a = removeLeaf(node.a, termId);
  const b = removeLeaf(node.b, termId);
  if (a === null) return b;
  if (b === null) return a;
  if (a === node.a && b === node.b) return node;
  return { ...node, a, b };
}

/** Update a split node's ratio. */
export function setRatio(node, id, ratio) {
  if (node.kind === "leaf") return node;
  if (node.id === id) return { ...node, ratio };
  return {
    ...node,
    a: setRatio(node.a, id, ratio),
    b: setRatio(node.b, id, ratio),
  };
}

/** Left-most leaf (used to pick a new focus after closing a pane). */
export function firstLeaf(node) {
  return node.kind === "leaf" ? node : firstLeaf(node.a);
}

/** All leaves in the tree (left-to-right). */
export function allLeaves(node) {
  if (!node) return [];
  if (node.kind === "leaf") return [node];
  return [...allLeaves(node.a), ...allLeaves(node.b)];
}
