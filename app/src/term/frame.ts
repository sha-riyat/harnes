// Decoder for frames produced by harnes-vt (`Frame::encode`).
// Header 16 bytes LE: cols u16, rows u16, cx u16, cy u16, visible u8, style u8, bg u32, pad u16.
// Then 14 bytes per cell: cp u32, fg BGR 3B, bg BGR 3B, flags u16, pad u16.

export const HEADER = 16;
export const CELL = 14;

export const BOLD = 1;
export const ITALIC = 2;
export const UNDERLINE = 4;
export const STRIKE = 8;
export const FAINT = 16;
export const INVISIBLE = 32;

export interface FrameHeader {
  cols: number;
  rows: number;
  cx: number;
  cy: number;
  cursorVisible: boolean;
  cursorStyle: number;
  bg: number;
}

export function header(v: DataView): FrameHeader {
  return {
    cols: v.getUint16(0, true),
    rows: v.getUint16(2, true),
    cx: v.getUint16(4, true),
    cy: v.getUint16(6, true),
    cursorVisible: v.getUint8(8) === 1,
    cursorStyle: v.getUint8(9),
    bg: v.getUint32(10, true) & 0xffffff,
  };
}

const css = new Map<number, string>();
/** 0xRRGGBB -> "#rrggbb", cached. */
export function color(rgb: number): string {
  let s = css.get(rgb);
  if (!s) {
    s = "#" + rgb.toString(16).padStart(6, "0");
    css.set(rgb, s);
  }
  return s;
}

/** Reads a 3-byte little-endian colour (stored as B, G, R). */
export function rgb3(v: DataView, off: number): number {
  return v.getUint8(off) | (v.getUint8(off + 1) << 8) | (v.getUint8(off + 2) << 16);
}
