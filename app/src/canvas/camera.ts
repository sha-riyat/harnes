// Camera maths for the design canvas. Pure functions, no DOM.

export type Camera = { x: number; y: number; z: number };
export type Box = { x: number; y: number; w: number; h: number };

export const MIN_Z = 0.15;
export const MAX_Z = 2;
const LIMIT = 100_000;

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
export const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

export function safeCamera(c: Camera): Camera {
  return { x: clamp(c.x, -LIMIT, LIMIT), y: clamp(c.y, -LIMIT, LIMIT), z: clamp(c.z, MIN_Z, MAX_Z) };
}

/** Zoom to `z` while the world point under screen point (px, py) stays put. */
export function zoomAt(c: Camera, z: number, px: number, py: number): Camera {
  const nz = clamp(z, MIN_Z, MAX_Z);
  const wx = (px - c.x) / c.z;
  const wy = (py - c.y) / c.z;
  return safeCamera({ x: px - wx * nz, y: py - wy * nz, z: nz });
}

/** Camera that shows every box with a margin, centred in a w x h viewport. */
export function fit(boxes: Box[], w: number, h: number, margin = 48): Camera {
  if (!boxes.length || w <= 0 || h <= 0) return { x: 0, y: 0, z: 1 };
  const x0 = Math.min(...boxes.map((b) => b.x));
  const y0 = Math.min(...boxes.map((b) => b.y));
  const x1 = Math.max(...boxes.map((b) => b.x + b.w));
  const y1 = Math.max(...boxes.map((b) => b.y + b.h));
  const z = clamp(Math.min((w - margin * 2) / (x1 - x0), (h - margin * 2) / (y1 - y0)), MIN_Z, 1);
  return safeCamera({ z, x: (w - (x1 - x0) * z) / 2 - x0 * z, y: (h - (y1 - y0) * z) / 2 - y0 * z });
}

/** Free space to the right of the last box, for an item added later. */
export function nextSlot(boxes: Box[], w: number, gap = 160): { x: number; y: number } {
  const right = boxes.length ? Math.max(...boxes.map((b) => b.x + b.w)) : -gap;
  return { x: right + gap, y: boxes.length ? Math.min(...boxes.map((b) => b.y)) : 0 };
}
