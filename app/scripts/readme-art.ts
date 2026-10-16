// Renders the README's animated ASCII as SVG: the same scenes and pieces
// the app draws, sampled into frames that CSS steps through. GitHub shows
// SVG in <img> with its animations, and the colours follow the reader's
// light or dark theme.
//
//   npm run readme:art   (writes docs/*.svg at the repository root)

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { NOISE, clamp01, hash, noise, smooth } from "../src/ui/ascii/noise";
import { decode, flow, mark, thinking, wire } from "../src/ui/ascii/pieces";
import { SCENES, type Cell } from "../src/ui/ascii/scenes";

// Run from app/ (npm script): the art goes to the repository root.
const OUT = join(process.cwd(), "..", "docs");
const RAMP = " .·:-=+*#";
const FONT = "ui-monospace, SFMono-Regular, 'Cascadia Mono', Menlo, Consolas, monospace";
const CW = 7.2;
const LH = 14;

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Frames of rows -> an SVG whose frames are shown one after another, forever. */
function svg(frames: string[][], cols: number, fps: number, extra = "", extraCss = ""): string {
  const rows = frames[0].length;
  const w = Math.round(cols * CW), h = rows * LH + 8;
  const dur = frames.length / fps;
  const slice = (100 / frames.length).toFixed(4);
  const body = frames
    .map((f, i) => {
      const lines = f.map((r, y) => (r.trim() ? `<tspan x="0" y="${y * LH + 12}">${esc(r.replace(/\s+$/, ""))}</tspan>` : "")).join("");
      return `<text style="animation-delay:${(i / fps).toFixed(3)}s">${lines}</text>`;
    })
    .join("\n");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" role="img">
<style>
text{font-family:${FONT};font-size:12px;white-space:pre;fill:#1f2328;visibility:hidden;animation:f ${dur.toFixed(3)}s infinite}
.k{fill:#1f2328}.m{fill:#6e7781}.bg{fill:#ffffff}
@media (prefers-color-scheme:dark){text,.k{fill:#e6edf3}.m{fill:#7d8590}.bg{fill:#0d1117}}
@keyframes f{0%{visibility:visible}${slice}%{visibility:hidden}100%{visibility:hidden}}
@media (prefers-reduced-motion:reduce){text{animation:none}text:first-of-type{visibility:visible}}
${extraCss}
</style>
${body}
${extra}
</svg>
`;
}

/** The empty-chat field: a few scenes, each dissolving into the next. */
function field(cols: number, rows: number, picks: number[], hold: number, fade: number, fps: number, seed = 3) {
  const scenes = picks.map((i) => SCENES[i % SCENES.length]());
  const span = hold + fade;
  const total = span * scenes.length;
  const a: Cell = { v: 0, ch: null }, b: Cell = { v: 0, ch: null };
  const frames: string[][] = [];
  for (let n = 0; n < total * fps; n++) {
    // Play time starts later so motifs are already formed.
    const t = n / fps + 20;
    const local = n / fps;
    const k = Math.floor(local / span);
    const mix = smooth(hold / span, 1, local / span - k);
    const sa = scenes[k % scenes.length], sb = scenes[(k + 1) % scenes.length];
    const grid = { cols, rows, aspect: cols / rows / 2 };
    sa.prepare?.(t, grid);
    if (mix > 0) sb.prepare?.(t, grid);
    const tick = Math.floor(t * 9);
    const out: string[] = [];
    for (let y = 0; y < rows; y++) {
      let row = "";
      for (let x = 0; x < cols; x++) {
        const u = (x / cols - 0.5) * grid.aspect * 2;
        const v = y / rows - 0.5;
        const front = noise(u * 1.3 + k, v * 2.6, seed + 77) * 0.7 + hash(x, y, seed) * 0.3;
        const useB = mix > front;
        const cell = useB ? b : a;
        (useB ? sb : sa).cell(u, v, t, x, y, cell);
        const edge = mix > 0 && mix < 1 && Math.abs(mix - front) < 0.06;
        const tone = clamp01(Math.pow(cell.v, 0.9) + (hash(x, y, tick + seed) - 0.5) * 0.14);
        let g = " ";
        if (cell.v > 0.07 && hash(x, y, seed + 1) < 0.8 * (0.35 + tone * 0.75)) g = cell.ch ?? RAMP[Math.round(tone * (RAMP.length - 1))];
        if (edge && hash(x, y, Math.floor(t * 20)) < 0.9) g = NOISE[Math.floor(hash(y, x, Math.floor(t * 20)) * NOISE.length)];
        row += g;
      }
      out.push(row);
    }
    frames.push(out);
  }
  return frames;
}

/** A crew at work, as the sidebar and the chat bar draw it. */
function crew(fps: number, seconds: number) {
  const rows: [string, string, string, number][] = [
    ["working", "orchestrator", "working", 0],
    ["blocked", "worker-L1", "needs you", 1],
    ["working", "worker-L2", "working", 2],
    ["starting", "worker-L3", "starting", 3],
    ["done", "reviewer", "ready", 4],
  ];
  const frames: string[][] = [];
  for (let n = 0; n < seconds * fps; n++) {
    const t = n / fps;
    const lines = rows.map(([s, name, label, i]) => ` ${mark(s, i)(t)}  ${name.padEnd(14)}${label.padStart(10)}`);
    const track = `plan ${wire(3, "done")(t)} bar ${wire(3, "done")(t)} ${decode("work")(t % 3)} ${wire(3, "now")(t)} review ${wire(3, "todo")(t)} gate ${wire(3, "todo")(t)} ready`;
    lines[0] += `      ${track}`;
    lines[1] += `      worker-L1 ${flow(12, 99, 1.6)(t)} lot-1-L1`;
    lines[2] += `      ${thinking(18)(t)}  thinking`;
    frames.push(["", ...lines, ""]);
  }
  return frames;
}

mkdirSync(OUT, { recursive: true });
const FIELD_COLS = 92, FIELD_ROWS = 18;
const label = `<rect class="bg" x="${Math.round((FIELD_COLS / 2 - 6) * CW)}" y="${(FIELD_ROWS / 2 - 1) * LH - 2}" width="${Math.round(12 * CW)}" height="${LH * 2}"/>` +
  `<text class="k" x="${Math.round((FIELD_COLS / 2) * CW)}" y="${(FIELD_ROWS / 2) * LH + 2}" text-anchor="middle" style="visibility:visible;animation:none;font-size:14px;letter-spacing:.12em">harnes</text>`;
writeFileSync(join(OUT, "field.svg"), svg(field(FIELD_COLS, FIELD_ROWS, [0, 5, 16, 6], 3.2, 1.4, 8), FIELD_COLS, 8, label));
writeFileSync(join(OUT, "crew.svg"), svg(crew(10, 6), 112, 10));
console.log("wrote", OUT);
