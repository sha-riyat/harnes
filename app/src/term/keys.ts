// Keyboard events -> xterm byte sequences.

const ESC = "\x1b";

function mod(e: KeyboardEvent): number {
  return 1 + (e.shiftKey ? 1 : 0) + (e.altKey ? 2 : 0) + (e.ctrlKey ? 4 : 0);
}

function csi(final: string, e: KeyboardEvent, num = "1"): string {
  const m = mod(e);
  return m > 1 ? `${ESC}[${num};${m}${final}` : num === "1" ? `${ESC}[${final}` : `${ESC}[${num}${final}`;
}

function tilde(n: number, e: KeyboardEvent): string {
  const m = mod(e);
  return m > 1 ? `${ESC}[${n};${m}~` : `${ESC}[${n}~`;
}

const F: Record<string, string | number> = {
  F1: "P", F2: "Q", F3: "R", F4: "S",
  F5: 15, F6: 17, F7: 18, F8: 19, F9: 20, F10: 21, F11: 23, F12: 24,
};

/** Returns the bytes to send, or null to let the browser handle the key. */
export function encodeKey(e: KeyboardEvent): string | null {
  // Let the paste event handle Ctrl+V / Ctrl+Shift+V; copy stays native.
  if (e.ctrlKey && (e.key === "v" || e.key === "V")) return null;
  if (e.ctrlKey && e.shiftKey && (e.key === "c" || e.key === "C")) return null;

  switch (e.key) {
    case "Enter": return e.shiftKey || e.altKey ? `${ESC}\r` : "\r";
    case "Backspace": return e.ctrlKey ? "\x08" : e.altKey ? `${ESC}\x7f` : "\x7f";
    case "Tab": return e.shiftKey ? `${ESC}[Z` : "\t";
    case "Escape": return ESC;
    case "ArrowUp": return csi("A", e);
    case "ArrowDown": return csi("B", e);
    case "ArrowRight": return csi("C", e);
    case "ArrowLeft": return csi("D", e);
    case "Home": return csi("H", e);
    case "End": return csi("F", e);
    case "PageUp": return tilde(5, e);
    case "PageDown": return tilde(6, e);
    case "Delete": return tilde(3, e);
    case "Insert": return tilde(2, e);
  }
  const f = F[e.key];
  if (f !== undefined) return typeof f === "string" ? (mod(e) > 1 ? `${ESC}[1;${mod(e)}${f}` : `${ESC}O${f}`) : tilde(f, e);

  if (e.key.length === 1) {
    if (e.ctrlKey && !e.altKey) {
      const c = e.key.toLowerCase().charCodeAt(0);
      if (c >= 97 && c <= 122) return String.fromCharCode(c - 96);
      if (e.key === " " || e.key === "@") return "\x00";
      if (e.key === "[") return ESC;
      if (e.key === "\\") return "\x1c";
      if (e.key === "]") return "\x1d";
      return null;
    }
    return e.altKey ? ESC + e.key : e.key;
  }
  return null;
}

/** Wraps pasted text in bracketed-paste markers. */
export function paste(text: string): string {
  return `${ESC}[200~${text.replace(/\r?\n/g, "\r")}${ESC}[201~`;
}
