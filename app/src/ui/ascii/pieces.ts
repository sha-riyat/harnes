// Text pieces: pure functions from play time to a string of monospace cells.
// Everything that lives in the interface is drawn by one of these.

import { NOISE, RAMP, clamp01, hash, noise } from "./noise";

const glyph = (v: number) => RAMP[Math.min(RAMP.length - 1, Math.max(0, Math.round(clamp01(v) * (RAMP.length - 1))))];
const noiseGlyph = (x: number, tick: number) => NOISE[Math.floor(hash(x, tick, 3) * NOISE.length)];

/** A strand that shimmers while something is thinking: a soft packet sweeping over a wave. */
export const thinking = (n = 12) => (t: number) => {
  const head = ((Math.sin(t * 1.3) + 1) / 2) * (n - 1);
  let s = "";
  for (let x = 0; x < n; x++) {
    const wave = 0.5 + 0.5 * Math.sin(x * 0.85 - t * 5.5);
    const near = Math.exp(-(((x - head) / 2.6) ** 2));
    const flick = (hash(x, Math.floor(t * 9)) - 0.5) * 0.18;
    s += glyph(0.12 + wave * 0.28 * (0.4 + near) + near * 0.55 + flick);
  }
  return s;
};

/** One cell for an agent's state: breathing while it works, blinking when it needs you. */
export const mark = (state: string, seed = 0) => (t: number) => {
  switch (state) {
    case "working":
      return glyph(0.35 + 0.6 * (0.5 + 0.5 * Math.sin(t * 3.2 + seed)));
    case "starting":
      return noiseGlyph(seed, Math.floor(t * 12));
    case "blocked":
      return Math.floor(t * 2) % 2 ? "!" : " ";
    case "exited":
      return "x";
    case "done":
      return "*";
    default:
      return "·";
  }
};

/**
 * A message travelling along `n` cells: faint track, a decaying tail, a bright head.
 * It runs `runs` times, then rests as an arrow.
 */
export const flow = (n: number, runs = 2, period = 1.4) => (t: number) => {
  if (t >= runs * period) return "·".repeat(Math.max(0, n - 1)) + ">";
  const head = ((t % period) / period) * (n + 4);
  let s = "";
  for (let x = 0; x < n; x++) {
    const d = head - x;
    s += d < 0 || d > 5 ? "·" : d < 1 ? "#" : glyph(0.85 - d * 0.16);
  }
  return s;
};

/** A connector between run steps: solid behind, flowing at the current one, faint ahead. */
export const wire = (n: number, mode: "done" | "now" | "todo") => (t: number) => {
  if (mode === "done") return "-".repeat(n);
  if (mode === "todo") return "·".repeat(n);
  let s = "";
  for (let x = 0; x < n; x++) s += glyph(0.15 + 0.75 * Math.max(0, Math.sin(x * 1.1 - t * 6)));
  return s;
};

/** A label that decodes from noise, then keeps one letter flickering now and then. */
export const decode = (label: string, settle = 0.7) => (t: number) => {
  let s = "";
  for (let i = 0; i < label.length; i++) {
    const shown = t >= settle * ((i + 1) / label.length);
    const tick = Math.floor(t * 14);
    const flicker = t > settle && i === Math.floor(t * 3) % (label.length + 8) && hash(i, tick) > 0.5;
    s += label[i] === " " || (shown && !flicker) ? label[i] : noiseGlyph(i, tick);
  }
  return s;
};

/** A slow horizontal band for narrow places (empty lists): `n` cells wide. */
export const drift = (n: number, seed = 0) => (t: number) => {
  let s = "";
  for (let x = 0; x < n; x++) s += glyph(noise(x * 0.18 + t * 0.6, t * 0.25, seed) * 0.9 - 0.1);
  return s;
};
