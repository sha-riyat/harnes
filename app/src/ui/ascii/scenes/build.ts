// Geometric scenes, constructive, things being drawn and composed.

import { clamp01, hash, noise } from "../noise";
import { TAU, inked, plain, type MakeScene } from "./core";

/** Halftone: a staggered lattice of dots sized by a soft moving field. */
const halftone = plain((u, v, t, x, y, out) => {
  const shift = (y >> 1) % 2 ? 2 : 0;
  const cell = (x + shift) % 4 === 0 && y % 2 === 0;
  const field = clamp01((noise(u * 0.9 - t * 0.12, v * 1.8 + t * 0.06, 51) - 0.2) * 1.5);
  out.v = cell ? clamp01(field * 1.2) : 0;
  out.ch = cell ? (field > 0.7 ? "@" : field > 0.5 ? "o" : field > 0.3 ? "·" : null) : null;
});

/** A blueprint: a faint grid, and layout boxes being drawn edge by edge. */
const blueprint = inked((ink, t, g) => {
  for (let y = 0; y < g.rows; y += 4) for (let x = 0; x < g.cols; x += 8) ink.plot(x, y, 0.2, "+");
  const era = Math.floor(t / 5);
  const p = (t % 5) / 3.5;
  for (let k = 0; k < 6; k++) {
    const w = 8 + Math.floor(hash(k, era, 101) * g.cols * 0.3);
    const h = 3 + Math.floor(hash(k, era, 102) * g.rows * 0.4);
    const x0 = Math.floor(hash(k, era, 103) * (g.cols - w));
    const y0 = Math.floor(hash(k, era, 104) * (g.rows - h));
    // Walk the perimeter up to this box's share of the drawing time.
    const per = 2 * (w + h);
    const drawn = per * clamp01(p * 1.4 - k * 0.12);
    for (let i = 0; i < drawn; i++) {
      const [x, y, c] = i < w ? [x0 + i, y0, "-"] : i < w + h ? [x0 + w, y0 + i - w, "|"] : i < 2 * w + h ? [x0 + w - (i - w - h), y0 + h, "-"] : [x0, y0 + h - (i - 2 * w - h), "|"];
      ink.plot(x, y, 0.85, c as string);
    }
    if (drawn >= per) for (const [cx, cy] of [[x0, y0], [x0 + w, y0], [x0, y0 + h], [x0 + w, y0 + h]]) ink.plot(cx, cy, 1, "+");
  }
});

/** Moiré: two fine gratings turning against each other. */
const moire = plain((u, v, t, _x, _y, out) => {
  const a = t * 0.05, b = -t * 0.037 + 0.4;
  const g1 = Math.sin((u * Math.cos(a) + v * 2 * Math.sin(a)) * 9);
  const g2 = Math.sin((u * Math.cos(b) + v * 2 * Math.sin(b)) * 9);
  out.v = clamp01(g1 * g2 * 1.1 - 0.1);
  out.ch = null;
});

/** Truchet tiles: diagonals that flip in a travelling wave. */
const truchet = plain((u, _v, t, x, y, out) => {
  const tx = x >> 1, ty = y;
  const flip = hash(tx, ty, 111) + 0.35 * Math.sin(u * 2 - t * 0.8) > 0.5;
  const diag = flip ? (x % 2 === 0 ? "/" : " ") : x % 2 === 1 ? "\\" : " ";
  const lit = 0.35 + 0.5 * noise(tx * 0.2 - t * 0.2, ty * 0.3, 112);
  out.v = diag === " " ? 0 : lit;
  out.ch = diag === " " ? null : diag;
});

/** Bauhaus: a circle, a square and a bar sliding and overlapping (xor). */
const bauhaus = plain((u, v, t, _x, _y, out) => {
  const cx = Math.sin(t * 0.21) * 0.9, cy = Math.cos(t * 0.17) * 0.12;
  const inCircle = Math.hypot(u - cx, (v - cy) * 2) < 0.42;
  const sx = Math.cos(t * 0.15) * 1.0, sy = Math.sin(t * 0.23) * 0.1;
  const inSquare = Math.abs(u - sx) < 0.38 && Math.abs(v - sy) * 2 < 0.38;
  const inBar = Math.abs(v - 0.28 * Math.sin(t * 0.11)) < 0.04;
  const n = Number(inCircle) + Number(inSquare) + Number(inBar);
  out.v = n % 2 ? 0.75 : n ? 0.25 : 0;
  out.ch = null;
});

/** A torus turning in light, after donut.c. */
const torus = inked((ink, t, g) => {
  const A = t * 0.6, B = t * 0.35;
  const [cA, sA, cB, sB] = [Math.cos(A), Math.sin(A), Math.cos(B), Math.sin(B)];
  const k1 = g.rows * 1.15;
  const z = new Float32Array(g.cols * g.rows);
  for (let th = 0; th < TAU; th += 0.09) {
    const [ct, st] = [Math.cos(th), Math.sin(th)];
    for (let ph = 0; ph < TAU; ph += 0.03) {
      const [cp, sp] = [Math.cos(ph), Math.sin(ph)];
      const cx = 2 + ct;
      const x = cx * (cB * cp + sA * sB * sp) - st * cA * sB;
      const y = cx * (sB * cp - sA * cB * sp) + st * cA * cB;
      const zz = 1 / (6 + cA * cx * sp + st * sA);
      const xp = Math.floor(g.cols / 2 + k1 * 2 * zz * x);
      const yp = Math.floor(g.rows / 2 - k1 * zz * y);
      const L = cp * ct * sB - cA * ct * sp - sA * st + cB * (cA * st - ct * sA * sp);
      if (xp < 0 || yp < 0 || xp >= g.cols || yp >= g.rows) continue;
      const i = yp * g.cols + xp;
      if (zz > z[i] && L > 0) {
        z[i] = zz;
        ink.v[i] = clamp01(L / 1.4 + 0.15);
      }
    }
  }
});

/** A pen plotter tracing Lissajous figures, the ink slowly fading. */
const plotter = inked((ink, t, g) => {
  const a = 3 + (Math.floor(t / 9) % 3), b = 2 + (Math.floor(t / 9) % 2), d = t * 0.2;
  for (let k = 0; k < 90; k++) {
    const s = t * 1.4 - k * 0.012;
    ink.plot(g.cols * (0.5 + 0.42 * Math.sin(a * s + d)), g.rows * (0.5 + 0.42 * Math.sin(b * s)), k === 0 ? 1 : 0.85, k === 0 ? "@" : null);
  }
}, 0.985);

/** A layout grid: twelve columns, baselines, and text blocks snapping in. */
const layout = inked((ink, t, g) => {
  const col = g.cols / 12;
  for (let c = 0; c <= 12; c++) for (let y = 0; y < g.rows; y += 2) ink.plot(c * col, y, 0.18, ":");
  const era = Math.floor(t / 4);
  const p = clamp01((t % 4) / 1.2);
  for (let k = 0; k < 5; k++) {
    const span = 2 + Math.floor(hash(k, era, 121) * 5);
    const c0 = Math.floor(hash(k, era, 122) * (12 - span));
    const y0 = 1 + Math.floor(hash(k, era, 123) * (g.rows - 6));
    const lines = 2 + Math.floor(hash(k, era, 124) * 4);
    const x0 = Math.round(c0 * col) + 1, x1 = Math.round((c0 + span) * col) - 1;
    for (let l = 0; l < lines; l++) {
      const end = x0 + (x1 - x0) * clamp01(p * 1.3 - l * 0.12) * (l === lines - 1 ? 0.6 : 1);
      for (let x = x0; x < end; x++) ink.plot(x, y0 + l, l === 0 ? 0.9 : 0.5, l === 0 ? "=" : "-");
    }
  }
});

/** A type specimen: big letterforms, rasterised once, breathing in tone. */
function specimen(): MakeScene {
  return () => {
    const LETTERS = ["Aa", "Gg", "&", "Rr", "Qq", "?"];
    let cover = new Float32Array(0);
    let key = "";
    let cols = 0;
    return {
      prepare(t, g) {
        const letter = LETTERS[Math.floor(t / 6) % LETTERS.length];
        const k = `${letter}:${g.cols}x${g.rows}`;
        cols = g.cols;
        if (k === key) return;
        key = k;
        const c = document.createElement("canvas");
        c.width = g.cols;
        c.height = g.rows * 2;
        const x = c.getContext("2d")!;
        x.fillStyle = "#fff";
        x.textAlign = "center";
        x.textBaseline = "middle";
        x.font = `700 ${Math.floor(c.height * 0.95)}px Georgia, "Times New Roman", serif`;
        x.setTransform(1, 0, 0, 1, 0, 0);
        x.fillText(letter, c.width / 2, c.height / 2);
        const d = x.getImageData(0, 0, c.width, c.height).data;
        cover = new Float32Array(g.cols * g.rows);
        for (let yy = 0; yy < g.rows; yy++) for (let xx = 0; xx < g.cols; xx++) cover[yy * g.cols + xx] = (d[(yy * 2 * c.width + xx) * 4 + 3] + d[((yy * 2 + 1) * c.width + xx) * 4 + 3]) / 510;
      },
      cell(u, v, t, x, y, out) {
        const c = cover[y * cols + x] ?? 0;
        out.v = c > 0.05 ? clamp01(c * (0.5 + 0.5 * noise(u * 1.5 + t * 0.2, v * 3, 131))) : 0;
        out.ch = null;
      },
    };
  };
}

/** Swatches: tonal steps sliding across, like a palette being built. */
const swatches = plain((u, v, t, x, _y, out) => {
  const col = Math.floor((x + t * 6) / 10);
  const step = (col % 7) / 6;
  const band = Math.abs(v) < 0.18 + 0.04 * Math.sin(col + t);
  out.v = band ? clamp01(0.15 + step * 0.8) : hash(col, 0, 141) < 0.15 && Math.abs(v) < 0.3 ? 0.15 : 0;
  out.ch = band && (x + Math.floor(t * 6)) % 10 === 0 ? "|" : null;
  void u;
});

/** Phyllotaxis: seeds at the golden angle, the head turning slowly. */
const phyllotaxis = inked((ink, t, g) => {
  const n = 260, golden = Math.PI * (3 - Math.sqrt(5));
  const grow = 1 + 0.08 * Math.sin(t * 0.4);
  for (let i = 0; i < n; i++) {
    const r = Math.sqrt(i / n) * 0.48 * grow;
    const a = i * golden + t * 0.15;
    ink.plot(g.cols / 2 + Math.cos(a) * r * g.rows * 2, g.rows / 2 + Math.sin(a) * r * g.rows, 0.3 + 0.7 * (1 - i / n), i % 13 === 0 ? "o" : null);
  }
});

export const BUILD: MakeScene[] = [blueprint, halftone, torus, truchet, specimen(), moire, plotter, bauhaus, layout, phyllotaxis, swatches];
