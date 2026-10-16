// Shared pieces for the field's scenes. A scene fills `out` for each cell:
// a brightness (0..1) and, when the motif needs it, a specific glyph.
// Coordinates: u runs across (about ±aspect), v runs down (-0.5..0.5),
// x/y are cell indices, t is play time in seconds. A scene may `prepare`
// once per frame (plotting into an Ink buffer) before cells are read.

export interface Cell {
  v: number;
  ch: string | null;
}

export interface Grid {
  cols: number;
  rows: number;
  aspect: number;
}

export interface Scene {
  prepare?(t: number, g: Grid): void;
  cell(u: number, v: number, t: number, x: number, y: number, out: Cell): void;
}

/** Scenes are made per field, so buffers are never shared. */
export type MakeScene = () => Scene;

export const TAU = Math.PI * 2;
export const DIR = "-\\|/";
export const dirGlyph = (a: number) => DIR[((Math.round(a / (Math.PI / 4)) % 4) + 4) % 4];

/** Every glyph a scene may ask for, so the atlas holds them all. */
export const EXTRA_GLYPHS = DIR + "|o@·+~^_xO*=:.";

/** A per-frame drawing surface: brightness and an optional glyph per cell. */
export class Ink {
  cols = 0;
  rows = 0;
  v = new Float32Array(0);
  ch: (string | null)[] = [];

  reset(g: Grid, decay = 0) {
    if (g.cols !== this.cols || g.rows !== this.rows) {
      this.cols = g.cols;
      this.rows = g.rows;
      this.v = new Float32Array(g.cols * g.rows);
      this.ch = new Array(g.cols * g.rows).fill(null);
      return;
    }
    if (decay > 0) {
      for (let i = 0; i < this.v.length; i++) {
        this.v[i] *= decay;
        if (this.v[i] < 0.08) this.ch[i] = null;
      }
    } else {
      this.v.fill(0);
      this.ch.fill(null);
    }
  }

  plot(x: number, y: number, v: number, ch: string | null = null) {
    const xi = Math.round(x), yi = Math.round(y);
    if (xi < 0 || yi < 0 || xi >= this.cols || yi >= this.rows) return;
    const i = yi * this.cols + xi;
    if (v >= this.v[i]) {
      this.v[i] = v;
      if (ch !== null) this.ch[i] = ch;
    }
  }

  /** A straight segment in cell space. */
  line(x0: number, y0: number, x1: number, y1: number, v: number, ch: string | null = null) {
    const n = Math.max(1, Math.ceil(Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0))));
    for (let k = 0; k <= n; k++) this.plot(x0 + ((x1 - x0) * k) / n, y0 + ((y1 - y0) * k) / n, v, ch);
  }

  read(x: number, y: number, out: Cell) {
    const i = y * this.cols + x;
    out.v = this.v[i] ?? 0;
    out.ch = this.ch[i] ?? null;
  }
}

/** A scene that only plots: `draw` fills the ink each frame. */
export function inked(draw: (ink: Ink, t: number, g: Grid) => void, decay = 0): MakeScene {
  return () => {
    const ink = new Ink();
    return {
      prepare(t, g) {
        ink.reset(g, decay);
        draw(ink, t, g);
      },
      cell(_u, _v, _t, x, y, out) {
        ink.read(x, y, out);
      },
    };
  };
}

/** A scene from a plain per-cell function. */
export const plain = (cell: Scene["cell"]): MakeScene => () => ({ cell });
