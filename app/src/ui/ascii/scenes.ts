// Scenes for the ASCII field. Each one fills `out` for a cell: a brightness
// (0..1) and, when the motif needs it, a specific glyph. Coordinates: u runs
// across (about ±aspect), v runs down (-0.5..0.5), t is play time in seconds.

import { clamp01, fbm, hash, noise } from "./noise";

export interface Cell {
  v: number;
  ch: string | null;
}

export type Scene = (u: number, v: number, t: number, x: number, y: number, out: Cell) => void;

const TAU = Math.PI * 2;
const DIR = "-\\|/";
const dirGlyph = (a: number) => DIR[((Math.round(a / (Math.PI / 4)) % 4) + 4) % 4];

/** Drifting cloud bands. */
const clouds: Scene = (u, v, t, _x, _y, out) => {
  let best = 0;
  for (let k = 0; k < 3; k++) {
    const y0 = -0.32 + k * 0.32 + 0.06 * Math.sin(t * 0.21 + k * 2.1);
    const warp = fbm(u * 1.4 + t * 0.1 + k * 5, v * 1.4 - t * 0.2, 11 + k);
    const sway = 0.08 * Math.sin(u * 1.1 + t * 0.3 + k) + warp * 0.22;
    const band = Math.exp(-Math.pow(Math.abs(v - y0 - sway) / (0.07 + 0.02 * k), 2));
    const puff = 0.35 + 0.9 * noise(u * 0.9 - t * (0.12 + k * 0.05), k * 3.3 + t * 0.04, 5 + k);
    best = Math.max(best, band * puff);
  }
  out.v = clamp01(best * 0.85);
  out.ch = null;
};

/** Rings spreading from two slow wandering centres, interfering. */
const ripples: Scene = (u, v, t, _x, _y, out) => {
  const ax = Math.sin(t * 0.13) * 0.9, ay = Math.cos(t * 0.17) * 0.22;
  const bx = Math.cos(t * 0.11 + 2) * 1.1, by = Math.sin(t * 0.19 + 1) * 0.25;
  const ra = Math.hypot(u - ax, (v - ay) * 1.6);
  const rb = Math.hypot(u - bx, (v - by) * 1.6);
  const w = Math.sin(ra * 16 - t * 2.4) * Math.exp(-ra * 1.1) + Math.sin(rb * 13 - t * 2) * Math.exp(-rb * 1.3);
  out.v = clamp01(w * 0.8);
  out.ch = null;
};

/** Streaks combed by a slow flow field; glyphs follow the current. */
const flow: Scene = (u, v, t, _x, _y, out) => {
  const a = noise(u * 0.4 + t * 0.04, v * 0.4, 21) * TAU;
  const along = u * Math.cos(a) + v * 2 * Math.sin(a);
  const streak = Math.pow(0.5 + 0.5 * Math.sin(along * 7 - t * 2.5), 4);
  const mask = noise(u * 0.8 - t * 0.12, v * 0.8 + t * 0.08, 8);
  out.v = clamp01(streak * (mask * 1.6 - 0.35));
  out.ch = out.v > 0.25 ? dirGlyph(a) : null;
};

/** A slow spiral: three arms turning around a drifting eye. */
const vortex: Scene = (u, v, t, _x, _y, out) => {
  const cx = Math.sin(t * 0.07) * 0.5;
  const du = u - cx, dv = v * 2;
  const r = Math.hypot(du, dv);
  const ang = Math.atan2(dv, du);
  const arms = 0.5 + 0.5 * Math.sin(ang * 3 + Math.log(r + 0.05) * 5 - t * 1.2);
  out.v = clamp01(Math.pow(arms, 3) * Math.exp(-r * 0.9) * 1.6 + Math.exp(-r * 9) * 0.6);
  out.ch = null;
};

/** Contour lines of a slowly breathing terrain. */
const contours: Scene = (u, v, t, _x, _y, out) => {
  const h = noise(u * 0.6 + t * 0.03, v * 1.2 - t * 0.02, 31) * 0.8 + noise(u * 1.4, v * 2.8, 32) * 0.2;
  const f = (h * 6 + t * 0.12) % 1;
  const line = Math.max(0, 1 - Math.abs(f - 0.5) * 5);
  out.v = clamp01(line * (0.4 + h * 0.8));
  out.ch = null;
};

/** Falling streams, each column at its own pace, with fading tails. */
const rain: Scene = (_u, _v, t, x, y, out) => {
  const speed = 4 + hash(x, 0, 41) * 10;
  const len = 6 + hash(x, 1, 41) * 14;
  const head = (t * speed + hash(x, 2, 41) * 200) % (len + 40);
  const d = head - y;
  const on = hash(x, 3, 41) < 0.45 && d >= 0 && d < len;
  out.v = on ? clamp01(1 - d / len) : 0;
  out.ch = on && d < 1 ? "|" : null;
};

/** Halftone: a lattice of dots sized by a soft moving field. */
const halftone: Scene = (u, v, t, x, y, out) => {
  const shift = (y >> 1) % 2 ? 2 : 0;
  const cell = (x + shift) % 4 === 0 && y % 2 === 0;
  const field = clamp01((noise(u * 0.9 - t * 0.12, v * 1.8 + t * 0.06, 51) - 0.2) * 1.5);
  const near = ((x + shift) % 4 === 1 || (x + shift) % 4 === 3) && y % 2 === 0;
  out.v = cell ? clamp01(field * 1.2) : near && field > 0.62 ? clamp01((field - 0.55) * 2) : 0;
  out.ch = cell ? (field > 0.7 ? "@" : field > 0.5 ? "o" : field > 0.3 ? "·" : null) : null;
};

export const SCENES: Scene[] = [clouds, flow, ripples, contours, vortex, rain, halftone];
export const EXTRA_GLYPHS = DIR + "|o@·";
