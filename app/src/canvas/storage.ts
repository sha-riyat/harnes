// Saved layout: camera and positions by registry id. Nothing else — items,
// names and sizes always come from the source registry.

import { clamp, finite, MAX_Z, MIN_Z, type Camera } from "./camera";

const KEY = "harnes.canvas.design";
const LIMIT = 100_000;
const MAX_ITEMS = 200;
const MAX_BYTES = 20_000;

export type Layout = { camera?: Camera; pos: Record<string, [number, number]> };

export function load(known: Set<string>, rev: number): Layout {
  const out: Layout = { pos: {} };
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw || raw.length > MAX_BYTES) return out;
    const doc = JSON.parse(raw);
    if (doc?.v !== 1 || doc.rev !== rev) return out; // an older source layout: start from the new one
    const c = doc.camera;
    if (c && finite(c.x) && finite(c.y) && finite(c.z)) {
      out.camera = { x: clamp(c.x, -LIMIT, LIMIT), y: clamp(c.y, -LIMIT, LIMIT), z: clamp(c.z, MIN_Z, MAX_Z) };
    }
    for (const [id, p] of Object.entries(doc.pos ?? {}).slice(0, MAX_ITEMS)) {
      const [x, y] = p as [unknown, unknown];
      if (known.has(id) && finite(x) && finite(y)) out.pos[id] = [clamp(x, -LIMIT, LIMIT), clamp(y, -LIMIT, LIMIT)];
    }
  } catch {
    // Storage unavailable or malformed: start from the source layout.
  }
  return out;
}

export function save(layout: Layout, rev: number) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ v: 1, rev, ...layout }));
  } catch {
    // Quota or private window: the canvas still works, it just forgets.
  }
}
