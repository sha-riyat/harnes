// A living field of characters on a canvas. It moves through several motifs
// (clouds, currents, ripples, contours, a vortex, rain, halftone), holding
// each for a while, then dissolving into the next along a noisy front.
// It decodes in on arrival and scrambles under the pointer. Monochrome: it
// takes the canvas' text colour. Glyphs come from a small atlas and only
// changed cells are redrawn.

import { onCleanup, onMount } from "solid-js";
import { animate } from "./clock";
import { NOISE, clamp01, hash, noise, smooth } from "./noise";
import { EXTRA_GLYPHS, SCENES, type Cell } from "./scenes";

/** Lighter ramp than the text pieces: the field should whisper. */
const RAMP = " .·:-=+*#";
const GLYPHS = [...new Set(RAMP + NOISE + EXTRA_GLYPHS)].join("");
const SLOT = new Map([...GLYPHS].map((g, i) => [g, i]));
const LEVELS = [0.2, 0.38, 0.6, 0.9];
const INTRO = 1.8;
const TRAIL = 0.7;
const HOLD = 13;
const FADE = 3.5;

export default function AsciiField(props: { height?: number; fill?: boolean; cell?: number; density?: number; seed?: number; class?: string }) {
  let canvas!: HTMLCanvasElement;

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    const seed = props.seed ?? 3;
    const density = props.density ?? 0.8;
    const cellH = props.cell ?? 12;
    const atlas = document.createElement("canvas");
    // The default field opens on clouds; a seeded one starts elsewhere.
    const first = props.seed ? Math.floor(hash(seed, 9) * SCENES.length) : 0;
    let cols = 0, rows = 0, cw = 0, ch = 0, dpr = 1;
    let prev = new Int16Array(0);
    const trail: { x: number; y: number; at: number }[] = [];
    const a: Cell = { v: 0, ch: null };
    const b: Cell = { v: 0, ch: null };
    let now = 0;

    const buildAtlas = () => {
      const style = getComputedStyle(canvas);
      const font = style.getPropertyValue("--mono") || "monospace";
      const gw = Math.ceil(cw), gh = Math.ceil(ch);
      atlas.width = gw * GLYPHS.length;
      atlas.height = gh * LEVELS.length;
      const g = atlas.getContext("2d")!;
      g.clearRect(0, 0, atlas.width, atlas.height);
      g.fillStyle = style.color;
      g.font = `300 ${Math.round(ch * 0.8)}px ${font}`;
      g.textAlign = "center";
      g.textBaseline = "middle";
      LEVELS.forEach((alpha, l) => {
        g.globalAlpha = alpha;
        [...GLYPHS].forEach((c, i) => g.fillText(c, (i + 0.5) * gw, (l + 0.5) * gh));
      });
      prev.fill(-1);
    };

    const resize = () => {
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      const w = canvas.clientWidth;
      const h = props.fill ? canvas.parentElement?.clientHeight ?? 200 : props.height ?? 160;
      cols = Math.max(8, Math.floor(w / (cellH * 0.6)));
      rows = Math.max(3, Math.floor(h / cellH));
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(rows * cellH * dpr);
      canvas.style.height = `${rows * cellH}px`;
      cw = canvas.width / cols;
      ch = canvas.height / rows;
      prev = new Int16Array(cols * rows).fill(-1);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      buildAtlas();
    };

    const frame = (t: number) => {
      now = t;
      const aspect = cols / rows / 2;
      const tick = Math.floor(t * 9);
      const fast = Math.floor(t * 20);
      const intro = clamp01(t / INTRO);
      const cycle = t / (HOLD + FADE);
      const k = Math.floor(cycle);
      const sa = SCENES[(first + k) % SCENES.length];
      const sb = SCENES[(first + k + 1) % SCENES.length];
      // 0 while holding, then 0..1 across the dissolve.
      const mix = smooth(HOLD / (HOLD + FADE), 1, cycle - k);
      const gw = Math.ceil(cw), gh = Math.ceil(ch);
      for (let y = 0; y < rows; y++) {
        for (let x = 0; x < cols; x++) {
          const u = (x / cols - 0.5) * aspect * 2;
          const v = y / rows - 0.5;
          const i = y * cols + x;
          // The dissolve front is shaped by noise, so motifs give way in patches.
          const front = noise(u * 1.3 + k, v * 2.6, seed + 77) * 0.7 + hash(x, y, seed) * 0.3;
          const useB = mix > front;
          const cell = useB ? b : a;
          (useB ? sb : sa)(u, v, t, x, y, cell);
          const edge = mix > 0 && mix < 1 && Math.abs(mix - front) < 0.06;
          const wobble = (hash(x, y, tick + seed) - 0.5) * 0.14;
          const tone = clamp01(Math.pow(cell.v, 0.9) + wobble);
          let glyph = "";
          if (cell.v > 0.07 && hash(x, y, seed + 1) < density * (0.35 + tone * 0.75)) glyph = cell.ch ?? RAMP[Math.round(tone * (RAMP.length - 1))];
          // Arrival: cells decode outwards from the middle.
          const order = Math.hypot(u / aspect, v * 2) / 1.4;
          const reveal = smooth(0, 1, (intro * 1.45 - order) / 0.45);
          let scramble = Math.max(1 - reveal, edge ? 0.9 : 0);
          for (const p of trail) {
            const life = 1 - (t - p.at) / TRAIL;
            if (life <= 0) continue;
            const d2 = ((x - p.x) / 6) ** 2 + ((y - p.y) / 3) ** 2;
            scramble = Math.max(scramble, Math.exp(-d2) * life * life);
          }
          if ((glyph || edge) && scramble > 0.02 && hash(x, y, fast) < scramble * 1.25) glyph = NOISE[Math.floor(hash(y, x, fast) * NOISE.length)];
          if (reveal <= 0.001 || glyph === " ") glyph = "";
          const slot = glyph ? SLOT.get(glyph) ?? -1 : -1;
          const level = Math.min(LEVELS.length - 1, Math.floor((edge ? 0.6 : tone) * LEVELS.length));
          const key = slot < 0 ? -1 : slot * LEVELS.length + level;
          if (key === prev[i]) continue;
          prev[i] = key;
          const x0 = Math.round(x * cw), y0 = Math.round(y * ch);
          const x1 = Math.round((x + 1) * cw), y1 = Math.round((y + 1) * ch);
          ctx.clearRect(x0, y0, x1 - x0, y1 - y0);
          if (key >= 0) ctx.drawImage(atlas, slot * gw, level * gh, x1 - x0, y1 - y0, x0, y0, x1 - x0, y1 - y0);
        }
      }
      while (trail.length && t - trail[0].at > TRAIL) trail.shift();
    };

    const onMove = (e: PointerEvent) => {
      const r = canvas.getBoundingClientRect();
      trail.push({ x: ((e.clientX - r.left) / r.width) * cols, y: ((e.clientY - r.top) / r.height) * rows, at: now });
      if (trail.length > 24) trail.shift();
    };

    resize();
    const anim = animate(canvas, 20, frame, INTRO + 4);
    const ro = new ResizeObserver(() => {
      const h = props.fill ? canvas.parentElement?.clientHeight ?? 0 : 0;
      if (Math.round(canvas.clientWidth * dpr) !== canvas.width || (props.fill && Math.floor(h / cellH) !== rows)) {
        resize();
        anim.redraw();
      }
    });
    ro.observe(canvas);
    if (props.fill && canvas.parentElement) ro.observe(canvas.parentElement);
    // Theme change: redraw the atlas in the new text colour.
    const recolor = () => {
      buildAtlas();
      anim.redraw();
    };
    const mo = new MutationObserver(recolor);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "class"] });
    const mq = matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", recolor);
    canvas.addEventListener("pointermove", onMove);
    onCleanup(() => {
      anim.stop();
      ro.disconnect();
      mo.disconnect();
      mq.removeEventListener("change", recolor);
      canvas.removeEventListener("pointermove", onMove);
    });
  });

  return <canvas ref={canvas} class={`ascii-field ${props.fill ? "fill" : ""} ${props.class ?? ""}`} aria-hidden="true" />;
}
