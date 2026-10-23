// Terminal colours follow the app theme. Ghostty resolves every cell to RGB
// for a dark terminal; in light mode the renderer flips lightness: greys
// invert, coloured text darkens enough to read on paper, coloured
// backgrounds turn pastel. Hue is kept, so a red error stays red.

import { createSignal } from "solid-js";

const LIGHT_BG = "#fafafa";

const [version, setVersion] = createSignal(0);
/** Bumps when the theme changes, so terminals redraw. */
export const themeVersion = version;

let light = false;
const cache = new Map<string, string>();

function read() {
  const next = getComputedStyle(document.documentElement).colorScheme.includes("light") && !getComputedStyle(document.documentElement).colorScheme.includes("dark");
  if (next !== light) {
    light = next;
    cache.clear();
    setVersion((v) => v + 1);
  }
}

if (typeof document !== "undefined") {
  read();
  new MutationObserver(read).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-mode", "class"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", read);
}

const hex = (rgb: number) => "#" + rgb.toString(16).padStart(6, "0");

function hsl(rgb: number): [number, number, number] {
  const r = ((rgb >> 16) & 255) / 255, g = ((rgb >> 8) & 255) / 255, b = (rgb & 255) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h / 6, s, l];
}

function css([h, s, l]: [number, number, number]) {
  return `hsl(${Math.round(h * 360)} ${Math.round(s * 100)}% ${Math.round(l * 100)}%)`;
}

function map(rgb: number, kind: "fg" | "bg"): string {
  if (!light) return hex(rgb);
  const key = `${kind}${rgb}`;
  let out = cache.get(key);
  if (!out) {
    const [h, s, l] = hsl(rgb);
    const inv = 1 - l;
    const grey = s < 0.25;
    const L = grey ? Math.min(0.97, Math.max(0.05, inv)) : kind === "fg" ? Math.min(inv, 0.4) : Math.max(inv, 0.84);
    out = css([h, s, L]);
    cache.set(key, out);
  }
  return out;
}

/** The terminal's default background. */
export const defaultBg = (rgb: number) => (light ? LIGHT_BG : hex(rgb));
export const bgColor = (rgb: number) => map(rgb, "bg");
export const fgColor = (rgb: number) => map(rgb, "fg");
export const cursorColor = () => (light ? "#111111" : "#f2f2f2");
