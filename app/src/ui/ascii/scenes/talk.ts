// Organic scenes, flowing, things passing between voices.

import { clamp01, fbm, hash, noise } from "../noise";
import { TAU, dirGlyph, inked, plain, type MakeScene } from "./core";

/** Drifting cloud bands. */
const clouds = plain((u, v, t, _x, _y, out) => {
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
});

/** Rings spreading from two slow wandering centres, interfering. */
const ripples = plain((u, v, t, _x, _y, out) => {
  const ax = Math.sin(t * 0.13) * 0.9, ay = Math.cos(t * 0.17) * 0.22;
  const bx = Math.cos(t * 0.11 + 2) * 1.1, by = Math.sin(t * 0.19 + 1) * 0.25;
  const ra = Math.hypot(u - ax, (v - ay) * 1.6);
  const rb = Math.hypot(u - bx, (v - by) * 1.6);
  out.v = clamp01((Math.sin(ra * 16 - t * 2.4) * Math.exp(-ra * 1.1) + Math.sin(rb * 13 - t * 2) * Math.exp(-rb * 1.3)) * 0.8);
  out.ch = null;
});

/** Streaks combed by a slow current; glyphs follow it. */
const current = plain((u, v, t, _x, _y, out) => {
  const a = noise(u * 0.4 + t * 0.04, v * 0.4, 21) * TAU;
  const along = u * Math.cos(a) + v * 2 * Math.sin(a);
  const streak = Math.pow(0.5 + 0.5 * Math.sin(along * 7 - t * 2.5), 4);
  out.v = clamp01(streak * (noise(u * 0.8 - t * 0.12, v * 0.8 + t * 0.08, 8) * 1.6 - 0.35));
  out.ch = out.v > 0.25 ? dirGlyph(a) : null;
});

/** A slow spiral turning around a drifting eye. */
const vortex = plain((u, v, t, _x, _y, out) => {
  const du = u - Math.sin(t * 0.07) * 0.5, dv = v * 2;
  const r = Math.hypot(du, dv);
  const arms = 0.5 + 0.5 * Math.sin(Math.atan2(dv, du) * 3 + Math.log(r + 0.05) * 5 - t * 1.2);
  out.v = clamp01(Math.pow(arms, 3) * Math.exp(-r * 0.9) * 1.6 + Math.exp(-r * 9) * 0.6);
  out.ch = null;
});

/** Contour lines of a slowly breathing terrain. */
const contours = plain((u, v, t, _x, _y, out) => {
  const h = noise(u * 0.6 + t * 0.03, v * 1.2 - t * 0.02, 31) * 0.8 + noise(u * 1.4, v * 2.8, 32) * 0.2;
  const f = (h * 6 + t * 0.12) % 1;
  out.v = clamp01(Math.max(0, 1 - Math.abs(f - 0.5) * 5) * (0.4 + h * 0.8));
  out.ch = null;
});

/** Falling streams, each column at its own pace, with fading tails. */
const rain = plain((_u, _v, t, x, y, out) => {
  const len = 6 + hash(x, 1, 41) * 14;
  const d = (t * (4 + hash(x, 0, 41) * 10) + hash(x, 2, 41) * 200) % (len + 40) - y;
  const on = hash(x, 3, 41) < 0.45 && d >= 0 && d < len;
  out.v = on ? clamp01(1 - d / len) : 0;
  out.ch = on && d < 1 ? "|" : null;
});

/** Oscilloscope traces: carriers modulated by slow envelopes. */
const signals = plain((u, v, t, _x, _y, out) => {
  let best = 0;
  for (let k = 0; k < 4; k++) {
    const y0 = -0.36 + k * 0.24;
    const env = 0.5 + 0.5 * Math.sin(u * 0.7 + t * (0.4 + k * 0.13) + k);
    const y = y0 + 0.07 * env * Math.sin(u * (6 + k * 3) - t * (2 + k));
    best = Math.max(best, Math.exp(-Math.pow((v - y) / 0.018, 2)) * (0.45 + env * 0.55));
  }
  out.v = clamp01(best);
  out.ch = best > 0.6 ? "~" : null;
});

/** Nodes drifting, linked to their neighbours, packets running the links. */
const network = inked((ink, t, g) => {
  const n = 9;
  const pts: [number, number][] = [];
  for (let i = 0; i < n; i++) {
    pts.push([
      g.cols * (0.08 + 0.84 * noise(i * 3.1, t * 0.05, 61)),
      g.rows * (0.12 + 0.76 * noise(i * 2.7 + 9, t * 0.05, 62)),
    ]);
  }
  for (let i = 0; i < n; i++) {
    const j = (i + 1 + (i % 3)) % n;
    const [x0, y0] = pts[i], [x1, y1] = pts[j];
    ink.line(x0, y0, x1, y1, 0.22, ".");
    const p = (t * 0.35 + i * 0.37) % 1;
    for (let k = 0; k < 3; k++) {
      const q = p - k * 0.025;
      if (q > 0) ink.plot(x0 + (x1 - x0) * q, y0 + (y1 - y0) * q, 0.95 - k * 0.25, k ? "=" : "*");
    }
  }
  for (const [x, y] of pts) ink.plot(x, y, 1, "O");
});

/** A flock: points on drifting orbits, leaving short trails. */
const murmuration = inked((ink, t, g) => {
  for (let i = 0; i < 70; i++) {
    const s = t * (0.18 + hash(i, 0, 71) * 0.1) + hash(i, 1, 71) * TAU;
    const cx = 0.5 + 0.28 * Math.sin(t * 0.11) + 0.12 * fbm(i * 0.3, t * 0.2, 72);
    const cy = 0.5 + 0.18 * Math.cos(t * 0.13) + 0.1 * fbm(i * 0.3 + 5, t * 0.2, 73);
    const r = 0.06 + 0.16 * hash(i, 2, 71);
    ink.plot(g.cols * (cx + r * Math.cos(s) * 1.6), g.rows * (cy + r * Math.sin(s * 1.3)), 0.9, null);
  }
}, 0.82);

/** Curtains of light hanging and swaying. */
const aurora = plain((u, v, t, _x, _y, out) => {
  let best = 0;
  for (let k = 0; k < 3; k++) {
    const c = Math.sin(v * 3 + t * 0.3 + k * 2) * 0.25 + (k - 1) * 0.7 + fbm(v * 2 + k, t * 0.15, 80 + k) * 0.4;
    const fade = clamp01(1 - Math.abs(v + 0.1) * 1.6);
    best = Math.max(best, Math.exp(-Math.pow((u - c) / 0.12, 2)) * fade);
  }
  out.v = clamp01(best);
  out.ch = best > 0.5 ? ":" : null;
});

/** Layered dunes rolling under a low horizon. */
const tide = plain((u, v, t, _x, _y, out) => {
  out.v = 0;
  out.ch = null;
  for (let k = 0; k < 4; k++) {
    const crest = 0.05 + k * 0.11 + 0.05 * Math.sin(u * (1.2 + k * 0.6) + t * (0.3 + k * 0.1) + k) + 0.03 * fbm(u * 2 + k, t * 0.1, 90 + k);
    if (v > crest) {
      out.v = 0.2 + k * 0.2;
      out.ch = v - crest < 0.03 ? (k % 2 ? "~" : "_") : null;
    }
  }
});

/** Messages writing themselves, left and right, scrolling up. */
const chatter = inked((ink, t, g) => {
  const scroll = t * 1.2;
  const first = Math.floor(scroll);
  for (let r = -1; r <= g.rows; r++) {
    const line = first + r;
    const msg = Math.floor(line / 3);
    if (((line % 3) + 3) % 3 === 2) continue; // a blank line between messages
    const y = r - (scroll - first);
    const right = ((msg % 2) + 2) % 2 === 1;
    const width = Math.floor(g.cols * (0.16 + hash(msg, 0, 95) * 0.24));
    const x0 = right ? g.cols - width - Math.floor(g.cols * 0.06) : Math.floor(g.cols * 0.06);
    // A message types itself in as it rises from the bottom.
    const typed = width * clamp01((g.rows - y) / 5);
    const wordLen = 3 + Math.floor(hash(msg, 1, 95) * 4);
    for (let i = 0; i < typed; i++) {
      if (hash(Math.floor(i / wordLen), line, 96) < 0.2 || i % wordLen === wordLen - 1) continue;
      ink.plot(x0 + i, y, right ? 0.8 : 0.45, right ? "=" : "-");
    }
  }
});

export const TALK: MakeScene[] = [clouds, chatter, current, network, ripples, signals, contours, murmuration, vortex, aurora, rain, tide];
