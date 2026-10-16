// Small deterministic noise helpers for the ASCII pieces.

/** Integer hash of two ints to [0, 1). */
export function hash(x: number, y: number, seed = 0): number {
  let h = Math.imul(x | 0, 374761393) ^ Math.imul(y | 0, 668265263) ^ Math.imul(seed | 0, 1442695041);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

const fade = (f: number) => f * f * f * (f * (f * 6 - 15) + 10);

/** Smooth value noise in [0, 1). */
export function noise(x: number, y: number, seed = 0): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const u = fade(x - xi);
  const v = fade(y - yi);
  const a = hash(xi, yi, seed);
  const b = hash(xi + 1, yi, seed);
  const c = hash(xi, yi + 1, seed);
  const d = hash(xi + 1, yi + 1, seed);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}

/** Two octaves of noise, centred on 0 (roughly -0.5..0.5). */
export function fbm(x: number, y: number, seed = 0): number {
  return noise(x, y, seed) * 0.66 + noise(x * 2.1, y * 2.1, seed + 7) * 0.34 - 0.5;
}

export const clamp01 = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v);
export const smooth = (a: number, b: number, v: number) => {
  const t = clamp01((v - a) / (b - a));
  return t * t * (3 - 2 * t);
};

/** Density ramp, light to heavy. */
export const RAMP = " .:-=+*#%@";
/** Glyphs shown while a cell is scrambled or decoding. */
export const NOISE = "!<>-_/[]{}=+*^?#%@";
