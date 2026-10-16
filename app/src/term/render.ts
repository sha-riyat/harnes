// Canvas 2D renderer for Ghostty frames.

import { BOLD, CELL, FAINT, HEADER, INVISIBLE, ITALIC, STRIKE, UNDERLINE, header, rgb3 } from "./frame";
import { bgColor, cursorColor, defaultBg, fgColor } from "./theme";

export const FONT_SIZE = 13;
export const FONT_FAMILY = '"Cascadia Mono", "Cascadia Code", Consolas, "JetBrains Mono", monospace';

export interface Metrics {
  w: number;
  h: number;
  base: number;
}

let cached: Metrics | null = null;
export function metrics(): Metrics {
  if (cached) return cached;
  const c = document.createElement("canvas").getContext("2d")!;
  c.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
  const m = c.measureText("M");
  const w = Math.ceil(m.width * 100) / 100;
  const h = Math.ceil(FONT_SIZE * 1.3);
  cached = { w, h, base: Math.round(h * 0.78) };
  return cached;
}

export function draw(canvas: HTMLCanvasElement, v: DataView, focused: boolean) {
  const hd = header(v);
  const m = metrics();
  const dpr = window.devicePixelRatio || 1;
  const W = Math.ceil(hd.cols * m.w);
  const H = hd.rows * m.h;
  if (canvas.width !== Math.round(W * dpr) || canvas.height !== Math.round(H * dpr)) {
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    canvas.style.width = `${W}px`;
    canvas.style.height = `${H}px`;
  }
  const ctx = canvas.getContext("2d", { alpha: false })!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = defaultBg(hd.bg);
  ctx.fillRect(0, 0, W, H);

  for (let y = 0; y < hd.rows; y++) {
    const rowOff = HEADER + y * hd.cols * CELL;
    // Background runs.
    let runStart = 0;
    let runBg = rgb3(v, rowOff + 7);
    for (let x = 1; x <= hd.cols; x++) {
      const bg = x < hd.cols ? rgb3(v, rowOff + x * CELL + 7) : -1;
      if (bg !== runBg) {
        if (runBg !== hd.bg) {
          ctx.fillStyle = bgColor(runBg);
          ctx.fillRect(runStart * m.w, y * m.h, (x - runStart) * m.w, m.h);
        }
        runStart = x;
        runBg = bg;
      }
    }
    // Glyphs.
    let font = -1;
    for (let x = 0; x < hd.cols; x++) {
      const off = rowOff + x * CELL;
      const cp = v.getUint32(off, true);
      if (cp === 0 || cp === 32) continue;
      const flags = v.getUint16(off + 10, true);
      if (flags & INVISIBLE) continue;
      const f = flags & (BOLD | ITALIC);
      if (f !== font) {
        font = f;
        ctx.font = `${f & ITALIC ? "italic " : ""}${f & BOLD ? "bold " : ""}${FONT_SIZE}px ${FONT_FAMILY}`;
      }
      const fg = fgColor(rgb3(v, off + 4));
      ctx.fillStyle = fg;
      ctx.globalAlpha = flags & FAINT ? 0.6 : 1;
      const px = x * m.w;
      const py = y * m.h;
      ctx.fillText(String.fromCodePoint(cp), px, py + m.base);
      if (flags & (UNDERLINE | STRIKE)) {
        ctx.fillRect(px, flags & UNDERLINE ? py + m.h - 2 : py + m.h / 2, m.w, 1);
      }
    }
    ctx.globalAlpha = 1;
  }

  if (hd.cursorVisible) {
    const px = hd.cx * m.w;
    const py = hd.cy * m.h;
    ctx.fillStyle = cursorColor();
    ctx.strokeStyle = cursorColor();
    if (!focused || hd.cursorStyle === 3) ctx.strokeRect(px + 0.5, py + 0.5, m.w - 1, m.h - 1);
    else if (hd.cursorStyle === 1) ctx.fillRect(px, py, 2, m.h);
    else if (hd.cursorStyle === 2) ctx.fillRect(px, py + m.h - 2, m.w, 2);
    else {
      ctx.globalAlpha = 0.55;
      ctx.fillRect(px, py, m.w, m.h);
      ctx.globalAlpha = 1;
    }
  }
}
