// The app's living ASCII field on a plain canvas: the same scenes, each
// dissolving into the next along a noisy front, scrambling under the pointer.
// It runs only while on screen and holds a still frame under reduced motion.

import { NOISE, clamp01, hash, noise, smooth } from "../../app/src/ui/ascii/noise";
import { SCENES, type Cell } from "../../app/src/ui/ascii/scenes";

const RAMP = " .·:-=+*#";
const HOLD = 9;
const FADE = 3;
const TRAIL = 0.7;
export const still = matchMedia("(prefers-reduced-motion: reduce)").matches;

export interface FieldOptions {
  cell?: number;
  density?: number;
  seed?: number;
  /** Indices into SCENES; all of them by default. */
  scenes?: number[];
  /** A vertical tone ramp in place of scenes: dark at the top, light at the bottom. */
  ramp?: boolean;
}

export function field(canvas: HTMLCanvasElement, o: FieldOptions = {}) {
  const ctx = canvas.getContext("2d")!;
  const seed = o.seed ?? 3;
  const density = o.density ?? 0.8;
  const cellH = o.cell ?? 14;
  const scenes = (o.scenes ?? SCENES.map((_, i) => i)).map((i) => SCENES[i % SCENES.length]());
  let cols = 0, rows = 0, cw = 0, ch = 0, color = "#fff", font = "monospace";
  const trail: { x: number; y: number; at: number }[] = [];
  const a: Cell = { v: 0, ch: null }, b: Cell = { v: 0, ch: null };
  let now = 0;

  const resize = () => {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    const w = canvas.clientWidth, h = canvas.clientHeight;
    cols = Math.max(8, Math.floor(w / (cellH * 0.6)));
    rows = Math.max(3, Math.floor(h / cellH));
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    cw = canvas.width / cols;
    ch = canvas.height / rows;
    const s = getComputedStyle(canvas);
    color = s.color;
    font = s.getPropertyValue("--mono") || "monospace";
  };

  const glyphAt = (x: number, y: number, t: number, k: number, mix: number, sa: (typeof scenes)[0], sb: (typeof scenes)[0], aspect: number) => {
    const u = (x / cols - 0.5) * aspect * 2;
    const v = y / rows - 0.5;
    if (o.ramp) {
      // A gradient written in characters: density climbs down the band.
      const tone = clamp01(y / (rows - 1) + (noise(x * 0.08 + t * 0.2, y * 0.2, seed) - 0.5) * 0.35);
      return { g: hash(x, y, Math.floor(t * 3) + seed) < tone ? RAMP[1 + Math.floor(tone * (RAMP.length - 2))] : "", tone };
    }
    const front = noise(u * 1.3 + k, v * 2.6, seed + 77) * 0.7 + hash(x, y, seed) * 0.3;
    const useB = mix > front;
    const cell = useB ? b : a;
    (useB ? sb : sa).cell(u, v, t, x, y, cell);
    const edge = mix > 0 && mix < 1 && Math.abs(mix - front) < 0.06;
    const tone = clamp01(Math.pow(cell.v, 0.9) + (hash(x, y, Math.floor(t * 9) + seed) - 0.5) * 0.14);
    let g = "";
    if (cell.v > 0.07 && hash(x, y, seed + 1) < density * (0.35 + tone * 0.75)) g = cell.ch ?? RAMP[Math.round(tone * (RAMP.length - 1))];
    let scramble = edge ? 0.9 : 0;
    for (const p of trail) {
      const life = 1 - (t - p.at) / TRAIL;
      if (life > 0) scramble = Math.max(scramble, Math.exp(-(((x - p.x) / 6) ** 2 + ((y - p.y) / 3) ** 2)) * life * life);
    }
    const fast = Math.floor(t * 20);
    if ((g || edge) && scramble > 0.02 && hash(x, y, fast) < scramble * 1.25) g = NOISE[Math.floor(hash(y, x, fast) * NOISE.length)];
    return { g: g === " " ? "" : g, tone: edge ? 0.6 : tone };
  };

  const frame = (t: number) => {
    now = t;
    const aspect = cols / rows / 2;
    const cycle = t / (HOLD + FADE);
    const k = Math.floor(cycle);
    const sa = scenes[k % scenes.length], sb = scenes[(k + 1) % scenes.length];
    const mix = smooth(HOLD / (HOLD + FADE), 1, cycle - k);
    const grid = { cols, rows, aspect };
    if (!o.ramp) {
      sa.prepare?.(t, grid);
      if (mix > 0) sb.prepare?.(t, grid);
    }
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = color;
    ctx.font = `300 ${Math.round(ch * 0.8)}px ${font}`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (let y = 0; y < rows; y++)
      for (let x = 0; x < cols; x++) {
        const { g, tone } = glyphAt(x, y, t, k, mix, sa, sb, aspect);
        if (!g) continue;
        ctx.globalAlpha = 0.18 + tone * 0.72;
        ctx.fillText(g, (x + 0.5) * cw, (y + 0.5) * ch);
      }
    while (trail.length && t - trail[0].at > TRAIL) trail.shift();
  };

  canvas.addEventListener("pointermove", (e) => {
    const r = canvas.getBoundingClientRect();
    trail.push({ x: ((e.clientX - r.left) / r.width) * cols, y: ((e.clientY - r.top) / r.height) * rows, at: now });
    if (trail.length > 24) trail.shift();
  });

  resize();
  new ResizeObserver(() => { resize(); frame(now); }).observe(canvas);
  const start = 20 + seed * 7; // open on a formed motif
  if (still) return frame(start);
  let visible = false, last = 0, t0 = performance.now();
  new IntersectionObserver(([e]) => (visible = e.isIntersecting)).observe(canvas);
  const loop = (ms: number) => {
    requestAnimationFrame(loop);
    if (!visible || document.hidden || ms - last < 50) return;
    last = ms;
    frame(start + (ms - t0) / 1000);
  };
  requestAnimationFrame(loop);
}
